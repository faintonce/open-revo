// OpenRevo - Display Color Calibration & Management Engine (ICM / WCS)
// Hybrid Windows Color Management shell: synchronizes with Windows Color System
// (mscms.dll & ICM registry) via single-slot associations and drives GPU hardware LUT
// via GDI per monitor for instant 0-latency response and 100% native uncalibrated fallback.
// Fully aligns with Windows 11 "Settings > System > Display > Color Management".

use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::Win32::Graphics::Gdi::HDC;

#[repr(C)]
pub struct GammaRamp {
    pub red: [u16; 256],
    pub green: [u16; 256],
    pub blue: [u16; 256],
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateDCW(
        pwsz_driver: *const u16,
        pwsz_device: *const u16,
        pwsz_port: *const u16,
        p_data: *const std::ffi::c_void,
    ) -> HDC;
    fn DeleteDC(hdc: HDC) -> windows::Win32::Foundation::BOOL;
    fn SetDeviceGammaRamp(hdc: HDC, lp_ramp: *const std::ffi::c_void) -> windows::Win32::Foundation::BOOL;
    fn GetICMProfileW(hdc: HDC, p_buf_size: *mut u32, psz_filename: *mut u16) -> windows::Win32::Foundation::BOOL;
    fn SetICMProfileW(hdc: HDC, psz_filename: *const u16) -> windows::Win32::Foundation::BOOL;
}

#[repr(C)]
struct DISPLAY_DEVICEW {
    cb: u32,
    device_name: [u16; 32],
    device_string: [u16; 128],
    state_flags: u32,
    device_id: [u16; 128],
    device_key: [u16; 128],
}

#[link(name = "user32")]
extern "system" {
    fn EnumDisplayDevicesW(
        lp_device: *const u16,
        i_dev_num: u32,
        lp_display_device: *mut DISPLAY_DEVICEW,
        dw_flags: u32,
    ) -> windows::Win32::Foundation::BOOL;
}

