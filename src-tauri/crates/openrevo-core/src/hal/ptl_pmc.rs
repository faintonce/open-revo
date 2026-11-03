//! P916F-PTL private PM2 control, accessed only through an existing verified port driver.
//! Protocol and synchronization derived from FanControl.MechrevoPtl (MIT); see resources/ptl/NOTICE.md.
use super::{cpu_temperature::PackageTemperatureReader, vault::ptl::*};
use parking_lot::Mutex;
use serde::Serialize;
use std::{
    fs::File,
    io,
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, String>;

trait PortIo {
    fn read(&self, port: u16) -> Result<u8>;
    fn write(&self, port: u16, value: u8) -> Result<()>;
}

struct PortDriver(File);
#[cfg(windows)]
impl PortDriver {
    fn open() -> Result<Self> {
        validate_machine()?;
        use sha2::{Digest, Sha256};
        use std::os::windows::fs::OpenOptionsExt;
        let windows = std::env::var_os("WINDIR").ok_or("Windows directory unavailable")?;
        let binary =
            std::fs::read(std::path::Path::new(&windows).join("System32/drivers/inpoutx64.sys"))
                .map_err(|e| format!("PTL port driver not installed: {e}"))?;
        if format!("{:x}", Sha256::digest(&binary)) != INPOUT_SHA256 {
            return Err("PTL port driver hash is not supported".into());
        }
        std::fs::OpenOptions::new()
            .access_mode(0xc0000000)
            .share_mode(3)
            .open(r"\\.\inpoutx64")
            .map(Self)
            .map_err(|e| format!("Cannot open PTL port driver (administrator required): {e}"))
    }

    fn ioctl(&self, code: u32, input: &[u8]) -> Result<(u32, [u8; 16])> {
        use std::os::windows::io::AsRawHandle;
        #[link(name = "kernel32")]
        extern "system" {
            fn DeviceIoControl(
                handle: isize,
                code: u32,
                input: *const std::ffi::c_void,
                input_len: u32,
                output: *mut std::ffi::c_void,
                output_len: u32,
                returned: *mut u32,
                overlapped: *mut std::ffi::c_void,
            ) -> i32;
        }
        let mut output = [0; 16];
        let mut returned = 0;
        let ok = unsafe {
            DeviceIoControl(
                self.0.as_raw_handle() as isize,
                code,
                input.as_ptr().cast(),
                input.len() as u32,
                output.as_mut_ptr().cast(),
                output.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error().to_string());
        }
        Ok((returned, output))
    }
}

#[cfg(windows)]
fn validate_machine() -> Result<()> {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };
    let mut key = HKEY::default();
    unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            w!("HARDWARE\\DESCRIPTION\\System\\BIOS"),
            0,
            KEY_READ,
            &mut key,
        )
    }
    .ok()
    .map_err(|e| e.to_string())?;
    let result = (|| {
        let read = |name| -> Result<String> {
            let mut data = [0u16; 128];
            let mut size = 256;
            let mut kind = REG_SZ;
            unsafe {
                RegQueryValueExW(
                    key,
                    name,
                    None,
                    Some(&mut kind),
                    Some(data.as_mut_ptr().cast()),
                    Some(&mut size),
                )
            }
            .ok()
            .map_err(|e| e.to_string())?;
            if kind != REG_SZ || size as usize > data.len() * 2 {
                return Err("Invalid BIOS identity value".into());
            }
            Ok(String::from_utf16_lossy(
                &data[..data.iter().position(|v| *v == 0).unwrap_or(data.len())],
            )
            .trim()
            .to_uppercase())
        };
        if read(w!("BaseBoardProduct"))? != "XINGYAO SERIES-P916F-PTL"
            || read(w!("SystemManufacturer"))? != "MECHREVO"
            || read(w!("BIOSVersion"))? != "1.00"
        {
            return Err("PM2 controls require the validated P916F-PTL BIOS 1.00 platform".into());
        }
        Ok(())
    })();
    unsafe {
        let _ = RegCloseKey(key);
    }
    result
}

