//! CPU package power control, independent of the chassis EC protocol.
//! Only contracts opting into intel_rapl may call the write API.
use super::intel_msr::{IntelMsr, ThreadAffinity};
use super::vault::intel::*;
use parking_lot::Mutex;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CpuPowerStatus {
    pub backend: &'static str,
    pub available: bool,
    pub pl1_writable: bool,
    pub pl2_writable: bool,
    pub pl4_writable: bool,
    pub temperature_writable: bool,
    pub ratio_writable: bool,
    pub voltage_writable: bool,
    pub locked: bool,
    pub active: bool,
    pub overridden: bool,
    pub write_verified: bool,
    pub pl1_watts: Option<f64>,
    pub pl2_watts: Option<f64>,
    pub pl1_enabled: bool,
    pub pl2_enabled: bool,
    pub pl1_window_seconds: Option<f64>,
    pub pl2_window_seconds: Option<f64>,
    pub min_watts: u16,
    pub max_watts: u16,
    pub error: Option<String>,
}

impl Default for CpuPowerStatus {
    fn default() -> Self {
        Self {
            backend: "intel_rapl",
            available: false,
            pl1_writable: false,
            pl2_writable: false,
            pl4_writable: false,
            temperature_writable: false,
            ratio_writable: false,
            voltage_writable: false,
            locked: false,
            active: false,
            overridden: false,
            write_verified: false,
            pl1_watts: None,
            pl2_watts: None,
            pl1_enabled: false,
            pl2_enabled: false,
            pl1_window_seconds: None,
            pl2_window_seconds: None,
            min_watts: 5,
            max_watts: 0,
            error: None,
        }
    }
}

trait RaplIo {
    fn read(&mut self, index: u64) -> Result<u64, String>;
    fn write_limit(&mut self, value: u64) -> Result<(), String>;
}
impl RaplIo for IntelMsr {
    fn read(&mut self, index: u64) -> Result<u64, String> {
        self.read_msr(index).map_err(|e| e.to_string())
    }
    fn write_limit(&mut self, value: u64) -> Result<(), String> {
        self.write_package_limit(value).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Copy)]
struct Snapshot {
    units: u64,
    limit: u64,
}
impl Snapshot {
    fn read(io: &mut impl RaplIo) -> Result<Self, String> {
        Ok(Self {
            units: io.read(MSR_RAPL_POWER_UNIT)?,
            limit: io.read(MSR_PKG_POWER_LIMIT)?,
        })
    }
    fn unit(self) -> f64 {
        2f64.powi(-((self.units & 15) as i32))
    }
    fn watts(self, shift: u32) -> f64 {
        ((self.limit >> shift) & POWER_VALUE_MASK) as f64 * self.unit()
    }
    fn bound(self) -> u16 {
        // Stay within the firmware's existing package budget. No invented model TDP.
        let max = self.watts(0).max(self.watts(32));
        if max < 5.0 || max > 255.0 {
            0
        } else {
            max.floor() as u16
        }
    }
    fn window(self, shift: u32) -> f64 {
        let encoded = (self.limit >> shift) & 127;
        let time_unit = 2f64.powi(-(((self.units >> 16) & 15) as i32));
        2f64.powi((encoded & 31) as i32) * (1.0 + ((encoded >> 5) & 3) as f64 / 4.0) * time_unit
    }
    fn encode(self, watts: [u16; 2], bound: u16) -> Result<u64, String> {
        if self.limit & PACKAGE_LOCK != 0 {
            return Err("CPU 功耗寄存器已被固件锁定".into());
        }
        if bound < 5 || watts[0] < 5 || watts[0] > watts[1] || watts[1] > bound {
            return Err(format!("CPU 功耗必须满足 5W ≤ PL1 ≤ PL2 ≤ {bound}W"));
        }
        let raw = watts.map(|w| (w as f64 / self.unit()).round() as u64);
        if raw.iter().any(|v| *v > POWER_VALUE_MASK) {
            return Err("CPU 功耗超出寄存器范围".into());
        }
        Ok((self.limit & !OWNED_FIELDS) | raw[0] | PL1_ENABLE | (raw[1] << 32) | PL2_ENABLE)
    }
}