#[allow(dead_code)]
#[link(name = "mscms")]
extern "system" {
    fn InstallColorProfileW(p_machine_name: *const u16, p_profile_name: *const u16) -> i32;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorProfileItem {
    pub id: String,                 // Unique identifier
    pub name: String,               // Human-readable display name
    pub file_name: String,          // File name in spool/drivers/color or user icc dir
    pub is_builtin: bool,           // System/Factory vs User custom
    pub description: String,        // Short description
}

fn default_selected_id() -> String {
    "std_srgb".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorCalibrationSetting {
    pub enabled: bool,
    #[serde(default = "default_selected_id")]
    pub selected_profile_id: String,
}

impl Default for MonitorCalibrationSetting {
    fn default() -> Self {
        Self {
            enabled: false,
            selected_profile_id: "std_srgb".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorCalibrationConfig {
    #[serde(default)]
    pub monitors: std::collections::HashMap<String, MonitorCalibrationSetting>,
    // Legacy single-monitor fields kept for backward compatibility
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_selected_id")]
    pub selected_profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorCalibrationState {
    pub enabled: bool,
    pub active_profile_id: String,
    pub active_profile_name: String,
    pub profiles: Vec<ColorProfileItem>,
    pub monitor_name: String,
    pub hdr_active: bool,
    pub current_device_name: String,
    pub monitors: Vec<crate::system::display::DisplayMonitor>,
}

const EMBEDDED_SRGB: &[u8] = include_bytes!("../../resources/profiles/sRGB.icm");
const EMBEDDED_DCIP3: &[u8] = include_bytes!("../../resources/profiles/DCIP3.icm");
const EMBEDDED_ADOBE_RGB: &[u8] = include_bytes!("../../resources/profiles/AdobeRGB.icm");

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn get_user_icc_dir() -> PathBuf {
    let dir = crate::system::config::get_config_dir().join("icc_profiles");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_calibration_config_path() -> PathBuf {
    crate::system::config::get_config_dir().join("color_calibration.json")
}

pub fn load_calibration_config() -> ColorCalibrationConfig {
    let path = get_calibration_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<ColorCalibrationConfig>(&content) {
                return cfg;
            }
        }
    }
    ColorCalibrationConfig::default()
}

pub fn save_calibration_config(cfg: &ColorCalibrationConfig) {
    let path = get_calibration_config_path();
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, json);
    }
}

/// Generate linear identity gamma ramp (0..65535, 100% native uncalibrated screen output)
pub fn identity_ramp() -> GammaRamp {
    let mut ramp = GammaRamp {
        red: [0; 256],
        green: [0; 256],
        blue: [0; 256],
    };
    for i in 0..256 {
        let val = (i as u32 * 65535 / 255) as u16;
        ramp.red[i] = val;
        ramp.green[i] = val;
        ramp.blue[i] = val;
    }
    ramp
}

/// Apply a hardware gamma ramp targeting a specific display device via its GDI DC
pub fn apply_hardware_ramp_for_device(device_name: &str, ramp: &GammaRamp) -> bool {
    let wide_name = to_wide(device_name);
    unsafe {
        let hdc = CreateDCW(std::ptr::null(), wide_name.as_ptr(), std::ptr::null(), std::ptr::null());
        if !hdc.0.is_null() {
            let res = SetDeviceGammaRamp(hdc, ramp as *const _ as *const _);
            let _ = DeleteDC(hdc);
            return res.as_bool();
        }
    }
    false
}

/// Parse VCGT (Video Card Gamma Table) tag from an ICC/ICM profile
pub fn parse_icc_vcgt(bytes: &[u8]) -> Option<GammaRamp> {
    if bytes.len() < 132 {
        return None;
    }

    let tag_count = u32::from_be_bytes([bytes[128], bytes[129], bytes[130], bytes[131]]) as usize;
    let mut offset = 132;

    for _ in 0..tag_count {
        if offset + 12 > bytes.len() {
            break;
        }
        let sig = &bytes[offset..offset + 4];
        let tag_offset = u32::from_be_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]) as usize;
        let tag_size = u32::from_be_bytes([bytes[offset + 8], bytes[offset + 9], bytes[offset + 10], bytes[offset + 11]]) as usize;

        if sig == b"vcgt" {
            if tag_offset + tag_size <= bytes.len() && tag_size >= 16 {
                let vcgt_data = &bytes[tag_offset..tag_offset + tag_size];
                let tag_type = u32::from_be_bytes([vcgt_data[8], vcgt_data[9], vcgt_data[10], vcgt_data[11]]);

                if tag_type == 0 && tag_size >= 22 {
                    let channels = u16::from_be_bytes([vcgt_data[12], vcgt_data[13]]) as usize;
                    let entry_count = u16::from_be_bytes([vcgt_data[14], vcgt_data[15]]) as usize;
                    let entry_size = u16::from_be_bytes([vcgt_data[16], vcgt_data[17]]) as usize;

                    if channels >= 3 && entry_count > 0 {
                        let mut ramp = GammaRamp {
                            red: [0; 256],
                            green: [0; 256],
                            blue: [0; 256],
                        };

                        let table_start = 18;
                        let stride = entry_count * entry_size;

                        for i in 0..256 {
                            let src_idx = (i * (entry_count - 1)) / 255;
                            let get_val = |ch: usize| -> u16 {
                                let ch_offset = table_start + ch * stride + src_idx * entry_size;
                                if ch_offset + entry_size <= vcgt_data.len() {
                                    if entry_size == 2 {
                                        u16::from_be_bytes([vcgt_data[ch_offset], vcgt_data[ch_offset + 1]])
                                    } else if entry_size == 1 {
                                        (vcgt_data[ch_offset] as u16) * 257
                                    } else {
                                        (i as u32 * 65535 / 255) as u16
                                    }
                                } else {
                                    (i as u32 * 65535 / 255) as u16
                                }
                            };

                            ramp.red[i] = get_val(0);
                            ramp.green[i] = get_val(1);
                            ramp.blue[i] = get_val(2);
                        }
                        return Some(ramp);
                    }
                }
            }
        }
        offset += 12;
    }

    None
}

/// Parse MHC2 (Microsoft Hardware Calibration v2) tag from an ICC/ICM profile
pub fn parse_icc_mhc2(bytes: &[u8]) -> Option<GammaRamp> {
    if bytes.len() < 132 {
        return None;
    }

    let tag_count = u32::from_be_bytes([bytes[128], bytes[129], bytes[130], bytes[131]]) as usize;
    let mut offset = 132;

    for _ in 0..tag_count {
        if offset + 12 > bytes.len() {
            break;
        }
        let sig = &bytes[offset..offset + 4];
        let tag_offset = u32::from_be_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]) as usize;
        let tag_size = u32::from_be_bytes([bytes[offset + 8], bytes[offset + 9], bytes[offset + 10], bytes[offset + 11]]) as usize;

        if sig == b"MHC2" {
            if tag_offset + tag_size <= bytes.len() && tag_size >= 36 {
                let mdata = &bytes[tag_offset..tag_offset + tag_size];
                if &mdata[0..4] != b"MHC2" {
                    return None;
                }

                let r_lut_off = u32::from_be_bytes([mdata[24], mdata[25], mdata[26], mdata[27]]) as usize;
                let g_lut_off = u32::from_be_bytes([mdata[28], mdata[29], mdata[30], mdata[31]]) as usize;
                let b_lut_off = u32::from_be_bytes([mdata[32], mdata[33], mdata[34], mdata[35]]) as usize;

                let parse_sf32_lut = |ch_off: usize| -> Option<[u16; 256]> {
                    // Each channel block starts with 8-byte header (sf32 + 4 reserved bytes), then 256 entries of 4-byte s15.16 values
                    if ch_off + 8 + 256 * 4 > mdata.len() {
                        return None;
                    }
                    let tag_type = &mdata[ch_off..ch_off + 4];
                    if tag_type != b"sf32" {
                        return None;
                    }

                    let mut channel = [0u16; 256];
                    for i in 0..256 {
                        let p = ch_off + 8 + i * 4;
                        let val_raw = i32::from_be_bytes([mdata[p], mdata[p + 1], mdata[p + 2], mdata[p + 3]]);
                        let norm = (val_raw as f32) / 65536.0;
                        channel[i] = (norm.clamp(0.0, 1.0) * 65535.0).round() as u16;
                    }
                    Some(channel)
                };

                if let (Some(red), Some(green), Some(blue)) = (
                    parse_sf32_lut(r_lut_off),
                    parse_sf32_lut(g_lut_off),
                    parse_sf32_lut(b_lut_off),
                ) {
                    return Some(GammaRamp { red, green, blue });
                }
            }
        }
        offset += 12;
    }

    None
}

/// Unified ICC/ICM curve parser: checks VCGT first, then Microsoft MHC2 hardware curves
pub fn parse_icc_ramp(bytes: &[u8]) -> Option<GammaRamp> {
    if let Some(ramp) = parse_icc_vcgt(bytes) {
        return Some(ramp);
    }
    if let Some(ramp) = parse_icc_mhc2(bytes) {
        return Some(ramp);
    }
    None
}

/// Load raw bytes of an ICC/ICM profile by filename
pub fn load_profile_bytes_by_name(file_name: &str) -> Option<Vec<u8>> {
    if file_name.is_empty() {
        return None;
    }
    let sys_spool_dir = PathBuf::from(r"C:\Windows\System32\spool\drivers\color");
    let spool_path = sys_spool_dir.join(file_name);
    let user_path = get_user_icc_dir().join(file_name);

    if spool_path.exists() {
        fs::read(&spool_path).ok()
    } else if user_path.exists() {
        fs::read(&user_path).ok()
    } else if file_name.eq_ignore_ascii_case("sRGB.icm") {
        Some(EMBEDDED_SRGB.to_vec())
    } else if file_name.eq_ignore_ascii_case("DCIP3.icm") {
        Some(EMBEDDED_DCIP3.to_vec())
    } else if file_name.eq_ignore_ascii_case("AdobeRGB.icm") {
        Some(EMBEDDED_ADOBE_RGB.to_vec())
    } else {
        None
    }
}

/// Apply hardware gamma ramp to GPU via GDI using an ICC/ICM file name
pub fn apply_hardware_ramp_from_profile_file(device_name: &str, file_name: &str) -> bool {
    if let Some(bytes) = load_profile_bytes_by_name(file_name) {
        if let Some(ramp) = parse_icc_ramp(&bytes) {
            return apply_hardware_ramp_for_device(device_name, &ramp);
        }
    }
    false
}

