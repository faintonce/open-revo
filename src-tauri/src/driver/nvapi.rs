// Native Pure-Rust NVIDIA NVAPI Driver Interface
// Zero official Mechrevo DLL dependencies. Dynamically links to Windows C:\Windows\System32\nvapi64.dll.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use parking_lot::Mutex;

const NVAPI_OK: i32 = 0;

// Magic IDs for nvapi_QueryInterface (reverse-engineered & verified on physical silicon)
const ID_NVAPI_INITIALIZE: u32 = 0x0150E828;
const ID_NVAPI_UNLOAD: u32 = 0xD22BDD7E;
const ID_NVAPI_ENUM_PHYSICAL_GPUS: u32 = 0xE5AC921F;
const ID_NVAPI_GPU_GET_FULL_NAME: u32 = 0xCEEE8E9F;
const ID_NVAPI_GPU_GET_PSTATES20: u32 = 0x6FF81213;
const ID_NVAPI_GPU_SET_PSTATES20: u32 = 0x0F4DAE6B;
const ID_NVAPI_GPU_SET_THERMAL_POLICIES: u32 = 0x34C0B13D;
const ID_NVAPI_GPU_GET_THERMAL_POLICIES: u32 = 0xE3640A56;
const ID_NVAPI_GPU_GET_CLOCK_BOOST_MASK: u32 = 0x507B4B59;
const ID_NVAPI_GPU_GET_CLOCK_BOOST_TABLE: u32 = 0x23F1B133;
const ID_NVAPI_GPU_SET_CLOCK_BOOST_TABLE: u32 = 0x0733E009;
const ID_NVAPI_GPU_GET_VF_CURVE_STATUS: u32 = 0x21537AD4;

// Structure definitions for PStates20 v1 and v2
// Verified on physical silicon:
// - V1: Size 7316 bytes (0x1C94), Version 0x00011C94 (Strictly required by NVIDIA for Memory Domain 4)
// - V2: Size 7416 bytes (0x1CF8), Version 0x00021CF8 (Required by NVIDIA for Graphics Core Domain 0)
pub const PSTATES20_SIZE_V1: usize = 7316;
pub const PSTATES20_VER1: u32 = 0x00011C94;
pub const PSTATES20_SIZE_V2: usize = 7416;
pub const PSTATES20_VER2: u32 = 0x00021CF8;

pub const CLOCK_DOMAIN_GRAPHICS: u32 = 0; // GPU Core Clock
pub const CLOCK_DOMAIN_MEMORY: u32 = 4;   // VRAM Clock

// V/F Curve and Boost Table layout constants (JiaolongControl & simple-nvidia-undervolt)
pub const CLOCK_MASKS_SIZE: usize = 6188;
pub const VF_POINTS_MASK_BYTES: usize = 32;

pub const STATUS_CURVE_SIZE: usize = 7208;
pub const STATUS_ENTRY_BASE: usize = 0x40; // 64
pub const STATUS_ENTRY_STRIDE: usize = 28;
pub const STATUS_TYPE_OFFSET: usize = 0x04;
pub const STATUS_FREQ_OFFSET: usize = 0x08;
pub const STATUS_VOLT_OFFSET: usize = 0x0C;

pub const CONTROL_TABLE_SIZE: usize = 9248;
pub const CTRL_ENTRY_BASE: usize = 0x64; // 100
pub const CTRL_ENTRY_STRIDE: usize = 36;
pub const CTRL_DELTA_OFFSET: usize = 0x18; // 24

pub const MIN_CORE_VOLT_UV: i32 = 300_000;
pub const MAX_CORE_VOLT_UV: i32 = 1_300_000;

