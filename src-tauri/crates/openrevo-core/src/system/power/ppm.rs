//! Windows PPM (Processor Power Management) and Powercfg Central Controller
//!
//! Enforces Level 1.2 Centralized Power & PPM Arbiter rule:
//! All `powercfg` execution permissions, Windows power scheme management,
//! SleepGuard policies, CPU frequency limits, and EPP (Energy Performance Preference)
//! tuning are strictly consolidated here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsPowerScheme {
    pub guid: String,
    pub name: String,
    pub is_active: bool,
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Internal safe runner for `powercfg` commands with no window and structured error reporting.
#[cfg(target_os = "windows")]
fn run_powercfg(args: &[&str]) -> Result<std::process::Output, String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("powercfg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("powercfg command execution failed: {}", e))
}

#[cfg(not(target_os = "windows"))]
fn run_powercfg(_args: &[&str]) -> Result<std::process::Output, String> {
    Err("powercfg is only supported on Windows".to_string())
}

/// Helper to decode ANSI/UTF-8 output from Windows console
#[cfg(target_os = "windows")]
pub fn decode_ansi_or_utf8(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            CodePage: u32,
            dwFlags: u32,
            lpMultiByteStr: *const u8,
            cbMultiByte: i32,
            lpWideCharStr: *mut u16,
            cchWideChar: i32,
        ) -> i32;
    }
    unsafe {
        let len = MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0);
        if len > 0 {
            let mut buf = vec![0u16; len as usize];
            let res = MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, buf.as_mut_ptr(), len);
            if res > 0 {
                return String::from_utf16_lossy(&buf);
            }
        }
    }
    String::from_utf8_lossy(bytes).to_string()
}

/// Enumerates all Windows power schemes via `powercfg /list`
pub fn get_windows_power_schemes() -> Result<Vec<WindowsPowerScheme>, String> {
    #[cfg(target_os = "windows")]
    {
        let output = run_powercfg(&["/list"])?;
        let text = decode_ansi_or_utf8(&output.stdout);
        let mut schemes = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in &parts {
                let clean = part.trim_matches(|c: char| !c.is_ascii_hexdigit() && c != '-');
                if clean.len() == 36 && clean.chars().filter(|&c| c == '-').count() == 4 {
                    let is_active = line.ends_with('*') || line.contains('*');
                    let raw_name = if let (Some(start), Some(end)) = (line.find('('), line.rfind(')')) {
                        if start < end {
                            line[start + 1..end].trim().to_string()
                        } else {
                            clean.to_string()
                        }
                    } else {
                        clean.to_string()
                    };

                    let display_name = match clean.to_lowercase().as_str() {
                        "381b4222-f694-41f0-9685-ff5bb260df2e" => "平衡 (Balanced)".to_string(),
                        "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c" => "高性能 (High Performance)".to_string(),
                        "a1841308-3541-4fab-bc81-f71556f20b4a" => "节能 (Power Saver)".to_string(),
                        "e9a42b02-d5df-448d-aa00-03f14749eb61" => "卓越性能 (Ultimate Performance)".to_string(),
                        _ => {
                            if raw_name.contains('\u{FFFD}') || raw_name.is_empty() {
                                format!("自定义方案 ({})", &clean[..8])
                            } else {
                                raw_name
                            }
                        }
                    };

                    schemes.push(WindowsPowerScheme {
                        guid: clean.to_string(),
                        name: display_name,
                        is_active,
                    });
                    break;
                }
            }
        }

        if schemes.is_empty() {
            schemes.push(WindowsPowerScheme {
                guid: "381b4222-f694-41f0-9685-ff5bb260df2e".to_string(),
                name: "平衡 (Balanced)".to_string(),
                is_active: true,
            });
        }

        Ok(schemes)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(vec![])
    }
}

/// Sets active Windows power scheme by GUID
pub fn set_active_windows_power_scheme(guid: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let clean = guid.trim();
        if clean.len() != 36 || clean.chars().any(|c| !c.is_ascii_hexdigit() && c != '-') {
            return Err("Invalid power scheme GUID format".to_string());
        }

        let output = run_powercfg(&["/setactive", clean])?;
        if output.status.success() {
            crate::log_info!("POWER_PPM", "Successfully activated Windows power scheme {}", clean);
            Ok(())
        } else {
            Err(format!("powercfg /setactive returned error code {:?}", output.status.code()))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = guid;
        Ok(())
    }
}