#[derive(Clone, Copy)]
struct Session {
    baseline: Snapshot,
    last: u64,
    max_watts: u16,
    verified: bool,
}

#[derive(Default, Clone)]
struct Policy {
    session: Option<Session>,
    error: Option<String>,
    limit_watts: Option<u16>,
}
impl Policy {
    fn bound(&self, snapshot: Snapshot) -> u16 {
        if let Some(limit) = self.limit_watts {
            if !(5..=255).contains(&limit) {
                return 0;
            }
            let encodable = (POWER_VALUE_MASK as f64 * snapshot.unit()).floor() as u16;
            return limit.min(encodable);
        }
        self.session.map_or(snapshot.bound(), |s| s.max_watts)
    }
    fn status(&mut self, io: &mut impl RaplIo) -> CpuPowerStatus {
        let mut status = CpuPowerStatus::default();
        match Snapshot::read(io) {
            Ok(snapshot) => {
                status.available = true;
                status.locked = snapshot.limit & PACKAGE_LOCK != 0;
                status.max_watts = self.bound(snapshot);
                status.pl1_writable = !status.locked && status.max_watts >= 5;
                status.pl2_writable = status.pl1_writable;
                status.pl1_watts = Some(snapshot.watts(0));
                status.pl2_watts = Some(snapshot.watts(32));
                status.pl1_enabled = snapshot.limit & PL1_ENABLE != 0;
                status.pl2_enabled = snapshot.limit & PL2_ENABLE != 0;
                status.pl1_window_seconds = Some(snapshot.window(17));
                status.pl2_window_seconds = Some(snapshot.window(49));
                status.active = self.session.is_some();
                status.overridden = self
                    .session
                    .is_some_and(|s| (snapshot.limit ^ s.last) & OWNED_FIELDS != 0);
                status.write_verified =
                    self.session.is_some_and(|s| s.verified) && !status.overridden;
                status.error = if status.overridden {
                    Some("CPU 功耗已被固件或其他程序覆盖；请切换预设模式后重新应用".into())
                } else if status.locked {
                    Some("CPU 功耗寄存器已被固件锁定，仅可读取".into())
                } else if status.max_watts < 5 {
                    Some("无法确定有效的原厂 CPU 功耗范围，仅可读取".into())
                } else {
                    self.error.clone()
                };
            }
            Err(e) => {
                status.active = self.session.is_some();
                status.error = Some(e);
            }
        }
        status
    }

    fn apply(&mut self, io: &mut impl RaplIo, watts: [u16; 2]) -> Result<(), String> {
        let before = Snapshot::read(io)?;
        if self
            .session
            .is_some_and(|s| (before.limit ^ s.last) & OWNED_FIELDS != 0)
        {
            return Err("CPU 功耗控制权已改变，请先退出自定义模式".into());
        }
        let bound = self.bound(before);
        let requested = before.encode(watts, bound)?;
        let old = self.session;
        if old.is_some_and(|s| s.verified) && requested == before.limit {
            return Ok(());
        }
        // Remember ownership before writing: even a failed IOCTL can have changed hardware.
        self.session = Some(Session {
            baseline: old.map_or(before, |s| s.baseline),
            last: requested,
            max_watts: bound,
            verified: false,
        });
        if let Err(error) = write_verified(io, requested) {
            let recovery = restore_matching(io, before.limit, requested).and_then(|()| {
                let actual = io.read(MSR_PKG_POWER_LIMIT)?;
                if (actual ^ before.limit) & OWNED_FIELDS != 0 {
                    return Err("固件改变了功耗设置，无法确认回滚结果".into());
                }
                Ok(())
            });
            if recovery.is_ok() {
                self.session = old;
            }
            return Err(match recovery {
                Ok(()) => format!("CPU 功耗写入失败，已回滚：{error}"),
                Err(e) => format!("CPU 功耗写入失败：{error}；回滚失败：{e}"),
            });
        }
        self.session.as_mut().unwrap().verified = true;
        self.error = None;
        Ok(())
    }

