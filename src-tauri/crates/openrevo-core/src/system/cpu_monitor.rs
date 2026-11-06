use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct FILETIME {
    pub dw_low_date_time: u32,
    pub dw_high_date_time: u32,
}

type GetSystemTimesFn = unsafe extern "system" fn(
    lp_idle_time: *mut FILETIME,
    lp_kernel_time: *mut FILETIME,
    lp_user_time: *mut FILETIME,
) -> i32;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct ProcessorPowerInformation {
    number: u32,
    max_mhz: u32,
    current_mhz: u32,
    mhz_limit: u32,
    max_idle_state: u32,
    current_idle_state: u32,
}

type CallNtPowerInformationFn = unsafe extern "system" fn(
    information_level: i32,
    input_buffer: *const std::ffi::c_void,
    input_buffer_length: u32,
    output_buffer: *mut std::ffi::c_void,
    output_buffer_length: u32,
) -> i32;

type PdhOpenQueryWFn = unsafe extern "system" fn(
    sz_data_source: *const u16,
    dw_user_data: usize,
    ph_query: *mut isize,
) -> i32;

type PdhAddEnglishCounterWFn = unsafe extern "system" fn(
    h_query: isize,
    sz_full_counter_path: *const u16,
    dw_user_data: usize,
    ph_counter: *mut isize,
) -> i32;