/// Applies a single SleepGuard policy via `powercfg`
pub fn apply_sleep_guard_option(option: &str, enable: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        match option {
            "disconnect_net" => {
                let val = if enable { "0" } else { "1" };
                let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "SUB_NONE", "f15576e8-98b7-4186-b944-eafa664402d9", val]);
                let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "SUB_NONE", "f15576e8-98b7-4186-b944-eafa664402d9", val]);
            }
            "block_away" => {
                let val = if enable { "0" } else { "1" };
                let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "238c9fa8-0aad-41ed-83f4-97be242c8f20", "25dfa149-5dd1-4736-b5ab-e8a37b5b8187", val]);
                let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "238c9fa8-0aad-41ed-83f4-97be242c8f20", "25dfa149-5dd1-4736-b5ab-e8a37b5b8187", val]);
            }
            "ignore_sysreq" => {
                let val = if enable { "0" } else { "1" };
                let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "238c9fa8-0aad-41ed-83f4-97be242c8f20", "a4b195f5-8225-47d8-8012-9d41369786e2", val]);
                let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "238c9fa8-0aad-41ed-83f4-97be242c8f20", "a4b195f5-8225-47d8-8012-9d41369786e2", val]);
            }
            "disable_wake_timers" => {
                let val = if enable { "0" } else { "1" };
                let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "238c9fa8-0aad-41ed-83f4-97be242c8f20", "bd3b718a-0680-4d9d-8ab2-e1d2b4ac806d", val]);
                let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "238c9fa8-0aad-41ed-83f4-97be242c8f20", "bd3b718a-0680-4d9d-8ab2-e1d2b4ac806d", val]);
            }
            "deep_sleep" => {
                let val = if enable { "1" } else { "0" };
                let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "2e601130-5351-4d9d-8e04-252966bad054", "d502f7ee-1dc7-4efd-a55d-f04b6f5c0545", val]);
                let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "2e601130-5351-4d9d-8e04-252966bad054", "d502f7ee-1dc7-4efd-a55d-f04b6f5c0545", val]);
            }
            "hibernate_30m" => {
                let val = if enable { "30" } else { "0" };
                let _ = run_powercfg(&["/change", "hibernate-timeout-dc", val]);
            }
            _ => return Err(format!("Unknown SleepGuard option: {}", option)),
        }
        let _ = run_powercfg(&["/setactive", "scheme_current"]);
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (option, enable);
        Ok(())
    }
}

/// Applies all SleepGuard configurations according to current preferences
pub fn apply_all_sleep_guard(enable: bool) {
    let _ = apply_sleep_guard_option("disconnect_net", enable);
    let _ = apply_sleep_guard_option("block_away", enable);
    let _ = apply_sleep_guard_option("ignore_sysreq", enable);
    let _ = apply_sleep_guard_option("disable_wake_timers", enable);
    let _ = apply_sleep_guard_option("deep_sleep", enable);
    let _ = apply_sleep_guard_option("hibernate_30m", enable);
}

#[cfg(target_os = "windows")]
const BLD_AMD_PPM_PROCESSOR_WIN11: &[u8] = include_bytes!("../../../../../resources/driver/bld/ppm/AMD.Power.Processor.2.ppkg");
#[cfg(target_os = "windows")]
const BLD_AMD_PPM_PROCESSOR_WIN10: &[u8] = include_bytes!("../../../../../resources/driver/bld/ppm/AMD.Power.Processor.ppkg");
#[cfg(target_os = "windows")]
const BLD_AMD_PPM_NVME: &[u8] = include_bytes!("../../../../../resources/driver/bld/ppm/AMD.Power.NVMe.ppkg");

#[cfg(target_os = "windows")]
fn is_windows_11() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\0".encode_utf16().collect();
    let val_name: Vec<u16> = "CurrentBuild\0".encode_utf16().collect();
    let mut hkey = HKEY::default();

    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(subkey.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
            let mut buf = [0u16; 64];
            let mut size = (buf.len() * 2) as u32;
            let mut data_type = REG_SZ;
            let res = RegQueryValueExW(
                hkey,
                PCWSTR(val_name.as_ptr()),
                None,
                Some(&mut data_type),
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut size),
            );
            let _ = RegCloseKey(hkey);
            if res.is_ok() {
                let len = (size as usize / 2).saturating_sub(1);
                let build_str = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                if let Ok(build_num) = build_str.parse::<u32>() {
                    return build_num >= 22000;
                }
            }
        }
    }
    true
}