    fn release(&mut self, io: &mut impl RaplIo) -> Result<(), String> {
        if let Some(s) = self.session {
            restore_matching(io, s.baseline.limit, s.last)?;
            self.session = None;
        }
        self.error = None;
        Ok(())
    }

    fn apply_committed<I: RaplIo>(
        &mut self,
        io: &mut I,
        watts: [u16; 2],
        select: impl FnOnce(&mut I) -> Result<(), String>,
        persist: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let before = Snapshot::read(io)?;
        let previous = self.clone();
        self.transition(io, Some(watts), select)?;
        if let Err(error) = persist() {
            let last = self
                .session
                .as_ref()
                .ok_or("CPU ownership lost during commit")?
                .last;
            let rollback = restore_matching(io, before.limit, last).and_then(|()| {
                if (io.read(MSR_PKG_POWER_LIMIT)? ^ before.limit) & OWNED_FIELDS != 0 {
                    return Err("CPU limits changed externally during rollback".into());
                }
                Ok(())
            });
            match rollback {
                Ok(()) => {
                    *self = previous;
                    Err(format!(
                        "Profile save failed; CPU settings restored: {error}"
                    ))
                }
                Err(rollback) => {
                    if let Some(session) = &mut self.session {
                        session.verified = false;
                    }
                    Err(format!(
                        "Profile save failed: {error}; CPU restore failed: {rollback}"
                    ))
                }
            }
        } else {
            Ok(())
        }
    }

    fn transition<I: RaplIo>(
        &mut self,
        io: &mut I,
        watts: Option<[u16; 2]>,
        select: impl FnOnce(&mut I) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(watts) = watts {
            let before = Snapshot::read(io)?;
            before.encode(watts, self.bound(before))?;
            if self.session.is_none() {
                select(io)?;
                let result = self.apply(io, watts);
                // The firmware selection is part of entering Custom. Preserve
                // the pre-entry budget, not a value changed by that selection.
                if let Some(session) = &mut self.session {
                    session.baseline = before;
                }
                return result;
            }
            self.apply(io, watts)
        } else {
            // Select the factory mode before releasing our MSR override. A
            // firmware mode change may copy the current custom limits back to
            // the CPU; restoring first lets that copy reinstate the old cap.
            // Keep any new limits supplied by firmware and restore only fields
            // that still match our last write. Failure retains the owner so a
            // later mode switch or shutdown can retry the restoration.
            select(io)?;
            self.release(io)
        }
    }
}

fn write_verified(io: &mut impl RaplIo, value: u64) -> Result<(), String> {
    io.write_limit(value)?;
    let actual = io.read(MSR_PKG_POWER_LIMIT)?;
    if actual != value {
        return Err("CPU 功耗回读与请求不一致".into());
    }
    Ok(())
}

fn restore_matching(io: &mut impl RaplIo, baseline: u64, last: u64) -> Result<(), String> {
    let current = io.read(MSR_PKG_POWER_LIMIT)?;
    let mut restored = current;
    // Restore only fields still owned by us. Preserve external changes and all other bits.
    for mask in [PL1_FIELDS, PL2_FIELDS] {
        if (current ^ last) & mask == 0 {
            restored = (restored & !mask) | (baseline & mask);
        }
    }
    if restored == current {
        return Ok(());
    }
    if current & PACKAGE_LOCK != 0 {
        return Err("CPU 功耗被锁定，无法恢复接管前设置".into());
    }
    write_verified(io, restored)
}

#[derive(Default)]
struct Controller {
    io: Option<IntelMsr>,
    policy: Policy,
}
static CONTROLLER: Mutex<Controller> = Mutex::new(Controller {
    io: None,
    policy: Policy {
        session: None,
        error: None,
        limit_watts: None,
    },
});

