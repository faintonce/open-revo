//! Software RPM transitions. Timers use unfiltered control-loop time, not sensor averages.
use super::fan_policy::{MAX_RPM, MIN_RPM};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RpmSmoothing {
    pub enabled: bool,
    pub rise_rpm_per_sec: u16,
    pub fall_rpm_per_sec: u16,
    pub fall_delay_secs: u8,
}
impl Default for RpmSmoothing {
    fn default() -> Self {
        Self {
            enabled: true,
            rise_rpm_per_sec: 1500,
            fall_rpm_per_sec: 300,
            fall_delay_secs: 3,
        }
    }
}
impl RpmSmoothing {
    pub fn validate(&self) -> Result<(), String> {
        if !(100..=5000).contains(&self.rise_rpm_per_sec)
            || self.rise_rpm_per_sec % 100 != 0
            || !(100..=2000).contains(&self.fall_rpm_per_sec)
            || self.fall_rpm_per_sec % 100 != 0
            || self.fall_delay_secs > 10
        {
            return Err("软件平滑：升速需为 100–5000 RPM/秒，降速为 100–2000 RPM/秒（步进 100），降速等待为 0–10 秒".into());
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct RpmTransition {
    current: Option<[u16; 2]>,
    time: Duration,
    credit: [u128; 2],
    direction: [i8; 2],
    fall_since: [Option<Duration>; 2],
}
impl RpmTransition {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn needs_seed(&self) -> bool {
        self.current.is_none()
    }
    pub fn configure(&mut self) {
        self.credit = [0; 2];
        self.direction = [0; 2];
        self.fall_since = [None; 2];
    }
    pub fn seed(&mut self, actual: [u16; 2], now: Duration) {
        self.current = Some(std::array::from_fn(|fan| {
            (((actual[fan] as u32 + 50) / 100) * 100).clamp(MIN_RPM[fan] as u32, MAX_RPM as u32)
                as u16
        }));
        self.time = now;
        self.configure();
    }
    /// Native start assistance may override our command; continue from what was sent.
    pub fn commit(&mut self, sent: [u16; 2]) {
        if let Some(previous) = self.current {
            for fan in 0..2 {
                if previous[fan] != sent[fan] {
                    self.credit[fan] = 0;
                    self.direction[fan] = 0;
                    self.fall_since[fan] = None;
                }
            }
        }
        self.current = Some(sent);
    }
    pub fn plan(
        &mut self,
        target: [u16; 2],
        settings: RpmSmoothing,
        now: Duration,
        hot: bool,
    ) -> [u16; 2] {
        let previous_time = self.time;
        let elapsed = now.saturating_sub(previous_time);
        self.time = now;
        let mut current = self.current.expect("RPM transition seeded before planning");
        for fan in 0..2 {
            let direction = match target[fan].cmp(&current[fan]) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => -1,
                _ => 0,
            };
            if !settings.enabled || hot || direction == 0 {
                current[fan] = if hot {
                    current[fan].max(target[fan])
                } else {
                    target[fan]
                };
                self.credit[fan] = 0;
                self.direction[fan] = 0;
                self.fall_since[fan] = None;
                continue;
            }
            if direction != self.direction[fan] {
                self.credit[fan] = 0;
                self.fall_since[fan] = if direction < 0 { Some(now) } else { None };
                self.direction[fan] = direction;
            }
            let (rate, allowed) = if direction > 0 {
                (settings.rise_rpm_per_sec, elapsed)
            } else {
                let ready = self.fall_since[fan].unwrap()
                    + Duration::from_secs(settings.fall_delay_secs as u64);
                (
                    settings.fall_rpm_per_sec,
                    now.saturating_sub(previous_time.max(ready)),
                )
            };
            // Keep sub-100-RPM credit across ticks; idle time and reversals cannot bank speed.
            let unit = 100 * 1_000_000_000u128;
            self.credit[fan] += allowed.as_nanos() * rate as u128;
            let steps = self.credit[fan] / unit;
            self.credit[fan] %= unit;
            let difference = current[fan].abs_diff(target[fan]) as u128;
            let change = (steps * 100).min(difference) as u16;
            current[fan] = if direction > 0 {
                current[fan] + change
            } else {
                current[fan] - change
            };
            if current[fan] == target[fan] {
                self.credit[fan] = 0;
            }
        }
        self.current = Some(current);
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }
    #[test]
    fn channels_rise_independently_and_fall_only_after_delay() {
        let mut ramp = RpmTransition::default();
        ramp.seed([2000, 4000], at(0));
        let settings = RpmSmoothing::default();
        assert_eq!(
            ramp.plan([5000, 2000], settings, at(0), false),
            [2000, 4000]
        );
        assert_eq!(
            ramp.plan([5000, 2000], settings, at(1), false),
            [3500, 4000]
        );
        assert_eq!(
            ramp.plan([5000, 2000], settings, at(2), false),
            [5000, 4000]
        );
        assert_eq!(
            ramp.plan([5000, 2000], settings, at(3), false),
            [5000, 4000]
        );
        assert_eq!(
            ramp.plan([5000, 2000], settings, at(4), false),
            [5000, 3700]
        );
    }
    #[test]
    fn small_elapsed_intervals_accumulate_without_overshoot() {
        let mut ramp = RpmTransition::default();
        ramp.seed([2000, 2000], at(0));
        let settings = RpmSmoothing {
            rise_rpm_per_sec: 100,
            ..RpmSmoothing::default()
        };
        for ms in [100, 300, 600, 999] {
            assert_eq!(
                ramp.plan([2100, 2100], settings, Duration::from_millis(ms), false),
                [2000, 2000]
            );
        }
        assert_eq!(
            ramp.plan([2100, 2100], settings, at(1), false),
            [2100, 2100]
        );
        ramp.plan([2100, 2100], settings, at(100), false);
        assert_eq!(
            ramp.plan([3000, 3000], settings, at(101), false),
            [2200, 2200]
        );
    }
    #[test]
    fn rising_temperature_cancels_downward_wait_and_cooling_starts_a_new_wait() {
        let mut ramp = RpmTransition::default();
        ramp.seed([3000, 3000], at(0));
        let settings = RpmSmoothing::default();
        ramp.plan([2000, 2000], settings, at(0), false);
        assert_eq!(
            ramp.plan([4000, 4000], settings, at(1), false),
            [4000, 4000]
        );
        ramp.plan([2000, 2000], settings, at(2), false);
        assert_eq!(
            ramp.plan([2000, 2000], settings, at(5), false),
            [4000, 4000]
        );
        assert_eq!(
            ramp.plan([2000, 2000], settings, at(6), false),
            [3700, 3700]
        );
    }
    #[test]
    fn high_temperature_bypasses_rise_and_holds_falls() {
        let mut ramp = RpmTransition::default();
        ramp.seed([2000, 5000], at(0));
        let settings = RpmSmoothing::default();
        assert_eq!(ramp.plan([7000, 3000], settings, at(0), true), [7000, 5000]);
        ramp.plan([3000, 3000], settings, at(1), false);
        assert_eq!(
            ramp.plan([3000, 3000], settings, at(4), false),
            [7000, 5000]
        );
        assert_eq!(
            ramp.plan([3000, 3000], settings, at(5), false),
            [6700, 4700]
        );
    }
    #[test]
    fn disabling_is_immediate_and_start_assistance_becomes_the_new_baseline() {
        let mut ramp = RpmTransition::default();
        ramp.seed([0, 0], at(0));
        let settings = RpmSmoothing::default();
        assert_eq!(
            ramp.plan([1200, 1000], settings, at(0), false),
            [1200, 1000]
        );
        ramp.commit([2000, 2000]);
        assert_eq!(
            ramp.plan([1200, 1000], settings, at(1), false),
            [2000, 2000]
        );
        assert_eq!(
            ramp.plan(
                [1200, 1000],
                RpmSmoothing {
                    enabled: false,
                    ..settings
                },
                at(1),
                false
            ),
            [1200, 1000]
        );
        ramp.clear();
        assert!(ramp.needs_seed());
    }
    #[test]
    fn invalid_options_are_rejected_and_missing_fields_use_defaults() {
        assert!(RpmSmoothing {
            fall_delay_secs: 11,
            ..RpmSmoothing::default()
        }
        .validate()
        .is_err());
        assert!(RpmSmoothing {
            rise_rpm_per_sec: 50,
            ..RpmSmoothing::default()
        }
        .validate()
        .is_err());
        assert!(RpmSmoothing {
            fall_rpm_per_sec: 2500,
            ..RpmSmoothing::default()
        }
        .validate()
        .is_err());
        let loaded: RpmSmoothing = serde_json::from_str("{}").unwrap();
        assert_eq!(loaded, RpmSmoothing::default());
    }
}
