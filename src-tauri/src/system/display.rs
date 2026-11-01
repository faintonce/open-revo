use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::sync::LazyLock;
use std::time::Duration;
use windows::core::PCSTR;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsA, ChangeDisplaySettingsExA, EnumDisplayDevicesA, EnumDisplaySettingsA,
    EnumDisplaySettingsExA, CDS_RESET, CDS_TYPE, CDS_UPDATEREGISTRY, DEVMODEA, DISPLAY_DEVICEA,
    DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE, DM_DISPLAYFREQUENCY,
    EDS_ROTATEDMODE, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE, ENUM_REGISTRY_SETTINGS,
};

use crate::system::query_cache::KeyedExpiringCache;

/// Cache key used when a query targets the primary display without naming it.
const PRIMARY_DISPLAY_KEY: &str = "\\\\.\\PRIMARY";

/// Probe result shared by the monitor and refresh-rate queries:
/// (width, height, current_hz, supported_rates_at_that_resolution).
type ModeInfo = (u32, u32, u32, Vec<u32>);

/// Enumerating every display mode costs ~22 ms on a 210-mode panel (measured on a
/// 2560x1600 240 Hz panel), and the telemetry heartbeat used to run it twice every
/// 1.5 s. The mode table only changes on resolution/refresh-rate switch or on a
/// monitor hot-plug, both of which trigger `invalidate_display_mode_cache`.
static MODE_INFO_CACHE: LazyLock<KeyedExpiringCache<String, Option<ModeInfo>>> =
    LazyLock::new(|| KeyedExpiringCache::new(Duration::from_secs(30)));

/// Drop every cached display-mode table. Call after changing resolution or refresh
/// rate, and on `WM_DISPLAYCHANGE`.
pub fn invalidate_display_mode_cache() {
    MODE_INFO_CACHE.invalidate_all();
}

/// Query current resolution, current refresh rate and every refresh rate offered at
/// that resolution for one display. `dev_name` of `None` targets the primary display.
/// Results are cached per display for [`MODE_INFO_CACHE`]'s TTL.
fn probe_mode_info(dev_name: Option<&str>) -> Option<ModeInfo> {
    let cache_key = dev_name.unwrap_or(PRIMARY_DISPLAY_KEY).to_string();
    MODE_INFO_CACHE.get_or_compute(cache_key, || probe_mode_info_uncached(dev_name))
}

