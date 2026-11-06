//! Read-only Intel integrated-GPU telemetry through installed IGCL and PMT drivers.
//! No EC/MMIO access, overclocking calls, or bundled vendor DLLs.

use serde::Serialize;
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, Serialize)]
pub struct GpuTelemetry {
    pub name: String,
    pub temperature_c: Option<u32>,
    pub temperature_source: Option<&'static str>,
    pub core_frequency_mhz: Option<u32>,
    pub usage_percent: Option<u32>,
    /// GPU-domain interval average; package power already includes this domain.
    pub power_watts: Option<f32>,
    pub error: Option<String>,
}

impl GpuTelemetry {
    fn unavailable(error: impl Into<String>) -> Self {
        Self {
            error: Some(error.into()),
            ..Self::default()
        }
    }
}

/// Only called when a declared GPU telemetry consumer is active. The worker
/// releases the driver context after ten seconds without requests, including
/// when the main window closes. It never polls hardware on its own.
pub fn sample() -> GpuTelemetry {
    static REQUESTS: OnceLock<mpsc::Sender<mpsc::Sender<GpuTelemetry>>> = OnceLock::new();
    let requests = REQUESTS.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<mpsc::Sender<GpuTelemetry>>();
        std::thread::Builder::new()
            .name("intel-gpu-telemetry".into())
            .spawn(move || {
                let mut context: Option<native::Context> = None;
                let mut failed_at: Option<Instant> = None;
                let mut cached: Option<(Instant, GpuTelemetry)> = None;
                loop {
                    let reply = match rx.recv_timeout(Duration::from_secs(10)) {
                        Ok(reply) => reply,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            context = None;
                            cached = None;
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if let Some((at, value)) = &cached {
                        if at.elapsed() < Duration::from_secs(1)
                            || (context.is_none()
                                && failed_at
                                    .is_some_and(|at| at.elapsed() < Duration::from_secs(30)))
                        {
                            let _ = reply.send(value.clone());
                            continue;
                        }
                    }
                    let value = (|| {
                        if context.is_none() {
                            context = Some(native::Context::open()?);
                        }
                        context.as_mut().unwrap().sample()
                    })();
                    let value = match value {
                        Ok(value) => {
                            failed_at = None;
                            value
                        }
                        Err(error) => {
                            context = None;
                            failed_at = Some(Instant::now());
                            GpuTelemetry::unavailable(error)
                        }
                    };
                    cached = Some((Instant::now(), value.clone()));
                    let _ = reply.send(value);
                }
            })
            .expect("Intel GPU telemetry worker");
        tx
    });
    let (tx, rx) = mpsc::channel();
    if requests.send(tx).is_err() {
        return GpuTelemetry::unavailable("Intel GPU telemetry worker stopped");
    }
    rx.recv_timeout(Duration::from_secs(2))
        .unwrap_or_else(|_| GpuTelemetry::unavailable("Intel GPU telemetry timed out"))
}

fn rounded_reading(value: f64, maximum: f64) -> Option<u32> {
    (value.is_finite() && value > 0.0 && value <= maximum).then(|| value.round() as u32)
}

fn activity_percent(previous: Option<(f64, f64)>, current: (f64, f64)) -> Option<u32> {
    let (old_time, old_busy) = previous?;
    let elapsed = current.0 - old_time;
    let busy = current.1 - old_busy;
    if !elapsed.is_finite() || !busy.is_finite() || elapsed <= 0.0 || busy < 0.0 {
        return None;
    }
    Some((busy / elapsed * 100.0).round().clamp(0.0, 100.0) as u32)
}

#[derive(Default)]
struct PowerMeter {
    previous: Option<(f64, f64)>,
}

