// OpenRevo - Power Management Commands
// Handles performance modes, fan boost, battery protection limits, sleep guards, and Windows power schemes.

use tauri::{AppHandle, Emitter, State};
use crate::driver::ec_bridge::SharedEcBridge;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PowerSettingsInfo {
    pub battery_percent: u8,
    pub is_ac_online: bool,
    pub is_charging: bool,           // Real hardware charging state (Win32 BatteryFlag & 0x08)
    pub battery_mode: String,        // "long_life" (100) | "balance" (80) | "station" (60)
    pub battery_limit: u8,           // actual hardware limit: 100, 80, 60, etc.
    pub battery_hardware_limit_enabled: bool,
    pub usb_charge_enabled: bool,
    pub ac_recovery_enabled: bool,
    pub high_perf_scheme: bool,
    pub sleep_guard_enabled: bool,
    pub sleep_disconnect_net: bool,
    pub sleep_block_away: bool,
    pub sleep_ignore_sysreq: bool,
    pub sleep_disable_wake_timers: bool,
    pub sleep_deep_sleep: bool,
    pub sleep_hibernate_30m: bool,
    pub sleep_auto_office: bool,
}

pub use crate::system::power::ppm::WindowsPowerScheme;

#[repr(C)]
struct SYSTEM_POWER_STATUS {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

extern "system" {
    fn GetSystemPowerStatus(lpSystemPowerStatus: *mut SYSTEM_POWER_STATUS) -> i32;
}

pub fn get_live_power_status() -> (u8, bool, bool) {
    let mut status = SYSTEM_POWER_STATUS {
        ac_line_status: 1,
        battery_flag: 0,
        battery_life_percent: 100,
        system_status_flag: 0,
        battery_life_time: 0,
        battery_full_life_time: 0,
    };
    let success = unsafe { GetSystemPowerStatus(&mut status) };
    if success != 0 {
        let percent = if status.battery_life_percent <= 100 {
            status.battery_life_percent
        } else {
            100
        };
        let is_ac = status.ac_line_status == 1;
        let is_charging = (status.battery_flag & 0x08) != 0;
        (percent, is_ac, is_charging)
    } else {
        (100, true, false)
    }
}

static DGPU_CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub fn is_discrete_gpu_name(name: &str) -> bool {
    crate::driver::gpu_bridge::is_discrete_gpu_name(name)
}

pub fn check_dgpu_present() -> bool {
    *DGPU_CACHE.get_or_init(|| {
        if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
            if !manifest.capabilities.discrete_gpu {
                return false;
            }
        }

        #[cfg(windows)]
        unsafe {
            use windows::core::{PCWSTR, PWSTR};
            use windows::Win32::System::Registry::{
                RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
            };

            // 0. Quick OEM registry contract probe
            let oem_key_str: Vec<u16> = "SOFTWARE\\OEM\\GamingCenter2\\ItemSupport\0".encode_utf16().collect();
            let mut h_oem = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(oem_key_str.as_ptr()), 0, KEY_READ, &mut h_oem).is_ok() {
                let mut val: u32 = 0;
                let mut val_size = std::mem::size_of::<u32>() as u32;
                let mut val_type = windows::Win32::System::Registry::REG_DWORD;
                let is_nv_val: Vec<u16> = "IsNvGpu\0".encode_utf16().collect();
                if RegQueryValueExW(
                    h_oem,
                    PCWSTR(is_nv_val.as_ptr()),
                    None,
                    Some(&mut val_type),
                    Some(&mut val as *mut u32 as *mut _),
                    Some(&mut val_size),
                ).is_ok() && val == 1 {
                    let _ = RegCloseKey(h_oem);
                    return true;
                }
                let _ = RegCloseKey(h_oem);
            }

            // 1. Dynamic Display Class registry enumeration (supports unlimited driver re-installations)
            let base_class_str: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\0".encode_utf16().collect();
            let mut h_base = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(base_class_str.as_ptr()), 0, KEY_READ, &mut h_base).is_ok() {
                let mut index = 0;
                loop {
                    let mut name_buf = [0u16; 64];
                    let mut name_len = name_buf.len() as u32;
                    let ret = RegEnumKeyExW(
                        h_base,
                        index,
                        PWSTR(name_buf.as_mut_ptr()),
                        &mut name_len,
                        None,
                        PWSTR::null(),
                        None,
                        None,
                    );
                    if ret.is_err() {
                        break;
                    }

                    let sub_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                    if sub_name.len() == 4 && sub_name.chars().all(|c| c.is_ascii_digit()) {
                        let full_subkey = format!("SYSTEM\\CurrentControlSet\\Control\\Class\\{{4d36e968-e325-11ce-bfc1-08002be10318}}\\{}\0", sub_name);
                        let wide_full: Vec<u16> = full_subkey.encode_utf16().collect();
                        let mut hkey = HKEY::default();
                        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_full.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
                            let mut buf = [0u16; 256];
                            let mut size = (buf.len() * 2) as u32;
                            let mut data_type = REG_SZ;
                            let val_name: Vec<u16> = "DriverDesc\0".encode_utf16().collect();

                            if RegQueryValueExW(
                                hkey,
                                PCWSTR(val_name.as_ptr()),
                                None,
                                Some(&mut data_type),
                                Some(buf.as_mut_ptr() as *mut _),
                                Some(&mut size),
                            ).is_ok() {
                                let len = (size as usize / 2).saturating_sub(1);
                                let desc = String::from_utf16_lossy(&buf[..len]);
                                if is_discrete_gpu_name(&desc) {
                                    let _ = RegCloseKey(hkey);
                                    let _ = RegCloseKey(h_base);
                                    return true;
                                }
                            }
                            let _ = RegCloseKey(hkey);
                        }
                    }
                    index += 1;
                }
                let _ = RegCloseKey(h_base);
            }
        }

        let gpu_bridge = crate::driver::gpu_bridge::get_global_gpu();
        let bridge_name = gpu_bridge.lock().get_name(0);
        if is_discrete_gpu_name(&bridge_name) {
            return true;
        }

        false
    })
}