#[cfg(windows)]
impl PortIo for PortDriver {
    fn read(&self, port: u16) -> Result<u8> {
        if port != DATA && port != STATUS_COMMAND {
            return Err("Port outside PTL PM2 channel".into());
        }
        let (n, data) = self.ioctl(READ_PORT, &port.to_le_bytes())?;
        if n != 1 {
            return Err("Invalid port read response length".into());
        }
        Ok(data[0])
    }
    fn write(&self, port: u16, value: u8) -> Result<()> {
        if port != DATA && port != STATUS_COMMAND {
            return Err("Port outside PTL PM2 channel".into());
        }
        let bytes = port.to_le_bytes();
        self.ioctl(WRITE_PORT, &[bytes[0], bytes[1], value])?;
        Ok(())
    }
}

#[cfg(not(windows))]
impl PortDriver {
    fn open() -> Result<Self> {
        Err("PTL PM2 requires Windows".into())
    }
}
#[cfg(not(windows))]
impl PortIo for PortDriver {
    fn read(&self, _: u16) -> Result<u8> {
        Err("Windows required".into())
    }
    fn write(&self, _: u16, _: u8) -> Result<()> {
        Err("Windows required".into())
    }
}

#[cfg(windows)]
mod gate {
    type Handle = *mut std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(attributes: *const u8, owner: i32, name: *const u16) -> Handle;
        fn WaitForSingleObject(handle: Handle, timeout: u32) -> u32;
        fn ReleaseMutex(handle: Handle) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }
    pub struct Gate(Handle);
    impl Gate {
        pub fn enter() -> super::Result<Self> {
            let name: Vec<u16> = "Local\\MechrevoPtlFanDiagnostic\0".encode_utf16().collect();
            let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
            if handle.is_null() {
                return Err("Cannot create PTL EC mutex".into());
            }
            match unsafe { WaitForSingleObject(handle, 3000) } {
                0 | 0x80 => Ok(Self(handle)),
                _ => {
                    unsafe {
                        CloseHandle(handle);
                    }
                    Err("PTL EC channel is busy".into())
                }
            }
        }
    }
    impl Drop for Gate {
        fn drop(&mut self) {
            unsafe {
                ReleaseMutex(self.0);
                CloseHandle(self.0);
            }
        }
    }
}
#[cfg(not(windows))]
mod gate {
    pub struct Gate;
    impl Gate {
        pub fn enter() -> super::Result<Self> {
            Err("Windows required".into())
        }
    }
}