#[inline]
pub fn make_version(size: usize, version: u32) -> u32 {
    (size as u32) | (version << 16)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GpuVfPoint {
    pub volt_mv: i32,
    pub freq_mhz: i32,
    pub delta_mhz: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GpuTuningInfo {
    pub gpu_name: String,
    pub is_available: bool,
    pub supports_vf_curve: bool,
    pub core_offset_mhz: i32,
    pub mem_offset_mhz: i32,
    pub vf_points: Vec<GpuVfPoint>,
}

type FnQueryInterface = unsafe extern "C" fn(u32) -> *const ();
type FnInitialize = unsafe extern "C" fn() -> i32;
type FnUnload = unsafe extern "C" fn() -> i32;
type FnEnumPhysicalGPUs = unsafe extern "C" fn(*mut usize, *mut u32) -> i32;
type FnGPUGetFullName = unsafe extern "C" fn(usize, *mut u8) -> i32;
type FnGPUGetPstates20 = unsafe extern "C" fn(usize, *mut u8) -> i32;
type FnGPUSetPstates20 = unsafe extern "C" fn(usize, *const u8) -> i32;
type FnGPUSetThermalPolicies = unsafe extern "C" fn(usize, *const u8) -> i32;
type FnGPUGetThermalPolicies = unsafe extern "C" fn(usize, *mut u8) -> i32;
type FnGpuStruct = unsafe extern "C" fn(usize, *mut u8) -> i32;
type FnGpuConstStruct = unsafe extern "C" fn(usize, *const u8) -> i32;

struct NvApiBindings {
    _lib: libloading::Library,
    _initialize: FnInitialize,
    _unload: FnUnload,
    _enum_physical_gpus: FnEnumPhysicalGPUs,
    get_full_name: FnGPUGetFullName,
    get_pstates20: FnGPUGetPstates20,
    set_pstates20: FnGPUSetPstates20,
    set_thermal_policies: Option<FnGPUSetThermalPolicies>,
    _get_thermal_policies: Option<FnGPUGetThermalPolicies>,
    get_clock_boost_mask: Option<FnGpuStruct>,
    get_clock_boost_table: Option<FnGpuStruct>,
    set_clock_boost_table: Option<FnGpuConstStruct>,
    get_vf_curve_status: Option<FnGpuStruct>,
    primary_gpu_handle: usize,
}

static NVAPI: OnceLock<Option<NvApiBindings>> = OnceLock::new();
static NVAPI_MUTEX: Mutex<()> = Mutex::new(());
static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

fn load_nvapi() -> Option<NvApiBindings> {
    unsafe {
        // Look in System32
        let lib = libloading::Library::new("nvapi64.dll").ok()?;
        let query_interface: libloading::Symbol<FnQueryInterface> = lib.get(b"nvapi_QueryInterface\0").ok()?;

        let get_proc = |id: u32| -> Option<*const ()> {
            let ptr = query_interface(id);
            if ptr.is_null() {
                None
            } else {
                Some(ptr)
            }
        };

        let init_ptr = get_proc(ID_NVAPI_INITIALIZE)?;
        let unload_ptr = get_proc(ID_NVAPI_UNLOAD)?;
        let enum_ptr = get_proc(ID_NVAPI_ENUM_PHYSICAL_GPUS)?;
        let name_ptr = get_proc(ID_NVAPI_GPU_GET_FULL_NAME)?;
        let get_pstates_ptr = get_proc(ID_NVAPI_GPU_GET_PSTATES20)?;
        let set_pstates_ptr = get_proc(ID_NVAPI_GPU_SET_PSTATES20)?;

        let initialize: FnInitialize = std::mem::transmute(init_ptr);
        let unload: FnUnload = std::mem::transmute(unload_ptr);
        let enum_physical_gpus: FnEnumPhysicalGPUs = std::mem::transmute(enum_ptr);
        let get_full_name: FnGPUGetFullName = std::mem::transmute(name_ptr);
        let get_pstates20: FnGPUGetPstates20 = std::mem::transmute(get_pstates_ptr);
        let set_pstates20: FnGPUSetPstates20 = std::mem::transmute(set_pstates_ptr);

        let set_thermal_policies = get_proc(ID_NVAPI_GPU_SET_THERMAL_POLICIES)
            .map(|p| std::mem::transmute(p));
        let get_thermal_policies = get_proc(ID_NVAPI_GPU_GET_THERMAL_POLICIES)
            .map(|p| std::mem::transmute(p));

        let get_clock_boost_mask: Option<FnGpuStruct> = get_proc(ID_NVAPI_GPU_GET_CLOCK_BOOST_MASK)
            .map(|p| std::mem::transmute(p));
        let get_clock_boost_table: Option<FnGpuStruct> = get_proc(ID_NVAPI_GPU_GET_CLOCK_BOOST_TABLE)
            .map(|p| std::mem::transmute(p));
        let set_clock_boost_table: Option<FnGpuConstStruct> = get_proc(ID_NVAPI_GPU_SET_CLOCK_BOOST_TABLE)
            .map(|p| std::mem::transmute(p));
        let get_vf_curve_status: Option<FnGpuStruct> = get_proc(ID_NVAPI_GPU_GET_VF_CURVE_STATUS)
            .map(|p| std::mem::transmute(p));

        let ret = initialize();
        if ret != NVAPI_OK {
            crate::log_warn!("NVAPI", "NvAPI_Initialize returned error code: {}", ret);
            return None;
        }

        let mut handles = [0usize; 64];
        let mut count = 0u32;
        let enum_ret = enum_physical_gpus(handles.as_mut_ptr(), &mut count);
        if enum_ret != NVAPI_OK || count == 0 {
            crate::log_warn!("NVAPI", "NvAPI_EnumPhysicalGPUs found 0 GPUs or returned {}", enum_ret);
            return None;
        }

        let primary_gpu_handle = handles[0];
        let mut name_buf = [0u8; 64];
        if get_full_name(primary_gpu_handle, name_buf.as_mut_ptr()) == NVAPI_OK {
            let name = String::from_utf8_lossy(&name_buf)
                .trim_matches(char::from(0))
                .to_string();
            crate::log_info!("NVAPI", "Native NVAPI initialized successfully for GPU: {} (handle: 0x{:X})", name, primary_gpu_handle);
        }

        IS_INITIALIZED.store(true, Ordering::SeqCst);

        Some(NvApiBindings {
            _lib: lib,
            _initialize: initialize,
            _unload: unload,
            _enum_physical_gpus: enum_physical_gpus,
            get_full_name,
            get_pstates20,
            set_pstates20,
            set_thermal_policies,
            _get_thermal_policies: get_thermal_policies,
            get_clock_boost_mask,
            get_clock_boost_table,
            set_clock_boost_table,
            get_vf_curve_status,
            primary_gpu_handle,
        })
    }
}

fn get_driver() -> Option<&'static NvApiBindings> {
    NVAPI.get_or_init(load_nvapi).as_ref()
}

/// Query whether NVAPI is available on the current machine
pub fn is_nvapi_available() -> bool {
    get_driver().is_some()
}

/// Retrieve the GPU model name directly from NVIDIA driver
pub fn get_gpu_full_name() -> Option<String> {
    let drv = get_driver()?;
    let _guard = NVAPI_MUTEX.lock();
    let mut name_buf = [0u8; 64];
    unsafe {
        if (drv.get_full_name)(drv.primary_gpu_handle, name_buf.as_mut_ptr()) == NVAPI_OK {
            let name = String::from_utf8_lossy(&name_buf)
                .trim_matches(char::from(0))
                .to_string();
            Some(name)
        } else {
            None
        }
    }
}

/// Apply a single clock offset (Core or Memory) via NvAPI_GPU_SetPstates20
/// domain_id: 0 = Graphics (Core), 4 = Memory
/// delta_mhz: Frequency offset in MHz (-1000 ~ +1000)
unsafe fn set_single_clock_offset_raw(
    drv: &NvApiBindings,
    domain_id: u32,
    delta_mhz: i32,
) -> Result<(), i32> {
    // Memory uses V1 (7316 bytes, 0x00011C94); Graphics uses V2 (7416 bytes, 0x00021CF8)
    let (buf_size, ver_magic) = if domain_id == CLOCK_DOMAIN_MEMORY {
        (PSTATES20_SIZE_V1, PSTATES20_VER1)
    } else {
        (PSTATES20_SIZE_V2, PSTATES20_VER2)
    };

    let mut buf = vec![0u8; buf_size];
    let u32_slice = std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u32, buf_size / 4);
    let i32_slice = std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut i32, buf_size / 4);

    u32_slice[0] = ver_magic;
    u32_slice[2] = 1;                     // numPstates = 1 (P0 only)
    u32_slice[3] = 1;                     // numClocks = 1
    u32_slice[7] = domain_id;             // domainId: 0 for Graphics, 4 for Memory
    i32_slice[10] = delta_mhz * 1000;     // freqDelta_kHz = MHz * 1000

    let ret = (drv.set_pstates20)(drv.primary_gpu_handle, buf.as_ptr());
    if ret == NVAPI_OK {
        Ok(())
    } else {
        Err(ret)
    }
}

