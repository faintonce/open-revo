// OpenRevo - Display & Color Tuning Commands
// Handles refresh rates, multi-monitor configuration, brightness, and color presets.

use tauri::{AppHandle, Emitter};
use crate::system::display;
use crate::system::display_tuning::{
    self, DisplayColorPreset, DisplayTuningState,
};

#[tauri::command]
pub fn switch_refresh_rate(hz: u32) -> Result<(), String> {
    display::set_refresh_rate(hz)?;
    let mut cfg = crate::system::config::load_config();
    cfg.refresh_rate = hz;
    crate::system::config::save_config(&cfg);
    Ok(())
}

#[tauri::command]
pub fn get_display_monitors() -> Result<Vec<display::DisplayMonitor>, String> {
    Ok(display::get_all_monitors())
}

#[tauri::command]
pub fn set_display_monitor_refresh_rate(device_name: String, hz: u32) -> Result<(), String> {
    display::set_monitor_refresh_rate(&device_name, hz)?;

    // Only persist cfg.refresh_rate if this is the primary or internal laptop display
    let monitors = display::get_all_monitors();
    let is_primary_or_internal = monitors
        .iter()
        .find(|m| m.device_name.eq_ignore_ascii_case(&device_name))
        .map(|m| m.is_primary || m.is_internal)
        .unwrap_or(false);

    if is_primary_or_internal || monitors.len() <= 1 {
        let mut cfg = crate::system::config::load_config();
        cfg.refresh_rate = hz;
        crate::system::config::save_config(&cfg);
    }
    Ok(())
}

#[tauri::command]
pub fn set_auto_min_refresh_on_battery(enable: bool) -> Result<bool, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.auto_min_refresh_on_battery = enable;
    crate::system::config::save_config(&cfg);
    Ok(enable)
}