struct Pmc<P: PortIo>(P);
impl<P: PortIo> Pmc<P> {
    fn status(&self) -> Result<u8> {
        match self.0.read(STATUS_COMMAND)? {
            255 => Err("PTL PM2 returned invalid status".into()),
            status => Ok(status),
        }
    }
    fn wait(&self, mask: u8, expected: u8) -> Result<()> {
        let start = Instant::now();
        while self.status()? & mask != expected {
            if start.elapsed() >= Duration::from_secs(1) {
                return Err("PTL PM2 handshake timed out".into());
            }
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }
    fn synchronize(&self) -> Result<()> {
        self.wait(IBF, 0)?;
        if self.status()? & OBF == 0 {
            return Ok(());
        }
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(5));
            let status = self.status()?;
            if status & IBF != 0 {
                return Err("Another client is using PTL PM2".into());
            }
            if status & OBF == 0 {
                return Ok(());
            }
        }
        self.0.read(DATA)?; // Consume at most one persistent stale response.
        for _ in 0..4 {
            thread::sleep(Duration::from_millis(5));
            if self.status()? & (IBF | OBF) != 0 {
                return Err("PTL PM2 is not idle after synchronization".into());
            }
        }
        Ok(())
    }
    fn exchange(&self, command: u8, parameter: u8, response: bool) -> Result<u8> {
        self.wait(IBF, 0)?;
        if self.status()? & OBF != 0 {
            self.synchronize()?;
        }
        // Never retry after writing any part of a command: a timeout may follow a successful write.
        if self.status()? & (IBF | OBF) != 0 {
            return Err("PTL PM2 became busy before command".into());
        }
        self.0.write(STATUS_COMMAND, command)?;
        self.wait(IBF, 0)?;
        self.0.write(DATA, parameter)?;
        if response {
            self.wait(OBF, OBF)?;
            self.0.read(DATA)
        } else {
            self.wait(IBF, 0)?;
            Ok(0)
        }
    }
    fn auto(&self) -> Result<()> {
        self.exchange(FAN_QUERY, FAN_AUTO, false).map(|_| ())
    }
    fn battery(&self) -> Result<BatteryState> {
        let enabled = self.exchange(BATTERY, BATTERY_ENABLED, true)?;
        let lower = self.exchange(BATTERY, BATTERY_LOWER, true)?;
        let upper = self.exchange(BATTERY, BATTERY_UPPER, true)?;
        if enabled > 1 || lower > 100 || upper > 100 {
            return Err("Invalid PTL battery response".into());
        }
        Ok(BatteryState {
            enabled: enabled == 1,
            lower,
            upper,
        })
    }
    fn threshold(&self, command: u8, value: u8) -> Result<()> {
        if self.exchange(command, value, true)? != value {
            return Err("Battery threshold echo mismatch".into());
        }
        Ok(())
    }
    fn write_battery(&self, value: BatteryState) -> Result<()> {
        self.exchange(BATTERY, BATTERY_DISABLE, false)?;
        self.threshold(BATTERY_SET_UPPER, value.upper)?;
        self.threshold(BATTERY_SET_LOWER, value.lower)?;
        if value.enabled {
            self.exchange(BATTERY, BATTERY_ENABLE, false)?;
        }
        if self.battery()? != value {
            return Err("Battery charge limit readback mismatch".into());
        }
        Ok(())
    }
    fn set_battery_limit(&self, limit: u8) -> Result<()> {
        let before = self.battery()?;
        // The second threshold requests ACTIVE DISCHARGE above it. Always leave it at 100%.
        let target = if limit == 100 {
            BatteryState {
                enabled: false,
                lower: 0,
                upper: 0,
            }
        } else {
            BatteryState {
                enabled: true,
                lower: limit,
                upper: 100,
            }
        };
        if before == target {
            return Ok(());
        }
        if let Err(error) = self.write_battery(target) {
            return Err(match self.write_battery(before) {
                Ok(()) => error,
                Err(rollback) => format!("{error}; battery rollback failed: {rollback}"),
            });
        }
        Ok(())
    }
    fn rpm(&self, fan: usize) -> Result<u16> {
        coherent_rpm(
            || self.exchange(FAN_QUERY, FAN_HIGH[fan], true),
            || self.exchange(FAN_QUERY, FAN_LOW[fan], true),
        )
    }
}

fn coherent_rpm(
    mut high: impl FnMut() -> Result<u8>,
    mut low: impl FnMut() -> Result<u8>,
) -> Result<u16> {
    let mut approximate = None;
    for _ in 0..3 {
        let a = high()?;
        let value = low()? as u16;
        let b = high()?;
        let rpm = value | ((b as u16) << 8);
        if rpm > 16000 {
            return Err("Invalid PTL fan tachometer reading".into());
        }
        if a == b {
            return Ok(rpm);
        }
        approximate = if a.abs_diff(b) <= 1 { Some(rpm) } else { None };
    }
    approximate.ok_or_else(|| "PTL fan RPM sample changed during read".into())
}

use super::fan_policy::{validate_targets, FanController, FanHardware, FanRpmGuard};
pub use super::fan_policy::{FanControlStatus, RpmCurvePoint, RpmFanCurves, RpmSmoothing};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BatteryState {
    pub enabled: bool,
    pub lower: u8,
    pub upper: u8,
}