#[cfg(target_os = "windows")]
fn install_provisioning_package(pkg_path: &std::path::Path) -> bool {
    let path_str = pkg_path.to_string_lossy();
    let ps_cmd = format!(
        "Install-ProvisioningPackage -PackagePath '{}' -QuietInstall -ForceInstall",
        path_str
    );
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                crate::log_info!("POWER_PPM", "Successfully installed provisioning package: {}", path_str);
                true
            } else {
                let err_msg = decode_ansi_or_utf8(&out.stderr);
                crate::log_warn!("POWER_PPM", "Provisioning package {} install warning: {}", path_str, err_msg.trim());
                false
            }
        }
        Err(e) => {
            crate::log_warn!("POWER_PPM", "Failed to launch powershell for package {}: {}", path_str, e);
            false
        }
    }
}

/// Comprehensive Factory PPM Restoration:
/// 1. Disengages any active performance slider overlay (PowerSetActiveOverlayScheme -> GUID_NULL).
/// 2. Performs powercfg -restoredefaultschemes to eliminate corrupted schemes and regenerate clean built-in templates.
/// 3. Sets active scheme to standard Windows Balanced (381b4222-f694-41f0-9685-ff5bb260df2e).
/// 4. Direct scrub of any residual 75b0ae3f keys from registry.
/// 5. Adaptive OEM Re-provisioning:
///    - BLD / AMD models: restarts AmdPpkgSvc service and re-installs official AMD Processor & NVMe PPKGs.
///    - TongFang / Intel models: dynamically scans and applies official Intel/TongFang PPM packages.
/// 6. Flushes Windows kernel power manager.
pub fn restore_official_power_plan() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        crate::log_info!("POWER_PPM", "Starting official power plan restoration...");

        // 1. Disengage any active Performance Overlay Scheme back to Balanced (GUID_NULL)
        if let Err(error) = crate::system::power_mode::apply_windows_power_overlay(2) {
            crate::log_warn!("POWER_MODE", "Windows mode synchronization failed: {}", error);
        }

        // 2. powercfg -restoredefaultschemes
        let _ = std::process::Command::new("powercfg")
            .arg("-restoredefaultschemes")
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        // 3. Activate Standard Balanced (381b4222-f694-41f0-9685-ff5bb260df2e)
        let _ = std::process::Command::new("powercfg")
            .args(["/setactive", "381b4222-f694-41f0-9685-ff5bb260df2e"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        // 4. Direct scrub of any residual 75b0ae3f keys in registry across canonical schemes
        let schemes = [
            "381b4222-f694-41f0-9685-ff5bb260df2e",
            "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
            "a1841308-3541-4fab-bc81-f71556f20b4a",
            "e9a42b02-d5df-448d-aa00-03f14749eb61",
        ];
        let freq_settings = [
            "75b0ae3f-bce0-45a7-8c89-c9611c25e100",
            "75b0ae3f-bce0-45a7-8c89-c9611c25e101",
            "75b0ae3f-bce0-45a7-8c89-c9611c25e102",
        ];
        for guid in &schemes {
            for s in &freq_settings {
                let reg_key = format!(
                    r"HKLM\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes\{}\54533251-82be-4824-96c1-47b60b740d00\{}",
                    guid, s
                );
                let _ = std::process::Command::new("reg")
                    .args(["delete", &reg_key, "/f"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
            }
        }

        // Sanitize throttle percentages and boost mode
        let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "SUB_PROCESSOR", "bc5038f7-23e0-4960-96da-33abaf5935ec", "100"]);
        let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "SUB_PROCESSOR", "bc5038f7-23e0-4960-96da-33abaf5935ec", "100"]);
        let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "SUB_PROCESSOR", "893dee8e-2bef-41e0-89c6-b55d0929964c", "5"]);
        let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "SUB_PROCESSOR", "893dee8e-2bef-41e0-89c6-b55d0929964c", "5"]);
        let _ = run_powercfg(&["/setacvalueindex", "scheme_current", "SUB_PROCESSOR", "be337238-0d82-4146-a960-4f3749d470c7", "2"]);
        let _ = run_powercfg(&["/setdcvalueindex", "scheme_current", "SUB_PROCESSOR", "be337238-0d82-4146-a960-4f3749d470c7", "2"]);

        // 5. Adaptive Re-provisioning for BLD (AMD) vs TongFang (Intel) OEM models
        let (_, is_amd_cpu) = crate::driver::uefi_nvram::get_cpu_info();
        let is_bld_manifest = crate::hal::declarative::load_matched_manifest()
            .map(|m| m.id.to_lowercase().contains("bld"))
            .unwrap_or(false);
        let is_bld_or_amd = is_amd_cpu || is_bld_manifest;

        let return_msg = if is_bld_or_amd {
            crate::log_info!("POWER_PPM", "Target platform detected as BLD/AMD. Deploying official AMD PPM packages...");

            // 5.1 If AmdPpkgSvc service exists, restart it to trigger native driver reload
            let has_amd_service = std::process::Command::new("sc.exe")
                .args(["query", "AmdPpkgSvc"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map(|o| {
                    let text = String::from_utf8_lossy(&o.stdout);
                    text.contains("STATE")
                })
                .unwrap_or(false);

            if has_amd_service {
                crate::log_info!("POWER_PPM", "Found active AmdPpkgSvc service, cycling to trigger driver PPM synchronization...");
                let _ = std::process::Command::new("net")
                    .args(["stop", "AmdPpkgSvc"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
                let _ = std::process::Command::new("net")
                    .args(["start", "AmdPpkgSvc"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
            }

            // 5.2 Extract and install embedded AMD PPM packages into temp directory
            let temp_dir = std::env::temp_dir().join("openrevo_ppm");
            let _ = std::fs::create_dir_all(&temp_dir);

            let is_win11 = is_windows_11();
            let (proc_pkg_name, proc_pkg_bytes) = if is_win11 {
                ("AMD.Power.Processor.2.ppkg", BLD_AMD_PPM_PROCESSOR_WIN11)
            } else {
                ("AMD.Power.Processor.ppkg", BLD_AMD_PPM_PROCESSOR_WIN10)
            };

            let proc_pkg_path = temp_dir.join(proc_pkg_name);
            let nvme_pkg_path = temp_dir.join("AMD.Power.NVMe.ppkg");

            let write_proc_ok = std::fs::write(&proc_pkg_path, proc_pkg_bytes).is_ok();
            let write_nvme_ok = std::fs::write(&nvme_pkg_path, BLD_AMD_PPM_NVME).is_ok();

            let mut installed_count = 0;
            if write_proc_ok && install_provisioning_package(&proc_pkg_path) {
                installed_count += 1;
            }
            if write_nvme_ok && install_provisioning_package(&nvme_pkg_path) {
                installed_count += 1;
            }

            // Clean up temporary package files safely
            let _ = std::fs::remove_file(&proc_pkg_path);
            let _ = std::fs::remove_file(&nvme_pkg_path);
            let _ = std::fs::remove_dir(&temp_dir);

            crate::log_info!("POWER_PPM", "BLD/AMD PPM package restoration complete ({} packages installed)", installed_count);
            "已成功恢复宝龙达/AMD 官方出厂电源计划与 PPM 处理器/NVMe 调度！".to_string()
        } else {
            crate::log_info!("POWER_PPM", "Target platform detected as TongFang/Intel. Scanning and applying OEM PPM packages...");

            // 5.3 Scan standard provisioning directories for Intel/TongFang PPM packages
            let mut installed_count = 0;
            let candidate_dirs = [
                r"C:\ProgramData\microsoft\provisioning",
                r"C:\Windows\Provisioning\Packages",
            ];

            for dir in &candidate_dirs {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext.eq_ignore_ascii_case("ppkg") {
                                let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                                if file_name.starts_with("ppm-")
                                    || file_name.contains("aistone")
                                    || file_name.contains("intel")
                                {
                                    if install_provisioning_package(&path) {
                                        installed_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Legacy fallback if specific file is present
            let default_aistone = r"C:\ProgramData\microsoft\provisioning\AistonePowerManagement.ppkg";
            if installed_count == 0 && std::path::Path::new(default_aistone).exists() {
                if install_provisioning_package(std::path::Path::new(default_aistone)) {
                    installed_count += 1;
                }
            }

            crate::log_info!("POWER_PPM", "TongFang/Intel PPM restoration complete ({} packages installed)", installed_count);
            "已成功恢复同方/Intel 官方出厂电源计划与 PPM 调度！".to_string()
        };

        let _ = run_powercfg(&["/setactive", "scheme_current"]);

        // Reset preferred scheme in config to None
        let mut cfg = crate::system::config::load_config();
        cfg.preferred_power_scheme = None;
        cfg.cpu_safety_guard = false;
        cfg.cpu_freq_limit = 0;
        cfg.cpu_freq_crash_guard = false;
        crate::system::config::save_config(&cfg);

        crate::log_info!("POWER_PPM", "Official factory power plan restoration succeeded");
        Ok(return_msg)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok("Non-Windows OS".to_string())
    }
}

pub fn deep_unlock_cpu_frequency() -> Result<(), String> {
    restore_official_power_plan().map(|_| ())
}