/// Ensure official embedded sRGB, DCIP3, and AdobeRGB profiles are unpacked on disk
pub fn ensure_standard_profiles_deployed() {
    let user_dir = get_user_icc_dir();
    let srgb_path = user_dir.join("sRGB.icm");
    if !srgb_path.exists() || srgb_path.metadata().map(|m| m.len()).unwrap_or(0) != EMBEDDED_SRGB.len() as u64 {
        let _ = fs::write(&srgb_path, EMBEDDED_SRGB);
    }
    let p3_path = user_dir.join("DCIP3.icm");
    if !p3_path.exists() || p3_path.metadata().map(|m| m.len()).unwrap_or(0) != EMBEDDED_DCIP3.len() as u64 {
        let _ = fs::write(&p3_path, EMBEDDED_DCIP3);
    }
    let adobe_path = user_dir.join("AdobeRGB.icm");
    if !adobe_path.exists() || adobe_path.metadata().map(|m| m.len()).unwrap_or(0) != EMBEDDED_ADOBE_RGB.len() as u64 {
        let _ = fs::write(&adobe_path, EMBEDDED_ADOBE_RGB);
    }

    // Clean up deprecated legacy profiles from older versions
    for old_file in &["DisplayP3.icc", "AdobeRGB1998.icc"] {
        let old_path = user_dir.join(old_file);
        if old_path.exists() {
            let _ = fs::remove_file(old_path);
        }
    }

    let sys_spool = PathBuf::from(r"C:\Windows\System32\spool\drivers\color");
    if sys_spool.exists() {
        for (name, path) in [("sRGB.icm", &srgb_path), ("DCIP3.icm", &p3_path), ("AdobeRGB.icm", &adobe_path)] {
            let sys_file = sys_spool.join(name);
            if !sys_file.exists() || sys_file.metadata().map(|m| m.len()).unwrap_or(0) != path.metadata().map(|m| m.len()).unwrap_or(0) {
                let _ = fs::copy(path, &sys_file);
                unsafe {
                    let p = to_wide(&sys_file.to_string_lossy());
                    let _ = InstallColorProfileW(std::ptr::null(), p.as_ptr());
                }
            }
        }
    }
}

/// Query the currently active ICM profile from Windows GDI for a specific display device
pub fn get_windows_active_icm_profile_for_device(device_name: &str) -> Option<String> {
    let wide_name = to_wide(device_name);
    unsafe {
        let hdc = CreateDCW(std::ptr::null(), wide_name.as_ptr(), std::ptr::null(), std::ptr::null());
        if !hdc.0.is_null() {
            let mut buf = [0u16; 512];
            let mut size = 512u32;
            let ok = GetICMProfileW(hdc, &mut size, buf.as_mut_ptr());
            let _ = DeleteDC(hdc);
            if ok.as_bool() && size > 1 {
                let full_path = String::from_utf16_lossy(&buf[..(size as usize).saturating_sub(1)]);
                let fname = Path::new(&full_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("")
                    .to_string();
                if !fname.is_empty() {
                    return Some(fname);
                }
            }
        }
    }
    None
}

/// Encode a list of strings into a UTF-16 double-null-terminated REG_MULTI_SZ buffer
fn encode_multi_sz(strings: &[&str]) -> Vec<u16> {
    let mut out = Vec::new();
    for s in strings {
        out.extend(OsStr::new(s).encode_wide());
        out.push(0);
    }
    out.push(0);
    out
}

/// Resolve all monitor class subkey indices (e.g. ["0002", "0007", "0008"]) strictly belonging to the specified display device
pub fn get_monitor_class_subkeys_for_device(device_name: &str) -> Vec<String> {
    let wide_name = to_wide(device_name);
    let mut subs = Vec::new();
    unsafe {
        for mon_idx in 0..8 {
            let mut dd_mon = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                device_name: [0; 32],
                device_string: [0; 128],
                state_flags: 0,
                device_id: [0; 128],
                device_key: [0; 128],
            };
            if !EnumDisplayDevicesW(wide_name.as_ptr(), mon_idx, &mut dd_mon, 0).as_bool() {
                break;
            }
            let key_str = String::from_utf16_lossy(&dd_mon.device_key);
            let trimmed = key_str.trim_matches('\0');
            if let Some(sub) = trimmed.split('\\').last() {
                if !sub.is_empty() && sub.chars().all(|c| c.is_ascii_digit()) && !subs.contains(&sub.to_string()) {
                    subs.push(sub.to_string());
                }
            }
        }
    }
    subs
}

/// Resolve the active primary monitor class subkey index (e.g. "0008") for a display device from GDI
pub fn get_monitor_class_subkey_for_device(device_name: &str) -> Option<String> {
    let wide_name = to_wide(device_name);
    let mut fallback_sub: Option<String> = None;
    unsafe {
        for mon_idx in 0..8 {
            let mut dd_mon = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                device_name: [0; 32],
                device_string: [0; 128],
                state_flags: 0,
                device_id: [0; 128],
                device_key: [0; 128],
            };
            if !EnumDisplayDevicesW(wide_name.as_ptr(), mon_idx, &mut dd_mon, 0).as_bool() {
                break;
            }
            let key_str = String::from_utf16_lossy(&dd_mon.device_key);
            let trimmed = key_str.trim_matches('\0');
            if let Some(sub) = trimmed.split('\\').last() {
                if !sub.is_empty() && sub.chars().all(|c| c.is_ascii_digit()) {
                    let is_attached = (dd_mon.state_flags & 0x00000001) != 0; // DISPLAY_DEVICE_ATTACHED_TO_DESKTOP
                    if is_attached {
                        return Some(sub.to_string());
                    }
                    if fallback_sub.is_none() {
                        fallback_sub = Some(sub.to_string());
                    }
                }
            }
        }
    }
    fallback_sub
}

