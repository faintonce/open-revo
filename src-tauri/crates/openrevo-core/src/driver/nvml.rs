//! NVML (NVIDIA Management Library) 动态绑定层
//!
//! 用于替代 `nvidia-smi` 子进程实现 GPU 实时遥测。
//!
//! ## 为什么需要它
//!
//! 原实现在后台线程里每 1500ms 执行一次 `nvidia-smi --query-gpu=...`。
//! 每次调用都是一次完整的 `CreateProcess` + NVML 初始化 + 进程退出，实测单次
//! 耗时 450~550ms，折合约 33% 单核占用（见 issue #121）。而 `nvidia-smi`
//! 本身只是 NVML 的命令行前端——直接调用 `nvml.dll` 可把单次采样压到微秒级，
//! 且不产生任何子进程。
//!
//! ## 生命周期
//!
//! 与 `nvidia-smi` 不同，持有 NVML 设备句柄会在驱动侧维持一个活动引用，
//! 可能阻止 dGPU 进入 RTD3 / D3Cold。因此本模块的约定是：
//!
//! - 由调用方（poller）**按需**创建 [`NvmlContext`]，不做全局常驻单例；
//! - 一旦采样返回 [`SampleError::Unreachable`]（设备已断电/不可达），
//!   调用方应立即 `drop` 上下文，触发 [`NvmlContext::shutdown`] 释放句柄；
//! - 恢复时重新 `open()` 即可，NVML 支持反复 init/shutdown。
//!
//! ## 失败语义
//!
//! 整个模块是可选的：`nvml.dll` 缺失（未装驱动）、符号缺失（驱动过旧）或
//! `nvmlInit_v2` 失败时，[`NvmlContext::open`] 返回 `Err`，调用方回退到
//! 原有的 `nvidia-smi` 路径，不构成硬依赖。

use std::ffi::c_void;

/// NVML 调用成功
const NVML_SUCCESS: i32 = 0;
/// 设备不存在（驱动未加载 / 无 NVIDIA 设备）
const NVML_ERROR_NOT_FOUND: i32 = 6;
/// 设备已丢失（PCI 不可达，典型场景为 dGPU 被切电 / D3Cold）
const NVML_ERROR_GPU_IS_LOST: i32 = 15;

/// 温度传感器：GPU 核心
const NVML_TEMPERATURE_GPU: u32 = 0;
/// 时钟域：图形核心
const NVML_CLOCK_GRAPHICS: u32 = 0;
/// 时钟域：显存
const NVML_CLOCK_MEM: u32 = 2;
/// 已弃用的最高 pstate 值，用于判断「是否处于空闲态」
pub const NVML_PSTATE_P8: u32 = 8;

#[repr(C)]
#[derive(Default, Clone, Copy)]
#[allow(dead_code)] // 字段布局由 NVML ABI 决定，未读取的字段也必须保留
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
#[allow(dead_code)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

/// 一次完整的 GPU 采样结果
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuSample {
    pub temp_c: u32,
    pub usage_pct: u32,
    pub core_clock_mhz: u32,
    pub mem_clock_mhz: u32,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
    pub power_w: f32,
    /// NVIDIA 性能状态，P0（最高性能）~ P15（最低）。P8 是典型的空闲档。
    pub pstate: u32,
}

impl GpuSample {
    /// 是否处于「空闲」态：P8 且 GPU 利用率接近 0。
    ///
    /// 仅作为**调整采样频率**的依据，不改变采样行为本身——即使空闲也照样采样，
    /// 只是把间隔拉长，避免为了判断「该不该读 GPU」而额外多读一次。
    pub fn is_idle(&self) -> bool {
        self.pstate >= NVML_PSTATE_P8 && self.usage_pct == 0
    }
}

/// 采样失败的原因，决定调用方如何处理
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleError {
    /// 设备不可达：应释放 NVML 上下文，暂停采样并等待恢复
    Unreachable(i32),
    /// 瞬时错误：保留上一次读数，下个周期重试
    Transient(i32),
}