#[tauri::command]
pub async fn get_display_brightness() -> Result<u8, String> {
    tauri::async_runtime::spawn_blocking(|| {
        display::get_brightness()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_display_brightness(app: AppHandle, brightness: u8) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        display::set_brightness(brightness)
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit("display-brightness-changed", brightness);
    Ok(())
}

#[tauri::command]
pub fn get_display_tuning_state() -> DisplayTuningState {
    display_tuning::get_current_tuning_state()
}

#[tauri::command]
pub fn set_display_tuning_enabled(enable: bool) -> Result<(), String> {
    let mut cfg = display_tuning::load_display_tuning_config();
    cfg.enabled = enable;
    display_tuning::save_display_tuning_config(&cfg);

    if enable {
        let state = display_tuning::get_current_tuning_state();
        display_tuning::apply_color_preset(&state.active_preset)?;
    } else {
        display_tuning::reset_display_tuning_to_default();
    }
    Ok(())
}

#[tauri::command]
pub fn apply_display_color_preset(
    preset: DisplayColorPreset,
) -> Result<(), String> {
    let mut cfg = display_tuning::load_display_tuning_config();
    cfg.active_preset_id = preset.id.clone();
    if preset.id == "preset_custom" {
        cfg.custom_preset = preset.clone();
    } else if preset.is_builtin {
        cfg.preset_overrides.insert(preset.id.clone(), preset.clone());
    } else {
        let _ = display_tuning::save_custom_preset_file(&preset);
    }
    display_tuning::save_display_tuning_config(&cfg);

    if cfg.enabled {
        display_tuning::apply_color_preset(&preset)?;
    }
    Ok(())
}

#[tauri::command]
pub fn reset_color_preset_to_default(preset_id: String) -> Result<DisplayTuningState, String> {
    display_tuning::reset_preset_override(&preset_id);
    let state = display_tuning::get_current_tuning_state();
    let target = state
        .presets
        .iter()
        .find(|p| p.id == preset_id)
        .cloned()
        .unwrap_or_else(|| state.active_preset.clone());

    if state.enabled {
        display_tuning::apply_color_preset(&target)?;
    }
    Ok(display_tuning::get_current_tuning_state())
}

#[tauri::command]
pub fn import_color_preset_content(
    content: String,
    filename: Option<String>,
) -> Result<DisplayColorPreset, String> {
    let preset = display_tuning::import_color_preset_json(&content, filename)?;
    let mut cfg = display_tuning::load_display_tuning_config();
    cfg.active_preset_id = preset.id.clone();
    display_tuning::save_display_tuning_config(&cfg);

    if cfg.enabled {
        display_tuning::apply_color_preset(&preset)?;
    }
    Ok(preset)
}

#[tauri::command]
pub fn reset_display_tuning() -> Result<DisplayTuningState, String> {
    let mut cfg = display_tuning::load_display_tuning_config();
    cfg.enabled = true;
    cfg.active_preset_id = "preset_vibrant".to_string();
    cfg.custom_preset = DisplayColorPreset::default();
    cfg.preset_overrides.clear();
    display_tuning::save_display_tuning_config(&cfg);

    let default_preset = DisplayColorPreset::default();
    display_tuning::apply_color_preset(&default_preset)?;
    Ok(display_tuning::get_current_tuning_state())
}

#[tauri::command]
pub fn turn_off_screen() -> Result<(), String> {
    crate::system::switches::turn_off_screen()
}

// -------------------------------------------------------------
// Color Calibration (ICM / WCS) Commands (Fully Async & Non-blocking)
// -------------------------------------------------------------

#[tauri::command]
pub async fn get_color_calibration_state(device_name: Option<String>) -> Result<crate::system::color_calibration::ColorCalibrationState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(crate::system::color_calibration::get_color_calibration_state(device_name.as_deref()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_color_profile(profile_id: String, device_name: Option<String>) -> Result<crate::system::color_calibration::ColorCalibrationState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dev = match device_name {
            Some(ref d) if !d.is_empty() => d.clone(),
            _ => crate::system::display::get_all_monitors()
                .into_iter()
                .find(|m| m.is_internal)
                .map(|m| m.device_name)
                .unwrap_or_else(|| r"\\.\DISPLAY1".to_string()),
        };
        crate::system::color_calibration::apply_profile_for_device(&dev, &profile_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_color_calibration_enabled(enable: bool, device_name: Option<String>) -> Result<crate::system::color_calibration::ColorCalibrationState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dev = match device_name {
            Some(ref d) if !d.is_empty() => d.clone(),
            _ => crate::system::display::get_all_monitors()
                .into_iter()
                .find(|m| m.is_internal)
                .map(|m| m.device_name)
                .unwrap_or_else(|| r"\\.\DISPLAY1".to_string()),
        };
        crate::system::color_calibration::set_calibration_enabled_for_device(&dev, enable)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_custom_color_profile(
    file_name: String,
    file_bytes: Vec<u8>,
    device_name: Option<String>,
) -> Result<crate::system::color_calibration::ColorCalibrationState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dev = match device_name {
            Some(ref d) if !d.is_empty() => d.clone(),
            _ => crate::system::display::get_all_monitors()
                .into_iter()
                .find(|m| m.is_internal)
                .map(|m| m.device_name)
                .unwrap_or_else(|| r"\\.\DISPLAY1".to_string()),
        };
        let item = crate::system::color_calibration::import_custom_icc(&file_name, &file_bytes)?;
        crate::system::color_calibration::apply_profile_for_device(&dev, &item.id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_custom_color_profile(profile_id: String, device_name: Option<String>) -> Result<crate::system::color_calibration::ColorCalibrationState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dev = match device_name {
            Some(ref d) if !d.is_empty() => d.clone(),
            _ => crate::system::display::get_all_monitors()
                .into_iter()
                .find(|m| m.is_internal)
                .map(|m| m.device_name)
                .unwrap_or_else(|| r"\\.\DISPLAY1".to_string()),
        };
        crate::system::color_calibration::delete_custom_icc(&profile_id, Some(&dev))?;
        Ok(crate::system::color_calibration::get_color_calibration_state(Some(&dev)))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_miniled_support() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| {
        // Query hardware capability via vault EC symbol
        if let Some(val) = crate::driver::wmi_ec::raw_driver_read_ec(crate::hal::vault::ec::ADDR_LOCAL_DIMMING_SUPPORT) {
            let is_supported = val != 0xFF && (val & crate::hal::vault::mask::MASK_LOCAL_DIMMING_SUPPORT) != 0;
            return Ok(is_supported);
        }
        if let Some(val) = crate::driver::wmi_mulong::mulong_read_ec(crate::hal::vault::ec::ADDR_LOCAL_DIMMING_SUPPORT) {
            let is_supported = val != 0xFF && (val & crate::hal::vault::mask::MASK_LOCAL_DIMMING_SUPPORT) != 0;
            return Ok(is_supported);
        }
        Ok(false)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn read_oem_miniled_registry() -> Option<bool> {
    unsafe {
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE,
            KEY_READ, REG_DWORD, HKEY,
        };
        use windows::core::w;

        let sub_keys = [
            w!(r"SOFTWARE\OEM\GamingCenter2\MySetting\LocalDimming"),
            w!(r"SOFTWARE\WOW6432Node\OEM\GamingCenter2\MySetting\LocalDimming"),
        ];

        for &sub_key in &sub_keys {
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub_key, 0, KEY_READ, &mut hkey).is_ok() {
                let mut data_type = REG_DWORD;
                let mut data = 0u32;
                let mut data_size = std::mem::size_of::<u32>() as u32;
                let status = RegQueryValueExW(
                    hkey,
                    w!("LocalDimmingStatus"),
                    None,
                    Some(&mut data_type),
                    Some(&mut data as *mut u32 as *mut u8),
                    Some(&mut data_size),
                );
                let _ = RegCloseKey(hkey);
                if status.is_ok() {
                    return Some(data == 1);
                }
            }
        }
    }
    None
}

fn write_oem_miniled_registry(enabled: bool) {
    unsafe {
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY_LOCAL_MACHINE,
            KEY_SET_VALUE, REG_DWORD, HKEY,
        };
        use windows::core::w;

        let sub_keys = [
            w!(r"SOFTWARE\OEM\GamingCenter2\MySetting\LocalDimming"),
            w!(r"SOFTWARE\WOW6432Node\OEM\GamingCenter2\MySetting\LocalDimming"),
        ];

        for &sub_key in &sub_keys {
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub_key, 0, KEY_SET_VALUE, &mut hkey).is_ok() {
                let val = if enabled { 1u32 } else { 0u32 };
                let _ = RegSetValueExW(
                    hkey,
                    w!("LocalDimmingStatus"),
                    0,
                    REG_DWORD,
                    Some(std::slice::from_raw_parts(
                        &val as *const u32 as *const u8,
                        std::mem::size_of::<u32>(),
                    )),
                );
                let _ = RegCloseKey(hkey);
            }
        }
    }
}

fn execute_miniled_cmd_fallback(cmd: u64) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let ps_script = format!(
        r#"$inst = Get-CimInstance -Namespace 'root\WMI' -ClassName 'AcpiTest_MULong' -ErrorAction Stop | Where-Object {{ $_.InstanceName -like '*1_1' }} | Select-Object -First 1; if (-not $inst) {{ $inst = Get-CimInstance -Namespace 'root\WMI' -ClassName 'AcpiTest_MULong' -ErrorAction Stop | Select-Object -First 1 }}; if ($inst) {{ $cmd = [uint64]{}; Invoke-CimMethod -InputObject $inst -MethodName 'GetSetULong' -Arguments @{{ Data = $cmd }} -ErrorAction Stop; exit 0 }} else {{ exit 1 }}"#,
        cmd
    );

    let status = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &ps_script])
        .status();

    match status {
        Ok(s) => s.success(),
        Err(e) => {
            crate::log_warn!("MINILED", "PowerShell CIM fallback failed to launch: {}", e);
            false
        }
    }
}