/// Determines whether the machine is a thin-and-light laptop or pure iGPU laptop
pub fn is_thin_and_light_or_pure_igpu() -> bool {
    // 1. Unified ultrabook / thin-and-light platform probe check
    if crate::hal::platform_probe::is_ultrabook() {
        return true;
    }

    // 2. Driver backend check
    let hal = crate::hal::get_hardware_driver();
    if hal.kind() != crate::hal::DriverKind::TongfangAcpi {
        return true;
    }

    // 3. Physical discrete GPU presence check
    if !check_dgpu_present() {
        return true;
    }

    false
}

/// Pure decision function for power online status across laptop topologies.
pub fn decide_power_online_status(is_thin_and_light: bool, ec_pwr: Option<u8>, is_ac: bool) -> bool {
    if let Some(pwr) = ec_pwr {
        if (pwr & crate::driver::wmi_ec::POWER_STATUS_DC_JACK) != 0 {
            return true;
        }

        if (pwr & crate::driver::wmi_ec::POWER_STATUS_TYPE_C_MASK) != 0 {
            if is_thin_and_light {
                return true;
            } else {
                crate::log_info!("POWER", "Gating high power modes on gaming laptop: Type-C PD active (PowerStatus=0x{:02X})", pwr);
                return false;
            }
        }
    }

    if !is_ac {
        return false;
    }

    if is_thin_and_light {
        return true;
    }

    true
}

/// Check whether the laptop is powered by the full-wattage power supply.
pub fn is_dc_round_plugged_in() -> bool {
    let is_thin_and_light = is_thin_and_light_or_pure_igpu();
    let ec_pwr = if crate::hal::platform_probe::is_tongfang_family() {
        crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_POWER_STATUS)
    } else {
        None
    };
    let (_, is_ac, _) = get_live_power_status();
    decide_power_online_status(is_thin_and_light, ec_pwr, is_ac)
}