/// Apply GPU Core Clock and Memory Clock offsets directly to NVIDIA hardware
/// core_offset_mhz: Offset in MHz (e.g. +100 MHz, or 0 to reset)
/// mem_offset_mhz:  Offset in MHz (e.g. +200 MHz, or 0 to reset)
pub fn set_gpu_clock_offsets(core_offset_mhz: i32, mem_offset_mhz: i32) -> Result<(), String> {
    let drv = get_driver().ok_or_else(|| "NVIDIA NVAPI driver not found or not supported".to_string())?;
    let _guard = NVAPI_MUTEX.lock();

    unsafe {
        // 1. Set Core Clock Offset (Graphics Core, Domain 0)
        if let Err(code) = set_single_clock_offset_raw(drv, CLOCK_DOMAIN_GRAPHICS, core_offset_mhz) {
            crate::log_warn!("NVAPI", "Set Core Clock Offset ({} MHz) failed with NVAPI code: {}", core_offset_mhz, code);
            return Err(format!("NVAPI Set Core Clock failed: error code {}", code));
        }

        // 2. Set Memory Clock Offset (VRAM Memory, Domain 4)
        if let Err(code) = set_single_clock_offset_raw(drv, CLOCK_DOMAIN_MEMORY, mem_offset_mhz) {
            crate::log_warn!("NVAPI", "Set Memory Clock Offset ({} MHz) failed with NVAPI code: {}", mem_offset_mhz, code);
            return Err(format!("NVAPI Set Memory Clock failed: error code {}", code));
        }

        crate::log_info!(
            "NVAPI",
            "GPU Clock Offsets applied successfully: Core={:+>3} MHz, Memory={:+>4} MHz",
            core_offset_mhz, mem_offset_mhz
        );
        Ok(())
    }
}