/// Query active ICM profile from HKCU registry ProfileAssociations for a device
pub fn get_registry_active_icm_for_device(device_name: &str) -> Option<String> {
    let subkey = get_monitor_class_subkey_for_device(device_name)?;
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RegOpenKeyExW, RegQueryValueExW, KEY_READ};

    let base_path_w = to_wide(&format!(
        r"Software\Microsoft\Windows NT\CurrentVersion\ICM\ProfileAssociations\Display\{{4d36e96e-e325-11ce-bfc1-08002be10318}}\{}",
        subkey
    ));

    unsafe {
        let mut hsub = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, windows::core::PCWSTR(base_path_w.as_ptr()), 0, KEY_READ, &mut hsub).is_ok() {
            let snap_w = to_wide("ICMProfileSnapshot");
            let mut buf = [0u16; 512];
            let mut size = 1024u32;
            if RegQueryValueExW(hsub, windows::core::PCWSTR(snap_w.as_ptr()), None, None, Some(buf.as_mut_ptr() as *mut u8), Some(&mut size)).is_ok() && size > 2 {
                let name = String::from_utf16_lossy(&buf[..(size as usize / 2)]).trim_matches('\0').to_string();
                let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
                if !name.is_empty() {
                    return Some(name);
                }
            }
            let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
        }
    }

    None
}

/// Synchronize active color profile to Windows Color System (GDI DC + Registry ProfileAssociations)
/// Strictly maintains a SINGLE slot: only this profile is in the device's Windows profile association list.
pub fn sync_profile_to_windows(device_name: &str, file_name: &str) {
    if file_name.is_empty() {
        return;
    }

    // 1. Set GDI device ICM profile
    let wide_dev = to_wide(device_name);
    let wide_file = to_wide(file_name);
    unsafe {
        let hdc = CreateDCW(std::ptr::null(), wide_dev.as_ptr(), std::ptr::null(), std::ptr::null());
        if !hdc.0.is_null() {
            let _ = SetICMProfileW(hdc, wide_file.as_ptr());
            let _ = DeleteDC(hdc);
        }
    }

    // Ensure profile is formally installed in system color spool
    let sys_spool_dir = PathBuf::from(r"C:\Windows\System32\spool\drivers\color");
    let spool_file = sys_spool_dir.join(file_name);
    if spool_file.exists() {
        let p_wide = to_wide(&spool_file.to_string_lossy());
        unsafe {
            let _ = InstallColorProfileW(std::ptr::null(), p_wide.as_ptr());
        }
    }

    // 2. Resolve subkey strictly for this specific target device
    let target_subkey = match get_monitor_class_subkey_for_device(device_name) {
        Some(s) => s,
        None => return,
    };

    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, RegCreateKeyExW, RegSetValueExW, KEY_READ, KEY_WRITE, REG_MULTI_SZ, REG_DWORD, REG_OPEN_CREATE_OPTIONS,
    };

    let base_path_w = to_wide(r"Software\Microsoft\Windows NT\CurrentVersion\ICM\ProfileAssociations\Display\{4d36e96e-e325-11ce-bfc1-08002be10318}");

    // SINGLE SLOT STRATEGY: Only this single file is kept in ICMProfile and ICMProfileSnapshot.
    let single_slot_multi_sz = encode_multi_sz(&[file_name]);

    let val_name = to_wide("ICMProfile");
    let snapshot_name = to_wide("ICMProfileSnapshot");
    let user_flag = to_wide("UsePerUserProfiles");
    let flag_val = 1u32.to_ne_bytes();

    unsafe {
        let mut hbase = windows::Win32::System::Registry::HKEY::default();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(base_path_w.as_ptr()),
            0,
            windows::core::PCWSTR::null(),
            REG_OPEN_CREATE_OPTIONS(0),
            KEY_READ | KEY_WRITE,
            None,
            &mut hbase,
            None,
        ).is_ok() {
            let sub_w = to_wide(&target_subkey);
            let mut hsub = windows::Win32::System::Registry::HKEY::default();
            if RegCreateKeyExW(
                hbase,
                windows::core::PCWSTR(sub_w.as_ptr()),
                0,
                windows::core::PCWSTR::null(),
                REG_OPEN_CREATE_OPTIONS(0),
                KEY_WRITE,
                None,
                &mut hsub,
                None,
            ).is_ok() {
                let _ = RegSetValueExW(
                    hsub,
                    windows::core::PCWSTR(val_name.as_ptr()),
                    0,
                    REG_MULTI_SZ,
                    Some(std::slice::from_raw_parts(
                        single_slot_multi_sz.as_ptr() as *const u8,
                        single_slot_multi_sz.len() * 2,
                    )),
                );
                let _ = RegSetValueExW(
                    hsub,
                    windows::core::PCWSTR(snapshot_name.as_ptr()),
                    0,
                    REG_MULTI_SZ,
                    Some(std::slice::from_raw_parts(
                        single_slot_multi_sz.as_ptr() as *const u8,
                        single_slot_multi_sz.len() * 2,
                    )),
                );
                let _ = RegSetValueExW(
                    hsub,
                    windows::core::PCWSTR(user_flag.as_ptr()),
                    0,
                    REG_DWORD,
                    Some(&flag_val),
                );
                let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
            }
            let _ = windows::Win32::System::Registry::RegCloseKey(hbase);
        }
    }
}