type PdhCollectQueryDataFn = unsafe extern "system" fn(h_query: isize) -> i32;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct PdhFmtCountervalue {
    pub c_status: u32,
    pub double_value: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PdhFmtCountervalueItem {
    pub sz_name: *const u16,
    pub fmt_value: PdhFmtCountervalue,
}

type PdhGetFormattedCounterArrayWFn = unsafe extern "system" fn(
    h_counter: isize,
    dw_format: u32,
    lpdw_buffer_size: *mut u32,
    lpdw_item_count: *mut u32,
    p_item_buffer: *mut u8,
) -> i32;

type PdhGetFormattedCounterValueFn = unsafe extern "system" fn(
    h_counter: isize,
    dw_format: u32,
    lpdw_type: *mut u32,
    p_value: *mut PdhFmtCountervalue,
) -> i32;

pub struct CpuMonitor {
    current_mhz: AtomicU32,
    base_mhz: AtomicU32,
    usage_pct: AtomicU32,
    cpu_temp: AtomicI32,
    power_dw: AtomicU32,
}

impl CpuMonitor {
    pub fn new() -> Arc<Self> {
        let monitor = Arc::new(Self {
            current_mhz: AtomicU32::new(2200),
            base_mhz: AtomicU32::new(2200),
            usage_pct: AtomicU32::new(5),
            cpu_temp: AtomicI32::new(0),
            power_dw: AtomicU32::new(0),
        });

        let monitor_clone = Arc::clone(&monitor);
        thread::spawn(move || {
            let mut prev_idle = 0u64;
            let mut prev_kernel = 0u64;
            let mut prev_user = 0u64;

            // Dynamically load GetSystemTimes from kernel32.dll
            let get_system_times: Option<libloading::Symbol<GetSystemTimesFn>> = unsafe {
                libloading::Library::new("kernel32.dll")
                    .ok()
                    .and_then(|lib| {
                        let leaked: &'static libloading::Library = Box::leak(Box::new(lib));
                        leaked.get(b"GetSystemTimes").ok()
                    })
            };

            // Dynamically load CallNtPowerInformation from powrprof.dll
            let call_nt_power: Option<libloading::Symbol<CallNtPowerInformationFn>> = unsafe {
                libloading::Library::new("powrprof.dll")
                    .ok()
                    .and_then(|lib| {
                        let leaked: &'static libloading::Library = Box::leak(Box::new(lib));
                        leaked.get(b"CallNtPowerInformation").ok()
                    })
            };

            // Dynamically load pdh.dll for power & processor performance.
            let pdh_lib: Option<&'static libloading::Library> = unsafe {
                libloading::Library::new("pdh.dll")
                    .ok()
                    .map(|lib| Box::leak(Box::new(lib)) as &'static libloading::Library)
            };
            let (pdh_query, pdh_pwr_counter, pdh_perf_counter, pdh_collect, pdh_get_array, pdh_get_val) = if let Some(lib) = pdh_lib {
                unsafe {
                    let open_query: Option<libloading::Symbol<PdhOpenQueryWFn>> = lib.get(b"PdhOpenQueryW").ok();
                    let add_counter: Option<libloading::Symbol<PdhAddEnglishCounterWFn>> = lib.get(b"PdhAddEnglishCounterW").ok();
                    let collect_data: Option<libloading::Symbol<PdhCollectQueryDataFn>> = lib.get(b"PdhCollectQueryData").ok();
                    let get_array: Option<libloading::Symbol<PdhGetFormattedCounterArrayWFn>> = lib.get(b"PdhGetFormattedCounterArrayW").ok();
                    let get_val: Option<libloading::Symbol<PdhGetFormattedCounterValueFn>> = lib.get(b"PdhGetFormattedCounterValue").ok();

                    if let (Some(open), Some(add), Some(collect), Some(get_arr), Some(get_v)) = (open_query, add_counter, collect_data, get_array, get_val) {
                        let mut q = 0isize;
                        if open(std::ptr::null(), 0, &mut q) == 0 {
                            let mut c_pwr = 0isize;
                            let pwr_path: Vec<u16> = r"\Energy Meter(*)\Power"
                                .encode_utf16()
                                .chain(Some(0))
                                .collect();
                            let has_pwr = add(q, pwr_path.as_ptr(), 0, &mut c_pwr) == 0;

                            let mut c_perf = 0isize;
                            let perf_path: Vec<u16> = r"\Processor Information(_Total)\% Processor Performance"
                                .encode_utf16()
                                .chain(Some(0))
                                .collect();
                            let has_perf = add(q, perf_path.as_ptr(), 0, &mut c_perf) == 0;

                            collect(q);
                            (
                                Some(q),
                                if has_pwr { Some(c_pwr) } else { None },
                                if has_perf { Some(c_perf) } else { None },
                                Some(collect),
                                Some(get_arr),
                                Some(get_v),
                            )
                        } else {
                            (None, None, None, None, None, None)
                        }
                    } else {
                        (None, None, None, None, None, None)
                    }
                }
            } else {
                (None, None, None, None, None, None)
            };

            let _num_cpus = num_cpus();
            let is_ultrabook = crate::hal::platform_probe::is_ultrabook();
            let mut package_temperature = crate::hal::cpu_temperature::PackageTemperatureReader::new();

            loop {
                if is_ultrabook && !crate::system::telemetry::is_telemetry_active() {
                    // Ultrabook background suspension: zero CPU / PDH polling when OpenRevo is not in foreground!
                    thread::sleep(Duration::from_millis(3000));
                    continue;
                }

                // 1. Calculate CPU Usage via GetSystemTimes
                if let Some(ref get_times) = get_system_times {
                    let mut idle = FILETIME::default();
                    let mut kernel = FILETIME::default();
                    let mut user = FILETIME::default();
                    if unsafe { get_times(&mut idle, &mut kernel, &mut user) } != 0 {
                        let idle_val = filetime_to_u64(&idle);
                        let kernel_val = filetime_to_u64(&kernel);
                        let user_val = filetime_to_u64(&user);

                        if prev_idle > 0 {
                            let idle_diff = idle_val.saturating_sub(prev_idle);
                            let kernel_diff = kernel_val.saturating_sub(prev_kernel);
                            let user_diff = user_val.saturating_sub(prev_user);
                            let total = kernel_diff + user_diff;

                            if total > 0 {
                                let usage = ((total.saturating_sub(idle_diff)) as f64 / total as f64 * 100.0) as u32;
                                monitor_clone.usage_pct.store(usage.min(100), Ordering::SeqCst);
                            }
                        }

                        prev_idle = idle_val;
                        prev_kernel = kernel_val;
                        prev_user = user_val;
                    }
                }

                // 2. Read real-time CPU frequency via CallNtPowerInformation (ProcessorInformation = 11)
                if let Some(ref func) = call_nt_power {
                    let mut info = vec![ProcessorPowerInformation::default(); 128];
                    let buf_len = (info.len() * std::mem::size_of::<ProcessorPowerInformation>()) as u32;
                    let status = unsafe {
                        func(
                            11, // ProcessorInformation
                            std::ptr::null(),
                            0,
                            info.as_mut_ptr() as *mut std::ffi::c_void,
                            buf_len,
                        )
                    };
                    if status == 0 {
                        let valid: Vec<&ProcessorPowerInformation> = info.iter().filter(|p| p.current_mhz > 0).collect();
                        if !valid.is_empty() {
                            let avg_mhz: u64 = valid.iter().map(|p| p.current_mhz as u64).sum::<u64>() / valid.len() as u64;
                            if let Some(first) = valid.first() {
                                if first.max_mhz > 0 {
                                    monitor_clone.base_mhz.store(first.max_mhz, Ordering::SeqCst);
                                }
                            }
                            // Fallback if PDH is unavailable
                            if pdh_perf_counter.is_none() {
                                monitor_clone.current_mhz.store(avg_mhz as u32, Ordering::SeqCst);
                            }
                        }
                    }
                }

                // 3. Read CPU Package temperature independently of PDH availability.
                // Only use genuine sensor readings. Zero means unavailable to the UI.
                let temperature = package_temperature.read_celsius().or_else(|| {
                    crate::hal::get_hardware_driver().read_cpu_temp_sensor()
                        .ok()
                        .filter(|temp| *temp > 0 && *temp < 125)
                        .map(i32::from)
                });
                monitor_clone.cpu_temp.store(temperature.unwrap_or(0), Ordering::SeqCst);

                // 4. Read Energy Meter power & Processor Performance via PDH.
                if let (Some(q), Some(ref collect), Some(ref get_array)) = (
                    pdh_query,
                    pdh_collect.as_ref(),
                    pdh_get_array.as_ref(),
                ) {
                    unsafe {
                        collect(q);

                        // B. Energy Meter Power
                        if let Some(c) = pdh_pwr_counter {
                            let mut buf_size = 0u32;
                            let mut item_count = 0u32;
                            let _ = get_array(c, 0x00000200 /* PDH_FMT_DOUBLE */, &mut buf_size, &mut item_count, std::ptr::null_mut());
                            if buf_size > 0 {
                                let mut buf = vec![0u8; buf_size as usize];
                                let ret = get_array(c, 0x00000200, &mut buf_size, &mut item_count, buf.as_mut_ptr());
                                if ret == 0 && item_count > 0 {
                                    let item_size = std::mem::size_of::<PdhFmtCountervalueItem>();
                                    let mut pkg_power_mw = None;
                                    let mut max_power_mw = 0.0f64;

                                    for i in 0..item_count as usize {
                                        let offset = i * item_size;
                                        if offset + item_size <= buf.len() {
                                            let item_ptr = buf.as_ptr().add(offset) as *const PdhFmtCountervalueItem;
                                            let val = (*item_ptr).fmt_value.double_value;
                                            if val > max_power_mw {
                                                max_power_mw = val;
                                            }

                                            if !(*item_ptr).sz_name.is_null() {
                                                let mut len = 0;
                                                while *(*item_ptr).sz_name.add(len) != 0 && len < 256 {
                                                    len += 1;
                                                }
                                                let slice = std::slice::from_raw_parts((*item_ptr).sz_name, len);
                                                let name = String::from_utf16_lossy(slice).to_ascii_lowercase();
                                                if name.contains("pkg") {
                                                    pkg_power_mw = Some(val);
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    let chosen_mw = pkg_power_mw.unwrap_or(max_power_mw);
                                    if chosen_mw > 0.0 && chosen_mw < 500_000.0 {
                                        // Store in deciwatts (0.1W units)
                                        let dw = (chosen_mw / 100.0).round() as u32;
                                        monitor_clone.power_dw.store(dw, Ordering::SeqCst);
                                    }
                                }
                            }
                        }

                        // C. Dynamic CPU Clock Frequency via % Processor Performance counter
                        if let (Some(c), Some(ref get_val)) = (pdh_perf_counter, pdh_get_val.as_ref()) {
                            let mut val = PdhFmtCountervalue::default();
                            let mut val_type = 0u32;
                            let ret = get_val(c, 0x00000200 /* PDH_FMT_DOUBLE */, &mut val_type, &mut val);
                            if ret == 0 && val.c_status == 0 && val.double_value > 0.0 {
                                let base = monitor_clone.base_mhz.load(Ordering::SeqCst);
                                if base > 0 {
                                    let live_mhz = ((val.double_value / 100.0) * (base as f64)).round() as u32;
                                    if live_mhz >= 400 && live_mhz <= 7000 {
                                        monitor_clone.current_mhz.store(live_mhz, Ordering::SeqCst);
                                    }
                                }
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(1500));
            }
        });

        monitor
    }

    pub fn get_temp(&self) -> i32 {
        self.cpu_temp.load(Ordering::SeqCst)
    }

    pub fn get_current_mhz(&self) -> u32 {
        self.current_mhz.load(Ordering::SeqCst)
    }

    pub fn get_base_mhz(&self) -> u32 {
        self.base_mhz.load(Ordering::SeqCst)
    }

    pub fn get_usage(&self) -> u32 {
        self.usage_pct.load(Ordering::SeqCst)
    }

    pub fn is_intel_13_14_gen(&self) -> bool {
        is_intel_13_or_14_gen()
    }

    pub fn get_brand(&self) -> String {
        get_cached_cpu_brand()
    }

    pub fn get_power_w(&self) -> f32 {
        let dw = self.power_dw.load(Ordering::SeqCst);
        if dw > 0 {
            return dw as f32 / 10.0;
        }

        // Fallback dynamic package power estimation (for AMD Ryzen APUs or systems where Windows RAPL is unavailable)
        let is_amd = is_amd_cpu();
        let is_hx = is_desktop_replacement_hx();
        let usage = self.usage_pct.load(Ordering::SeqCst) as f32;
        let cur_mhz = self.current_mhz.load(Ordering::SeqCst) as f32;
        let base_mhz = self.base_mhz.load(Ordering::SeqCst).max(1000) as f32;

        let freq_ratio = (cur_mhz / base_mhz).clamp(0.6, 2.5);

        // Mobile APU/CPU Package power curve:
        // Idle (usage ~ 1-5%): ~5W - 8W
        // Full load turbo: 54W - 65W (APU / standard mobile), 82.5W (TDP base for 110W+ desktop-derived HX series)
        let base_idle = if is_amd { 5.5f32 } else { 8.0f32 };
        let max_tdp = if is_amd {
            if is_hx { 82.5f32 } else { 54.0f32 }
        } else {
            if is_hx { 95.0f32 } else { 65.0f32 }
        };

        let load_factor = (usage / 100.0).powf(1.15);
        let est_w = base_idle + (max_tdp - base_idle) * load_factor * (freq_ratio / 1.15).clamp(0.75, 1.35);

        (est_w * 10.0).round() / 10.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    Intel,
    Amd,
    Other,
}

/// Physical CPUID Vendor Probe (Zero OS String Guesswork)
pub fn get_cpu_vendor() -> CpuVendor {
    static VENDOR: std::sync::OnceLock<CpuVendor> = std::sync::OnceLock::new();
    *VENDOR.get_or_init(|| {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::__cpuid;
            let res = __cpuid(0);
            let mut vendor_bytes = [0u8; 12];
            vendor_bytes[0..4].copy_from_slice(&res.ebx.to_le_bytes());
            vendor_bytes[4..8].copy_from_slice(&res.edx.to_le_bytes());
            vendor_bytes[8..12].copy_from_slice(&res.ecx.to_le_bytes());
            let vendor_str = String::from_utf8_lossy(&vendor_bytes);
            if vendor_str == "AuthenticAMD" {
                return CpuVendor::Amd;
            } else if vendor_str == "GenuineIntel" {
                return CpuVendor::Intel;
            }
        }
        CpuVendor::Other
    })
}

pub fn is_amd_cpu() -> bool {
    get_cpu_vendor() == CpuVendor::Amd
}

pub fn is_intel_cpu() -> bool {
    get_cpu_vendor() == CpuVendor::Intel
}

/// Hardware Truth: Identifies heavy multi-core processors (>= 20 threads)
/// purely via CPU topology rather than string guessing.
pub fn is_high_core_count_cpu() -> bool {
    static IS_HIGH: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *IS_HIGH.get_or_init(|| {
        num_cpus() >= 20
    })
}

#[inline]
pub fn is_desktop_replacement_hx() -> bool {
    is_high_core_count_cpu()
}

static BRAND_CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

pub fn get_cached_cpu_brand() -> String {
    BRAND_CACHE
        .get_or_init(|| {
            let raw = get_cpu_brand_string();
            if raw.is_empty() {
                "Unknown CPU".to_string()
            } else {
                raw
            }
        })
        .clone()
}

pub fn get_cpu_brand_string() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::__cpuid;
        let mut brand_bytes = [0u8; 48];
        for (i, func) in [0x80000002u32, 0x80000003, 0x80000004].iter().enumerate() {
            let res = __cpuid(*func);
            let offset = i * 16;
            brand_bytes[offset..offset + 4].copy_from_slice(&res.eax.to_le_bytes());
            brand_bytes[offset + 4..offset + 8].copy_from_slice(&res.ebx.to_le_bytes());
            brand_bytes[offset + 8..offset + 12].copy_from_slice(&res.ecx.to_le_bytes());
            brand_bytes[offset + 12..offset + 16].copy_from_slice(&res.edx.to_le_bytes());
        }
        String::from_utf8_lossy(&brand_bytes)
            .trim_matches(char::from(0))
            .trim()
            .to_string()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        String::new()
    }
}

pub fn is_intel_13_or_14_gen() -> bool {
    if !is_intel_cpu() {
        return false;
    }
    let brand = get_cpu_brand_string().to_lowercase();
    if brand.contains("13th") || brand.contains("14th") {
        return true;
    }
    let gen_patterns = [
        "13980", "13950", "13900", "13850", "13700", "13650", "13620", "13600", "13500", "13450", "13420", "13400",
        "14900", "14700", "14650", "14600", "14500", "14450", "14400",
    ];
    gen_patterns.iter().any(|p| brand.contains(p))
}

pub fn parse_cpu_max_mhz(brand: &str) -> u32 {
    let b = brand.to_uppercase();

    // 1. Intel Raptor Lake Refresh (14th Gen Mobile & Desktop)
    if b.contains("14900") { return 5800; }
    if b.contains("14700K") { return 5600; }
    if b.contains("14700") { return 5500; } // 14700HX / 14700
    if b.contains("14650") { return 5200; } // 14650HX
    if b.contains("14600") { return 5300; } // 14600HX / 14600K
    if b.contains("14500") { return 4900; } // 14500HX
    if b.contains("14450") { return 4800; } // 14450HX
    if b.contains("14400") { return 4700; }

    // 2. Intel Raptor Lake (13th Gen Mobile & Desktop)
    if b.contains("13980") { return 5600; } // 13980HX
    if b.contains("13950") { return 5500; } // 13950HX
    if b.contains("13900") { return 5400; } // 13900HX / 13900H / 13900HK
    if b.contains("13700") { return 5000; } // 13700HX / 13700H
    if b.contains("13650") { return 4900; } // 13650HX
    if b.contains("13620") { return 4900; } // 13620H
    if b.contains("13600") { return 5100; } // 13600HX / 13600K
    if b.contains("13500") { return 4700; } // 13500HX / 13500H
    if b.contains("13450") { return 4600; } // 13450HX
    if b.contains("13420") { return 4600; } // 13420H
    if b.contains("13400") { return 4600; }

    // 3. Intel Alder Lake (12th Gen Mobile & Desktop)
    if b.contains("12950") || b.contains("12900") { return 5000; } // 12900HX / 12900H
    if b.contains("12850") || b.contains("12800") { return 4800; } // 12800HX
    if b.contains("12700") { return 4700; } // 12700H
    if b.contains("12650") { return 4700; } // 12650H
    if b.contains("12600") { return 4500; }
    if b.contains("12500") { return 4500; } // 12500H
    if b.contains("12450") { return 4400; } // 12450H
    if b.contains("12400") { return 4400; }

    // 4. Intel Core Ultra (Series 1 & Series 2)
    if b.contains("ULTRA") || b.contains("CORE(TM) ULTRA") {
        if b.contains("285") { return 5700; }
        if b.contains("265") { return 5500; }
        if b.contains("245") { return 5200; }
        if b.contains("258") || b.contains("256") { return 4800; }
        if b.contains("228") || b.contains("226") { return 4500; }
        if b.contains("185") { return 5100; } // Ultra 9 185H
        if b.contains("165") { return 5000; } // Ultra 7 165H
        if b.contains("155") { return 4800; } // Ultra 7 155H
        if b.contains("135") { return 4600; } // Ultra 5 135H
        if b.contains("125") { return 4500; } // Ultra 5 125H
        if b.contains("ULTRA 9") { return 5100; }
        if b.contains("ULTRA 7") { return 4800; }
        if b.contains("ULTRA 5") { return 4500; }
    }

    // 5. AMD Ryzen High-End, Mobile & Desktop
    if b.contains("9955") || b.contains("9950") { return 5700; }
    if b.contains("9900") { return 5600; }
    if b.contains("9700") { return 5500; }
    if b.contains("9600") { return 5400; }
    if b.contains("7950") { return 5700; }
    if b.contains("7900") { return 5600; }
    if b.contains("7945") { return 5400; } // 7945HX / 7945HX3D
    if b.contains("7940") || b.contains("8945") { return 5200; } // 7940HS / 8945HS
    if b.contains("7845") { return 5200; } // 7845HX
    if b.contains("7745") { return 5100; } // 7745HX
    if b.contains("7840") || b.contains("8845") { return 5100; } // 7840H / 8845H
    if b.contains("7735") || b.contains("6800") { return 4750; } // 7735H / 6800H
    if b.contains("7640") || b.contains("8645") { return 5000; }
    if b.contains("7535") || b.contains("6600") { return 4550; }
    if b.contains("7800X3D") { return 5000; }
    if b.contains("AI 9 HX") || b.contains("AI 9 370") || b.contains("AI 9 HX 375") { return 5100; }
    if b.contains("AI 9 365") { return 5000; }

    // 6. Generic tier-based heuristics (safe fallback, never blind 5800)
    if b.contains("I9") || b.contains("RYZEN 9") { return 5400; }
    if b.contains("I7") || b.contains("RYZEN 7") { return 5000; }
    if b.contains("I5") || b.contains("RYZEN 5") { return 4600; }
    if b.contains("I3") || b.contains("RYZEN 3") { return 4300; }

    // General safe standard frequency limit
    5000
}

pub fn get_cpu_max_mhz() -> u32 {
    // 1. Try CPUID Leaf 0x16 (if available and unmasked by hypervisor/VBS)
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::__cpuid;
        let res = __cpuid(0x16);
        let ebx = res.ebx as u32;
        if ebx >= 2000 && ebx <= 7000 {
            return ebx;
        }
    }

    // 2. Comprehensive official spec database matching
    let brand = get_cached_cpu_brand();
    let spec_max = parse_cpu_max_mhz(&brand);

    // 3. Dynamic adjustment: if live measured peak frequency is higher, adapt to it
    let peak_observed = GLOBAL_CPU_MONITOR.get().map(|m| m.current_mhz.load(Ordering::SeqCst)).unwrap_or(0);
    if peak_observed > spec_max && peak_observed <= 7000 {
        peak_observed
    } else {
        spec_max
    }
}

fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dw_high_date_time as u64) << 32) | (ft.dw_low_date_time as u64)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(24)
}

static GLOBAL_CPU_MONITOR: std::sync::OnceLock<Arc<CpuMonitor>> = std::sync::OnceLock::new();

pub fn get_global_cpu_monitor() -> Arc<CpuMonitor> {
    GLOBAL_CPU_MONITOR.get_or_init(|| CpuMonitor::new()).clone()
}

/// Returns the native hardware Junction Temperature limit (TjMax) of the host CPU.
/// Dynamically respects user/manifest config `cpu_tjmax`, or auto-detects architecture defaults:
/// 105°C for Intel Core Ultra 200 (Arrow Lake), Core Ultra 100 (Meteor Lake), Lunar Lake,
/// and 100°C standard baseline for legacy/standard Intel and AMD mobile processors.
pub fn get_cpu_native_tjmax() -> u8 {
    let cfg = crate::system::config::load_config();
    if let Some(tj) = cfg.cpu_tjmax {
        if (85..=115).contains(&tj) {
            return tj;
        }
    }
    static HARDWARE_TJMAX: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    *HARDWARE_TJMAX.get_or_init(detect_hardware_tjmax)
}

fn detect_hardware_tjmax() -> u8 {
    let cfg = crate::system::config::load_config();

    // 1. AMD CPU architecture: AMD generally uses literal Celsius in EC
    if is_amd_cpu() {
        if cfg.cpu_temp_target_max >= 105 {
            return cfg.cpu_temp_target_max;
        }
        return 100;
    }

    // 2. Intel CPU Microarchitecture & Model Recognition
    let brand = get_cached_cpu_brand().to_uppercase();

    // 2.1 Arrow Lake (Core Ultra 200 series, e.g. 285HX, 265HX, 245HX, 285K, 265K, 245K, 255H)
    // Official Intel specification: Maximum Junction Temperature = 105°C
    if brand.contains("ARROW LAKE")
        || (brand.contains("ULTRA") && (
            brand.contains("285") || brand.contains("265") || brand.contains("245")
            || brand.contains("258") || brand.contains("256") || brand.contains("228") || brand.contains("226")
        ))
    {
        crate::log_info!("CPU_TJMAX", "Detected Intel Arrow Lake / Core Ultra 200 Series: Native TjMax = 105°C (Brand: {})", brand);
        return 105;
    }

    // 2.2 Meteor Lake (Core Ultra 100 series, e.g. 185H, 165H, 155H, 135H, 125H)
    // Official Intel specification: Maximum Junction Temperature = 105°C
    if brand.contains("METEOR LAKE")
        || (brand.contains("ULTRA") && (
            brand.contains("185") || brand.contains("165") || brand.contains("155")
            || brand.contains("135") || brand.contains("125")
        ))
    {
        crate::log_info!("CPU_TJMAX", "Detected Intel Meteor Lake / Core Ultra 100 Series: Native TjMax = 105°C (Brand: {})", brand);
        return 105;
    }

    // 2.3 Lunar Lake (Core Ultra 200V series)
    if brand.contains("LUNAR LAKE") {
        crate::log_info!("CPU_TJMAX", "Detected Intel Lunar Lake Series: Native TjMax = 105°C (Brand: {})", brand);
        return 105;
    }

    // 2.4 Raptor Lake Refresh & Raptor Lake HX (14th & 13th Gen HX Mobile)
    // High-power HX processors where machine config or BIOS provides 105°C ceiling
    if is_intel_13_or_14_gen() && cfg.cpu_temp_target_max >= 105 {
        crate::log_info!("CPU_TJMAX", "Intel 13th/14th Gen HX configured with 105°C target max: Native TjMax = 105°C");
        return 105;
    }

    // 3. Default standard baseline for legacy & standard Intel Mobile CPUs
    100
}

static CPU_MONITOR: std::sync::OnceLock<std::sync::Arc<CpuMonitor>> = std::sync::OnceLock::new();

pub fn get_cpu_monitor() -> std::sync::Arc<CpuMonitor> {
    CPU_MONITOR.get_or_init(CpuMonitor::new).clone()
}