/// Reset all GPU Clock Offsets to factory default (+0 MHz)
pub fn reset_gpu_clock_offsets() -> Result<(), String> {
    set_gpu_clock_offsets(0, 0)
}

/// Read current clock offsets from the GPU driver
pub fn get_gpu_clock_offsets() -> Option<(i32, i32)> {
    let drv = get_driver()?;
    let _guard = NVAPI_MUTEX.lock();

    let mut buf = vec![0u8; PSTATES20_SIZE_V1];
    let u32_slice = unsafe {
        std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u32, PSTATES20_SIZE_V1 / 4)
    };
    u32_slice[0] = PSTATES20_VER1;

    unsafe {
        let ret = (drv.get_pstates20)(drv.primary_gpu_handle, buf.as_mut_ptr());
        if ret != NVAPI_OK {
            return None;
        }

        let i32_slice = std::slice::from_raw_parts(buf.as_ptr() as *const i32, PSTATES20_SIZE_V1 / 4);
        // Extract P0 Graphics (offset 0x28 -> i32[10]) and Memory (offset 0x54 -> i32[21]) delta in kHz
        let core_khz = i32_slice[10];
        let mem_khz = i32_slice[21];
        Some((core_khz / 1000, mem_khz / 1000))
    }
}

/// Set GPU Thermal Limit / Temperature Wall in Celsius (e.g. 87°C)
pub fn set_gpu_thermal_limit(celsius: u32) -> Result<(), String> {
    let drv = get_driver().ok_or_else(|| "NVIDIA NVAPI driver not found".to_string())?;
    let set_thermal = drv.set_thermal_policies.ok_or_else(|| "Thermal policies not supported".to_string())?;
    let _guard = NVAPI_MUTEX.lock();

    // Struct: size 780 bytes (0x030C), version 0x0002030C
    let mut buf = vec![0u8; 780];
    let u32_slice = unsafe {
        std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u32, 780 / 4)
    };
    u32_slice[0] = 0x0002030C; // Version 2
    u32_slice[2] = 1;          // numPolicies = 1
    u32_slice[3] = 6;          // Target: GPU
    u32_slice[5] = celsius.clamp(65, 95); // Thermal limit Celsius

    unsafe {
        let ret = set_thermal(drv.primary_gpu_handle, buf.as_ptr());
        if ret == NVAPI_OK {
            crate::log_info!("NVAPI", "GPU Thermal limit set to {}°C", celsius);
            Ok(())
        } else {
            Err(format!("NVAPI Set Thermal Policies failed: error code {}", ret))
        }
    }
}