/// Clear active color profile association from Windows GDI and Registry ProfileAssociations strictly for this specific display device.
/// Setting UsePerUserProfiles to 1 with an EMPTY profile list tells Windows that the user explicitly chose no profile,
/// strictly blocking Windows Calibration Loader and reboot sequence from falling back to OEM factory profiles.
pub fn clear_windows_profile_association(device_name: &str) {
    let target_subkeys = get_monitor_class_subkeys_for_device(device_name);
    if target_subkeys.is_empty() {
        return;
    }

    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RegOpenKeyExW, RegDeleteValueW, RegSetValueExW, KEY_WRITE, REG_DWORD, REG_MULTI_SZ};
    let empty_multi_sz = encode_multi_sz(&[]);
    let flag_val = 1u32.to_ne_bytes(); // 1 = UsePerUserProfiles enabled, preventing Windows from falling back to HKLM/OEM driver profile

    for sub in target_subkeys {
        unsafe {
            let sub_path = format!(r"Software\Microsoft\Windows NT\CurrentVersion\ICM\ProfileAssociations\Display\{{4d36e96e-e325-11ce-bfc1-08002be10318}}\{}", sub);
            let sub_w = to_wide(&sub_path);
            let mut hsub = windows::Win32::System::Registry::HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, windows::core::PCWSTR(sub_w.as_ptr()), 0, KEY_WRITE, &mut hsub).is_ok() {
                let snap_w = to_wide("ICMProfileSnapshot");
                let _ = RegDeleteValueW(hsub, windows::core::PCWSTR(snap_w.as_ptr()));
                let icm_w = to_wide("ICMProfile");
                let _ = RegSetValueExW(
                    hsub,
                    windows::core::PCWSTR(icm_w.as_ptr()),
                    0,
                    REG_MULTI_SZ,
                    Some(std::slice::from_raw_parts(
                        empty_multi_sz.as_ptr() as *const u8,
                        empty_multi_sz.len() * 2,
                    )),
                );
                let user_flag = to_wide("UsePerUserProfiles");
                let _ = RegSetValueExW(hsub, windows::core::PCWSTR(user_flag.as_ptr()), 0, REG_DWORD, Some(&flag_val));
                let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
            }
        }
    }
}

/// Detect machine-specific factory profile for laptop internal screen
/// Returns Option<(File Name, Friendly Model Label)>
pub fn detect_machine_factory_profile() -> Option<(String, String)> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RegOpenKeyExA, RegQueryValueExA, KEY_READ};
    use windows::core::PCSTR;

    let sys_spool_dir = PathBuf::from(r"C:\Windows\System32\spool\drivers\color");
    if !sys_spool_dir.exists() {
        return None;
    }

    // 1. Check monitor class keys in HKLM
    let base_path = b"System\\CurrentControlSet\\Control\\Class\\{4d36e96e-e325-11ce-bfc1-08002be10318}\0";
    let mut hbase = windows::Win32::System::Registry::HKEY::default();
    unsafe {
        if RegOpenKeyExA(HKEY_LOCAL_MACHINE, PCSTR(base_path.as_ptr()), 0, KEY_READ, &mut hbase).is_ok() {
            let mut subkey_name = [0u8; 128];
            let mut subkey_len = 128u32;
            let mut idx = 0u32;
            while idx < 16 && windows::Win32::System::Registry::RegEnumKeyExA(
                hbase,
                idx,
                windows::core::PSTR(subkey_name.as_mut_ptr()),
                &mut subkey_len,
                None,
                windows::core::PSTR(std::ptr::null_mut()),
                None,
                None,
            ).is_ok() {
                let name_str = std::str::from_utf8(&subkey_name[..subkey_len as usize]).unwrap_or("");
                let full_sub = format!("System\\CurrentControlSet\\Control\\Class\\{{4d36e96e-e325-11ce-bfc1-08002be10318}}\\{}\0", name_str);
                let mut hsub = windows::Win32::System::Registry::HKEY::default();
                if RegOpenKeyExA(HKEY_LOCAL_MACHINE, PCSTR(full_sub.as_ptr()), 0, KEY_READ, &mut hsub).is_ok() {
                    let mut buf = [0u8; 512];
                    let mut buf_len = 512u32;
                    let val_name = b"ICMProfile\0";
                    if RegQueryValueExA(
                        hsub,
                        PCSTR(val_name.as_ptr()),
                        None,
                        None,
                        Some(buf.as_mut_ptr()),
                        Some(&mut buf_len),
                    ).is_ok() && buf_len > 2 {
                        let wide_slice = std::slice::from_raw_parts(buf.as_ptr() as *const u16, (buf_len / 2) as usize);
                        let reg_file = String::from_utf16_lossy(wide_slice).trim_matches('\0').to_string();
                        if !reg_file.is_empty() {
                            let spool_candidate = sys_spool_dir.join(&reg_file);
                            if spool_candidate.exists() {
                                let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
                                let _ = windows::Win32::System::Registry::RegCloseKey(hbase);
                                let model_label = reg_file.split('_').next().unwrap_or("出厂原装").to_string();
                                return Some((reg_file, model_label));
                            }
                        }
                    }
                    let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
                }
                idx += 1;
                subkey_len = 128;
            }
            let _ = windows::Win32::System::Registry::RegCloseKey(hbase);
        }
    }

    // 2. Scan spool directory for OEM vendor screen prefixes
    if let Ok(entries) = fs::read_dir(&sys_spool_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let fname = entry.file_name().to_string_lossy().to_string();
                let f_upper = fname.to_uppercase();
                if (f_upper.starts_with("BOE")
                    || f_upper.starts_with("CSO")
                    || f_upper.starts_with("AUO")
                    || f_upper.starts_with("INN")
                    || f_upper.starts_with("XMI"))
                    && (f_upper.ends_with(".ICM") || f_upper.ends_with(".ICC"))
                {
                    let model_label = fname.split('_').next().unwrap_or("出厂原装").to_string();
                    return Some((fname, model_label));
                }
            }
        }
    }

    None
}

/// Detect if HDR is currently active via registry (lightweight, non-blocking)
pub fn is_hdr_active() -> bool {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RegOpenKeyExA, RegQueryValueExA, KEY_READ, REG_DWORD};
    use windows::core::PCSTR;

    let path = b"SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers\\MonitorDataStore\0";
    let mut hkey = windows::Win32::System::Registry::HKEY::default();
    unsafe {
        if RegOpenKeyExA(HKEY_LOCAL_MACHINE, PCSTR(path.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
            let mut subkey_name = [0u8; 128];
            let mut subkey_len = 128u32;
            let mut idx = 0u32;
            while idx < 4 && windows::Win32::System::Registry::RegEnumKeyExA(
                hkey,
                idx,
                windows::core::PSTR(subkey_name.as_mut_ptr()),
                &mut subkey_len,
                None,
                windows::core::PSTR(std::ptr::null_mut()),
                None,
                None,
            ).is_ok() {
                let name_str = std::str::from_utf8(&subkey_name[..subkey_len as usize]).unwrap_or("");
                let full_sub = format!("SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers\\MonitorDataStore\\{}\0", name_str);
                let mut hsub = windows::Win32::System::Registry::HKEY::default();
                if RegOpenKeyExA(HKEY_LOCAL_MACHINE, PCSTR(full_sub.as_ptr()), 0, KEY_READ, &mut hsub).is_ok() {
                    let mut val_type = REG_DWORD;
                    let mut val_data = 0u32;
                    let mut val_len = 4u32;
                    let val_name = b"HDREnabled\0";
                    if RegQueryValueExA(
                        hsub,
                        PCSTR(val_name.as_ptr()),
                        None,
                        Some(&mut val_type as *mut _),
                        Some(&mut val_data as *mut _ as *mut u8),
                        Some(&mut val_len as *mut _),
                    ).is_ok() && val_data == 1 {
                        let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
                        let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
                        return true;
                    }
                    let _ = windows::Win32::System::Registry::RegCloseKey(hsub);
                }
                idx += 1;
                subkey_len = 128;
            }
            let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
        }
    }
    false
}