#[tauri::command]
pub fn set_power_mode(mode: u8, app: AppHandle, ec_state: State<'_, SharedEcBridge>) -> Result<(), String> {
    let is_thin_and_light = is_thin_and_light_or_pure_igpu();
    let is_dc = is_dc_round_plugged_in();
    let mut cfg = crate::system::config::load_config();
    // Gaming laptops must not run Beast (3) or Custom (4) on battery/Type-C to avoid PD overcurrent trips.
    // Thin & light laptops rely on onboard EC power envelopes (e.g. 28W/30W/35W) and are fully exempt.
    if !is_thin_and_light && (mode == 3 || mode == 4) && !is_dc && !cfg.unlock_battery_power_limit {
        let msg = if check_dgpu_present() {
            "在电池或 Type-C 供电下，狂暴模式与自定义模式不可用。请连接 DC 圆口电源适配器，或在【系统】设置中开启“解除离电限制”。"
        } else {
            "在电池供电下，狂暴模式与自定义模式不可用。请连接电源适配器，或在【系统】设置中开启“解除离电限制”。"
        };
        return Err(msg.to_string());
    }
    let is_declarative = crate::hal::declarative::load_matched_manifest().is_some();
    let is_tongfang_acpi_channel = crate::driver::wmi_ec::is_acpi_driver_available()
        || crate::hal::declarative::load_matched_manifest()
            .map(|m| m.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. })))
            .unwrap_or(false);

    if is_tongfang_acpi_channel {
        // Tongfang ODM models (both Gaming laptops and Thin & Lights like Wujie 14X/15X):
        // Must invoke apply_full_power_mode to commit physical MODE_INDEX, TRIGGER_BYTE, and PL targets!
        ec_state.lock().set_power_mode(mode);
    } else if is_thin_and_light || is_declarative {
        let hal = crate::hal::get_hardware_driver();
        let hw_mode = match mode {
            1 => crate::hal::HardwarePowerMode::Office,
            2 => crate::hal::HardwarePowerMode::Balance,
            3 => crate::hal::HardwarePowerMode::Turbo,
            4 => crate::hal::HardwarePowerMode::Custom,
            _ => crate::hal::HardwarePowerMode::Balance,
        };
        if let Err(e) = hal.set_performance_mode(hw_mode) {
            crate::log_warn!("POWER_CMD", "Direct HAL set_performance_mode failed: {}", e);
        } else {
            crate::log_info!("POWER_CMD", "Direct HAL applied performance mode {:?}", hw_mode);
        }
        ec_state.lock().set_power_mode_from_hardware(mode);
        if mode == 4 {
            if cfg.custom_profile.fan_isolated_output || cfg.custom_profile.fan_curve_enabled {
                crate::driver::bld_driver_mgr::set_bld_fan_curve(
                    cfg.custom_profile.cpu_fan_curve.clone(),
                    cfg.custom_profile.gpu_fan_curve.clone(),
                    cfg.custom_profile.fan_ramp_rate,
                    cfg.custom_profile.fan_ramp_speed_ms,
                );
            } else if !cfg.fan_boost {
                crate::driver::bld_driver_mgr::restore_bld_fan_auto();
            }
        } else if !cfg.fan_boost {
            crate::driver::bld_driver_mgr::restore_bld_fan_auto();
        }
    } else {
        ec_state.lock().set_power_mode(mode);
    }
    cfg.power_mode = mode;
    if is_dc {
        cfg.power_mode_ac = mode;
    } else {
        cfg.power_mode_battery = mode;
    }
    crate::system::config::save_config(&cfg);
    crate::update_tray_power_mode(mode);
    crate::system::osd_window::show_power_mode(mode);
    crate::system::osd_window::wake_osd_if_enabled(&app);
    let _ = app.emit("physical-mode-switched", mode);
    let _ = app.emit_to("main", "physical-mode-switched", mode);
    let _ = app.emit_to("osd", "physical-mode-switched", mode);
    Ok(())
}