type FnInitV2 = unsafe extern "C" fn() -> i32;
type FnShutdown = unsafe extern "C" fn() -> i32;
type FnGetHandleByIndexV2 = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
type FnGetTemperature = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;
type FnGetClockInfo = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;
type FnGetUtilization = unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> i32;
type FnGetMemoryInfo = unsafe extern "C" fn(*mut c_void, *mut NvmlMemory) -> i32;
type FnGetPowerUsage = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type FnGetName = unsafe extern "C" fn(*mut c_void, *mut i8, u32) -> i32;
type FnGetPerfState = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;

/// 已解析的函数表 + 动态库句柄（保持加载状态）
struct NvmlApi {
    _lib: libloading::Library,
    init_v2: FnInitV2,
    shutdown: FnShutdown,
    get_handle: FnGetHandleByIndexV2,
    get_temperature: FnGetTemperature,
    get_clock_info: FnGetClockInfo,
    get_utilization: FnGetUtilization,
    get_memory_info: FnGetMemoryInfo,
    get_power_usage: FnGetPowerUsage,
    get_name: FnGetName,
    get_perf_state: FnGetPerfState,
}

// SAFETY: NvmlApi 内只有 HMODULE（libloading::Library 已是 Send + Sync）与函数指针，
// 不含 Rust 侧可变状态，故 Send/Sync 由编译器自动推导。
// NVML 自身保证其 API 线程安全（不同设备句柄可并发访问）。

/// 从动态库解析单个符号并复制为裸函数指针。
///
/// SAFETY: 调用方必须保证 `T` 与该符号的真实 ABI 完全一致，否则调用时是未定义行为。
unsafe fn resolve<T: Copy>(lib: &libloading::Library, name: &[u8]) -> Result<T, String> {
    let sym: libloading::Symbol<'_, T> = lib.get(name).map_err(|e| {
        let printable = String::from_utf8_lossy(&name[..name.len().saturating_sub(1)]).to_string();
        format!("缺少符号 {printable}: {e}")
    })?;
    Ok(*sym)
}

/// 一个已初始化的 NVML 会话
///
/// 生命周期语义：`open()` 时执行 `nvmlInit_v2`，`drop` 时执行 `nvmlShutdown`。
/// 不要把它做成长期驻留的全局单例——那样会在设备空闲时持续持有驱动侧引用。
pub struct NvmlContext {
    api: NvmlApi,
    device: *mut c_void,
}

// SAFETY: device 是 NVML 内部的不透明句柄，随上下文一起移动；NvmlApi 已声明为 Send。
unsafe impl Send for NvmlContext {}

impl NvmlContext {
    /// 加载 `nvml.dll`、解析符号并初始化会话。
    ///
    /// 任一环节失败都返回 `Err(String)`（含可读原因），调用方应回退到 `nvidia-smi`。
    pub fn open() -> Result<Self, String> {
        let lib = unsafe { libloading::Library::new("nvml.dll") }
            .map_err(|e| format!("加载 nvml.dll 失败: {e}"))?;

        // 集中解析符号：缺任何一个都视为驱动过旧，整体回退
        unsafe {
            let init_v2: FnInitV2 = resolve(&lib, b"nvmlInit_v2\0")?;
            let shutdown: FnShutdown = resolve(&lib, b"nvmlShutdown\0")?;
            let get_handle: FnGetHandleByIndexV2 = resolve(&lib, b"nvmlDeviceGetHandleByIndex_v2\0")?;
            let get_temperature: FnGetTemperature = resolve(&lib, b"nvmlDeviceGetTemperature\0")?;
            let get_clock_info: FnGetClockInfo = resolve(&lib, b"nvmlDeviceGetClockInfo\0")?;
            let get_utilization: FnGetUtilization = resolve(&lib, b"nvmlDeviceGetUtilizationRates\0")?;
            let get_memory_info: FnGetMemoryInfo = resolve(&lib, b"nvmlDeviceGetMemoryInfo\0")?;
            let get_power_usage: FnGetPowerUsage = resolve(&lib, b"nvmlDeviceGetPowerUsage\0")?;
            let get_name: FnGetName = resolve(&lib, b"nvmlDeviceGetName\0")?;
            let get_perf_state: FnGetPerfState = resolve(&lib, b"nvmlDeviceGetPerformanceState\0")?;

            let api = NvmlApi {
                _lib: lib,
                init_v2,
                shutdown,
                get_handle,
                get_temperature,
                get_clock_info,
                get_utilization,
                get_memory_info,
                get_power_usage,
                get_name,
                get_perf_state,
            };

            let rc = (api.init_v2)();
            if rc != NVML_SUCCESS {
                return Err(format!("nvmlInit_v2 返回错误码 {rc}"));
            }

            let mut device: *mut c_void = std::ptr::null_mut();
            let rc = (api.get_handle)(0, &mut device);
            if rc != NVML_SUCCESS || device.is_null() {
                // 初始化成功但拿不到设备：必须回滚，否则引用计数不会归零
                let _ = (api.shutdown)();
                return Err(format!("nvmlDeviceGetHandleByIndex_v2 返回错误码 {rc}"));
            }

            Ok(Self { api, device })
        }
    }