impl Controller {
    fn connect(&mut self) -> Result<(&mut IntelMsr, &mut Policy), String> {
        self.policy.limit_watts = super::declarative::load_matched_manifest()
            .filter(|m| m.capabilities.cpu_power_backend.as_deref() == Some("intel_rapl"))
            .and_then(|m| m.capabilities.cpu_power_max_watts);

        if self.io.is_none() {
            self.io =
                Some(IntelMsr::open().map_err(|e| format!("PawnIO CPU 功耗接口不可用：{e}"))?);
        }
        Ok((self.io.as_mut().unwrap(), &mut self.policy))
    }
}

/// Read-only probe. Never tests writability by changing a register.
pub fn status() -> CpuPowerStatus {
    let mut c = CONTROLLER.lock();
    let result = (|| {
        let _affinity = ThreadAffinity::pin().map_err(|e| e.to_string())?;
        let (io, policy) = c.connect()?;
        Ok::<_, String>(policy.status(io))
    })();
    result.unwrap_or_else(|error| CpuPowerStatus {
        error: Some(error),
        ..Default::default()
    })
}

pub fn is_active() -> bool {
    CONTROLLER.lock().policy.session.is_some()
}

pub fn ac_online() -> bool {
    #[repr(C)]
    #[derive(Default)]
    struct PowerStatus {
        ac: u8,
        battery: u8,
        percent: u8,
        flags: u8,
        lifetime: u32,
        full_lifetime: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemPowerStatus(status: *mut PowerStatus) -> i32;
    }
    let mut power = PowerStatus::default();
    unsafe { GetSystemPowerStatus(&mut power) != 0 && power.ac == 1 }
}

/// Serializes firmware transitions, live edits, restoration and exit under one owner.
pub fn transition(
    watts: Option<[u16; 2]>,
    select_firmware: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    transition_inner(watts, false, select_firmware)
}

/// Live edits must never reactivate a session canceled by a concurrent mode switch.
pub fn update(watts: [u16; 2]) -> Result<(), String> {
    transition_inner(Some(watts), true, || Ok(()))
}

/// Persist a verified profile while holding the same owner lock used by mode
/// changes. A storage failure restores the previous register fields and owner.
pub fn commit_profile(
    watts: [u16; 2],
    active_only: bool,
    select_firmware: impl FnOnce() -> Result<(), String>,
    restore_firmware: impl FnOnce() -> Result<(), String>,
    persist: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let mut c = CONTROLLER.lock();
    if active_only && c.policy.session.is_none() {
        return Err("CPU custom mode was canceled; apply the profile again".into());
    }
    validate_power_writer()?;
    let _affinity = ThreadAffinity::pin().map_err(|e| e.to_string())?;
    let (io, policy) = c.connect()?;
    let result = policy.apply_committed(io, watts, |_| select_firmware(), persist);
    if let Err(error) = result {
        let error = match restore_firmware() {
            Ok(()) => error,
            Err(restore) => format!("{error}; firmware restore failed: {restore}"),
        };
        policy.error = Some(error.clone());
        return Err(error);
    }
    Ok(())
}

fn validate_power_writer() -> Result<(), String> {
    if !ac_online() {
        return Err("Connect AC power before applying CPU limits".into());
    }
    if std::net::TcpStream::connect_timeout(
        &([127, 0, 0, 1], 18946).into(),
        std::time::Duration::from_millis(100),
    )
    .is_ok()
    {
        return Err("Close the power plugin agent before using native CPU control".into());
    }
    Ok(())
}

