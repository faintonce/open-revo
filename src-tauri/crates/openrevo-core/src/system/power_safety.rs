//! Short-lived statistics and advisory checks for manual CPU power changes.
//! No hardware access, persistent storage, or independent polling thread.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const HISTORY_MS: u64 = 120_000;
const FRESH_MS: u64 = 5_000;
const RECENT_MS: u64 = 15_000;
const STABLE_MS: u64 = 30_000;

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerSource {
    Measured,
    Estimated,
    #[default]
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct CpuSample {
    pub at_ms: u64,
    pub load: Option<f64>,
    pub temperature: Option<f64>,
    pub power: Option<f64>,
    pub source: PowerSource,
    pub fan_rpm: Option<f64>,
    pub gpu_power: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PowerSnapshot {
    pub sample_count: usize,
    pub span_ms: u64,
    pub sample_age_ms: Option<u64>,
    pub load_pct: Option<f64>,
    pub power_p95_w: Option<f64>,
    pub power_source: PowerSource,
    pub temperature_c: Option<f64>,
    pub temperature_age_ms: Option<u64>,
    pub temperature_slope_c_s: Option<f64>,
    pub thermal_gain_c_w: Option<f64>,
    pub gpu_rise_w: Option<f64>,
    pub fan_rpm: Option<f64>,
    pub ready: bool,
}

#[derive(Default)]
pub struct PowerHistory {
    samples: VecDeque<CpuSample>,
}

fn valid(value: Option<f64>, min: f64, max: f64) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v >= min && *v <= max)
}

fn quantile(mut values: Vec<f64>, fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[((values.len() - 1) as f64 * fraction).ceil() as usize])
}