/// List all available color profiles tailored for the monitor
pub fn list_all_color_profiles(is_internal: bool) -> Vec<ColorProfileItem> {
    ensure_standard_profiles_deployed();
    let mut list = Vec::new();

    // 1. Default Native profile
    list.push(ColorProfileItem {
        id: "default_native".to_string(),
        name: "系统默认 (未校准)".to_string(),
        file_name: String::new(),
        is_builtin: true,
        description: "屏幕未限缩的默认物理输出，色彩最饱满广阔".to_string(),
    });

    // 2. Factory Profile (specifically for laptop internal screen)
    if is_internal {
        if let Some((factory_file, model_label)) = detect_machine_factory_profile() {
            list.push(ColorProfileItem {
                id: format!("factory_{}", factory_file),
                name: format!("原厂出厂校色 ({})", model_label),
                file_name: factory_file,
                is_builtin: true,
                description: "厂商产线高精度校色仪生成的专属屏幕校准曲线".to_string(),
            });
        }
    }

    // 3. Official Standard Profiles (available for all displays)
    list.push(ColorProfileItem {
        id: "std_srgb".to_string(),
        name: "标准 sRGB (官方校准)".to_string(),
        file_name: "sRGB.icm".to_string(),
        is_builtin: true,
        description: "机械革命官方原厂 sRGB 模式，消除广色域过饱和（红脸）".to_string(),
    });

    list.push(ColorProfileItem {
        id: "std_p3".to_string(),
        name: "DCI-P3 影视广色域 (官方校准)".to_string(),
        file_name: "DCIP3.icm".to_string(),
        is_builtin: true,
        description: "好莱坞数字影院广色域，观影剪辑首选".to_string(),
    });

    list.push(ColorProfileItem {
        id: "std_adobergb".to_string(),
        name: "Adobe RGB (官方校准)".to_string(),
        file_name: "AdobeRGB.icm".to_string(),
        is_builtin: true,
        description: "覆盖更宽广的青绿色调，专业摄影与高质量出版规范".to_string(),
    });

    // 4. User Custom Profiles (in %APPDATA%\OpenRevo\icc_profiles and system spool)
    let user_dir = get_user_icc_dir();
    if let Ok(entries) = fs::read_dir(&user_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if ext == "icm" || ext == "icc" {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname == "sRGB.icm"
                        || fname == "DCIP3.icm"
                        || fname == "AdobeRGB.icm"
                        || fname == "DisplayP3.icc"
                        || fname == "AdobeRGB1998.icc"
                    {
                        continue;
                    }
                    let id = format!("custom_{}", fname);
                    if !list.iter().any(|item| item.id == id || item.file_name == fname) {
                        list.push(ColorProfileItem {
                            id,
                            name: fname.replace(".icm", "").replace(".icc", ""),
                            file_name: fname,
                            is_builtin: false,
                            description: "用户自定义导入的校色特性文件".to_string(),
                        });
                    }
                }
            }
        }
    }

    list
}

/// Get complete frontend-facing state per target monitor (Zero-blocking, instant return)
pub fn get_color_calibration_state(target_dev: Option<&str>) -> ColorCalibrationState {
    let monitors = crate::system::display::get_all_monitors();
    let selected_mon = if let Some(name) = target_dev {
        monitors.iter().find(|m| m.device_name == name)
    } else {
        monitors.iter().find(|m| m.is_internal).or_else(|| monitors.first())
    }
    .cloned()
    .unwrap_or_else(|| {
        crate::system::display::DisplayMonitor {
            device_name: "\\\\.\\DISPLAY1".into(),
            friendly_name: "内建主屏幕".into(),
            is_primary: true,
            is_internal: true,
            width: 2560,
            height: 1600,
            current_hz: 240,
            supported_rates: vec![60, 240],
        }
    });

    let is_internal = selected_mon.is_internal;
    let dev_name = selected_mon.device_name.clone();
    let profiles = list_all_color_profiles(is_internal);
    let hdr = is_hdr_active();

    let cfg = load_calibration_config();
    let mon_setting = cfg.monitors.get(&dev_name);

    // Read current active profile strictly from registry association (ICMProfileSnapshot)
    // ONLY consider calibrated if registry explicitly has an active profile snapshot
    let win_active_file = get_registry_active_icm_for_device(&dev_name);

    // Match against known profile catalog
    let matched_profile = win_active_file.as_ref().and_then(|win_f| {
        profiles.iter().find(|p| p.file_name.eq_ignore_ascii_case(win_f))
    });

    let (enabled, display_id) = if let Some(matched) = matched_profile {
        // Windows currently has an active calibrated profile!
        // Reflect Windows' real state in the UI so both sides match.
        (true, matched.id.clone())
    } else if let Some(setting) = mon_setting {
        if setting.enabled && setting.selected_profile_id != "default_native" {
            (true, setting.selected_profile_id.clone())
        } else {
            (false, "default_native".to_string())
        }
    } else {
        (false, "default_native".to_string())
    };

    let active_name = profiles
        .iter()
        .find(|p| p.id == display_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "系统默认 (未校准)".to_string());

    ColorCalibrationState {
        enabled,
        active_profile_id: display_id,
        active_profile_name: active_name,
        profiles,
        monitor_name: selected_mon.friendly_name.clone(),
        hdr_active: hdr,
        current_device_name: dev_name,
        monitors,
    }
}