impl PowerMeter {
    fn sample(&mut self, seconds: Option<f64>, joules: Option<f64>) -> Option<f32> {
        let current = seconds.zip(joules).filter(|(time, energy)| {
            time.is_finite() && *time >= 0.0 && energy.is_finite() && *energy >= 0.0
        });
        // An unavailable counter clears the baseline. Resets and long gaps
        // establish a fresh baseline, so the next normal interval can recover.
        let previous = std::mem::replace(&mut self.previous, current);
        let (time, energy) = current?;
        let (old_time, old_energy) = previous?;
        let elapsed = time - old_time;
        let consumed = energy - old_energy;
        // IGCL timestamps have millisecond resolution. Never average across
        // a suspend/idle gap longer than the driver's ten-second context idle.
        if !(0.001..=10.0).contains(&elapsed) || consumed < 0.0 {
            return None;
        }
        let watts = (consumed / elapsed) as f32;
        watts.is_finite().then_some(watts)
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::{c_char, c_void};
    use std::mem::size_of;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE,
        KEY_READ, RRF_RT_REG_SZ,
    };

    // ABI: Intel's igcl_api.h, API 1.0, Version 0 structures (cdecl).
    // https://github.com/intel/drivers.gpu.control-library/blob/master/include/igcl_api.h
    type Handle = *mut c_void;
    type Init = unsafe extern "C" fn(*mut InitArgs, *mut Handle) -> u32;
    type Close = unsafe extern "C" fn(Handle) -> u32;
    type Enumerate = unsafe extern "C" fn(Handle, *mut u32, *mut Handle) -> u32;
    type Properties<T> = unsafe extern "C" fn(Handle, *mut T) -> u32;
    type Temperature = unsafe extern "C" fn(Handle, *mut f64) -> u32;