#[tauri::command]
pub fn set_fan_boost(
    app: AppHandle,
    enable: bool,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<(), String> {
    let support_flags = crate::driver::wmi_ec::get_hardware_support_flags();
    if !support_flags.has_fan_boost {
        crate::log_info!("POWER_CMD", "Fan boost is not supported on this platform.");
        return Ok(());
    }
    ec_state.lock().set_fan_boost(enable);
    let mut cfg = crate::system::config::load_config();
    cfg.fan_boost = enable;
    crate::system::config::save_config(&cfg);
    crate::update_tray_fan_boost(enable);

    crate::system::osd_window::show_fan_boost(enable);
    crate::system::osd_window::wake_osd_if_enabled(&app);
    let _ = app.emit("fan-boost-changed", enable);
    let _ = app.emit_to("main", "fan-boost-changed", enable);
    let _ = app.emit_to("osd", "fan-boost-changed", enable);
    let _ = app.emit("physical-fan-boost-switched", enable);
    let _ = app.emit_to("main", "physical-fan-boost-switched", enable);
    let _ = app.emit_to("osd", "physical-fan-boost-switched", enable);

    Ok(())
}

#[tauri::command]
pub fn set_battery_limit(limit: u8, ec_state: State<'_, SharedEcBridge>) -> Result<(), String> {
    ec_state.lock().set_battery_limit(limit);
    let mut cfg = crate::system::config::load_config();
    cfg.battery_limit = limit;
    cfg.battery_mode = if limit >= 95 {
        "long_life".to_string()
    } else if limit <= 65 {
        "station".to_string()
    } else {
        "balance".to_string()
    };
    crate::system::config::save_config(&cfg);
    Ok(())
}

#[tauri::command]
pub fn get_power_settings(
    ec_state: State<'_, SharedEcBridge>,
) -> PowerSettingsInfo {
    let cfg = crate::system::config::load_config();
    let (percent, _, is_charging) = get_live_power_status();
    let is_dc = is_dc_round_plugged_in();
    let hw_limit = ec_state.lock().get_battery_limit();
    let effective_limit = if hw_limit > 0 { hw_limit } else { cfg.battery_limit };
    let mode = if effective_limit >= 95 {
        "long_life".to_string()
    } else if effective_limit <= 65 {
        "station".to_string()
    } else {
        "balance".to_string()
    };

    let usb_charge = if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
        let is_on = (v & crate::driver::wmi_ec::TRIGGER_USB_CHARGE) != 0;
        if cfg.usb_charge_enabled && !is_on {
            crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, v | crate::driver::wmi_ec::TRIGGER_USB_CHARGE);
            true
        } else {
            is_on
        }
    } else {
        cfg.usb_charge_enabled
    };

    let ac_recovery = if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY) {
        let is_on = (v & crate::driver::wmi_ec::AC_RECOVERY_BITMASK) != 0;
        if cfg.ac_recovery_enabled && !is_on {
            crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY, v | crate::driver::wmi_ec::AC_RECOVERY_BITMASK);
            true
        } else {
            is_on
        }
    } else {
        cfg.ac_recovery_enabled
    };

#[cfg(target_os = "windows")]
fn get_active_power_scheme_guid() -> String {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RegOpenKeyExW, RegQueryValueExW, RegCloseKey, KEY_READ, REG_SZ};
    use windows::core::PCWSTR;

    let base_path: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes\0".encode_utf16().collect();
    let val_name: Vec<u16> = "ActivePowerScheme\0".encode_utf16().collect();
    let mut h_base = windows::Win32::System::Registry::HKEY::default();

    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(base_path.as_ptr()), 0, KEY_READ, &mut h_base).is_ok() {
            let mut buf = [0u16; 64];
            let mut buf_len = (buf.len() * 2) as u32;
            let mut val_type = REG_SZ;
            let res = RegQueryValueExW(
                h_base,
                PCWSTR(val_name.as_ptr()),
                None,
                Some(&mut val_type),
                Some(buf.as_mut_ptr() as *mut u8),
                Some(&mut buf_len),
            );
            let _ = RegCloseKey(h_base);
            if res.is_ok() {
                let char_len = (buf_len / 2) as usize;
                let slice = if char_len > 0 && buf[char_len - 1] == 0 {
                    &buf[..char_len - 1]
                } else {
                    &buf[..char_len]
                };
                let guid_str = String::from_utf16_lossy(slice).trim().to_lowercase();
                if guid_str.len() == 36 {
                    return guid_str;
                }
            }
        }
    }
    "381b4222-f694-41f0-9685-ff5bb260df2e".to_string()
}