/// Apply a specific profile to a targeted monitor via Windows native color system & GPU hardware LUT
pub fn apply_profile_for_device(device_name: &str, profile_id: &str) -> Result<ColorCalibrationState, String> {
    let monitors = crate::system::display::get_all_monitors();
    let is_internal = monitors.iter().find(|m| m.device_name == device_name).map(|m| m.is_internal).unwrap_or(true);
    let profiles = list_all_color_profiles(is_internal);

    let item = profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("未找到指定的校色方案: {}", profile_id))?;

    if item.id == "default_native" || item.file_name.is_empty() {
        // 1. Hardware GPU LUT: write pure linear 1:1 identity ramp (0..65535)
        // This instantly clears all software gamma curves and restores 100% native panel output!
        let ramp = identity_ramp();
        apply_hardware_ramp_for_device(device_name, &ramp);

        // 2. Clear Windows profile associations to let Windows fall back to native uncalibrated state
        clear_windows_profile_association(device_name);

        let mut cfg = load_calibration_config();
        cfg.monitors.insert(device_name.to_string(), MonitorCalibrationSetting {
            enabled: false,
            selected_profile_id: "default_native".to_string(),
        });
        save_calibration_config(&cfg);
    } else {
        // 1. Hardware GPU LUT: apply VCGT or MHC2 curve embedded in ICC for instant 0-latency feedback
        if let Some(bytes) = load_profile_bytes_by_name(&item.file_name) {
            if let Some(ramp) = parse_icc_ramp(&bytes) {
                apply_hardware_ramp_for_device(device_name, &ramp);
            } else if is_internal {
                // If profile has no embedded curves, preserve factory panel calibration if available
                if let Some((factory_file, _)) = detect_machine_factory_profile() {
                    if let Some(fbytes) = load_profile_bytes_by_name(&factory_file) {
                        if let Some(framp) = parse_icc_ramp(&fbytes) {
                            apply_hardware_ramp_for_device(device_name, &framp);
                        }
                    }
                } else {
                    apply_hardware_ramp_for_device(device_name, &identity_ramp());
                }
            } else {
                apply_hardware_ramp_for_device(device_name, &identity_ramp());
            }
        }

        // 2. Delegate to Windows Color System (Single-slot registry + GDI association)
        sync_profile_to_windows(device_name, &item.file_name);

        let mut cfg = load_calibration_config();
        cfg.monitors.insert(device_name.to_string(), MonitorCalibrationSetting {
            enabled: true,
            selected_profile_id: profile_id.to_string(),
        });
        save_calibration_config(&cfg);
    }

    Ok(get_color_calibration_state(Some(device_name)))
}

/// Toggle handler for a specific monitor
pub fn set_calibration_enabled_for_device(device_name: &str, enable: bool) -> Result<ColorCalibrationState, String> {
    let monitors = crate::system::display::get_all_monitors();
    let is_internal = monitors.iter().find(|m| m.device_name == device_name).map(|m| m.is_internal).unwrap_or(true);
    let profiles = list_all_color_profiles(is_internal);

    let cfg = load_calibration_config();
    let mon_setting = cfg.monitors.get(device_name);

    if enable {
        let target_id = if let Some(setting) = mon_setting {
            if setting.selected_profile_id != "default_native" && profiles.iter().any(|p| p.id == setting.selected_profile_id) {
                setting.selected_profile_id.clone()
            } else {
                profiles.iter().find(|p| p.id != "default_native").map(|p| p.id.clone()).unwrap_or_else(|| "default_native".into())
            }
        } else {
            profiles.iter().find(|p| p.id != "default_native").map(|p| p.id.clone()).unwrap_or_else(|| "default_native".into())
        };

        apply_profile_for_device(device_name, &target_id)
    } else {
        apply_profile_for_device(device_name, "default_native")
    }
}

/// Import a user custom ICC/ICM profile
pub fn import_custom_icc(file_name: &str, file_bytes: &[u8]) -> Result<ColorProfileItem, String> {
    if file_bytes.len() < 128 {
        return Err("文件过小，不是有效的 ICC/ICM 色彩配置文件".to_string());
    }

    let clean_fname = Path::new(file_name)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "文件名无效".to_string())?
        .to_string();

    let ext = Path::new(&clean_fname)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "icm" && ext != "icc" {
        return Err("仅支持导入 .icm 或 .icc 格式的校色配置文件".to_string());
    }

    let user_path = get_user_icc_dir().join(&clean_fname);
    fs::write(&user_path, file_bytes).map_err(|e| format!("保存校色文件失败: {}", e))?;

    let sys_spool = PathBuf::from(r"C:\Windows\System32\spool\drivers\color");
    if sys_spool.exists() {
        let sys_file = sys_spool.join(&clean_fname);
        let _ = fs::copy(&user_path, &sys_file);
        unsafe {
            let p = to_wide(&sys_file.to_string_lossy());
            let _ = InstallColorProfileW(std::ptr::null(), p.as_ptr());
        }
    }

    let item = ColorProfileItem {
        id: format!("custom_{}", clean_fname),
        name: clean_fname.replace(".icm", "").replace(".icc", ""),
        file_name: clean_fname,
        is_builtin: false,
        description: "用户自定义导入的校色特性文件".to_string(),
    };

    Ok(item)
}

/// Delete a custom imported profile
pub fn delete_custom_icc(profile_id: &str, device_name: Option<&str>) -> Result<(), String> {
    let monitors = crate::system::display::get_all_monitors();
    let is_internal = monitors.first().map(|m| m.is_internal).unwrap_or(true);
    let profiles = list_all_color_profiles(is_internal);

    let item = profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| "未找到指定的配置项".to_string())?;

    if item.is_builtin {
        return Err("无法删除系统或出厂预设的校色文件".to_string());
    }

    let user_path = get_user_icc_dir().join(&item.file_name);
    if user_path.exists() {
        let _ = fs::remove_file(user_path);
    }

    let dev = device_name.unwrap_or("\\\\.\\DISPLAY1");
    let cfg = load_calibration_config();
    if let Some(setting) = cfg.monitors.get(dev) {
        if setting.selected_profile_id == profile_id {
            let _ = apply_profile_for_device(dev, "default_native");
        }
    }

    Ok(())
}