fn transition_inner(
    watts: Option<[u16; 2]>,
    require_active: bool,
    select_firmware: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let mut c = CONTROLLER.lock();
    if require_active && c.policy.session.is_none() {
        return Err("CPU 自定义模式已经退出，请重新应用方案".into());
    }
    if watts.is_some() {
        validate_power_writer()?;
    }
    // Factory modes remain usable when PawnIO is missing.
    if watts.is_none() && c.policy.session.is_none() {
        return select_firmware();
    }
    let _affinity = ThreadAffinity::pin().map_err(|e| e.to_string())?;
    let (io, policy) = c.connect()?;
    let result = policy.transition(io, watts, |_| select_firmware());
    if let Err(error) = &result {
        policy.error = Some(error.clone());
    }
    result
}

pub fn shutdown() -> Result<(), String> {
    let mut c = CONTROLLER.lock();
    if c.policy.session.is_none() {
        return Ok(());
    }
    let _affinity = ThreadAffinity::pin().map_err(|e| e.to_string())?;
    let (io, policy) = c.connect()?;
    policy.release(io)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        units: u64,
        value: u64,
        writes: Vec<u64>,
        ignore_next: bool,
    }
    impl Fake {
        fn new(exponent: u64) -> Self {
            let scale = 1 << exponent;
            Self {
                units: exponent | (10 << 16),
                value: 45 * scale
                    | PL1_ENABLE
                    | ((80 * scale | (1 << 15)) << 32)
                    | (79 << 17)
                    | (45 << 49)
                    | (1 << 16)
                    | (1 << 48),
                writes: vec![],
                ignore_next: false,
            }
        }
    }
    impl RaplIo for Fake {
        fn read(&mut self, index: u64) -> Result<u64, String> {
            Ok(if index == MSR_RAPL_POWER_UNIT {
                self.units
            } else {
                self.value
            })
        }
        fn write_limit(&mut self, v: u64) -> Result<(), String> {
            self.writes.push(v);
            if self.ignore_next {
                self.ignore_next = false;
            } else {
                self.value = v;
            }
            Ok(())
        }
    }
    #[test]
    fn failed_profile_save_restores_limits_and_does_not_activate_custom() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut policy = Policy::default();
        let result =
            policy.apply_committed(&mut io, [20, 30], |_| Ok(()), || Err("disk full".into()));
        assert!(result.unwrap_err().contains("disk full"));
        assert_eq!(io.value, original);
        assert!(policy.session.is_none());
    }

    #[test]
    fn failed_live_profile_save_preserves_previous_custom_session_and_baseline() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut policy = Policy::default();
        policy.apply(&mut io, [20, 30]).unwrap();
        let custom = io.value;
        policy
            .apply_committed(
                &mut io,
                [25, 35],
                |_| panic!("Already active"),
                || Err("read-only profile".into()),
            )
            .unwrap_err();
        assert_eq!(io.value, custom);
        assert!(policy.session.unwrap().verified);
        policy.release(&mut io).unwrap();
        assert_eq!(io.value, original);
    }

    #[test]
    fn failed_hardware_write_never_persists_profile() {
        let mut io = Fake::new(3);
        io.ignore_next = true;
        let mut policy = Policy::default();
        assert!(policy
            .apply_committed(
                &mut io,
                [20, 30],
                |_| Ok(()),
                || panic!("No verified write")
            )
            .is_err());
    }
    #[test]
    fn decodes_decimal_watts_and_real_units() {
        let mut io = Fake::new(3);
        io.value += 3;
        let s = Policy::default().status(&mut io);
        assert_eq!(s.pl1_watts, Some(45.375));
        assert_eq!(s.pl2_watts, Some(80.0));
        assert_eq!(s.pl1_window_seconds, Some(48.0));
        assert!(io.writes.is_empty());
        assert!(
            !s.pl4_writable && !s.ratio_writable && !s.temperature_writable && !s.voltage_writable
        );
    }
    #[test]
    fn preserves_time_clamp_reserved_and_high_pl2_bits() {
        for exponent in [0, 3, 8] {
            let mut io = Fake::new(exponent);
            let before = io.value;
            let mut p = Policy::default();
            p.apply(&mut io, [20, 30]).unwrap();
            assert_eq!(io.value & !OWNED_FIELDS, before & !OWNED_FIELDS);
            let s = p.status(&mut io);
            assert_eq!(s.pl1_watts, Some(20.0));
            assert_eq!(s.pl2_watts, Some(30.0));
            assert!(s.write_verified);
            p.release(&mut io).unwrap();
            assert_eq!(io.value, before);
        }
    }
    #[test]
    fn rejects_lock_order_and_range_without_writes() {
        let mut io = Fake::new(3);
        let mut p = Policy::default();
        for watts in [[0, 30], [40, 30], [20, 81]] {
            assert!(p.apply(&mut io, watts).is_err());
        }
        io.value |= PACKAGE_LOCK;
        assert!(!p.status(&mut io).pl1_writable);
        assert!(p.apply(&mut io, [20, 30]).is_err());
        assert!(io.writes.is_empty());
    }
    #[test]
    fn readback_failure_rolls_back_and_does_not_activate() {
        let mut io = Fake::new(3);
        let original = io.value;
        io.ignore_next = true;
        let mut p = Policy::default();
        assert!(p.apply(&mut io, [20, 30]).is_err());
        assert_eq!(io.value, original);
        assert!(p.session.is_none());
    }
    #[test]
    fn live_edits_keep_original_baseline_and_ceiling() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [20, 30]).unwrap();
        p.apply(&mut io, [35, 70]).unwrap();
        assert_eq!(p.status(&mut io).max_watts, 80);
        p.release(&mut io).unwrap();
        assert_eq!(io.value, original);
    }
    #[test]
    fn external_override_yields_and_restores_only_owned_channel() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [20, 30]).unwrap();
        io.value = (io.value & !PL1_FIELDS) | 25 * 8 | PL1_ENABLE;
        let external = io.value;
        assert!(p.status(&mut io).overridden);
        assert!(p.apply(&mut io, [30, 40]).is_err());
        assert_eq!(io.writes.len(), 1);
        p.release(&mut io).unwrap();
        assert_eq!(io.value & PL1_FIELDS, external & PL1_FIELDS);
        assert_eq!(io.value & PL2_FIELDS, original & PL2_FIELDS);
    }
    #[test]
    fn invalid_profile_and_firmware_failure_cannot_activate_custom() {
        let mut io = Fake::new(3);
        let mut p = Policy::default();
        p.transition(&mut io, Some([30, 20]), |_| {
            panic!("Invalid profile reached firmware")
        })
        .unwrap_err();
        let original = io.value;
        p.transition(&mut io, Some([20, 30]), |_| {
            Err("Firmware rejected mode".into())
        })
        .unwrap_err();
        assert!(p.session.is_none());
        assert!(io.writes.is_empty());
        assert_eq!(io.value, original);
    }
    #[test]
    fn captures_budget_before_entering_custom_and_does_not_repeat_mode_writes() {
        let mut io = Fake::new(3);
        let mut p = Policy::default();
        let original = io.value;
        p.transition(&mut io, Some([20, 30]), |io| {
            io.value = (io.value & !PL1_FIELDS) | 50 * 8 | PL1_ENABLE;
            Ok(())
        })
        .unwrap();
        p.transition(&mut io, Some([30, 40]), |_| {
            panic!("Live edit must not reset firmware limits")
        })
        .unwrap();
        p.transition(&mut io, None, |io| {
            assert_eq!(io.value & PL1_FIELDS, 30 * 8 | PL1_ENABLE);
            Ok(())
        })
        .unwrap();
        assert_eq!(io.value, original);
        assert!(p.session.is_none());
    }

    #[test]
    fn exit_clears_twelve_watt_limit_copied_back_by_factory_mode_selection() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.transition(&mut io, Some([12, 17]), |_| Ok(())).unwrap();
        let custom = io.value;
        p.transition(&mut io, None, |io| {
            // OEM mode selection can copy the active package budget. The
            // final restoration must run after this firmware operation.
            io.value = custom;
            Ok(())
        }).unwrap();
        assert_eq!(io.value, original);
        assert!(!p.status(&mut io).active);
    }

    #[test]
    fn exit_keeps_new_factory_limits_and_unrelated_firmware_bits() {
        let mut io = Fake::new(3);
        let mut p = Policy::default();
        p.apply(&mut io, [12, 17]).unwrap();
        let factory = (io.value & !OWNED_FIELDS)
            | 28 * 8 | PL1_ENABLE | ((40 * 8) << 32) | PL2_ENABLE;
        let factory = factory ^ (1 << 23) ^ (1 << 61);
        p.transition(&mut io, None, |io| {
            io.value = factory;
            Ok(())
        }).unwrap();
        assert_eq!(io.value, factory);
        assert_eq!(io.writes.len(), 1); // No write over the firmware's new budget.
        assert!(!p.status(&mut io).active);
    }

    #[test]
    fn exit_restores_only_the_channel_left_at_the_custom_limit() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [12, 17]).unwrap();
        p.transition(&mut io, None, |io| {
            io.value = (io.value & !PL1_FIELDS) | 28 * 8 | PL1_ENABLE;
            Ok(())
        }).unwrap();
        assert_eq!(io.value & PL1_FIELDS, 28 * 8 | PL1_ENABLE);
        assert_eq!(io.value & PL2_FIELDS, original & PL2_FIELDS);
        assert!(!p.status(&mut io).active);
    }

    #[test]
    fn failed_exit_restore_keeps_owner_and_can_be_retried() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [12, 17]).unwrap();
        io.ignore_next = true;
        assert!(p.transition(&mut io, None, |_| Ok(())).is_err());
        assert!(p.status(&mut io).active);
        assert_eq!(p.status(&mut io).pl1_watts, Some(12.0));
        p.transition(&mut io, None, |_| Ok(())).unwrap();
        assert_eq!(io.value, original);
        assert!(!p.status(&mut io).active);
    }

    #[test]
    fn failed_factory_switch_does_not_write_limits_or_lose_ownership() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [12, 17]).unwrap();
        let custom = io.value;
        let writes = io.writes.len();
        assert!(p.transition(&mut io, None, |_| Err("Firmware rejected mode".into())).is_err());
        assert_eq!(io.value, custom);
        assert_eq!(io.writes.len(), writes);
        assert!(p.status(&mut io).write_verified);
        p.release(&mut io).unwrap();
        assert_eq!(io.value, original);
    }
    #[test]
    fn failed_exit_recovers_previous_custom_session() {
        let mut io = Fake::new(3);
        let mut p = Policy::default();
        p.apply(&mut io, [20, 30]).unwrap();
        let custom = io.value;
        p.transition(&mut io, None, |_| Err("Firmware rejected mode".into()))
            .unwrap_err();
        assert_eq!(io.value, custom);
        assert!(p.status(&mut io).write_verified);
    }
    #[test]
    fn exit_preserves_new_time_windows_and_reserved_bits() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [20, 30]).unwrap();
        io.value ^= (1 << 23) | (1 << 61);
        let changed = io.value;
        p.release(&mut io).unwrap();
        assert_eq!(io.value & !OWNED_FIELDS, changed & !OWNED_FIELDS);
        assert_eq!(io.value & OWNED_FIELDS, original & OWNED_FIELDS);
    }
    #[test]
    fn identical_live_profile_does_not_rewrite_cpu_registers() {
        let mut io = Fake::new(3);
        let mut p = Policy::default();
        p.apply(&mut io, [20, 30]).unwrap();
        p.apply(&mut io, [20, 30]).unwrap();
        assert_eq!(io.writes.len(), 1);
    }
    #[test]
    fn locked_restoration_keeps_pending_owner_for_retry() {
        let mut io = Fake::new(3);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [20, 30]).unwrap();
        io.value |= PACKAGE_LOCK;
        assert!(p.release(&mut io).is_err());
        assert!(p.session.is_some());
        io.value &= !PACKAGE_LOCK;
        p.release(&mut io).unwrap();
        assert_eq!(io.value, original);
    }

    #[test]
    fn disabled_limits_are_enabled_and_their_original_flags_are_restored() {
        let mut io = Fake::new(3);
        io.value &= !(PL1_ENABLE | PL2_ENABLE);
        let original = io.value;
        let mut p = Policy::default();
        p.apply(&mut io, [20, 30]).unwrap();
        assert!(p.status(&mut io).pl1_enabled && p.status(&mut io).pl2_enabled);
        p.release(&mut io).unwrap();
        assert_eq!(io.value, original);
    }

    #[test]
    fn failed_transport_after_a_write_still_restores_the_register() {
        struct FailsOnce(Fake, bool);
        impl RaplIo for FailsOnce {
            fn read(&mut self, index: u64) -> Result<u64, String> {
                self.0.read(index)
            }
            fn write_limit(&mut self, value: u64) -> Result<(), String> {
                self.0.write_limit(value)?;
                if self.1 {
                    self.1 = false;
                    return Err("Write transport failed".into());
                }
                Ok(())
            }
        }
        let mut io = FailsOnce(Fake::new(3), true);
        let original = io.0.value;
        let mut p = Policy::default();
        assert!(p.apply(&mut io, [20, 30]).is_err());
        assert_eq!(io.0.value, original);
        assert_eq!(io.0.writes.len(), 2);
        assert!(p.session.is_none());
    }

    #[test]
    fn firmware_clamping_cannot_be_reported_as_successful_rollback() {
        struct Clamps(Fake);
        impl RaplIo for Clamps {
            fn read(&mut self, index: u64) -> Result<u64, String> {
                self.0.read(index)
            }
            fn write_limit(&mut self, value: u64) -> Result<(), String> {
                self.0.write_limit(value)?;
                self.0.value = (self.0.value & !POWER_VALUE_MASK) | 18 * 8;
                Ok(())
            }
        }
        let mut io = Clamps(Fake::new(3));
        let mut p = Policy::default();
        assert!(p.apply(&mut io, [20, 30]).unwrap_err().contains("回滚失败"));
        let status = p.status(&mut io);
        assert!(status.overridden);
        assert!(!status.write_verified);
    }

    #[test]
    fn declared_limit_allows_fifty_watts_from_a_low_power_preset() {
        let mut io = Fake::new(3);
        io.value = (io.value & !OWNED_FIELDS) | 15 * 8 | PL1_ENABLE | ((20 * 8) << 32) | PL2_ENABLE;
        let original = io.value;
        let mut p = Policy {
            limit_watts: Some(50),
            ..Default::default()
        };
        assert_eq!(p.status(&mut io).max_watts, 50);
        assert!(io.writes.is_empty());
        let turbo = Fake::new(3).value;
        p.transition(&mut io, Some([40, 50]), |io| {
            io.value = turbo;
            Ok(())
        })
        .unwrap();
        assert_eq!(p.status(&mut io).pl1_watts, Some(40.0));
        assert_eq!(p.status(&mut io).pl2_watts, Some(50.0));
        p.apply(&mut io, [50, 50]).unwrap();
        assert_eq!(p.status(&mut io).max_watts, 50);
        p.release(&mut io).unwrap();
        assert_eq!(io.value, original);
    }

    #[test]
    fn declared_fifty_watt_limit_rejects_overrides_from_cpu_and_preset_values() {
        let mut io = Fake::new(3); // Firmware currently permits 80 W.
        let mut p = Policy {
            limit_watts: Some(50),
            ..Default::default()
        };
        for watts in [[40, 51], [51, 51], [50, 49], [4, 50]] {
            assert!(p.apply(&mut io, watts).is_err());
        }
        assert!(io.writes.is_empty());
        p.apply(&mut io, [5, 5]).unwrap();
        assert_eq!(p.status(&mut io).max_watts, 50);
        p.apply(&mut io, [40, 50]).unwrap();
        assert!(p.apply(&mut io, [40, 80]).is_err());
    }
}