struct NativeFans {
    pmc: Option<Pmc<PortDriver>>,
    rpm_guard: FanRpmGuard,
    epoch: Instant,
}
impl Default for NativeFans {
    fn default() -> Self {
        Self {
            pmc: None,
            rpm_guard: FanRpmGuard::default(),
            epoch: Instant::now(),
        }
    }
}
impl NativeFans {
    fn connect(&mut self) -> Result<&Pmc<PortDriver>> {
        if self.pmc.is_none() {
            let manifest =
                super::declarative::load_matched_manifest().ok_or("No matching PTL declaration")?;
            if !manifest.uses_ptl_pmc() {
                return Err("Current model does not declare PTL PM2 control".into());
            }
            let pmc = Pmc(PortDriver::open()?);
            pmc.synchronize()?;
            let mode = pmc.exchange(MODE_QUERY[0], MODE_QUERY[1], true)?;
            if !(1..=3).contains(&mode) {
                return Err("PTL PM2 firmware compatibility query failed".into());
            }
            self.pmc = Some(pmc);
        }
        Ok(self.pmc.as_ref().unwrap())
    }
    fn perform<T>(&mut self, action: impl FnOnce(&Pmc<PortDriver>) -> Result<T>) -> Result<T> {
        let result = action(self.connect()?);
        if result.is_err() {
            self.pmc = None;
        }
        result
    }
}
impl FanHardware for NativeFans {
    fn read_rpm(&mut self) -> Result<[u16; 2]> {
        self.perform(|pmc| Ok([pmc.rpm(0)?, pmc.rpm(1)?]))
    }
    fn automatic(&mut self) -> Result<()> {
        self.rpm_guard.reset();
        self.perform(Pmc::auto)
    }
    fn rpm(&mut self, targets: [u16; 2]) -> Result<[u16; 2]> {
        let actual = self.perform(|pmc| Ok([pmc.rpm(0)?, pmc.rpm(1)?]))?;
        let applied = self
            .rpm_guard
            .apply(targets, actual, self.epoch.elapsed())?;
        self.perform(|pmc| {
            for (fan, rpm) in applied.iter().enumerate() {
                pmc.exchange(FAN_RPM[fan], ((rpm + 50) / 100) as u8, false)?;
            }
            Ok(())
        })?;
        Ok(applied)
    }
    fn probe(&mut self) -> Result<()> {
        self.perform(|pmc| {
            let mode = pmc.exchange(MODE_QUERY[0], MODE_QUERY[1], true)?;
            if !(1..=3).contains(&mode) {
                return Err("Invalid PTL PM2 recovery response".into());
            }
            Ok(())
        })
    }
}
type Controller = FanController<NativeFans>;
static CONTROLLER: OnceLock<Mutex<Controller>> = OnceLock::new();
fn controller() -> &'static Mutex<Controller> {
    CONTROLLER.get_or_init(|| Mutex::new(Controller::new(NativeFans::default())))
}
static HEARTBEAT: OnceLock<()> = OnceLock::new();
fn start_heartbeat() {
    HEARTBEAT.get_or_init(|| {
        thread::Builder::new()
            .name("ptl-fan-safety".into())
            .spawn(|| {
                let mut sensor = PackageTemperatureReader::new();
                let epoch = Instant::now();
                let mut last = Instant::now();
                loop {
                    thread::sleep(Duration::from_secs(1));
                    let gap_healthy = last.elapsed() <= Duration::from_secs(5);
                    last = Instant::now();
                    let mut c = controller().lock();
                    if !c.needs_tick() {
                        continue;
                    }
                    match gate::Gate::enter() {
                        Ok(_gate) => {
                            let temp = sensor.read_celsius().and_then(|t| u8::try_from(t).ok());
                            c.tick(temp, epoch.elapsed(), gap_healthy);
                        }
                        Err(e) => c.note_failure(e),
                    }
                }
            })
            .expect("Cannot start PTL fan safety worker");
    });
}
pub fn set_manual_rpm(targets: [u16; 2]) -> Result<()> {
    validate_targets(targets)?;
    let temp = PackageTemperatureReader::new()
        .read_celsius()
        .and_then(|t| u8::try_from(t).ok());
    start_heartbeat();
    let mut c = controller().lock();
    let _gate = gate::Gate::enter()?;
    c.select_manual(targets, temp)
}
pub fn set_rpm_curves(curves: RpmFanCurves) -> Result<()> {
    curves.validate()?;
    let temp = PackageTemperatureReader::new()
        .read_celsius()
        .and_then(|t| u8::try_from(t).ok());
    start_heartbeat();
    let mut c = controller().lock();
    match gate::Gate::enter() {
        Ok(_gate) => c.select_curve(curves, temp),
        Err(e) => {
            c.queue_curve(curves)?;
            c.note_failure(e);
            Ok(())
        }
    }
}
pub fn set_smoothing(smoothing: RpmSmoothing) -> Result<()> {
    controller().lock().set_smoothing(smoothing)
}
/// Compatibility input: old percentages become explicit RPM targets; no PWM curve remains active.
pub fn set_curve(curve: Vec<(u8, u8)>) -> Result<()> {
    let mut curves = RpmFanCurves::from_legacy(&curve, &curve)?;
    curves.smoothing = status().smoothing;
    set_rpm_curves(curves)
}
pub fn restore_auto() -> Result<()> {
    start_heartbeat();
    let mut c = controller().lock();
    c.cancel();
    match gate::Gate::enter() {
        Ok(_gate) => c.automatic(),
        Err(e) => {
            c.note_failure(e.clone());
            Err(e)
        }
    }
}
pub fn suspend() {
    if let Some(c) = CONTROLLER.get() {
        let mut c = c.lock();
        if !c.needs_tick() {
            return;
        }
        match gate::Gate::enter() {
            Ok(_gate) => c.suspend(),
            Err(e) => {
                c.suspend_without_io();
                c.note_failure(e);
            }
        }
    }
}
pub fn resume() {
    if let Some(c) = CONTROLLER.get() {
        c.lock().resume();
    }
}
pub fn shutdown() {
    if let Some(c) = CONTROLLER.get() {
        let mut c = c.lock();
        if !c.needs_tick() {
            return;
        }
        c.cancel();
        match gate::Gate::enter() {
            Ok(_gate) => {
                let _ = c.shutdown();
            }
            Err(e) => c.note_failure(e),
        }
    }
}
/// Kept for existing diagnostic callers. Shutdown cancels future resumption.
pub fn suspend_or_exit() {
    shutdown();
}
pub fn status() -> FanControlStatus {
    controller().lock().status()
}
pub fn read_rpm() -> Result<[u16; 2]> {
    let mut c = controller().lock();
    let _gate = gate::Gate::enter()?;
    c.hardware.perform(|pmc| Ok([pmc.rpm(0)?, pmc.rpm(1)?]))
}
pub fn battery_state() -> Result<BatteryState> {
    let mut c = controller().lock();
    let _gate = gate::Gate::enter()?;
    c.hardware.perform(Pmc::battery)
}
pub fn set_battery_limit(limit: u8) -> Result<()> {
    if ![60, 80, 100].contains(&limit) {
        return Err("充电上限仅支持 60%、80%、100%".into());
    }
    let mut c = controller().lock();
    let _gate = gate::Gate::enter()?;
    c.hardware.perform(|pmc| pmc.set_battery_limit(limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};
    struct FakeState {
        battery: BatteryState,
        command: u8,
        output: VecDeque<u8>,
        writes: Vec<(u16, u8)>,
        fail_lower_once: bool,
        fail_command_once: bool,
    }
    #[derive(Clone)]
    struct FakePort(Rc<RefCell<FakeState>>);
    impl FakePort {
        fn new(battery: BatteryState) -> Self {
            Self(Rc::new(RefCell::new(FakeState {
                battery,
                command: 0,
                output: VecDeque::new(),
                writes: vec![],
                fail_lower_once: false,
                fail_command_once: false,
            })))
        }
    }
    impl PortIo for FakePort {
        fn read(&self, port: u16) -> Result<u8> {
            let mut s = self.0.borrow_mut();
            if port == STATUS_COMMAND {
                return Ok(if s.output.is_empty() { 0 } else { OBF });
            }
            s.output
                .pop_front()
                .ok_or_else(|| "No fake response".into())
        }
        fn write(&self, port: u16, value: u8) -> Result<()> {
            let mut s = self.0.borrow_mut();
            s.writes.push((port, value));
            if port == STATUS_COMMAND {
                s.command = value;
                if s.fail_command_once {
                    s.fail_command_once = false;
                    return Err("Command write failed".into());
                }
                return Ok(());
            }
            let reply = match (s.command, value) {
                (BATTERY, BATTERY_DISABLE) => {
                    s.battery = BatteryState {
                        enabled: false,
                        lower: 0,
                        upper: 0,
                    };
                    None
                }
                (BATTERY, BATTERY_ENABLE) => {
                    s.battery.enabled = true;
                    None
                }
                (BATTERY, BATTERY_ENABLED) => Some(u8::from(s.battery.enabled)),
                (BATTERY, BATTERY_LOWER) => Some(s.battery.lower),
                (BATTERY, BATTERY_UPPER) => Some(s.battery.upper),
                (BATTERY_SET_UPPER, _) => {
                    s.battery.upper = value;
                    Some(value)
                }
                (BATTERY_SET_LOWER, _) => {
                    s.battery.lower = value;
                    if s.fail_lower_once {
                        s.fail_lower_once = false;
                        return Err("Threshold write failed after acceptance".into());
                    }
                    Some(value)
                }
                _ => None,
            };
            if let Some(reply) = reply {
                s.output.push_back(reply);
            }
            Ok(())
        }
    }
    #[test]
    fn battery_failure_restores_previous_hardware_state() {
        let before = BatteryState {
            enabled: true,
            lower: 80,
            upper: 100,
        };
        let io = FakePort::new(before);
        io.0.borrow_mut().fail_lower_once = true;
        assert!(Pmc(io.clone()).set_battery_limit(60).is_err());
        assert_eq!(io.0.borrow().battery, before);
    }
    #[test]
    fn charge_limit_never_requests_active_discharge() {
        let io = FakePort::new(BatteryState {
            enabled: false,
            lower: 0,
            upper: 0,
        });
        let pmc = Pmc(io.clone());
        pmc.set_battery_limit(60).unwrap();
        assert_eq!(
            pmc.battery().unwrap(),
            BatteryState {
                enabled: true,
                lower: 60,
                upper: 100
            }
        );
        let writes = io.0.borrow().writes.clone();
        let threshold_index = writes
            .windows(2)
            .position(|w| w == [(STATUS_COMMAND, BATTERY_SET_UPPER), (DATA, 100)])
            .unwrap();
        let enable_index = writes
            .windows(2)
            .position(|w| w == [(STATUS_COMMAND, BATTERY), (DATA, BATTERY_ENABLE)])
            .unwrap();
        assert!(threshold_index < enable_index);
        pmc.set_battery_limit(100).unwrap();
        assert!(!pmc.battery().unwrap().enabled);
    }
    #[test]
    fn a_partially_sent_command_is_never_retried() {
        let io = FakePort::new(BatteryState {
            enabled: false,
            lower: 0,
            upper: 0,
        });
        io.0.borrow_mut().fail_command_once = true;
        assert!(Pmc(io.clone()).exchange(FAN_PWM, 128, false).is_err());
        assert_eq!(io.0.borrow().writes, [(STATUS_COMMAND, FAN_PWM)]);
    }
    #[test]
    fn tachometer_retries_torn_samples() {
        let mut high = [1, 2, 2, 2].into_iter();
        assert_eq!(
            coherent_rpm(|| Ok(high.next().unwrap()), || Ok(8)).unwrap(),
            520
        );
    }
    #[test]
    fn manual_input_rejected_without_hardware() {
        assert!(set_manual_rpm([0, 4000]).is_err());
        assert!(set_manual_rpm([4000, 8000]).is_err());
    }
}