#[cfg(target_os = "windows")]
fn read_power_setting_dc_dword(scheme_guid: &str, subgroup: &str, setting: &str) -> Option<u32> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RegOpenKeyExW, RegQueryValueExW, RegCloseKey, KEY_READ, REG_DWORD};
    use windows::core::PCWSTR;

    let try_paths = if subgroup.is_empty()
        || subgroup.eq_ignore_ascii_case("sub_none")
        || subgroup.eq_ignore_ascii_case("fea3413e-7e05-4911-9a71-700331f1c294")
    {
        vec![
            format!(
                "SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes\\{}\\{}\0",
                scheme_guid, setting
            ),
            format!(
                "SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes\\{}\\fea3413e-7e05-4911-9a71-700331f1c294\\{}\0",
                scheme_guid, setting
            ),
        ]
    } else {
        vec![
            format!(
                "SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes\\{}\\{}\\{}\0",
                scheme_guid, subgroup, setting
            ),
            format!(
                "SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes\\{}\\{}\0",
                scheme_guid, setting
            ),
        ]
    };

    let val_name: Vec<u16> = "DCSettingIndex\0".encode_utf16().collect();

    for path_str in try_paths {
        let path_w: Vec<u16> = path_str.encode_utf16().collect();
        let mut h_key = windows::Win32::System::Registry::HKEY::default();

        unsafe {
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(path_w.as_ptr()), 0, KEY_READ, &mut h_key).is_ok() {
                let mut val: u32 = 0;
                let mut val_len = std::mem::size_of::<u32>() as u32;
                let mut val_type = REG_DWORD;
                let res = RegQueryValueExW(
                    h_key,
                    PCWSTR(val_name.as_ptr()),
                    None,
                    Some(&mut val_type),
                    Some(&mut val as *mut u32 as *mut u8),
                    Some(&mut val_len),
                );
                let _ = RegCloseKey(h_key);
                if res.is_ok() {
                    return Some(val);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub fn query_sleep_guard_registry_states(cfg: &crate::system::config::AppConfig) -> (bool, bool, bool, bool, bool, bool, bool) {
    let scheme = get_active_power_scheme_guid();

    // 1. disconnect_net: SUB_NONE / f15576e8... (0 = disconnect network during standby)
    let disconnect_net = read_power_setting_dc_dword(
        &scheme,
        "SUB_NONE",
        "f15576e8-98b7-4186-b944-eafa664402d9",
    ).map(|v| v == 0).unwrap_or(cfg.sleep_disconnect_net);

    // 2. block_away: 238c9fa8... / 25dfa149... (0 = disable away mode)
    let block_away = read_power_setting_dc_dword(
        &scheme,
        "238c9fa8-0aad-41ed-83f4-97be242c8f20",
        "25dfa149-5dd1-4736-b5ab-e8a37b5b8187",
    ).map(|v| v == 0).unwrap_or(cfg.sleep_block_away);

    // 3. ignore_sysreq: 238c9fa8... / a4b195f5... (0 = ignore system required requests)
    let ignore_sysreq = read_power_setting_dc_dword(
        &scheme,
        "238c9fa8-0aad-41ed-83f4-97be242c8f20",
        "a4b195f5-8225-47d8-8012-9d41369786e2",
    ).map(|v| v == 0).unwrap_or(cfg.sleep_ignore_sysreq);

    // 4. disable_wake_timers: 238c9fa8... / bd3b718a... (0 = disable wake timers)
    let disable_wake_timers = read_power_setting_dc_dword(
        &scheme,
        "238c9fa8-0aad-41ed-83f4-97be242c8f20",
        "bd3b718a-0680-4d9d-8ab2-e1d2b4ac806d",
    ).map(|v| v == 0).unwrap_or(cfg.sleep_disable_wake_timers);

    // 5. deep_sleep: 2e601130... / d502f7ee... (1 = enable deep sleep)
    let deep_sleep = read_power_setting_dc_dword(
        &scheme,
        "2e601130-5351-4d9d-8e04-252966bad054",
        "d502f7ee-1dc7-4efd-a55d-f04b6f5c0545",
    ).map(|v| v == 1).unwrap_or(cfg.sleep_deep_sleep);

    // 6. hibernate_30m: 238c9fa8... / 9d7815a6... (timeout <= 1800s and > 0)
    let hibernate_30m = read_power_setting_dc_dword(
        &scheme,
        "238c9fa8-0aad-41ed-83f4-97be242c8f20",
        "9d7815a6-7ee4-497e-8888-515a05f02364",
    ).map(|v| v > 0 && v <= 1800).unwrap_or(cfg.sleep_hibernate_30m);

    // Master guard: all 6 active policies must be met
    let master_guard = disconnect_net && block_away && ignore_sysreq && disable_wake_timers && deep_sleep && hibernate_30m;

    (master_guard, disconnect_net, block_away, ignore_sysreq, disable_wake_timers, deep_sleep, hibernate_30m)
}

#[cfg(not(target_os = "windows"))]
pub fn query_sleep_guard_registry_states(cfg: &crate::system::config::AppConfig) -> (bool, bool, bool, bool, bool, bool, bool) {
    (
        cfg.sleep_guard_enabled,
        cfg.sleep_disconnect_net,
        cfg.sleep_block_away,
        cfg.sleep_ignore_sysreq,
        cfg.sleep_disable_wake_timers,
        cfg.sleep_deep_sleep,
        cfg.sleep_hibernate_30m,
    )
}

    let (
        sleep_guard_enabled,
        sleep_disconnect_net,
        sleep_block_away,
        sleep_ignore_sysreq,
        sleep_disable_wake_timers,
        sleep_deep_sleep,
        sleep_hibernate_30m,
    ) = query_sleep_guard_registry_states(&cfg);

    PowerSettingsInfo {
        battery_percent: percent,
        is_ac_online: is_dc,
        is_charging,
        battery_mode: mode,
        battery_limit: effective_limit,
        battery_hardware_limit_enabled: cfg.battery_hardware_limit_enabled,
        usb_charge_enabled: usb_charge,
        ac_recovery_enabled: ac_recovery,
        high_perf_scheme: cfg.high_perf_scheme,
        sleep_guard_enabled,
        sleep_disconnect_net,
        sleep_block_away,
        sleep_ignore_sysreq,
        sleep_disable_wake_timers,
        sleep_deep_sleep,
        sleep_hibernate_30m,
        sleep_auto_office: cfg.sleep_auto_office,
    }
}

pub fn apply_sleep_guard_powercfg(option: &str, enable: bool) {
    if let Err(e) = crate::system::power::ppm::apply_sleep_guard_option(option, enable) {
        crate::log_warn!("POWER_CMD", "Failed to apply sleep guard option {}: {}", option, e);
    }
}

pub fn apply_all_sleep_guard(enable: bool) {
    crate::system::power::ppm::apply_all_sleep_guard(enable);
}

#[tauri::command]
pub fn set_sleep_guard_option(
    option: String,
    enable: bool,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<PowerSettingsInfo, String> {
    let mut cfg = crate::system::config::load_config();
    match option.as_str() {
        "disconnect_net" => cfg.sleep_disconnect_net = enable,
        "block_away" => cfg.sleep_block_away = enable,
        "ignore_sysreq" => cfg.sleep_ignore_sysreq = enable,
        "disable_wake_timers" => cfg.sleep_disable_wake_timers = enable,
        "deep_sleep" => cfg.sleep_deep_sleep = enable,
        "hibernate_30m" => cfg.sleep_hibernate_30m = enable,
        _ => return Err("Invalid sleep guard option".into()),
    }
    apply_sleep_guard_powercfg(&option, enable);
    crate::system::config::save_config(&cfg);
    Ok(get_power_settings(ec_state))
}

#[tauri::command]
pub fn set_master_sleep_guard(
    enable: bool,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<PowerSettingsInfo, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.sleep_guard_enabled = enable;
    cfg.sleep_disconnect_net = enable;
    cfg.sleep_block_away = enable;
    cfg.sleep_ignore_sysreq = enable;
    cfg.sleep_disable_wake_timers = enable;
    cfg.sleep_deep_sleep = enable;
    cfg.sleep_hibernate_30m = enable;
    apply_all_sleep_guard(enable);
    crate::system::config::save_config(&cfg);
    Ok(get_power_settings(ec_state))
}

#[tauri::command]
pub fn set_sleep_auto_office(
    enable: bool,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<PowerSettingsInfo, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.sleep_auto_office = enable;
    crate::system::config::save_config(&cfg);
    Ok(get_power_settings(ec_state))
}

#[tauri::command]
pub fn set_battery_mode(
    mode: String,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<PowerSettingsInfo, String> {
    let limit = match mode.as_str() {
        "long_life" => 100,
        "station" => 60,
        _ => 80, // "balance"
    };

    let mut cfg = crate::system::config::load_config();
    cfg.battery_mode = mode.clone();
    cfg.battery_limit = limit;
    crate::system::config::save_config(&cfg);

    ec_state.lock().apply_battery_limit_with_override(limit, cfg.battery_hardware_limit_enabled);

    Ok(get_power_settings(ec_state))
}

#[tauri::command]
pub fn set_battery_hardware_limit(
    enable: bool,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<PowerSettingsInfo, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.battery_hardware_limit_enabled = enable;
    crate::system::config::save_config(&cfg);

    ec_state.lock().set_battery_hardware_limit_override(enable);

    Ok(get_power_settings(ec_state))
}

#[tauri::command]
pub fn set_power_option(
    option: String,
    enable: bool,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<PowerSettingsInfo, String> {
    let mut cfg = crate::system::config::load_config();
    match option.as_str() {
        "usb_charge" => {
            cfg.usb_charge_enabled = enable;
            if let Some(mut val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
                if enable { val |= crate::driver::wmi_ec::TRIGGER_USB_CHARGE; } else { val &= !crate::driver::wmi_ec::TRIGGER_USB_CHARGE; }
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, val);
            } else {
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, if enable { crate::driver::wmi_ec::TRIGGER_USB_CHARGE } else { 0x00 });
            }
        }
        "ac_recovery" => {
            cfg.ac_recovery_enabled = enable;
            if let Some(mut val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY) {
                if enable { val |= crate::driver::wmi_ec::AC_RECOVERY_BITMASK; } else { val &= !crate::driver::wmi_ec::AC_RECOVERY_BITMASK; }
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY, val);
            } else {
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY, if enable { crate::driver::wmi_ec::AC_RECOVERY_BITMASK } else { 0x00 });
            }
        }
        "high_perf" => {
            cfg.high_perf_scheme = enable;
            if enable {
                crate::system::power_mode::apply_windows_power_overlay(cfg.power_mode);
            }
        }
        _ => return Err("Invalid power option".into()),
    }
    crate::system::config::save_config(&cfg);
    Ok(get_power_settings(ec_state))
}

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

#[tauri::command]
pub fn get_windows_power_schemes() -> Result<Vec<WindowsPowerScheme>, String> {
    #[cfg(target_os = "windows")]
    {
        crate::system::power::ppm::get_windows_power_schemes()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn set_active_windows_power_scheme(guid: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        crate::system::power::ppm::set_active_windows_power_scheme(&guid)?;
        let mut cfg = crate::system::config::load_config();
        cfg.preferred_power_scheme = Some(guid.trim().to_string());
        crate::system::config::save_config(&cfg);
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = guid;
        Ok(())
    }
}

#[tauri::command]
pub fn restore_official_power_plan() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        crate::system::power::ppm::restore_official_power_plan()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok("Non-Windows OS".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discrete_gpu_name_classification() {
        assert!(is_discrete_gpu_name("NVIDIA GeForce RTX 4060 Laptop GPU"));
        assert!(is_discrete_gpu_name("NVIDIA GeForce RTX 3070 Laptop GPU"));
        assert!(is_discrete_gpu_name("NVIDIA GeForce GTX 1650 Ti"));
        assert!(is_discrete_gpu_name("AMD Radeon RX 7600M XT"));
        assert!(is_discrete_gpu_name("Intel(R) Arc(TM) A730M Graphics"));
        assert!(is_discrete_gpu_name("Intel Arc A370M"));
        assert!(is_discrete_gpu_name("Intel Arc Pro A60M"));

        assert!(!is_discrete_gpu_name("Intel(R) Arc(TM) Graphics"));
        assert!(!is_discrete_gpu_name("Intel Arc Graphics"));
        assert!(!is_discrete_gpu_name("AMD Radeon 780M Graphics"));
        assert!(!is_discrete_gpu_name("AMD Radeon 890M Graphics"));
        assert!(!is_discrete_gpu_name("AMD Radeon(TM) Graphics"));
        assert!(!is_discrete_gpu_name("Intel(R) Iris(R) Xe Graphics"));
        assert!(!is_discrete_gpu_name("Intel(R) UHD Graphics"));
    }

    #[test]
    fn test_wujie_thin_and_light_power_decision() {
        assert!(decide_power_online_status(true, Some(0x02), true));
        assert!(decide_power_online_status(true, Some(0x06), true));
        assert!(decide_power_online_status(true, Some(0x14), true));
        assert!(decide_power_online_status(true, Some(0x02), false));
        assert!(!decide_power_online_status(true, Some(0x00), false));
        assert!(decide_power_online_status(true, Some(0x00), true));
        assert!(decide_power_online_status(true, None, true));
    }

    #[test]
    fn test_gaming_laptop_power_decision() {
        assert!(decide_power_online_status(false, Some(0x81), true));
        assert!(decide_power_online_status(false, Some(0x01), true));
        assert!(decide_power_online_status(false, Some(0x81), false));
        assert!(!decide_power_online_status(false, Some(0x02), true));
        assert!(!decide_power_online_status(false, Some(0x06), true));
        assert!(!decide_power_online_status(false, Some(0x3E), true));
        assert!(!decide_power_online_status(false, Some(0x00), false));
        assert!(decide_power_online_status(false, Some(0x00), true));
        assert!(decide_power_online_status(false, None, true));
        assert!(!decide_power_online_status(false, Some(0x00), false));
        assert!(!decide_power_online_status(false, None, false));
    }

    #[test]
    fn test_dual_power_mode_defaults_and_migration() {
        let default_cfg = crate::system::config::AppConfig::default();
        assert_eq!(default_cfg.power_mode, 2);
        assert_eq!(default_cfg.power_mode_ac, 3);
        assert_eq!(default_cfg.power_mode_battery, 2);

        // Test migration from legacy JSON with saved_power_mode_on_battery
        let legacy_json = r#"{"power_mode": 2, "saved_power_mode_on_battery": 3}"#;
        let parsed: crate::system::config::AppConfig = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(parsed.power_mode, 2);
        assert_eq!(parsed.power_mode_ac, 3); // defaulted
        assert_eq!(parsed.power_mode_battery, 2); // defaulted
    }
}
