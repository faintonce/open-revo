//! Read-only, per-sample NVML access for the existing Core GPU poller.

use libloading::Library;
use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::ptr;

const NVML_SUCCESS: i32 = 0;
const NVML_CLOCK_GRAPHICS: u32 = 0;
const NVML_CLOCK_MEM: u32 = 2;
const NVML_TEMPERATURE_GPU: u32 = 0;

type Device = *mut c_void;
type Init = unsafe extern "C" fn() -> i32;
type GetCount = unsafe extern "C" fn(*mut u32) -> i32;
type GetHandle = unsafe extern "C" fn(u32, *mut Device) -> i32;
type GetName = unsafe extern "C" fn(Device, *mut c_char, u32) -> i32;
type GetMetric = unsafe extern "C" fn(Device, u32, *mut u32) -> i32;
type GetPower = unsafe extern "C" fn(Device, *mut u32) -> i32;
type GetUtilization = unsafe extern "C" fn(Device, *mut Utilization) -> i32;
type GetMemory = unsafe extern "C" fn(Device, *mut Memory) -> i32;

#[repr(C)]
#[derive(Default)]
struct Utilization {
    gpu: u32,
    memory: u32,
}

#[repr(C)]
#[derive(Default)]
struct Memory {
    total: u64,
    free: u64,
    used: u64,
}

struct Session(Init);

impl Drop for Session {
    fn drop(&mut self) {
        // The shutdown symbol belongs to the library held by read_single_gpu.
        unsafe { (self.0)() };
    }
}

pub(super) struct Sample {
    pub name: String,
    pub graphics_mhz: u32,
    pub memory_mhz: u32,
    pub temperature_c: u32,
    pub usage_percent: u32,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub power_mw: u32,
}

pub(super) fn read_single_gpu() -> Option<Sample> {
    let path = PathBuf::from(std::env::var_os("SystemRoot")?).join("System32/nvml.dll");

    // SAFETY: Symbols use the NVML C ABI. Each output buffer matches its
    // declaration. Session is dropped before the library on every return path.
    unsafe {
        let lib = Library::new(path).ok()?;
        let init = lib.get::<Init>(b"nvmlInit_v2\0").ok()?;
        let shutdown = lib.get::<Init>(b"nvmlShutdown\0").ok()?;
        let get_count = lib.get::<GetCount>(b"nvmlDeviceGetCount_v2\0").ok()?;
        let get_handle = lib
            .get::<GetHandle>(b"nvmlDeviceGetHandleByIndex_v2\0")
            .ok()?;
        let get_name = lib.get::<GetName>(b"nvmlDeviceGetName\0").ok()?;
        let get_clock = lib.get::<GetMetric>(b"nvmlDeviceGetClockInfo\0").ok()?;
        let get_temperature = lib.get::<GetMetric>(b"nvmlDeviceGetTemperature\0").ok()?;
        let get_utilization = lib
            .get::<GetUtilization>(b"nvmlDeviceGetUtilizationRates\0")
            .ok()?;
        let get_memory = lib.get::<GetMemory>(b"nvmlDeviceGetMemoryInfo\0").ok()?;
        let get_power = lib.get::<GetPower>(b"nvmlDeviceGetPowerUsage\0").ok()?;

        if init() != NVML_SUCCESS {
            return None;
        }
        let _session = Session(*shutdown);

        let mut count = 0;
        let mut device = ptr::null_mut();
        if get_count(&mut count) != NVML_SUCCESS
            || count != 1
            || get_handle(0, &mut device) != NVML_SUCCESS
            || device.is_null()
        {
            return None;
        }

        let mut name = [0u8; 96];
        let mut graphics_mhz = 0;
        let mut memory_mhz = 0;
        let mut temperature_c = 0;
        let mut utilization = Utilization::default();
        let mut memory = Memory::default();
        let mut power_mw = 0;
        if get_name(device, name.as_mut_ptr().cast(), name.len() as u32) != NVML_SUCCESS
            || get_clock(device, NVML_CLOCK_GRAPHICS, &mut graphics_mhz) != NVML_SUCCESS
            || get_clock(device, NVML_CLOCK_MEM, &mut memory_mhz) != NVML_SUCCESS
            || get_temperature(device, NVML_TEMPERATURE_GPU, &mut temperature_c) != NVML_SUCCESS
            || get_utilization(device, &mut utilization) != NVML_SUCCESS
            || get_memory(device, &mut memory) != NVML_SUCCESS
            || get_power(device, &mut power_mw) != NVML_SUCCESS
        {
            return None;
        }

        let name = CStr::from_bytes_until_nul(&name)
            .ok()?
            .to_str()
            .ok()?
            .trim();
        if name.is_empty()
            || graphics_mhz > 10_000
            || memory_mhz > 30_000
            || !(1..125).contains(&temperature_c)
            || utilization.gpu > 100
            || memory.total == 0
            || memory.used > memory.total
            || power_mw >= 1_000_000
        {
            return None;
        }

        Some(Sample {
            name: name.to_owned(),
            graphics_mhz,
            memory_mhz,
            temperature_c,
            usage_percent: utilization.gpu,
            total_bytes: memory.total,
            used_bytes: memory.used,
            power_mw,
        })
    }
}