#[tauri::command]
pub fn get_miniled_status() -> Result<bool, String> {
    if let Some(oem_status) = read_oem_miniled_registry() {
        return Ok(oem_status);
    }
    let cfg = crate::system::config::load_config();
    Ok(cfg.miniled_enabled)
}

#[tauri::command]
pub async fn set_miniled_status(enabled: bool) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cmd = if enabled {
            crate::hal::vault::miniled::CMD_MINILED_ON
        } else {
            crate::hal::vault::miniled::CMD_MINILED_OFF
        };

        let mut ok = crate::driver::wmi_mulong::mulong_exec_raw(cmd);
        if !ok {
            crate::log_info!("MINILED", "Native COM mulong_exec_raw returned false, running resilient CIM fallback...");
            ok = execute_miniled_cmd_fallback(cmd);
        }

        if ok {
            let mut cfg = crate::system::config::load_config();
            cfg.miniled_enabled = enabled;
            crate::system::config::save_config(&cfg);

            // Sync with OEM registry so official control center UI stays coherent
            write_oem_miniled_registry(enabled);

            crate::log_info!("MINILED", "MiniLED local dimming toggled to: {}", enabled);
            Ok(enabled)
        } else {
            crate::log_warn!("MINILED", "Failed to dispatch MiniLED local dimming command via ACPI-WMI");
            Err("Failed to dispatch MiniLED command to ACPI-WMI bridge".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}




// ---------------------------------------------------------------------------
// Cloud ICC Retrieval Commands (for Tongfang / Uniwill OEM factory profiles)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_cloud_icc_identity() -> Result<crate::system::cloud_icc::CloudIccIdentity, String> {
    tauri::async_runtime::spawn_blocking(crate::system::cloud_icc::get_cloud_icc_identity)
        .await
        .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn fetch_cloud_icc_profile(
    candidate: String,
    method: Option<String>,
    server_url: Option<String>,
) -> Result<crate::system::cloud_icc::CloudIccFetchHit, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let m = method.as_deref().unwrap_or("auto").to_lowercase();

        let target_servers: Vec<&str> = if let Some(ref s) = server_url {
            vec![s.as_str()]
        } else if m == "40" || m == "mac" {
            vec![
                crate::system::cloud_icc::SERVER_UNIWILL,
                crate::system::cloud_icc::SERVER_AISTONE,
            ]
        } else if m == "50" || m == "sn" {
            vec![
                crate::system::cloud_icc::SERVER_AISTONE,
                crate::system::cloud_icc::SERVER_UNIWILL,
            ]
        } else {
            let is_mac_candidate = candidate
                .split('_')
                .last()
                .map(|s| s.len() == 12 && s.chars().all(|c| c.is_ascii_hexdigit()))
                .unwrap_or(false);
            if is_mac_candidate {
                vec![
                    crate::system::cloud_icc::SERVER_UNIWILL,
                    crate::system::cloud_icc::SERVER_AISTONE,
                ]
            } else {
                vec![
                    crate::system::cloud_icc::SERVER_AISTONE,
                    crate::system::cloud_icc::SERVER_UNIWILL,
                ]
            }
        };

        for srv in target_servers {
            if let Some(hit) = crate::system::cloud_icc::probe_single_candidate(srv, &candidate, 6) {
                return Ok(hit);
            }
        }
        Err(format!("No cloud ICC found for candidate '{}'", candidate))
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}