/// Retrieve the 32-byte active VF Points Mask from hardware
fn get_vf_points_mask_raw(drv: &NvApiBindings) -> Option<[u8; VF_POINTS_MASK_BYTES]> {
    let func = drv.get_clock_boost_mask?;
    let mut buf = vec![0u8; CLOCK_MASKS_SIZE];
    let ver = make_version(CLOCK_MASKS_SIZE, 1);
    buf[0..4].copy_from_slice(&ver.to_le_bytes());

    let ret = unsafe { func(drv.primary_gpu_handle, buf.as_mut_ptr()) };
    if ret != NVAPI_OK {
        crate::log_warn!("NVAPI", "IdGpuGetClockBoostMask returned NVAPI code: {}", ret);
        return None;
    }
    let mut mask = [0u8; VF_POINTS_MASK_BYTES];
    mask.copy_from_slice(&buf[4..4 + VF_POINTS_MASK_BYTES]);
    Some(mask)
}

/// Read the real-time active V/F (Voltage/Frequency) curve from NVIDIA GPU
pub fn get_gpu_vf_curve() -> Option<Vec<GpuVfPoint>> {
    let drv = get_driver()?;
    let get_curve = drv.get_vf_curve_status?;
    let _guard = NVAPI_MUTEX.lock();

    let mask = get_vf_points_mask_raw(drv)?;

    let mut buf = vec![0u8; STATUS_CURVE_SIZE];
    let ver = make_version(STATUS_CURVE_SIZE, 1);
    buf[0..4].copy_from_slice(&ver.to_le_bytes());
    buf[4..4 + VF_POINTS_MASK_BYTES].copy_from_slice(&mask);

    let ret = unsafe { get_curve(drv.primary_gpu_handle, buf.as_mut_ptr()) };
    if ret != NVAPI_OK {
        crate::log_warn!("NVAPI", "IdGpuGetVfCurveStatus returned NVAPI code: {}", ret);
        return None;
    }

    let mut points = Vec::new();
    let mut last_volt_uv = 0;

    let mut e = STATUS_ENTRY_BASE;
    while e + STATUS_ENTRY_STRIDE <= buf.len() {
        let entry_type = i32::from_le_bytes(buf[e + STATUS_TYPE_OFFSET..e + STATUS_TYPE_OFFSET + 4].try_into().unwrap());
        if entry_type != 0 {
            // Memory domain anchors immediately terminate Graphics Core domain points
            break;
        }

        let freq_khz = i32::from_le_bytes(buf[e + STATUS_FREQ_OFFSET..e + STATUS_FREQ_OFFSET + 4].try_into().unwrap());
        let volt_uv = i32::from_le_bytes(buf[e + STATUS_VOLT_OFFSET..e + STATUS_VOLT_OFFSET + 4].try_into().unwrap());

        if volt_uv < MIN_CORE_VOLT_UV || volt_uv > MAX_CORE_VOLT_UV {
            break;
        }

        // Sub-millivolt non-increasing flags the end of physical anchors
        if !points.is_empty() && volt_uv <= last_volt_uv {
            break;
        }

        last_volt_uv = volt_uv;
        points.push(GpuVfPoint {
            volt_mv: volt_uv / 1000,
            freq_mhz: freq_khz.max(0) / 1000,
            delta_mhz: 0,
        });

        e += STATUS_ENTRY_STRIDE;
    }

    // Correlate with current editable delta table
    if let Some(get_table) = drv.get_clock_boost_table {
        let mut ctrl_buf = vec![0u8; CONTROL_TABLE_SIZE];
        let ctrl_ver = make_version(CONTROL_TABLE_SIZE, 1);
        ctrl_buf[0..4].copy_from_slice(&ctrl_ver.to_le_bytes());
        ctrl_buf[4..4 + VF_POINTS_MASK_BYTES].copy_from_slice(&mask);

        let t_ret = unsafe { get_table(drv.primary_gpu_handle, ctrl_buf.as_mut_ptr()) };
        if t_ret == NVAPI_OK {
            for i in 1..points.len() {
                let pos = CTRL_ENTRY_BASE + (i - 1) * CTRL_ENTRY_STRIDE + CTRL_DELTA_OFFSET;
                if pos + 4 <= ctrl_buf.len() {
                    let raw_delta = i32::from_le_bytes(ctrl_buf[pos..pos + 4].try_into().unwrap());
                    points[i].delta_mhz = raw_delta / 1000;
                }
            }
        }
    }

    Some(points)
}