fn probe_mode_info_uncached(dev_name: Option<&str>) -> Option<ModeInfo> {
    // The Win32 calls want a NUL-terminated PCSTR; keep it alive for the whole probe.
    let dev_cstr = dev_name.and_then(|name| CString::new(name).ok());

    unsafe {
        let mut dm_curr: DEVMODEA = std::mem::zeroed();
        dm_curr.dmSize = std::mem::size_of::<DEVMODEA>() as u16;

        let got_curr = match dev_cstr.as_ref() {
            Some(cstr) => {
                let pc_dev_name = PCSTR(cstr.as_ptr() as *const u8);
                EnumDisplaySettingsExA(pc_dev_name, ENUM_CURRENT_SETTINGS, &mut dm_curr, EDS_ROTATEDMODE)
                    .as_bool()
                    || EnumDisplaySettingsA(pc_dev_name, ENUM_CURRENT_SETTINGS, &mut dm_curr).as_bool()
                    || EnumDisplaySettingsA(pc_dev_name, ENUM_REGISTRY_SETTINGS, &mut dm_curr).as_bool()
            }
            None => {
                EnumDisplaySettingsA(None, ENUM_CURRENT_SETTINGS, &mut dm_curr).as_bool()
                    || EnumDisplaySettingsA(None, ENUM_REGISTRY_SETTINGS, &mut dm_curr).as_bool()
            }
        };

        if !got_curr || dm_curr.dmPelsWidth == 0 || dm_curr.dmPelsHeight == 0 {
            return None;
        }

        let target_width = dm_curr.dmPelsWidth;
        let target_height = dm_curr.dmPelsHeight;
        let current_hz = dm_curr.dmDisplayFrequency;

        let mut rates = BTreeSet::new();
        let mut mode_idx = 0u32;

        loop {
            let mut dm_mode: DEVMODEA = std::mem::zeroed();
            dm_mode.dmSize = std::mem::size_of::<DEVMODEA>() as u16;

            let got_mode = match dev_cstr.as_ref() {
                Some(cstr) => {
                    let pc_dev_name = PCSTR(cstr.as_ptr() as *const u8);
                    EnumDisplaySettingsExA(
                        pc_dev_name,
                        ENUM_DISPLAY_SETTINGS_MODE(mode_idx),
                        &mut dm_mode,
                        EDS_ROTATEDMODE,
                    )
                    .as_bool()
                        || EnumDisplaySettingsA(pc_dev_name, ENUM_DISPLAY_SETTINGS_MODE(mode_idx), &mut dm_mode)
                            .as_bool()
                }
                None => EnumDisplaySettingsA(None, ENUM_DISPLAY_SETTINGS_MODE(mode_idx), &mut dm_mode).as_bool(),
            };

            if !got_mode {
                break;
            }

            // Match regular or portrait-rotated resolution
            let matches_res = (dm_mode.dmPelsWidth == target_width && dm_mode.dmPelsHeight == target_height)
                || (dm_mode.dmPelsWidth == target_height && dm_mode.dmPelsHeight == target_width);

            if matches_res && dm_mode.dmDisplayFrequency > 0 {
                rates.insert(dm_mode.dmDisplayFrequency);
            }
            mode_idx += 1;
        }

        // Normalize typical Windows 59Hz reporting to standard 60Hz
        let normalized_curr_hz = if current_hz == 59 { 60 } else { current_hz };

        if rates.is_empty() && normalized_curr_hz > 0 {
            rates.insert(normalized_curr_hz);
        }

        Some((
            target_width,
            target_height,
            normalized_curr_hz,
            rates.into_iter().collect(),
        ))
    }
}


/// Information for an individual connected and active display monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMonitor {
    pub device_name: String,           // e.g. "\\.\DISPLAY1"
    pub friendly_name: String,         // e.g. "原厂内置屏 (京东方 2.5K)" or "外接电竞屏 (Xiaomi)"
    pub is_primary: bool,              // Whether this is the primary Windows desktop monitor
    pub is_internal: bool,             // Whether this is the laptop built-in screen
    pub width: u32,                    // Native width in pixels (e.g. 2560)
    pub height: u32,                   // Native height in pixels (e.g. 1600)
    pub current_hz: u32,               // Current active refresh rate (e.g. 240)
    pub supported_rates: Vec<u32>,     // Supported refresh rates at current resolution (e.g. [60, 240])
}