    /// 设备名称，例如 `NVIDIA GeForce RTX 4070 Laptop GPU`
    pub fn name(&self) -> Option<String> {
        let mut buf = [0i8; 96];
        let rc = unsafe { (self.api.get_name)(self.device, buf.as_mut_ptr(), buf.len() as u32) };
        if rc != NVML_SUCCESS {
            return None;
        }
        let bytes: Vec<u8> = buf
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| *c as u8)
            .collect();
        if bytes.is_empty() {
            return None;
        }
        Some(String::from_utf8_lossy(&bytes).trim().to_string())
    }

    /// 采集一次完整快照。
    ///
    /// 温度是**状态判定的锚点**：只有它失败时才有必要区分「设备断电」和「瞬时故障」。
    /// 其余字段（功耗、显存、pstate）取不到时按默认值留空，不整体失败——避免因为
    /// 某个可选指标缺失就让整个遥测停摆。
    pub fn sample(&self) -> Result<GpuSample, SampleError> {
        let mut out = GpuSample::default();

        // 1. 温度（状态锚点，失败即判定设备状态）
        let mut temp = 0u32;
        let rc = unsafe { (self.api.get_temperature)(self.device, NVML_TEMPERATURE_GPU, &mut temp) };
        if rc == NVML_ERROR_GPU_IS_LOST || rc == NVML_ERROR_NOT_FOUND {
            return Err(SampleError::Unreachable(rc));
        }
        if rc != NVML_SUCCESS {
            return Err(SampleError::Transient(rc));
        }
        out.temp_c = temp;

        // 2. 核心 / 显存频率
        let mut clk = 0u32;
        if unsafe { (self.api.get_clock_info)(self.device, NVML_CLOCK_GRAPHICS, &mut clk) } == NVML_SUCCESS {
            out.core_clock_mhz = clk;
        }
        let mut mclk = 0u32;
        if unsafe { (self.api.get_clock_info)(self.device, NVML_CLOCK_MEM, &mut mclk) } == NVML_SUCCESS {
            out.mem_clock_mhz = mclk;
        }

        // 3. 利用率
        let mut util = NvmlUtilization::default();
        if unsafe { (self.api.get_utilization)(self.device, &mut util) } == NVML_SUCCESS {
            out.usage_pct = util.gpu;
        }

        // 4. 显存
        let mut mem = NvmlMemory::default();
        if unsafe { (self.api.get_memory_info)(self.device, &mut mem) } == NVML_SUCCESS {
            out.vram_total_mb = mem.total / (1024 * 1024);
            out.vram_used_mb = mem.used / (1024 * 1024);
        }

        // 5. 功耗（NVML 返回毫瓦）
        let mut mw = 0u32;
        if unsafe { (self.api.get_power_usage)(self.device, &mut mw) } == NVML_SUCCESS {
            out.power_w = mw as f32 / 1000.0;
        }

        // 6. 性能状态（用于空闲判定）
        let mut pstate = 0u32;
        if unsafe { (self.api.get_perf_state)(self.device, &mut pstate) } == NVML_SUCCESS {
            out.pstate = pstate;
        }

        Ok(out)
    }

    /// 显式结束会话。通常不必直接调用——`Drop` 会自动执行。
    pub fn shutdown(&self) {
        unsafe {
            let _ = (self.api.shutdown)();
        }
    }
}

impl Drop for NvmlContext {
    fn drop(&mut self) {
        self.shutdown();
    }
}