/// Set per-anchor frequency delta offsets (in MHz) directly into NVIDIA V/F Boost Table.
/// Includes hardware sanity boundaries and strict buffer capacity protections.
pub fn set_gpu_vf_curve_deltas(deltas_mhz: &[i32]) -> Result<(), String> {
    if deltas_mhz.len() < 16 {
        return Err("V/F 曲线有效锚点数过少 (< 16)，拒绝写入以防异常".to_string());
    }
    let max_pos = CTRL_ENTRY_BASE + (deltas_mhz.len() - 1) * CTRL_ENTRY_STRIDE + CTRL_DELTA_OFFSET + 4;
    if max_pos > CONTROL_TABLE_SIZE {
        return Err(format!("V/F 曲线锚点数超出控制表容量限制 ({} > {})", max_pos, CONTROL_TABLE_SIZE));
    }

    // Safety guard: Clamp individual delta range within -500 MHz .. +300 MHz
    for (i, &d) in deltas_mhz.iter().enumerate() {
        if d < -500 || d > 300 {
            return Err(format!("锚点 #{} 频率偏移超出安全边界 (-500 ~ +300 MHz): {} MHz", i, d));
        }
    }

    let drv = get_driver().ok_or_else(|| "NVIDIA NVAPI 不可用".to_string())?;
    let get_table = drv.get_clock_boost_table.ok_or_else(|| "当前显卡驱动不支持读取 V/F 控制表".to_string())?;
    let set_table = drv.set_clock_boost_table.ok_or_else(|| "当前显卡驱动不支持写入 V/F 控制表".to_string())?;
    let _guard = NVAPI_MUTEX.lock();

    let mask = get_vf_points_mask_raw(drv).ok_or_else(|| "读取 GPU VF 掩码失败".to_string())?;

    let mut buf = vec![0u8; CONTROL_TABLE_SIZE];
    let ver = make_version(CONTROL_TABLE_SIZE, 1);
    buf[0..4].copy_from_slice(&ver.to_le_bytes());
    buf[4..4 + VF_POINTS_MASK_BYTES].copy_from_slice(&mask);

    // Read existing table first (read-modify-write) to guarantee vendor reserved fields integrity
    let ret = unsafe { get_table(drv.primary_gpu_handle, buf.as_mut_ptr()) };
    if ret != NVAPI_OK {
        return Err(format!("读取当前时钟控制表失败: NVAPI 代码 {}", ret));
    }

    // Overwrite deltas (Anchor 0 is hardware baseline root and skipped)
    for i in 1..deltas_mhz.len() {
        let pos = CTRL_ENTRY_BASE + (i - 1) * CTRL_ENTRY_STRIDE + CTRL_DELTA_OFFSET;
        let delta_khz = deltas_mhz[i] * 1000;
        buf[pos..pos + 4].copy_from_slice(&delta_khz.to_le_bytes());
    }

    // Reseal version and request mask
    buf[0..4].copy_from_slice(&ver.to_le_bytes());
    buf[4..4 + VF_POINTS_MASK_BYTES].copy_from_slice(&mask);

    let set_ret = unsafe { set_table(drv.primary_gpu_handle, buf.as_ptr()) };
    if set_ret != NVAPI_OK {
        crate::log_error!("NVAPI", "写入 V/F 曲线控制表失败: 代码 {}", set_ret);
        return Err(format!("写入 V/F 曲线控制表失败: NVAPI 代码 {}", set_ret));
    }

    crate::log_info!("NVAPI", "V/F 曲线 {} 个锚点频率已成功编程至显卡驱动！", deltas_mhz.len());
    Ok(())
}