/// Enumerate all active displays and their supported refresh rate matrix
pub fn get_all_monitors() -> Vec<DisplayMonitor> {
    let mut monitors = Vec::new();

    unsafe {
        let mut dev_num = 0u32;
        let mut adapter_dev: DISPLAY_DEVICEA = std::mem::zeroed();
        adapter_dev.cb = std::mem::size_of::<DISPLAY_DEVICEA>() as u32;

        while EnumDisplayDevicesA(None, dev_num, &mut adapter_dev, 0).as_bool() {
            let state_flags = adapter_dev.StateFlags;
            let is_attached = (state_flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0;
            let is_primary = (state_flags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0;

            if is_attached {
                let dev_name = cstr_to_string(&adapter_dev.DeviceName);
                if !dev_name.is_empty() {
                    let dev_cstr = CString::new(dev_name.clone()).unwrap_or_default();
                    let pc_dev_name = PCSTR(dev_cstr.as_ptr() as *const u8);

                    // 1. Query child monitor device to get monitor friendly name / hardware ID
                    let mut monitor_name = String::new();
                    let mut monitor_id = String::new();
                    let mut mon_idx = 0u32;
                    while mon_idx < 8 {
                        let mut mon_dev: DISPLAY_DEVICEA = std::mem::zeroed();
                        mon_dev.cb = std::mem::size_of::<DISPLAY_DEVICEA>() as u32;
                        if !EnumDisplayDevicesA(pc_dev_name, mon_idx, &mut mon_dev, 0).as_bool() {
                            break;
                        }
                        let m_name = cstr_to_string(&mon_dev.DeviceString);
                        let m_id = cstr_to_string(&mon_dev.DeviceID);
                        let is_mon_attached = (mon_dev.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0;
                        if is_mon_attached || (!m_name.is_empty() && monitor_name.is_empty()) {
                            monitor_name = m_name;
                            monitor_id = m_id;
                            if is_mon_attached {
                                break;
                            }
                        }
                        mon_idx += 1;
                    }

                    // 2. Query current resolution, refresh rate and the supported rate matrix
                    //    (cached per display; see probe_mode_info)
                    if let Some((target_width, target_height, normalized_curr_hz, supported_rates)) =
                        probe_mode_info(Some(&dev_name))
                    {
                        // 4. Determine whether internal laptop screen and build friendly label
                        let is_internal = is_likely_internal_screen(&dev_name, &monitor_name, &monitor_id, is_primary);
                        let friendly_name = format_friendly_screen_name(
                            &dev_name,
                            &monitor_name,
                            &monitor_id,
                            target_width,
                            target_height,
                            is_internal,
                            is_primary,
                        );

                        monitors.push(DisplayMonitor {
                            device_name: dev_name,
                            friendly_name,
                            is_primary,
                            is_internal,
                            width: target_width,
                            height: target_height,
                            current_hz: normalized_curr_hz,
                            supported_rates,
                        });
                    }
                }
            }

            adapter_dev = std::mem::zeroed();
            adapter_dev.cb = std::mem::size_of::<DISPLAY_DEVICEA>() as u32;
            dev_num += 1;
        }
    }

    // Fallback: If no monitors enumerated (unlikely), provide virtual primary
    if monitors.is_empty() {
        let current_hz = get_refresh_rate();
        let supported = get_available_refresh_rates();
        monitors.push(DisplayMonitor {
            device_name: "\\\\.\\DISPLAY1".into(),
            friendly_name: "默认显示器 (2560x1600)".into(),
            is_primary: true,
            is_internal: true,
            width: 2560,
            height: 1600,
            current_hz,
            supported_rates: supported,
        });
    }

    monitors
}

/// Set refresh rate for a specific display device
pub fn set_monitor_refresh_rate(device_name: &str, hz: u32) -> Result<(), String> {
    unsafe {
        let dev_cstr = CString::new(device_name).map_err(|e| e.to_string())?;
        let pc_dev_name = PCSTR(dev_cstr.as_ptr() as *const u8);

        let mut dm: DEVMODEA = std::mem::zeroed();
        dm.dmSize = std::mem::size_of::<DEVMODEA>() as u16;

        let got_settings = EnumDisplaySettingsExA(
            pc_dev_name,
            ENUM_CURRENT_SETTINGS,
            &mut dm,
            EDS_ROTATEDMODE,
        )
        .as_bool()
            || EnumDisplaySettingsA(pc_dev_name, ENUM_CURRENT_SETTINGS, &mut dm).as_bool()
            || EnumDisplaySettingsA(pc_dev_name, ENUM_REGISTRY_SETTINGS, &mut dm).as_bool();

        if got_settings {
            dm.dmDisplayFrequency = hz;
            dm.dmFields |= DM_DISPLAYFREQUENCY;

            // Strategy 1: Save to registry AND apply dynamically immediately (CDS_UPDATEREGISTRY | CDS_RESET)
            let mut res = ChangeDisplaySettingsExA(
                pc_dev_name,
                Some(&dm),
                None,
                CDS_UPDATEREGISTRY | CDS_RESET,
                None,
            );

            // Strategy 2: If combined flags rejected by driver, try dynamic switch (CDS_RESET)
            if res.0 != 0 {
                res = ChangeDisplaySettingsExA(
                    pc_dev_name,
                    Some(&dm),
                    None,
                    CDS_RESET,
                    None,
                );
            }

            // Strategy 3: Dynamic change without reset flag
            if res.0 != 0 {
                res = ChangeDisplaySettingsExA(
                    pc_dev_name,
                    Some(&dm),
                    None,
                    CDS_TYPE(0),
                    None,
                );
            }

            if res.0 == 0 {
                // Notify Desktop Window Manager to re-evaluate and apply desktop layout
                let _ = ChangeDisplaySettingsExA(None, None, None, CDS_TYPE(0), None);
                // The mode table just changed; drop cached probe results for every display.
                invalidate_display_mode_cache();
                return Ok(());
            }
            return Err(format!(
                "ChangeDisplaySettingsEx failed on {} with code: {}",
                device_name, res.0
            ));
        }

        Err(format!(
            "EnumDisplaySettings failed on display device {}",
            device_name
        ))
    }
}

/// Helper to convert C null-terminated fixed i8 buffer to String
fn cstr_to_string(bytes: &[i8]) -> String {
    let u8_bytes: Vec<u8> = bytes.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
    String::from_utf8_lossy(&u8_bytes).trim().to_string()
}

/// Heuristic to detect whether a screen is the laptop's internal panel
fn is_likely_internal_screen(
    dev_name: &str,
    mon_name: &str,
    mon_id: &str,
    is_primary: bool,
) -> bool {
    let combined = format!("{} {} {}", mon_name, mon_id, dev_name).to_uppercase();
    // Typical internal OEM panel vendors
    if combined.contains("BOE")
        || combined.contains("CSOT")
        || combined.contains("CSO")
        || combined.contains("LGD")
        || combined.contains("AUO")
        || combined.contains("SHP")
        || combined.contains("EDP")
    {
        return true;
    }
    // Typically primary DISPLAY1 is internal on gaming laptops
    if dev_name.ends_with("1") && is_primary {
        return true;
    }
    false
}

/// Create a sleek, professional, human-readable display label
fn format_friendly_screen_name(
    _dev_name: &str,
    mon_name: &str,
    mon_id: &str,
    width: u32,
    height: u32,
    is_internal: bool,
    _is_primary: bool,
) -> String {
    let res_str = if width >= 3840 {
        "4K"
    } else if width >= 2560 {
        if height == 1600 { "2.5K 16:10" } else { "2K 16:9" }
    } else if width >= 1920 {
        "1080P"
    } else {
        ""
    };

    let mut brand = String::new();
    let upper = format!("{} {}", mon_name, mon_id).to_uppercase();

    if upper.contains("BOE") {
        brand = "京东方 BOE".into();
    } else if upper.contains("XIAOMI") || upper.contains("REDMI") || upper.contains("XMI") {
        brand = "小米 / Redmi".into();
    } else if upper.contains("ASUS") || upper.contains("ROG") {
        brand = "华硕 ROG".into();
    } else if upper.contains("AOC") {
        brand = "AOC".into();
    } else if upper.contains("DELL") || upper.contains("ALIENWARE") {
        brand = "戴尔 Dell".into();
    } else if upper.contains("LG") {
        brand = "LG UltraGear".into();
    } else if upper.contains("SAMSUNG") || upper.contains("SEC") {
        brand = "三星 Samsung".into();
    } else if !mon_name.is_empty() && mon_name != "Generic PnP Monitor" {
        brand = mon_name.to_string();
    }

    if is_internal {
        if !brand.is_empty() {
            format!("{} [{}]", brand, res_str)
        } else {
            format!("内置屏 [{}]", res_str)
        }
    } else {
        if !brand.is_empty() {
            format!("{} [{}]", brand, res_str)
        } else {
            format!("外接显示器 [{}]", res_str)
        }
    }
}

// ----------------------------------------------------------------------------
// Legacy Compatible Single-Display API
// ----------------------------------------------------------------------------

/// Get current primary display refresh rate (Hz)
pub fn get_refresh_rate() -> u32 {
    unsafe {
        let mut dm: DEVMODEA = std::mem::zeroed();
        dm.dmSize = std::mem::size_of::<DEVMODEA>() as u16;

        if EnumDisplaySettingsA(None, ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
            return dm.dmDisplayFrequency;
        }
        60
    }
}

/// Set primary display refresh rate (Hz)
pub fn set_refresh_rate(hz: u32) -> Result<(), String> {
    unsafe {
        let mut dm: DEVMODEA = std::mem::zeroed();
        dm.dmSize = std::mem::size_of::<DEVMODEA>() as u16;

        if EnumDisplaySettingsA(None, ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
            dm.dmDisplayFrequency = hz;
            dm.dmFields |= DM_DISPLAYFREQUENCY;

            let res = ChangeDisplaySettingsA(Some(&dm), CDS_UPDATEREGISTRY);
            if res.0 == 0 {
                invalidate_display_mode_cache();
                return Ok(());
            }
            return Err(format!("ChangeDisplaySettings failed with code: {}", res.0));
        }
        Err("EnumDisplaySettings failed to get current settings".into())
    }
}

/// Enumerate all available refresh rates supported on primary display
pub fn get_available_refresh_rates() -> Vec<u32> {
    probe_mode_info(None)
        .map(|(_, _, _, rates)| rates)
        .unwrap_or_else(|| vec![60])
}

// ----------------------------------------------------------------------------
// Screen Brightness Controls (Native Win32 LCD DeviceIoControl, 0 PowerShell)
// ----------------------------------------------------------------------------

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct DisplayBrightness {
    uc_display_policy: u8,
    uc_ac_brightness: u8,
    uc_dc_brightness: u8,
}

const IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS: u32 = 0x00230498;
const IOCTL_VIDEO_SET_DISPLAY_BRIGHTNESS: u32 = 0x0023049C;
const GENERIC_READ_WRITE: u32 = 0xC0000000;
const FILE_SHARE_READ_WRITE: u32 = 3;
const OPEN_EXISTING: u32 = 3;

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileA(
        lpFileName: *const u8,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut std::ffi::c_void,
    ) -> isize;
    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    fn DeviceIoControl(
        hDevice: isize,
        dwIoControlCode: u32,
        lpInBuffer: *const std::ffi::c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut std::ffi::c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut std::ffi::c_void,
    ) -> i32;
}

/// Get current laptop display brightness (0..100) via native Win32 IOCTL
pub fn get_brightness() -> Result<u8, String> {
    unsafe {
        let name = b"\\\\.\\LCD\0";
        let h = CreateFileA(
            name.as_ptr(),
            GENERIC_READ_WRITE,
            FILE_SHARE_READ_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );

        if h == -1 || h == 0 {
            return Err("Failed to open \\\\.\\LCD device handle".into());
        }

        let mut data = DisplayBrightness::default();
        let mut bytes_returned = 0u32;
        let ok = DeviceIoControl(
            h,
            IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS,
            std::ptr::null(),
            0,
            &mut data as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<DisplayBrightness>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        CloseHandle(h as *mut std::ffi::c_void);

        if ok != 0 {
            let val = data.uc_ac_brightness.max(data.uc_dc_brightness).min(100);
            Ok(val)
        } else {
            Err("IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS failed".into())
        }
    }
}

/// Set laptop display brightness (0..100) via native Win32 IOCTL
pub fn set_brightness(level: u8) -> Result<(), String> {
    let clamped = level.min(100);
    unsafe {
        let name = b"\\\\.\\LCD\0";
        let h = CreateFileA(
            name.as_ptr(),
            GENERIC_READ_WRITE,
            FILE_SHARE_READ_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );

        if h == -1 || h == 0 {
            return Err("Failed to open \\\\.\\LCD device handle".into());
        }

        let mut in_data = DisplayBrightness {
            uc_display_policy: 0,
            uc_ac_brightness: clamped,
            uc_dc_brightness: clamped,
        };
        let mut bytes_returned = 0u32;
        let ok = DeviceIoControl(
            h,
            IOCTL_VIDEO_SET_DISPLAY_BRIGHTNESS,
            &mut in_data as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<DisplayBrightness>() as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        CloseHandle(h as *mut std::ffi::c_void);

        if ok != 0 {
            Ok(())
        } else {
            Err("IOCTL_VIDEO_SET_DISPLAY_BRIGHTNESS failed".into())
        }
    }
}