impl PowerHistory {
    pub fn record(&mut self, mut sample: CpuSample) {
        if self
            .samples
            .back()
            .is_some_and(|last| sample.at_ms <= last.at_ms || sample.at_ms - last.at_ms > FRESH_MS)
        {
            self.samples.clear();
        }
        sample.load = valid(sample.load, 0.0, 100.0);
        sample.temperature = valid(sample.temperature, 1.0, 125.0);
        sample.power = valid(sample.power, 0.1, 500.0);
        if sample.power.is_none() {
            sample.source = PowerSource::Unavailable;
        }
        sample.fan_rpm = valid(sample.fan_rpm, 1.0, 20_000.0);
        sample.gpu_power = valid(sample.gpu_power, 0.0, 600.0);
        while self
            .samples
            .front()
            .is_some_and(|s| sample.at_ms.saturating_sub(s.at_ms) > HISTORY_MS)
        {
            self.samples.pop_front();
        }
        if self.samples.len() >= 128 {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    // Enrich the CPU sample with data already collected by the shared heartbeat.
    pub fn enrich(&mut self, now: u64, fan_rpm: f64, gpu_power: Option<f64>) {
        if let Some(last) = self
            .samples
            .back_mut()
            .filter(|s| now.saturating_sub(s.at_ms) <= 2_000)
        {
            last.fan_rpm = valid(Some(fan_rpm), 1.0, 20_000.0);
            last.gpu_power = valid(gpu_power, 0.0, 600.0);
        }
    }

    pub fn snapshot(&self, now: u64) -> PowerSnapshot {
        let recent: Vec<_> = self
            .samples
            .iter()
            .filter(|s| s.at_ms <= now && now - s.at_ms <= RECENT_MS)
            .collect();
        let Some(last) = recent.last() else {
            return PowerSnapshot::default();
        };
        let age = now.saturating_sub(last.at_ms);
        let complete: Vec<_> = recent
            .iter()
            .copied()
            .filter(|s| s.load.is_some() && s.power.is_some())
            .collect();
        let measured: Vec<_> = complete
            .iter()
            .copied()
            .filter(|s| s.source == PowerSource::Measured)
            .collect();
        // Prefer a recent measured series; never label estimated samples as measured.
        let measured_recent = measured
            .last()
            .is_some_and(|s| now.saturating_sub(s.at_ms) <= FRESH_MS);
        let use_measured =
            measured.len() >= 4 && measured_recent && last.source == PowerSource::Measured;
        let powers: Vec<f64> = if use_measured {
            measured.iter().filter_map(|s| s.power).collect()
        } else {
            complete.iter().filter_map(|s| s.power).collect()
        };
        let temperatures: Vec<_> = recent
            .iter()
            .copied()
            .filter(|s| s.temperature.is_some())
            .collect();
        let temp_last = temperatures.last();
        let temp_age = temp_last.map(|s| now.saturating_sub(s.at_ms));
        let temperature = temp_last
            .and_then(|s| s.temperature)
            .filter(|_| temp_age.is_some_and(|a| a <= FRESH_MS));
        let slope = temperatures
            .first()
            .zip(temp_last)
            .and_then(|(first, last)| {
                let seconds = last.at_ms.saturating_sub(first.at_ms) as f64 / 1000.0;
                (seconds >= 6.0)
                    .then(|| (last.temperature.unwrap() - first.temperature.unwrap()) / seconds)
            });
        let span = complete
            .first()
            .zip(complete.last())
            .map_or(0, |(a, b)| b.at_ms - a.at_ms);
        let load = (!complete.is_empty())
            .then(|| complete.iter().filter_map(|s| s.load).sum::<f64>() / complete.len() as f64);

        // Learn only from measured, comparable cooling/GPU conditions with a usable
        // power excursion. Poor fits retain the conservative prediction prior.
        let comparable: Vec<_> = self
            .samples
            .iter()
            .filter(|s| {
                s.at_ms <= now
                    && now - s.at_ms <= HISTORY_MS
                    && s.source == PowerSource::Measured
                    && s.power.is_some()
                    && s.temperature.is_some()
                    && s.load.is_some()
                    && last
                        .load
                        .zip(s.load)
                        .is_some_and(|(a, b)| (a - b).abs() <= 10.0)
                    && last
                        .fan_rpm
                        .zip(s.fan_rpm)
                        .is_some_and(|(a, b)| (a - b).abs() <= a * 0.15)
                    && last
                        .gpu_power
                        .zip(s.gpu_power)
                        .is_some_and(|(a, b)| (a - b).abs() <= 5.0)
            })
            .collect();
        let gain = if comparable.len() >= 8
            && comparable.last().unwrap().at_ms - comparable[0].at_ms >= STABLE_MS
        {
            let n = comparable.len() as f64;
            let mean_p = comparable.iter().map(|s| s.power.unwrap()).sum::<f64>() / n;
            let mean_t = comparable
                .iter()
                .map(|s| s.temperature.unwrap())
                .sum::<f64>()
                / n;
            let pp = comparable
                .iter()
                .map(|s| (s.power.unwrap() - mean_p).powi(2))
                .sum::<f64>();
            let tt = comparable
                .iter()
                .map(|s| (s.temperature.unwrap() - mean_t).powi(2))
                .sum::<f64>();
            let pt = comparable
                .iter()
                .map(|s| (s.power.unwrap() - mean_p) * (s.temperature.unwrap() - mean_t))
                .sum::<f64>();
            let low = comparable
                .iter()
                .map(|s| s.power.unwrap())
                .fold(f64::INFINITY, f64::min);
            let high = comparable
                .iter()
                .map(|s| s.power.unwrap())
                .fold(0.0, f64::max);
            (high - low >= 10.0 && pp > 0.0 && tt > 0.0 && pt > 0.0 && pt * pt / (pp * tt) >= 0.6)
                .then(|| (pt / pp).clamp(0.15, 0.8))
        } else {
            None
        };
        let gpu_rise = recent
            .first()
            .and_then(|s| s.gpu_power)
            .zip(last.gpu_power)
            .map(|(a, b)| (b - a).max(0.0));
        PowerSnapshot {
            sample_count: complete.len(),
            span_ms: span,
            sample_age_ms: Some(age),
            load_pct: load.filter(|_| age <= FRESH_MS),
            power_p95_w: quantile(powers, 0.95).filter(|_| age <= FRESH_MS),
            power_source: if age > FRESH_MS || last.power.is_none() {
                PowerSource::Unavailable
            } else if use_measured || complete.iter().all(|s| s.source == PowerSource::Measured) {
                PowerSource::Measured
            } else {
                PowerSource::Estimated
            },
            temperature_c: temperature,
            temperature_age_ms: temp_age,
            temperature_slope_c_s: slope.filter(|_| temperature.is_some()),
            thermal_gain_c_w: gain,
            gpu_rise_w: gpu_rise,
            fan_rpm: last.fan_rpm,
            ready: age <= FRESH_MS
                && complete.len() >= 6
                && span >= 7_500
                && last.load.is_some()
                && last.power.is_some(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CpuPowerLimits {
    pub pl1: u16,
    pub pl2: u16,
    pub pl4: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PowerChangeRequest {
    #[serde(default)]
    pub confirmed: bool,
    pub context: String,
    pub current: CpuPowerLimits,
    pub proposed: CpuPowerLimits,
    pub factory: CpuPowerLimits,
    pub factory_verified: bool,
    pub office_pl1: u16,
    pub minimum_pl1: u16,
    pub margin: u16,
    pub temperature_limit_c: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PowerAssessment {
    pub level: &'static str,
    pub requires_confirmation: bool,
    pub reasons: Vec<String>,
    pub predicted_temperature_c: Option<f64>,
    pub snapshot: PowerSnapshot,
}

#[derive(Default)]
pub struct PowerSafetySession {
    context: String,
    baseline: Option<CpuPowerLimits>,
    current: Option<CpuPowerLimits>,
    stable_since_ms: u64,
}

impl PowerSafetySession {
    pub fn observe(
        &mut self,
        context: &str,
        current: CpuPowerLimits,
        snapshot: &PowerSnapshot,
        temperature_limit: f64,
        now: u64,
    ) {
        if self.context != context || self.baseline.is_none() || now < self.stable_since_ms {
            self.context = context.to_string();
            self.baseline = Some(current);
            self.current = Some(current);
            self.stable_since_ms = now;
        } else if self.current != Some(current) {
            self.current = Some(current);
            self.stable_since_ms = now;
        } else if now - self.stable_since_ms >= STABLE_MS
            && snapshot.ready
            && snapshot
                .temperature_c
                .is_some_and(|t| t < temperature_limit - 8.0)
            && snapshot
                .temperature_slope_c_s
                .is_some_and(|s| s.abs() < 0.2)
        {
            self.baseline = Some(current);
        }
    }

    pub fn assess(
        &mut self,
        request: &PowerChangeRequest,
        snapshot: PowerSnapshot,
        now: u64,
    ) -> Result<PowerAssessment, String> {
        let next = request.proposed;
        if next.pl1 < request.minimum_pl1
            || next.pl1 > next.pl2
            || u32::from(next.pl2) + u32::from(request.margin) > u32::from(next.pl4)
            || !request.temperature_limit_c.is_finite()
            || !(50.0..=125.0).contains(&request.temperature_limit_c)
        {
            return Err("功率或温度范围无效，请检查联动设置。".to_string());
        }
        // Remember an explicitly accepted risk band, without changing hardware or
        // rewinding the current limits when an asynchronous acknowledgement arrives.
        if request.confirmed {
            if self.context != request.context {
                return Err("功率方案已变化，请重新核对。".to_string());
            }
            self.baseline = Some(next);
            self.stable_since_ms = now;
            return Ok(PowerAssessment {
                level: "low",
                requires_confirmation: false,
                reasons: vec![],
                predicted_temperature_c: None,
                snapshot,
            });
        }
        self.observe(
            &request.context,
            request.current,
            &snapshot,
            request.temperature_limit_c,
            now,
        );
        let baseline = self.baseline.unwrap_or(request.current);
        let mut high = Vec::new();
        let mut notes = Vec::new();
        if next == request.current {
            return Ok(PowerAssessment {
                level: "low",
                requires_confirmation: false,
                reasons: vec![],
                predicted_temperature_c: None,
                snapshot,
            });
        }
        // A device-relative floor protects responsiveness even when the machine is idle.
        let floor = request
            .minimum_pl1
            .max(((request.office_pl1 as f64 * 0.5).ceil() as u16).min(25));
        if next.pl1 < request.current.pl1 && next.pl1 < floor {
            high.push(format!(
                "持续功率将降至 {}W，低于本机交互保护参考值 {}W，可能造成明显卡顿。",
                next.pl1, floor
            ));
        }
        let limits = [
            ("持续功率", next.pl1, baseline.pl1, request.factory.pl1),
            ("短时功率", next.pl2, baseline.pl2, request.factory.pl2),
            ("峰值功率", next.pl4, baseline.pl4, request.factory.pl4),
        ];
        for (name, target, base, factory) in limits {
            if request.factory_verified
                && factory > 0
                && target as f64 > factory as f64 * 1.1
                && target > base.saturating_add(5)
            {
                high.push(format!(
                    "{}将提高到 {}W，超过原厂参考 {}W，尚无此档位的稳定运行记录。",
                    name, target, factory
                ));
            }
        }
        if !request.factory_verified
            && next.pl1 as f64 > baseline.pl1 as f64 * 1.5
            && next.pl1 > baseline.pl1.saturating_add(25)
        {
            high.push(
                "缺少可验证的原厂功率参考，本次持续功率增幅较大，请确认散热与供电条件。"
                    .to_string(),
            );
        }
        let base_budget = baseline.pl1.max(1) as f64;
        let reduction = ((base_budget - next.pl1 as f64) / base_budget).max(0.0);
        let load = snapshot.load_pct.unwrap_or(0.0);
        let demand = snapshot.power_p95_w.map(|p| p * 1.15);
        if snapshot.ready {
            let measured = snapshot.power_source == PowerSource::Measured;
            let loss_threshold = if measured { 0.25 } else { 0.35 };
            let load_threshold = if measured { 70.0 } else { 80.0 };
            if load >= load_threshold
                && reduction >= loss_threshold
                && next.pl1 < request.current.pl1
                && demand.is_some_and(|p| (next.pl1 as f64) < p * 0.85)
            {
                let source = if measured {
                    "近期实测功耗"
                } else {
                    "近期负载估算功耗"
                };
                high.push(format!("近期 CPU 负载约 {:.0}%，持续功率较稳定档位降低 {:.0}%，可能无法覆盖当前任务需求（依据{}）。", load, reduction * 100.0, source));
            } else if reduction >= 0.25 && next.pl1 < request.current.pl1 {
                notes.push("持续功率下降较多，请留意任务速度与操作响应。".to_string());
            }
        } else if reduction >= 0.5
            && baseline.pl1.saturating_sub(next.pl1) >= 20
            && next.pl1 < request.current.pl1
        {
            high.push(
                "近期有效采样不足，本次持续功率下降超过一半，无法确认当前任务仍可流畅运行。"
                    .to_string(),
            );
        }
        let increased = (next.pl1 as f64 - request.current.pl1 as f64).max(0.0)
            + (next.pl2 as f64 - request.current.pl2 as f64).max(0.0) * 0.25
            + (next.pl4 as f64 - request.current.pl4 as f64).max(0.0) * 0.05;
        let cumulative_increase = (next.pl1 as f64 - baseline.pl1 as f64).max(0.0)
            + (next.pl2 as f64 - baseline.pl2 as f64).max(0.0) * 0.25
            + (next.pl4 as f64 - baseline.pl4 as f64).max(0.0) * 0.05;
        let predicted = if increased > 0.0 && snapshot.ready && load >= 60.0 {
            snapshot.temperature_c.map(|temperature| {
                let demand = demand
                    .unwrap_or(request.current.pl1 as f64)
                    .max(request.current.pl1 as f64 * 1.2);
                let extra = increased.min((demand - request.current.pl1 as f64).max(0.0));
                let rise = snapshot.temperature_slope_c_s.unwrap_or(0.0).max(0.0) * 5.0;
                let gpu_penalty = snapshot.gpu_rise_w.unwrap_or(0.0).min(30.0) * 0.05;
                temperature
                    + extra.max(0.0) * snapshot.thermal_gain_c_w.unwrap_or(0.35)
                    + rise
                    + gpu_penalty
            })
        } else {
            None
        };
        if increased.max(cumulative_increase) >= 5.0
            && predicted.is_some_and(|t| t >= request.temperature_limit_c - 3.0)
        {
            high.push(format!(
                "当前负载和升温趋势下，预计温度约 {:.0}°C，接近设定温度边界 {:.0}°C。",
                predicted.unwrap(),
                request.temperature_limit_c
            ));
        }
        if !snapshot.ready || snapshot.power_source != PowerSource::Measured {
            notes.push(
                if snapshot.ready {
                    "当前功耗含估算数据，风险判断采用较低置信度。"
                } else {
                    "近期数据不足或已过期，暂使用原厂范围和变化幅度保护。"
                }
                .to_string(),
            );
        }
        if snapshot.ready && snapshot.temperature_c.is_none() {
            notes.push("近期温度数据不可用，本次仅采用功率范围和负载保护。".to_string());
        }
        let requires_confirmation = !high.is_empty();
        let level = if requires_confirmation {
            "high"
        } else if notes.is_empty() {
            "low"
        } else {
            "moderate"
        };
        high.extend(notes);
        Ok(PowerAssessment {
            level,
            requires_confirmation,
            reasons: high,
            predicted_temperature_c: predicted,
            snapshot,
        })
    }
}

#[cfg(test)]
mod tests;
