//! Temperature-to-RPM policy and temporary firmware handover, independent of port I/O.
pub use super::fan_smoothing::RpmSmoothing;
use super::fan_smoothing::RpmTransition;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MIN_RPM: [u16; 2] = [1200, 1000];
pub const MAX_RPM: u16 = 7500;
pub const STARTUP_RPM: u16 = 2000;
const STARTUP_TIME: Duration = Duration::from_secs(2);
const STALL_TIMEOUT: Duration = Duration::from_secs(3);
const RESTART_WINDOW: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FanRpmLimits {
    pub minimum: [u16; 2],
    pub maximum: u16,
    pub step: u16,
    pub startup_rpm: u16,
    pub startup_secs: u64,
}
impl Default for FanRpmLimits {
    fn default() -> Self {
        Self {
            minimum: MIN_RPM,
            maximum: MAX_RPM,
            step: 100,
            startup_rpm: STARTUP_RPM,
            startup_secs: STARTUP_TIME.as_secs(),
        }
    }
}
pub(crate) fn validate_targets(targets: [u16; 2]) -> Result<(), String> {
    for (fan, target) in targets.into_iter().enumerate() {
        if !(MIN_RPM[fan]..=MAX_RPM).contains(&target) {
            return Err(format!(
                "风扇 {} 目标转速范围为 {}–{} RPM",
                fan + 1,
                MIN_RPM[fan],
                MAX_RPM
            ));
        }
    }
    Ok(())
}