/// Enumerate all active display device names attached to the desktop (e.g. `\\.\DISPLAY1`).
pub fn enumerate_active_display_devices() -> Vec<String> {
    let mut devices = Vec::new();
    const DISPLAY_DEVICE_ATTACHED_TO_DESKTOP: u32 = 0x0000_0001;
    unsafe {
        for idx in 0..16 {
            let mut dd = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                device_name: [0; 32],
                device_string: [0; 128],
                state_flags: 0,
                device_id: [0; 128],
                device_key: [0; 128],
            };
            if !EnumDisplayDevicesW(std::ptr::null(), idx, &mut dd, 0).as_bool() {
                break;
            }
            if (dd.state_flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0 {
                let name = String::from_utf16_lossy(&dd.device_name)
                    .trim_matches('\0')
                    .to_string();
                if !name.is_empty() {
                    devices.push(name);
                }
            }
        }
    }
    devices
}

/// Resolve the currently active ICC profile's hardware gamma ramp for a display device,
/// checking both OpenRevo's per-monitor config and Windows Color Management (GDI/Registry).
pub fn get_effective_base_ramp_for_device(device_name: &str) -> Option<GammaRamp> {
    let cfg = load_calibration_config();
    if let Some(setting) = cfg.monitors.get(device_name) {
        if setting.enabled && setting.selected_profile_id != "default_native" {
            let is_internal = device_name.eq_ignore_ascii_case("\\\\.\\DISPLAY1");
            let profiles = list_all_color_profiles(is_internal);
            if let Some(prof) = profiles.into_iter().find(|p| p.id == setting.selected_profile_id) {
                if !prof.file_name.is_empty() {
                    if let Some(bytes) = load_profile_bytes_by_name(&prof.file_name) {
                        if let Some(ramp) = parse_icc_ramp(&bytes) {
                            return Some(ramp);
                        }
                    }
                }
            }
        }
    }

    let icm_file = get_windows_active_icm_profile_for_device(device_name)
        .or_else(|| get_registry_active_icm_for_device(device_name));
    if let Some(file_name) = icm_file {
        if let Some(bytes) = load_profile_bytes_by_name(&file_name) {
            if let Some(ramp) = parse_icc_ramp(&bytes) {
                return Some(ramp);
            }
        }
    }

    None
}

/// Restore each active display's hardware gamma LUT back to its configured/Windows ICC profile
/// (or linear identity ramp if no calibrated profile is active).
pub fn restore_active_icc_ramps_for_all_devices() -> bool {
    let devices = enumerate_active_display_devices();
    if devices.is_empty() {
        return false;
    }
    for dev in devices {
        if let Some(ramp) = get_effective_base_ramp_for_device(&dev) {
            let ok = apply_hardware_ramp_for_device(&dev, &ramp);
            crate::log_info!(
                "COLOR_CAL",
                "Restored active ICC hardware gamma ramp for {} -> ok={}",
                dev,
                ok
            );
        } else {
            let _ = apply_hardware_ramp_for_device(&dev, &identity_ramp());
        }
    }
    true
}

/// Initialize color calibration environment on system startup:
/// Deploys standard embedded color profiles to spool directory so Windows Color Management can recognize them.
/// Zero hardware LUT injection at startup: Windows native Color System (Calibration Loader) and GPU drivers
/// automatically load associated profiles on login. OpenRevo never forces hardware ramps on boot.
pub fn init_color_calibration() {
    ensure_standard_profiles_deployed();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_official_icm_profiles() {
        // 1. sRGB.icm (contains Microsoft MHC2 curves)
        let srgb_ramp = parse_icc_ramp(EMBEDDED_SRGB);
        assert!(srgb_ramp.is_some(), "Failed to parse official sRGB.icm");
        let r = srgb_ramp.unwrap();
        assert_eq!(r.red[0], 0);
        assert!(r.red[128] > 28000 && r.red[128] < 36000, "sRGB midtone crushed: {}", r.red[128]);
        assert!(r.red[255] > 60000);

        // 2. DCIP3.icm
        let p3_ramp = parse_icc_ramp(EMBEDDED_DCIP3);
        assert!(p3_ramp.is_some(), "Failed to parse official DCIP3.icm");
        let r2 = p3_ramp.unwrap();
        assert_eq!(r2.red[0], 0);
        assert!(r2.red[128] > 25000 && r2.red[128] < 35000, "DCIP3 midtone crushed: {}", r2.red[128]);
        assert!(r2.red[255] > 60000);

        // 3. AdobeRGB.icm
        let argb_ramp = parse_icc_ramp(EMBEDDED_ADOBE_RGB);
        assert!(argb_ramp.is_some(), "Failed to parse official AdobeRGB.icm");
        let r3 = argb_ramp.unwrap();
        assert_eq!(r3.red[0], 0);
        assert!(r3.red[128] > 28000 && r3.red[128] < 36000, "AdobeRGB midtone crushed: {}", r3.red[128]);
        assert!(r3.red[255] > 60000);
    }

    #[test]
    fn test_list_all_color_profiles() {
        let internal_profiles = list_all_color_profiles(true);
        assert!(!internal_profiles.is_empty());
        assert_eq!(internal_profiles[0].id, "default_native");
        assert!(internal_profiles.iter().any(|p| p.id == "std_srgb"));
        assert!(internal_profiles.iter().any(|p| p.id == "std_p3"));
        assert!(internal_profiles.iter().any(|p| p.id == "std_adobergb"));
        assert!(!internal_profiles.iter().any(|p| p.file_name == "AdobeRGB1998.icc" || p.file_name == "DisplayP3.icc"));

        // Secondary / External monitor must also have official standard profiles!
        let external_profiles = list_all_color_profiles(false);
        assert!(!external_profiles.is_empty());
        assert_eq!(external_profiles[0].id, "default_native");
        assert!(external_profiles.iter().any(|p| p.id == "std_srgb"));
        assert!(external_profiles.iter().any(|p| p.id == "std_p3"));
        assert!(external_profiles.iter().any(|p| p.id == "std_adobergb"));
        // External monitor must NOT have internal factory panel profile
        assert!(!external_profiles.iter().any(|p| p.id.starts_with("factory_")));
    }
}