    #[repr(C)]
    #[derive(Default)]
    struct ApplicationId {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }
    #[repr(C)]
    #[derive(Default)]
    struct InitArgs {
        size: u32,
        version: u8,
        app_version: u32,
        flags: u32,
        supported_version: u32,
        application_id: ApplicationId,
    }
    #[repr(C)]
    struct DeviceProperties {
        size: u32,
        version: u8,
        device_id: Handle,
        device_id_size: u32,
        device_type: u32,
        functions: u32,
        driver_version: u64,
        firmware_version: [u64; 3],
        vendor: u32,
        device: u32,
        revision: u32,
        eus: u32,
        subslices: u32,
        slices: u32,
        name: [c_char; 100],
        adapter_flags: u32,
        frequency: u32,
        subsystem: u16,
        subsystem_vendor: u16,
        bdf: [u8; 3],
        xe_cores: u32,
        reserved: [u8; 108],
    }
    #[repr(C)]
    struct FrequencyProperties {
        size: u32,
        version: u8,
        domain: u32,
        can_control: u8,
        min: f64,
        max: f64,
    }
    #[repr(C)]
    struct FrequencyState {
        size: u32,
        version: u8,
        voltage: f64,
        request: f64,
        tdp: f64,
        efficient: f64,
        actual: f64,
        throttle_reasons: u32,
    }
    #[repr(C)]
    struct TemperatureProperties {
        size: u32,
        version: u8,
        kind: u32,
        max: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    union Value {
        i8_: i8,
        u8_: u8,
        i16_: i16,
        u16_: u16,
        i32_: i32,
        u32_: u32,
        i64_: i64,
        u64_: u64,
        f32_: f32,
        f64_: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Item {
        supported: u8,
        units: u32,
        kind: u32,
        value: Value,
    }
    impl Item {
        fn read(&self, units: u32) -> Option<f64> {
            if self.supported == 0 || self.units != units {
                return None;
            }
            // Read only the union member selected by the driver's data type.
            let value = unsafe {
                match self.kind {
                    0 => self.value.i8_ as f64,
                    1 => self.value.u8_ as f64,
                    2 => self.value.i16_ as f64,
                    3 => self.value.u16_ as f64,
                    4 => self.value.i32_ as f64,
                    5 => self.value.u32_ as f64,
                    6 => self.value.i64_ as f64,
                    7 => self.value.u64_ as f64,
                    8 => self.value.f32_ as f64,
                    9 => self.value.f64_,
                    _ => return None,
                }
            };
            value.is_finite().then_some(value)
        }
    }
    #[repr(C)]
    struct Psu {
        supported: u8,
        kind: u32,
        energy: Item,
        voltage: Item,
    }
    #[repr(C)]
    struct PowerTelemetry {
        size: u32,
        version: u8,
        timestamp: Item,
        gpu_energy: Item,
        gpu_voltage: Item,
        gpu_frequency: Item,
        gpu_temperature: Item,
        global_activity: Item,
        render_activity: Item,
        media_activity: Item,
        gpu_limits: [u8; 5],
        vram_energy: Item,
        vram_voltage: Item,
        vram_frequency: Item,
        vram_effective_frequency: Item,
        vram_read_counter: Item,
        vram_write_counter: Item,
        vram_temperature: Item,
        vram_limits: [u8; 5],
        card_energy: Item,
        psu: [Psu; 5],
        fan_speed: [Item; 5],
        gpu_vr_temp: Item,
        vram_vr_temp: Item,
        sa_vr_temp: Item,
        effective_clock: Item,
        overvoltage_percent: Item,
        power_percent: Item,
        temperature_percent: Item,
        vram_read: Item,
        vram_write: Item,
    }

    fn sized<T>() -> T {
        // All ABI members admit zero, including raw pointers and integer booleans.
        let mut value: T = unsafe { std::mem::zeroed() };
        unsafe {
            *(&mut value as *mut T).cast::<u32>() = size_of::<T>() as u32;
        }
        value
    }
    fn check(result: u32, operation: &str) -> Result<(), String> {
        if result == 0 {
            Ok(())
        } else {
            Err(format!("{operation}: IGCL error {result:#010x}"))
        }
    }
    fn enumerate(function: Enumerate, parent: Handle) -> Result<Vec<Handle>, String> {
        let mut count = 0;
        check(
            unsafe { function(parent, &mut count, std::ptr::null_mut()) },
            "enumerate count",
        )?;
        if count > 128 {
            return Err("Invalid IGCL enumeration count".into());
        }
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut handles = vec![std::ptr::null_mut(); count as usize];
        let capacity = count;
        check(
            unsafe { function(parent, &mut count, handles.as_mut_ptr()) },
            "enumerate handles",
        )?;
        if count > capacity {
            return Err("IGCL enumeration changed".into());
        }
        handles.truncate(count as usize);
        handles.retain(|handle| !handle.is_null());
        Ok(handles)
    }

    struct RegistryKey(HKEY);
    impl Drop for RegistryKey {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
    fn driver_paths() -> Result<Vec<String>, String> {
        let path: Vec<u16> =
            "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\0"
                .encode_utf16()
                .collect();
        let mut key = HKEY::default();
        unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(path.as_ptr()),
                0,
                KEY_READ,
                &mut key,
            )
        }
        .ok()
        .map_err(|e| format!("Display driver registry: {e}"))?;
        let key = RegistryKey(key);
        let value: Vec<u16> = "ControlApiPath\0".encode_utf16().collect();
        let mut paths = Vec::new();
        for index in 0..1024 {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            if unsafe {
                RegEnumKeyExW(
                    key.0,
                    index,
                    PWSTR(name.as_mut_ptr()),
                    &mut len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                )
            }
            .is_err()
            {
                break;
            }
            if len != 4
                || !name[..4]
                    .iter()
                    .all(|v| (b'0' as u16..=b'9' as u16).contains(v))
            {
                continue;
            }
            let mut data = [0u16; 2048];
            let mut bytes = std::mem::size_of_val(&data) as u32;
            if unsafe {
                RegGetValueW(
                    key.0,
                    PCWSTR(name.as_ptr()),
                    PCWSTR(value.as_ptr()),
                    RRF_RT_REG_SZ,
                    None,
                    Some(data.as_mut_ptr().cast()),
                    Some(&mut bytes),
                )
            }
            .is_ok()
            {
                let end = data.iter().position(|v| *v == 0).unwrap_or(data.len());
                let path = String::from_utf16_lossy(&data[..end]);
                if std::path::Path::new(&path).is_absolute() && !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }

    pub(super) struct Context {
        _library: libloading::Library,
        api: Handle,
        device: Handle,
        close: Close,
        telemetry: Option<Properties<PowerTelemetry>>,
        frequency_state: Properties<FrequencyState>,
        temperature_state: Temperature,
        frequencies: Vec<Handle>,
        temperatures: Vec<(u32, Handle)>,
        name: String,
        previous_activity: Option<(f64, f64)>,
        power_meter: PowerMeter,
        pmt_temperature: crate::hal::intel_pmt::GpuTemperature,
    }
    impl Drop for Context {
        fn drop(&mut self) {
            unsafe {
                (self.close)(self.api);
            }
        }
    }
    impl Context {
        pub(super) fn open() -> Result<Self, String> {
            let mut errors = Vec::new();
            for path in driver_paths()? {
                match Self::open_path(&path) {
                    Ok(context) => return Ok(context),
                    Err(error) => errors.push(error),
                }
            }
            Err(if errors.is_empty() {
                "No installed Intel IGCL runtime".into()
            } else {
                errors.join("; ")
            })
        }
        fn open_path(path: &str) -> Result<Self, String> {
            // Load an absolute HKLM driver path; dependencies use the driver and
            // system directories, never the working directory or PATH.
            let library: libloading::Library =
                unsafe { libloading::os::windows::Library::load_with_flags(path, 0x100 | 0x1000) }
                    .map_err(|e| e.to_string())?
                    .into();
            unsafe fn symbol<T: Copy>(
                library: &libloading::Library,
                name: &[u8],
            ) -> Result<T, String> {
                library
                    .get::<T>(name)
                    .map(|v| *v)
                    .map_err(|e| e.to_string())
            }
            let init: Init = unsafe { symbol(&library, b"ctlInit\0")? };
            let close: Close = unsafe { symbol(&library, b"ctlClose\0")? };
            let devices: Enumerate = unsafe { symbol(&library, b"ctlEnumerateDevices\0")? };
            let properties: Properties<DeviceProperties> =
                unsafe { symbol(&library, b"ctlGetDeviceProperties\0")? };
            let freq_enum: Enumerate = unsafe { symbol(&library, b"ctlEnumFrequencyDomains\0")? };
            let freq_props: Properties<FrequencyProperties> =
                unsafe { symbol(&library, b"ctlFrequencyGetProperties\0")? };
            let temp_enum: Enumerate = unsafe { symbol(&library, b"ctlEnumTemperatureSensors\0")? };
            let temp_props: Properties<TemperatureProperties> =
                unsafe { symbol(&library, b"ctlTemperatureGetProperties\0")? };
            let frequency_state = unsafe { symbol(&library, b"ctlFrequencyGetState\0")? };
            let temperature_state = unsafe { symbol(&library, b"ctlTemperatureGetState\0")? };
            let telemetry = unsafe { symbol(&library, b"ctlPowerTelemetryGet\0").ok() };
            let mut args: InitArgs = sized();
            args.app_version = 1 << 16;
            args.flags = 1; // CTL_INIT_FLAG_USE_LEVEL_ZERO: required for telemetry.
            let mut api = std::ptr::null_mut();
            check(unsafe { init(&mut args, &mut api) }, "ctlInit")?;
            let mut context = Self {
                _library: library,
                api,
                device: std::ptr::null_mut(),
                close,
                telemetry,
                frequency_state,
                temperature_state,
                frequencies: Vec::new(),
                temperatures: Vec::new(),
                name: String::new(),
                previous_activity: None,
                power_meter: Default::default(),
                pmt_temperature: Default::default(),
            };
            for device in enumerate(devices, api)? {
                let mut props: DeviceProperties = sized();
                if unsafe { properties(device, &mut props) } != 0
                    || props.device_type != 1
                    || props.vendor != 0x8086
                    || props.adapter_flags & 1 == 0
                {
                    continue;
                }
                context.device = device;
                let bytes: Vec<u8> = props
                    .name
                    .iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8)
                    .collect();
                context.name = String::from_utf8_lossy(&bytes).into_owned();
                break;
            }
            if context.device.is_null() {
                return Err("No Intel integrated graphics adapter exposed by IGCL".into());
            }
            for frequency in enumerate(freq_enum, context.device).unwrap_or_default() {
                let mut props: FrequencyProperties = sized();
                if unsafe { freq_props(frequency, &mut props) } == 0 && props.domain == 0 {
                    context.frequencies.push(frequency); // CTL_FREQ_DOMAIN_GPU; exclude media/memory.
                }
            }
            // Unsupported temperature enumeration must not discard working clocks.
            for temperature in enumerate(temp_enum, context.device).unwrap_or_default() {
                let mut props: TemperatureProperties = sized();
                if unsafe { temp_props(temperature, &mut props) } == 0 && props.kind <= 1 {
                    context.temperatures.push((props.kind, temperature));
                }
            }
            context
                .temperatures
                .sort_by_key(|(kind, _)| std::cmp::Reverse(*kind)); // GPU before global.
            Ok(context)
        }
        pub(super) fn sample(&mut self) -> Result<GpuTelemetry, String> {
            let mut result = GpuTelemetry {
                name: self.name.clone(),
                ..GpuTelemetry::default()
            };
            let mut succeeded = false;
            let mut energy_snapshot = (None, None);
            if let Some(telemetry) = self.telemetry {
                let mut data: PowerTelemetry = sized();
                if unsafe { telemetry(self.device, &mut data) } == 0 {
                    succeeded = true;
                    // CTL_UNITS_TIME_SECONDS and CTL_UNITS_ENERGY_JOULES.
                    // This is the GPU counter, never the package/card counter.
                    energy_snapshot = (data.timestamp.read(7), data.gpu_energy.read(6));
                    result.temperature_c = data
                        .gpu_temperature
                        .read(5)
                        .and_then(|v| rounded_reading(v, 150.0));
                    result.core_frequency_mhz = data
                        .gpu_frequency
                        .read(0)
                        .and_then(|v| rounded_reading(v, 10_000.0));
                    if let (Some(time), Some(busy)) =
                        (data.timestamp.read(7), data.global_activity.read(7))
                    {
                        result.usage_percent =
                            activity_percent(self.previous_activity, (time, busy));
                        self.previous_activity = Some((time, busy));
                    } else {
                        self.previous_activity = None;
                    }
                }
            }
            result.power_watts = self
                .power_meter
                .sample(energy_snapshot.0, energy_snapshot.1);
            if result.core_frequency_mhz.is_none() {
                for frequency in &self.frequencies {
                    let mut state: FrequencyState = sized();
                    if unsafe { (self.frequency_state)(*frequency, &mut state) } == 0 {
                        succeeded = true;
                        result.core_frequency_mhz = rounded_reading(state.actual, 10_000.0);
                        if result.core_frequency_mhz.is_some() {
                            break;
                        }
                    }
                }
            }
            if result.temperature_c.is_none() {
                for (_, temperature) in &self.temperatures {
                    let mut value = 0.0;
                    if unsafe { (self.temperature_state)(*temperature, &mut value) } == 0 {
                        succeeded = true;
                        result.temperature_c = rounded_reading(value, 150.0);
                        if result.temperature_c.is_some() {
                            break;
                        }
                    }
                }
            }
            if result.temperature_c.is_some() {
                result.temperature_source = Some("intel_igcl");
            } else if let Some(temperature) = self.pmt_temperature.read() {
                result.temperature_c = Some(temperature);
                result.temperature_source = Some("intel_pmt_gt_max");
                succeeded = true;
            }
            if !succeeded {
                return Err(
                    "Intel GPU telemetry unavailable; driver context will be retried".into(),
                );
            }
            Ok(result)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn igcl_abi_matches_x64_header() {
            assert_eq!(size_of::<InitArgs>(), 36);
            assert_eq!(size_of::<DeviceProperties>(), 320);
            assert_eq!(size_of::<Item>(), 24);
            assert_eq!(size_of::<FrequencyProperties>(), 32);
            assert_eq!(size_of::<FrequencyState>(), 56);
            assert_eq!(size_of::<TemperatureProperties>(), 24);
            assert_eq!(size_of::<PowerTelemetry>(), 1024);
            assert_eq!(std::mem::offset_of!(PowerTelemetry, gpu_temperature), 104);
            assert_eq!(std::mem::offset_of!(PowerTelemetry, global_activity), 128);
        }
        #[test]
        fn telemetry_respects_support_units_and_number_type() {
            let mut item = Item {
                supported: 1,
                units: 5,
                kind: 9,
                value: Value { f64_: 42.7 },
            };
            assert_eq!(
                item.read(5).and_then(|v| rounded_reading(v, 150.0)),
                Some(43)
            );
            item.supported = 0;
            assert_eq!(item.read(5), None);
            item.supported = 1;
            assert_eq!(item.read(0), None);
            item.kind = 5;
            item.value = Value { u32_: 900 };
            assert_eq!(item.read(5), Some(900.0));
            item.kind = 8;
            item.value = Value { f32_: 41.5 };
            assert_eq!(item.read(5), Some(41.5));
            item.kind = 9;
            item.value = Value { f64_: f64::NAN };
            assert_eq!(item.read(5), None);
        }
    }
}

#[cfg(not(windows))]
mod native {
    use super::*;
    pub(super) struct Context;
    impl Context {
        pub(super) fn open() -> Result<Self, String> {
            Err("Intel IGCL telemetry requires Windows".into())
        }
        pub(super) fn sample(&mut self) -> Result<GpuTelemetry, String> {
            Err("Intel IGCL telemetry requires Windows".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_driver_sentinels_instead_of_showing_fake_values() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 151.0] {
            assert_eq!(rounded_reading(value, 150.0), None);
        }
        assert_eq!(rounded_reading(899.6, 10_000.0), Some(900));
    }
    #[test]
    fn utilization_uses_elapsed_time_and_handles_reset() {
        assert_eq!(activity_percent(None, (1.0, 0.0)), None);
        assert_eq!(activity_percent(Some((1.0, 0.0)), (3.0, 0.5)), Some(25));
        assert_eq!(activity_percent(Some((3.0, 0.5)), (1.0, 0.1)), None);
        assert_eq!(activity_percent(Some((3.0, 0.5)), (4.0, 0.1)), None);
        assert_eq!(activity_percent(Some((3.0, 0.5)), (3.0, 0.5)), None);
    }
    #[test]
    fn power_uses_decimal_gpu_joules_and_actual_sample_interval() {
        let mut meter = PowerMeter::default();
        assert_eq!(meter.sample(Some(10.0), Some(100.0)), None);
        assert_eq!(meter.sample(Some(11.5), Some(107.5)), Some(5.0));
        assert_eq!(meter.sample(Some(14.0), Some(108.125)), Some(0.25));
        assert_eq!(meter.sample(Some(15.0), Some(108.125)), Some(0.0));
    }
    #[test]
    fn power_recovers_after_energy_timestamp_resets_and_suspend() {
        let mut meter = PowerMeter::default();
        assert_eq!(meter.sample(Some(10.0), Some(100.0)), None);
        assert_eq!(meter.sample(Some(11.0), Some(1.0)), None);
        assert_eq!(meter.sample(Some(12.0), Some(3.0)), Some(2.0));
        assert_eq!(meter.sample(Some(1.0), Some(4.0)), None);
        assert_eq!(meter.sample(Some(2.0), Some(6.0)), Some(2.0));
        assert_eq!(meter.sample(Some(20.0), Some(100.0)), None);
        assert_eq!(meter.sample(Some(21.0), Some(101.0)), Some(1.0));
        assert_eq!(meter.sample(Some(21.0), Some(101.0)), None);
    }
    #[test]
    fn power_unavailable_or_invalid_counters_clear_stale_baselines() {
        for invalid in [None, Some(-1.0), Some(f64::NAN), Some(f64::INFINITY)] {
            let mut meter = PowerMeter::default();
            assert_eq!(meter.sample(Some(1.0), Some(10.0)), None);
            assert_eq!(meter.sample(Some(2.0), invalid), None);
            assert_eq!(meter.sample(Some(3.0), Some(12.0)), None);
            assert_eq!(meter.sample(invalid, Some(13.0)), None);
            assert_eq!(meter.sample(Some(5.0), Some(14.0)), None);
            assert_eq!(meter.sample(Some(6.0), Some(15.0)), Some(1.0));
        }
    }
}