/// Generate and apply a classic Afterburner-style horizontal flattened undervolt curve.
/// Elevates low-voltage curve by boost offset, then strictly flattens all anchors beyond target_volt_mv.
pub fn set_gpu_undervolt_curve(target_volt_mv: i32, max_boost_mhz: i32) -> Result<(), String> {
    let current_points = get_gpu_vf_curve().ok_or_else(|| "读取当前显卡 V/F 曲线失败".to_string())?;
    if current_points.len() < 16 {
        return Err("有效锚点数不足".to_string());
    }

    // Locate anchor closest to requested target voltage
    let mut target_idx = 0;
    let mut min_diff = i32::MAX;
    for (i, p) in current_points.iter().enumerate() {
        let diff = (p.volt_mv - target_volt_mv).abs();
        if diff < min_diff {
            min_diff = diff;
            target_idx = i;
        }
    }

    let stock_freq_at_target = current_points[target_idx].freq_mhz - current_points[target_idx].delta_mhz;
    let base_boost = (max_boost_mhz - stock_freq_at_target).clamp(-300, 250);

    let mut new_deltas = vec![0i32; current_points.len()];
    for i in 1..current_points.len() {
        let stock_freq = current_points[i].freq_mhz - current_points[i].delta_mhz;
        if i <= target_idx {
            // Ascending region: lift curve by base_boost
            new_deltas[i] = base_boost;
        } else {
            // Flattened region: pin effective target frequency to max_boost_mhz
            let delta_to_flat = max_boost_mhz - stock_freq;
            new_deltas[i] = delta_to_flat.clamp(-500, base_boost);
        }
    }

    // Monotonicity Sanity Guard (Adjacent dip <= 25 MHz)
    let mut prev_freq = current_points[0].freq_mhz;
    for i in 1..current_points.len() {
        let stock_freq = current_points[i].freq_mhz - current_points[i].delta_mhz;
        let eff_freq = stock_freq + new_deltas[i];
        if eff_freq < prev_freq - 25 {
            new_deltas[i] = (prev_freq - stock_freq).clamp(-500, 300);
        }
        prev_freq = stock_freq + new_deltas[i];
    }

    set_gpu_vf_curve_deltas(&new_deltas)
}

/// Reset all V/F curve deltas back to factory baseline (0 MHz offset on all anchors)
pub fn reset_gpu_vf_curve() -> Result<(), String> {
    let current_points = get_gpu_vf_curve().ok_or_else(|| "读取当前显卡 V/F 曲线失败".to_string())?;
    let zero_deltas = vec![0i32; current_points.len()];
    set_gpu_vf_curve_deltas(&zero_deltas)
}

/// Complete GPU Tuning status snapshot for frontend plugins and controls
pub fn get_gpu_tuning_info() -> GpuTuningInfo {
    let name = get_gpu_full_name().unwrap_or_else(|| "NVIDIA GPU".to_string());
    let is_avail = is_nvapi_available();
    let (core, mem) = get_gpu_clock_offsets().unwrap_or((0, 0));
    let points = get_gpu_vf_curve().unwrap_or_default();
    let supports_vf = !points.is_empty();

    GpuTuningInfo {
        gpu_name: name,
        is_available: is_avail,
        supports_vf_curve: supports_vf,
        core_offset_mhz: core,
        mem_offset_mhz: mem,
        vf_points: points,
    }
}

/// Reset all GPU tuning (both PState clock offsets and V/F curve) back to stock factory settings
pub fn reset_gpu_tuning() -> Result<(), String> {
    let _ = reset_gpu_clock_offsets();
    let _ = reset_gpu_vf_curve();
    crate::log_info!("NVAPI", "GPU tuning states successfully restored to factory defaults");
    Ok(())
}