/// Non-blocking start assistance and bounded stall recovery; called under the EC mutex.
#[derive(Default)]
pub(crate) struct FanRpmGuard {
    active: bool,
    boost_until: [Option<Duration>; 2],
    zero_since: [Option<Duration>; 2],
    restarts: [std::collections::VecDeque<Duration>; 2],
}
impl FanRpmGuard {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn apply(
        &mut self,
        targets: [u16; 2],
        actual: [u16; 2],
        now: Duration,
    ) -> Result<[u16; 2], String> {
        validate_targets(targets)?;
        let mut applied = targets;
        for fan in 0..2 {
            while self.restarts[fan]
                .front()
                .is_some_and(|time| now.saturating_sub(*time) >= RESTART_WINDOW)
            {
                self.restarts[fan].pop_front();
            }
            if !self.active && targets[fan] < STARTUP_RPM {
                self.boost_until[fan] = Some(now + STARTUP_TIME);
            }
            if actual[fan] == 0 {
                let since = *self.zero_since[fan].get_or_insert(now);
                if now.saturating_sub(since) >= STALL_TIMEOUT {
                    return Err(format!("风扇 {} 持续停转，恢复原厂温控", fan + 1));
                }
                if self.active && self.boost_until[fan].is_none() && targets[fan] < STARTUP_RPM {
                    if self.restarts[fan].len() >= 2 {
                        return Err(format!("风扇 {} 反复停转，恢复原厂温控", fan + 1));
                    }
                    self.restarts[fan].push_back(now);
                    self.boost_until[fan] = Some(now + STARTUP_TIME);
                }
            } else {
                self.zero_since[fan] = None;
            }
            if let Some(until) = self.boost_until[fan] {
                if now < until {
                    applied[fan] = targets[fan].max(STARTUP_RPM);
                } else {
                    if actual[fan] == 0 {
                        return Err(format!("风扇 {} 起转失败，恢复原厂温控", fan + 1));
                    }
                    self.boost_until[fan] = None;
                }
            }
        }
        self.active = true;
        Ok(applied)
    }
}
pub const PROTECT_CELSIUS: u8 = 90;
pub const RECOVER_CELSIUS: u8 = 85;
pub const RECOVERY_TIME: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpmCurvePoint {
    pub temp: u8,
    pub rpm: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpmFanCurves {
    #[serde(default)]
    pub smoothing: RpmSmoothing,
    pub fan0: Vec<RpmCurvePoint>,
    pub fan1: Vec<RpmCurvePoint>,
    #[serde(default)]
    pub linked: bool,
}
impl Default for RpmFanCurves {
    fn default() -> Self {
        let fan0 = [
            (40, 2200),
            (50, 2200),
            (60, 3200),
            (68, 4000),
            (75, 5000),
            (82, 6000),
            (88, 7000),
            (95, 7500),
        ]
        .into_iter()
        .map(|(temp, rpm)| RpmCurvePoint { temp, rpm })
        .collect::<Vec<_>>();
        Self {
            smoothing: RpmSmoothing::default(),
            fan1: fan0.clone(),
            fan0,
            linked: true,
        }
    }
}
impl RpmFanCurves {
    pub fn validate(&self) -> Result<(), String> {
        self.smoothing.validate()?;
        for (fan, curve) in [&self.fan0, &self.fan1].into_iter().enumerate() {
            if !(2..=32).contains(&curve.len())
                || curve.iter().any(|p| {
                    p.temp > 100 || !(MIN_RPM[fan]..=MAX_RPM).contains(&p.rpm) || p.rpm % 100 != 0
                })
                || curve
                    .windows(2)
                    .any(|p| p[0].temp >= p[1].temp || p[0].rpm > p[1].rpm)
            {
                return Err(format!(
                    "风扇 {} 曲线需有 2–32 个递增温度节点，转速为 {}–{} RPM，按 100 RPM 递增或保持",
                    fan + 1,
                    MIN_RPM[fan],
                    MAX_RPM
                ));
            }
        }
        if self.linked && self.fan0 != self.fan1 {
            return Err("联动模式下两只风扇的曲线必须一致".into());
        }
        Ok(())
    }
    /// Preserve legacy temperature knots and relative intensity, not physical PWM-to-RPM calibration.
    pub fn from_legacy(fan0: &[(u8, u8)], fan1: &[(u8, u8)]) -> Result<Self, String> {
        let convert = |curve: &[(u8, u8)]| -> Result<Vec<RpmCurvePoint>, String> {
            if !(2..=32).contains(&curve.len())
                || curve.iter().any(|&(t, d)| t > 100 || d > 100)
                || curve
                    .windows(2)
                    .any(|p| p[0].0 >= p[1].0 || p[0].1 > p[1].1)
            {
                return Err("旧百分比曲线节点无效".into());
            }
            Ok(curve
                .iter()
                .map(|&(temp, duty)| {
                    let rpm = 2200 + ((duty.max(25) as u32 - 25) * 5300 + 37) / 75;
                    RpmCurvePoint {
                        temp,
                        rpm: (((rpm + 50) / 100) * 100).min(MAX_RPM as u32) as u16,
                    }
                })
                .collect())
        };
        let fan0 = convert(fan0)?;
        let fan1 = convert(fan1)?;
        let result = Self {
            smoothing: RpmSmoothing::default(),
            linked: fan0 == fan1,
            fan0,
            fan1,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn targets(&self, temp: u8) -> [u16; 2] {
        fn interpolate(curve: &[RpmCurvePoint], temp: u8) -> u16 {
            for pair in curve.windows(2) {
                let (a, b) = (&pair[0], &pair[1]);
                if temp <= a.temp {
                    return a.rpm;
                }
                if temp < b.temp {
                    let rpm = a.rpm as u32
                        + (temp - a.temp) as u32 * (b.rpm - a.rpm) as u32
                            / (b.temp - a.temp) as u32;
                    return (((rpm + 50) / 100) * 100) as u16;
                }
            }
            curve.last().expect("validated RPM curve").rpm
        }
        [interpolate(&self.fan0, temp), interpolate(&self.fan1, temp)]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FanControlStatus {
    pub smoothing: RpmSmoothing,
    pub smoothing_active: bool,
    pub transition_rpm: [u16; 2],
    pub rpm_limits: FanRpmLimits,
    pub mode: String,
    pub requested_mode: String,
    pub target_rpm: [u16; 2],
    pub applied_rpm: [u16; 2],
    pub cpu_temperature: Option<u8>,
    pub resume_pending: bool,
    pub recovery_remaining_secs: u64,
    pub error: Option<String>,
}

pub(crate) trait FanHardware {
    fn read_rpm(&mut self) -> Result<[u16; 2], String>;
    fn automatic(&mut self) -> Result<(), String>;
    fn rpm(&mut self, targets: [u16; 2]) -> Result<[u16; 2], String>;
    fn probe(&mut self) -> Result<(), String>;
}
#[derive(Clone)]
enum Requested {
    Auto,
    Manual([u16; 2]),
    Curve(RpmFanCurves),
}

pub(crate) struct FanController<H: FanHardware> {
    pub hardware: H,
    requested: Requested,
    status: FanControlStatus,
    protected: bool,
    restore_pending: bool,
    suspended: bool,
    healthy_since: Option<Duration>,
    transition: RpmTransition,
    now: Duration,
}
impl<H: FanHardware> FanController<H> {
    pub fn new(hardware: H) -> Self {
        Self {
            hardware,
            requested: Requested::Auto,
            protected: false,
            restore_pending: false,
            suspended: false,
            healthy_since: None,
            transition: RpmTransition::default(),
            now: Duration::ZERO,
            status: FanControlStatus {
                smoothing: RpmSmoothing::default(),
                smoothing_active: false,
                transition_rpm: [0, 0],
                rpm_limits: FanRpmLimits::default(),
                mode: "auto".into(),
                requested_mode: "auto".into(),
                target_rpm: [0, 0],
                applied_rpm: [0, 0],
                cpu_temperature: None,
                resume_pending: false,
                recovery_remaining_secs: 0,
                error: None,
            },
        }
    }
    pub fn status(&self) -> FanControlStatus {
        self.status.clone()
    }
    pub fn set_smoothing(&mut self, smoothing: RpmSmoothing) -> Result<(), String> {
        smoothing.validate()?;
        if self.status.smoothing != smoothing {
            self.transition.configure();
            self.status.smoothing = smoothing;
        }
        Ok(())
    }
    pub fn needs_tick(&self) -> bool {
        self.restore_pending || !matches!(self.requested, Requested::Auto)
    }
    fn update_intent(&mut self) {
        self.status.requested_mode = match self.requested {
            Requested::Auto => "auto",
            Requested::Manual(_) => "manual",
            Requested::Curve(_) => "curve",
        }
        .into();
        self.status.resume_pending =
            self.protected && matches!(self.requested, Requested::Curve(_));
        if !self.status.resume_pending {
            self.status.recovery_remaining_secs = 0;
        }
    }
    fn targets(&self, temp: u8) -> Option<[u16; 2]> {
        match &self.requested {
            Requested::Auto => None,
            Requested::Manual(t) => Some(*t),
            Requested::Curve(c) => Some(c.targets(temp)),
        }
    }
    fn drive(&mut self, temp: u8) -> Result<(), String> {
        let targets = self.targets(temp).ok_or("No fan targets selected")?;
        self.send_targets(targets, temp)?;
        self.status.mode = match self.requested {
            Requested::Curve(_) => "curve",
            _ => "manual",
        }
        .into();
        self.status.cpu_temperature = Some(temp);
        self.status.error = None;
        self.protected = false;
        self.restore_pending = false;
        self.healthy_since = None;
        self.update_intent();
        Ok(())
    }
    fn send_targets(&mut self, targets: [u16; 2], temp: u8) -> Result<(), String> {
        if self.transition.needs_seed() {
            let actual = self.hardware.read_rpm()?;
            self.transition.seed(actual, self.now);
        }
        let command = self.transition.plan(
            targets,
            self.status.smoothing,
            self.now,
            temp >= RECOVER_CELSIUS || targets == [MAX_RPM; 2],
        );
        let applied = self.hardware.rpm(command)?;
        self.transition.commit(applied);
        self.status.target_rpm = targets;
        self.status.transition_rpm = command;
        self.status.applied_rpm = applied;
        self.status.smoothing_active = command != targets;
        Ok(())
    }
    fn reset_transition(&mut self) {
        self.transition.clear();
        self.status.transition_rpm = [0, 0];
        self.status.smoothing_active = false;
    }
    fn restore(&mut self) -> Result<(), String> {
        self.reset_transition();
        self.restore_pending = true;
        self.status.mode = "recovering".into();
        self.status.target_rpm = [0, 0];
        self.status.applied_rpm = [0, 0];
        self.hardware.automatic()?;
        self.restore_pending = false;
        self.status.mode = if self.protected {
            if self.suspended {
                "paused"
            } else {
                "protected"
            }
        } else {
            "auto"
        }
        .into();
        Ok(())
    }
    /// Record loss of the channel without sending bytes outside the transport mutex.
    pub fn note_failure(&mut self, reason: String) {
        self.reset_transition();
        if matches!(self.requested, Requested::Manual(_)) {
            self.requested = Requested::Auto;
        }
        self.protected = matches!(self.requested, Requested::Curve(_));
        self.restore_pending = true;
        self.healthy_since = None;
        self.status.mode = "recovering".into();
        self.status.target_rpm = [0, 0];
        self.status.applied_rpm = [0, 0];
        self.status.error = Some(reason);
        self.status.recovery_remaining_secs = RECOVERY_TIME.as_secs();
        self.update_intent();
    }
    fn protect(&mut self, reason: String, force_restore: bool) {
        let should_restore = force_restore || !self.protected || self.restore_pending;
        self.note_failure(reason);
        if should_restore {
            if let Err(e) = self.restore() {
                self.status.error = Some(format!(
                    "{}；恢复自动失败：{e}",
                    self.status.error.as_deref().unwrap_or("风扇保护")
                ));
            }
        } else {
            self.restore_pending = false;
            self.status.mode = if self.suspended {
                "paused"
            } else {
                "protected"
            }
            .into();
        }
    }
    pub fn queue_curve(&mut self, curves: RpmFanCurves) -> Result<(), String> {
        curves.validate()?;
        self.set_smoothing(curves.smoothing)?;
        self.requested = Requested::Curve(curves);
        self.suspended = false;
        self.healthy_since = None;
        if self.protected {
            self.status.recovery_remaining_secs = RECOVERY_TIME.as_secs();
        }
        self.update_intent();
        Ok(())
    }
    pub fn select_curve(&mut self, curves: RpmFanCurves, temp: Option<u8>) -> Result<(), String> {
        self.queue_curve(curves)?;
        self.status.cpu_temperature = temp;
        if self.protected {
            return Ok(());
        } // Editing a protected curve must not bypass recovery dwell time.
        match temp.filter(|t| (1..PROTECT_CELSIUS).contains(t)) {
            Some(t) => {
                if let Err(e) = self.drive(t) {
                    self.protect(e, true);
                }
            }
            None => self.protect("CPU 温度不可用或达到 90°C，暂由固件温控".into(), false),
        }
        Ok(()) // Validated intent is saved; status reports whether it is active or awaiting recovery.
    }
    pub fn select_manual(&mut self, targets: [u16; 2], temp: Option<u8>) -> Result<(), String> {
        validate_targets(targets)?;
        let temp = temp
            .filter(|t| (1..PROTECT_CELSIUS).contains(t))
            .ok_or("CPU 温度不可用或达到 90°C，无法启用固定转速")?;
        let targets = targets.map(|rpm| ((rpm + 50) / 100) * 100);
        match self.send_targets(targets, temp) {
            Ok(()) => (),
            Err(e) => {
                self.protect(e.clone(), true);
                return Err(e);
            }
        }
        self.requested = Requested::Manual(targets);
        self.suspended = false;
        self.protected = false;
        self.restore_pending = false;
        self.healthy_since = None;
        self.status.mode = "manual".into();
        self.status.cpu_temperature = Some(temp);
        self.status.error = None;
        self.update_intent();
        Ok(())
    }
    /// Cancel before trying I/O so failure to restore cannot resurrect a user-disabled curve.
    pub fn cancel(&mut self) {
        self.reset_transition();
        self.requested = Requested::Auto;
        self.protected = false;
        self.healthy_since = None;
        self.restore_pending = true;
        self.status.mode = "recovering".into();
        self.status.target_rpm = [0, 0];
        self.status.applied_rpm = [0, 0];
        self.update_intent();
    }
    pub fn automatic(&mut self) -> Result<(), String> {
        self.cancel();
        let result = self.restore();
        self.status.error = result.as_ref().err().cloned();
        result
    }
    pub fn suspend(&mut self) {
        self.suspended = true;
        if !self.needs_tick() {
            return;
        }
        self.protect("休眠暂停，唤醒且状态稳定后恢复曲线".into(), true);
    }
    pub fn suspend_without_io(&mut self) {
        self.suspended = true;
    }
    pub fn resume(&mut self) {
        self.suspended = false;
        self.healthy_since = None;
        if self.protected && !self.restore_pending {
            self.status.mode = "protected".into();
        }
    }
    pub fn shutdown(&mut self) -> Result<(), String> {
        self.suspended = true;
        self.automatic()
    }
    pub fn tick(&mut self, temp: Option<u8>, now: Duration, gap_healthy: bool) {
        self.now = now;
        self.status.cpu_temperature = temp;
        if self.suspended || matches!(self.requested, Requested::Auto) {
            if self.restore_pending {
                if let Err(e) = self.restore() {
                    self.status.error = Some(e);
                }
            }
            return;
        }
        if !gap_healthy {
            self.protect("后台调度中断，暂由固件温控".into(), true);
            return;
        }
        let Some(temp) = temp.filter(|t| (1..PROTECT_CELSIUS).contains(t)) else {
            self.protect("CPU 温度不可用或达到 90°C，暂由固件温控".into(), false);
            return;
        };
        if self.restore_pending {
            if let Err(e) = self.restore() {
                self.status.error = Some(e);
                self.healthy_since = None;
                return;
            }
        }
        if self.protected {
            if temp > RECOVER_CELSIUS {
                self.healthy_since = None;
                self.status.recovery_remaining_secs = RECOVERY_TIME.as_secs();
                return;
            }
            if let Err(e) = self.hardware.probe() {
                self.protect(e, true);
                return;
            }
            let since = *self.healthy_since.get_or_insert(now);
            let remaining = RECOVERY_TIME.saturating_sub(now.saturating_sub(since));
            self.status.recovery_remaining_secs =
                remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0);
            if !remaining.is_zero() {
                return;
            }
        }
        if let Err(e) = self.drive(temp) {
            self.protect(e, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct MockFan {
        calls: Vec<Option<[u16; 2]>>,
        fail_probe: bool,
        fail_write: bool,
        fail_restore: bool,
        boost: bool,
        actual: Option<[u16; 2]>,
        fail_read: bool,
    }
    impl FanHardware for MockFan {
        fn read_rpm(&mut self) -> Result<[u16; 2], String> {
            if self.fail_read {
                return Err("tachometer offline".into());
            }
            Ok(self
                .actual
                .unwrap_or_else(|| self.calls.last().copied().flatten().unwrap_or([2200, 2200])))
        }
        fn automatic(&mut self) -> Result<(), String> {
            self.calls.push(None);
            if self.fail_restore {
                Err("offline".into())
            } else {
                Ok(())
            }
        }
        fn rpm(&mut self, rpm: [u16; 2]) -> Result<[u16; 2], String> {
            if self.fail_write {
                return Err("write failed".into());
            }
            self.calls.push(Some(rpm));
            Ok(if self.boost {
                rpm.map(|r| r.max(STARTUP_RPM))
            } else {
                rpm
            })
        }
        fn probe(&mut self) -> Result<(), String> {
            if self.fail_probe {
                Err("offline".into())
            } else {
                Ok(())
            }
        }
    }
    fn controller() -> FanController<MockFan> {
        let mut c = FanController::new(MockFan::default());
        c.select_curve(RpmFanCurves::default(), Some(45)).unwrap();
        c
    }
    fn tick(c: &mut FanController<MockFan>, temp: Option<u8>, second: u64) {
        c.tick(temp, Duration::from_secs(second), true);
    }
    #[test]
    fn rpm_curve_is_independent_and_rounds_to_firmware_resolution() {
        let mut curves = RpmFanCurves::default();
        curves.linked = false;
        curves.fan1.iter_mut().for_each(|p| p.rpm = p.rpm.max(3000));
        curves.validate().unwrap();
        assert_eq!(curves.targets(45), [2200, 3000]);
        assert_eq!(curves.targets(55), [2700, 3100]);
        assert_eq!(curves.targets(100), [7500, 7500]);
        curves.fan0[1].rpm = 2100;
        assert!(curves.validate().is_err());
    }
    #[test]
    fn legacy_percentages_are_migrated_without_treating_percent_as_rpm() {
        let c = RpmFanCurves::from_legacy(
            &[(40, 25), (50, 25), (95, 100)],
            &[(40, 25), (50, 25), (95, 100)],
        )
        .unwrap();
        assert_eq!(c.targets(45), [2200, 2200]);
        assert_eq!(c.targets(100), [7500, 7500]);
        assert!(c.linked);
        assert!(RpmFanCurves::from_legacy(&[(40, 101), (50, 100)], &[]).is_err());
    }
    #[test]
    fn overheating_recovers_only_after_five_healthy_seconds() {
        let mut c = controller();
        tick(&mut c, Some(90), 1);
        assert_eq!(c.status.mode, "protected");
        assert!(c.status.resume_pending);
        tick(&mut c, Some(86), 2);
        assert_eq!(c.status.recovery_remaining_secs, 5);
        for s in 3..8 {
            tick(&mut c, Some(85), s);
            assert_eq!(c.status.mode, "protected");
        }
        tick(&mut c, Some(85), 8);
        assert_eq!(c.status.mode, "curve");
        assert!(!c.status.resume_pending);
        assert_eq!(c.status.target_rpm, [6500, 6500]);
    }
    #[test]
    fn bad_sensor_resets_recovery_and_never_substitutes_temperature() {
        let mut c = controller();
        tick(&mut c, None, 1);
        assert_eq!(c.status.target_rpm, [0, 0]);
        tick(&mut c, Some(40), 2);
        tick(&mut c, Some(40), 6);
        tick(&mut c, None, 7);
        tick(&mut c, Some(40), 8);
        tick(&mut c, Some(40), 12);
        assert_eq!(c.status.mode, "protected");
        tick(&mut c, Some(40), 13);
        assert_eq!(c.status.mode, "curve");
    }
    #[test]
    fn communication_must_remain_healthy_before_resuming() {
        let mut c = controller();
        c.hardware.fail_write = true;
        tick(&mut c, Some(45), 1);
        c.hardware.fail_write = false;
        tick(&mut c, Some(45), 2);
        c.hardware.fail_probe = true;
        tick(&mut c, Some(45), 5);
        c.hardware.fail_probe = false;
        tick(&mut c, Some(45), 6);
        tick(&mut c, Some(45), 10);
        assert_eq!(c.status.mode, "protected");
        tick(&mut c, Some(45), 11);
        assert_eq!(c.status.mode, "curve");
    }
    #[test]
    fn explicit_auto_cancels_pending_curve_even_if_restore_fails() {
        let mut c = controller();
        tick(&mut c, Some(90), 1);
        c.hardware.fail_restore = true;
        assert!(c.automatic().is_err());
        assert!(!c.status.resume_pending);
        c.hardware.fail_restore = false;
        tick(&mut c, Some(40), 2);
        let calls = c.hardware.calls.len();
        tick(&mut c, Some(40), 20);
        assert_eq!(c.status.mode, "auto");
        assert_eq!(c.hardware.calls.len(), calls);
    }
    #[test]
    fn edited_curve_during_protection_waits_for_recovery() {
        let mut c = controller();
        tick(&mut c, Some(90), 1);
        let mut curves = RpmFanCurves::default();
        curves.fan0[0].rpm = 2000;
        curves.fan1 = curves.fan0.clone();
        c.select_curve(curves, Some(40)).unwrap();
        assert_eq!(c.status.mode, "protected");
        tick(&mut c, Some(40), 2);
        tick(&mut c, Some(40), 7);
        assert_eq!(c.status.target_rpm, [2000, 2000]);
    }
    #[test]
    fn suspend_waits_for_wake_and_shutdown_never_resumes() {
        let mut c = controller();
        c.suspend();
        let calls = c.hardware.calls.len();
        tick(&mut c, Some(40), 30);
        assert_eq!(c.hardware.calls.len(), calls);
        c.resume();
        tick(&mut c, Some(40), 31);
        tick(&mut c, Some(40), 36);
        assert_eq!(c.status.mode, "curve");
        c.shutdown().unwrap();
        tick(&mut c, Some(40), 50);
        assert_eq!(c.status.mode, "auto");
        assert_eq!(c.status.requested_mode, "auto");
        assert!(!c.status.resume_pending);
    }
    #[test]
    fn invalid_curve_is_rejected_without_changing_active_policy() {
        let mut c = controller();
        let mut curves = RpmFanCurves::default();
        curves.fan0[0].rpm = 1999;
        assert!(c.select_curve(curves, Some(40)).is_err());
        assert_eq!(c.status.mode, "curve");
    }
    #[test]
    fn transport_mutex_failure_queues_restore_without_unlocked_io() {
        let mut c = controller();
        let writes = c.hardware.calls.len();
        c.note_failure("mutex busy".into());
        assert_eq!(c.hardware.calls.len(), writes);
        assert!(c.status.resume_pending);
        tick(&mut c, Some(45), 1);
        assert_eq!(c.hardware.calls.last(), Some(&None));
        tick(&mut c, Some(45), 6);
        assert_eq!(c.status.mode, "curve");
    }
    #[test]
    fn scheduling_gap_requires_a_fresh_recovery_window() {
        let mut c = controller();
        c.tick(Some(45), Duration::from_secs(20), false);
        assert_eq!(c.status.mode, "protected");
        tick(&mut c, Some(45), 21);
        tick(&mut c, Some(45), 25);
        assert_eq!(c.status.mode, "protected");
        tick(&mut c, Some(45), 26);
        assert_eq!(c.status.mode, "curve");
    }
    #[test]
    fn curve_can_be_selected_during_sensor_failure_and_resume_later() {
        let mut c = FanController::new(MockFan::default());
        c.select_curve(RpmFanCurves::default(), None).unwrap();
        assert_eq!(c.status.requested_mode, "curve");
        assert_eq!(c.status.mode, "protected");
        tick(&mut c, Some(45), 1);
        tick(&mut c, Some(45), 6);
        assert_eq!(c.status.target_rpm, [2200, 2200]);
    }
    #[test]
    fn fixed_rpm_returns_to_auto_on_suspend_and_does_not_resume() {
        let mut c = controller();
        c.select_manual([3000, 3200], Some(45)).unwrap();
        c.suspend();
        assert_eq!(c.status.mode, "auto");
        assert_eq!(c.status.requested_mode, "auto");
        c.resume();
        tick(&mut c, Some(40), 10);
        tick(&mut c, Some(40), 20);
        assert_eq!(c.hardware.calls.last(), Some(&None));
        assert!(!c.status.resume_pending);
    }
    #[test]
    fn failed_firmware_restore_blocks_curve_and_restarts_recovery_when_it_succeeds() {
        let mut c = controller();
        c.hardware.fail_restore = true;
        tick(&mut c, Some(90), 1);
        assert_eq!(c.status.mode, "recovering");
        tick(&mut c, Some(45), 2);
        tick(&mut c, Some(45), 10);
        assert_eq!(c.status.mode, "recovering");
        assert_eq!(
            c.hardware
                .calls
                .iter()
                .filter(|call| call.is_some())
                .count(),
            1
        );
        c.hardware.fail_restore = false;
        tick(&mut c, Some(45), 11);
        tick(&mut c, Some(45), 15);
        assert_eq!(c.status.mode, "protected");
        tick(&mut c, Some(45), 16);
        assert_eq!(c.status.mode, "curve");
        assert_eq!(c.status.target_rpm, [2200, 2200]);
    }
    #[test]
    fn ranges_are_independent_and_linked_curves_use_the_higher_floor() {
        assert!(validate_targets([1200, 1000]).is_ok());
        assert!(validate_targets([1100, 1000]).is_err());
        assert!(validate_targets([1000, 1000]).is_err());
        assert!(validate_targets([1200, 900]).is_err());
        assert!(validate_targets([7500, 7500]).is_ok());
        assert!(validate_targets([7600, 7500]).is_err());
        let points = |rpm| {
            vec![
                RpmCurvePoint { temp: 40, rpm },
                RpmCurvePoint { temp: 95, rpm },
            ]
        };
        let mut curves = RpmFanCurves {
            smoothing: RpmSmoothing::default(),
            fan0: points(1200),
            fan1: points(1000),
            linked: false,
        };
        curves.validate().unwrap();
        assert_eq!(curves.targets(50), [1200, 1000]);
        curves.linked = true;
        assert!(curves.validate().is_err());
        curves.fan0 = points(1000);
        assert!(curves.validate().is_err());
        curves.fan0 = points(1200);
        curves.fan1 = curves.fan0.clone();
        curves.validate().unwrap();
    }
    #[test]
    fn low_speed_start_boost_is_timed_and_never_delays_a_higher_target() {
        let mut guard = FanRpmGuard::default();
        assert_eq!(
            guard
                .apply([1200, 3000], [0, 2800], Duration::ZERO)
                .unwrap(),
            [2000, 3000]
        );
        assert_eq!(
            guard
                .apply([1200, 3000], [1600, 2900], Duration::from_secs(1))
                .unwrap(),
            [2000, 3000]
        );
        assert_eq!(
            guard
                .apply([1200, 3000], [1900, 2900], Duration::from_secs(2))
                .unwrap(),
            [1200, 3000]
        );
        guard.reset();
        guard
            .apply([1200, 1000], [500, 500], Duration::ZERO)
            .unwrap();
        assert_eq!(
            guard
                .apply([5000, 6000], [1000, 1000], Duration::from_secs(1))
                .unwrap(),
            [5000, 6000]
        );
    }
    #[test]
    fn stalled_fan_is_boosted_independently_and_cannot_restart_forever() {
        let mut guard = FanRpmGuard::default();
        guard
            .apply([1200, 1000], [1800, 1800], Duration::ZERO)
            .unwrap();
        guard
            .apply([1200, 1000], [1800, 1800], Duration::from_secs(2))
            .unwrap();
        for second in [3, 6] {
            assert_eq!(
                guard
                    .apply([1200, 1000], [0, 1000], Duration::from_secs(second))
                    .unwrap(),
                [2000, 1000]
            );
            assert_eq!(
                guard
                    .apply([1200, 1000], [1900, 1000], Duration::from_secs(second + 2))
                    .unwrap(),
                [1200, 1000]
            );
        }
        assert!(guard
            .apply([1200, 1000], [0, 1000], Duration::from_secs(9))
            .unwrap_err()
            .contains("反复停转"));
    }
    #[test]
    fn failed_start_and_stalled_high_target_return_an_error() {
        let mut guard = FanRpmGuard::default();
        guard.apply([1200, 1000], [0, 0], Duration::ZERO).unwrap();
        assert!(guard
            .apply([1200, 1000], [0, 0], Duration::from_secs(2))
            .unwrap_err()
            .contains("起转失败"));
        guard.reset();
        guard.apply([4000, 4000], [0, 0], Duration::ZERO).unwrap();
        assert!(guard
            .apply([4000, 4000], [0, 0], Duration::from_secs(3))
            .unwrap_err()
            .contains("持续停转"));
    }
    #[test]
    fn old_stall_attempts_expire_but_live_edits_do_not_reset_the_window() {
        let mut guard = FanRpmGuard::default();
        guard
            .apply([1200, 1000], [1800, 1800], Duration::ZERO)
            .unwrap();
        guard
            .apply([1200, 1000], [1800, 1800], Duration::from_secs(2))
            .unwrap();
        for second in [3, 6, 36] {
            guard
                .apply([1200, 1000], [0, 1000], Duration::from_secs(second))
                .unwrap();
            guard
                .apply([1300, 1000], [1800, 1000], Duration::from_secs(second + 2))
                .unwrap();
        }
    }
    #[test]
    fn status_distinguishes_requested_rpm_from_start_assistance() {
        let mut c = controller();
        c.set_smoothing(RpmSmoothing {
            enabled: false,
            ..RpmSmoothing::default()
        })
        .unwrap();
        c.hardware.boost = true;
        c.select_manual([1200, 1000], Some(45)).unwrap();
        assert_eq!(c.status.target_rpm, [1200, 1000]);
        assert_eq!(c.status.applied_rpm, [2000, 2000]);
        assert_eq!(c.status.rpm_limits.minimum, [1200, 1000]);
        c.hardware.boost = false;
        tick(&mut c, Some(45), 3);
        assert_eq!(c.status.applied_rpm, [1200, 1000]);
        c.automatic().unwrap();
        assert_eq!(c.status.applied_rpm, [0, 0]);
    }
    #[test]
    fn manual_changes_use_smoothing_and_disabling_applies_the_target_on_next_tick() {
        let mut c = controller();
        c.select_manual([5000, 4000], Some(40)).unwrap();
        assert_eq!(c.status.target_rpm, [5000, 4000]);
        assert_eq!(c.status.applied_rpm, [2200, 2200]);
        tick(&mut c, Some(40), 1);
        assert_eq!(c.status.applied_rpm, [3700, 3700]);
        c.set_smoothing(RpmSmoothing {
            enabled: false,
            ..RpmSmoothing::default()
        })
        .unwrap();
        tick(&mut c, Some(40), 2);
        assert_eq!(c.status.applied_rpm, [5000, 4000]);
        assert!(!c.status.smoothing_active);
    }
    #[test]
    fn thermal_and_sensor_protection_bypass_an_active_transition() {
        for bad in [None, Some(90)] {
            let mut c = controller();
            tick(&mut c, Some(75), 1);
            assert!(c.status.smoothing_active);
            tick(&mut c, bad, 2);
            assert_eq!(c.hardware.calls.last(), Some(&None));
            assert!(c.status.resume_pending);
            assert!(!c.status.smoothing_active);
            assert_eq!(c.status.transition_rpm, [0, 0]);
        }
        let mut c = controller();
        tick(&mut c, Some(85), 1);
        assert_eq!(c.status.applied_rpm, [6500, 6500]);
        assert!(!c.status.smoothing_active);
    }
    #[test]
    fn recovered_curve_starts_from_firmware_rpm_instead_of_old_ramp_state() {
        let mut c = controller();
        tick(&mut c, Some(75), 1);
        tick(&mut c, None, 2);
        c.hardware.actual = Some([4500, 4200]);
        tick(&mut c, Some(40), 3);
        tick(&mut c, Some(40), 8);
        assert_eq!(c.status.mode, "curve");
        assert_eq!(c.status.target_rpm, [2200, 2200]);
        assert_eq!(c.status.applied_rpm, [4500, 4200]);
        tick(&mut c, Some(40), 12);
        assert_eq!(c.status.applied_rpm, [4200, 3900]);
    }
    #[test]
    fn live_curve_edits_preserve_the_downward_wait_when_settings_do_not_change() {
        let mut c = FanController::new(MockFan {
            actual: Some([4500, 4500]),
            ..Default::default()
        });
        c.select_curve(RpmFanCurves::default(), Some(40)).unwrap();
        tick(&mut c, Some(40), 2);
        let mut curves = RpmFanCurves::default();
        for p in &mut curves.fan0[..2] {
            p.rpm = 2300;
        }
        curves.fan1 = curves.fan0.clone();
        c.select_curve(curves, Some(40)).unwrap();
        tick(&mut c, Some(40), 4);
        assert_eq!(c.status.applied_rpm, [4200, 4200]);
    }
    #[test]
    fn seed_read_failure_hands_control_to_firmware_and_preserves_curve_intent() {
        let mut c = FanController::new(MockFan {
            fail_read: true,
            ..Default::default()
        });
        c.select_curve(RpmFanCurves::default(), Some(40)).unwrap();
        assert_eq!(c.status.mode, "protected");
        assert!(c.status.resume_pending);
        assert_eq!(c.hardware.calls, [None]);
        assert_eq!(c.status.applied_rpm, [0, 0]);
    }
    #[test]
    fn configuring_smoothing_in_auto_does_not_activate_control_or_touch_hardware() {
        let mut c = FanController::new(MockFan::default());
        let settings = RpmSmoothing {
            rise_rpm_per_sec: 2000,
            fall_delay_secs: 5,
            ..Default::default()
        };
        c.set_smoothing(settings).unwrap();
        assert_eq!(c.status.smoothing, settings);
        assert_eq!(c.status.mode, "auto");
        assert!(!c.needs_tick());
        assert!(c.hardware.calls.is_empty());
        assert!(c
            .set_smoothing(RpmSmoothing {
                fall_delay_secs: 11,
                ..settings
            })
            .is_err());
        assert_eq!(c.status.smoothing, settings);
    }
    #[test]
    fn native_start_assistance_overrides_smoothing_and_becomes_its_baseline() {
        let mut c = FanController::new(MockFan {
            actual: Some([0, 0]),
            boost: true,
            ..Default::default()
        });
        c.select_manual([1200, 1000], Some(40)).unwrap();
        assert_eq!(c.status.transition_rpm, [1200, 1000]);
        assert_eq!(c.status.applied_rpm, [2000, 2000]);
        c.hardware.boost = false;
        tick(&mut c, Some(40), 1);
        assert_eq!(c.status.applied_rpm, [2000, 2000]);
    }
    #[test]
    fn fan_boost_bypasses_the_ramp_even_at_low_temperature() {
        let mut c = controller();
        c.select_manual([MAX_RPM; 2], Some(40)).unwrap();
        assert_eq!(c.status.applied_rpm, [MAX_RPM; 2]);
        assert!(!c.status.smoothing_active);
    }
}
