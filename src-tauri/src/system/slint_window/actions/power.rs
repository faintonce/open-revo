use tauri::AppHandle;
use tauri::Manager;
use super::super::sync::{sync_slint_power_schemes, sync_slint_power_state};

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "toggle_sleep_auto_office" => {
            if let Some(enable) = val.get("enable").and_then(|v| v.as_bool()) {
                let mut cfg = crate::system::config::load_config();
                cfg.sleep_auto_office = enable;
                let _ = crate::system::config::save_config(&cfg);
                sync_slint_power_state();
            }
            true
        }
        "toggle_master_sleep_guard" => {
            if let Some(enable) = val.get("enable").and_then(|v| v.as_bool()) {
                let mut cfg = crate::system::config::load_config();
                cfg.sleep_guard_enabled = enable;
                cfg.sleep_disconnect_net = enable;
                cfg.sleep_block_away = enable;
                cfg.sleep_ignore_sysreq = enable;
                cfg.sleep_disable_wake_timers = enable;
                cfg.sleep_deep_sleep = enable;
                cfg.sleep_hibernate_30m = enable;
                crate::commands::power_cmd::apply_all_sleep_guard(enable);
                let _ = crate::system::config::save_config(&cfg);
                sync_slint_power_state();
            }
            true
        }
        "toggle_sleep_guard_option" => {
            if let (Some(opt), Some(enable)) = (val.get("option").and_then(|v| v.as_str()), val.get("enable").and_then(|v| v.as_bool())) {
                let mut cfg = crate::system::config::load_config();
                match opt {
                    "disconnect_net" => cfg.sleep_disconnect_net = enable,
                    "block_away" => cfg.sleep_block_away = enable,
                    "ignore_sysreq" => cfg.sleep_ignore_sysreq = enable,
                    "disable_wake_timers" => cfg.sleep_disable_wake_timers = enable,
                    "deep_sleep" => cfg.sleep_deep_sleep = enable,
                    "hibernate_30m" => cfg.sleep_hibernate_30m = enable,
                    _ => {}
                }
                crate::commands::power_cmd::apply_sleep_guard_powercfg(opt, enable);
                let _ = crate::system::config::save_config(&cfg);
                sync_slint_power_state();
            }
            true
        }
        "toggle_power_option" => {
            if let (Some(opt), Some(enable)) = (val.get("option").and_then(|v| v.as_str()), val.get("enable").and_then(|v| v.as_bool())) {
                let mut cfg = crate::system::config::load_config();
                let _ = crate::commands::power_cmd::apply_power_option(
                    &mut cfg,
                    opt,
                    enable,
                    crate::driver::wmi_ec::wmi_modify_ec_mask,
                    crate::system::power_mode::set_windows_power_overlay,
                );
                let _ = crate::system::config::save_config(&cfg);
                sync_slint_power_state();
            }
            true
        }
        "select_battery_mode" => {
            if let Some(mode) = val.get("mode").and_then(|v| v.as_str()) {
                let limit = match mode {
                    "long_life" => 100,
                    "station" => 60,
                    _ => 80,
                };
                let mut cfg = crate::system::config::load_config();
                cfg.battery_mode = mode.to_string();
                cfg.battery_limit = limit;
                let _ = crate::system::config::save_config(&cfg);
                let ec = crate::driver::ec_bridge::get_global_ec();
                ec.lock().apply_battery_limit_with_override(limit, cfg.battery_hardware_limit_enabled);
                sync_slint_power_state();
            }
            true
        }
        "set_power_mode" => {
            if let Some(mode) = val.get("mode").and_then(|m| m.as_i64()) {
                let mode_u8 = mode as u8;
                let is_thin_and_light = crate::commands::power_cmd::is_thin_and_light_or_pure_igpu();
                let is_dc = crate::commands::power_cmd::is_dc_round_plugged_in();
                let mut cfg = crate::system::config::load_config();

                if !is_thin_and_light && (mode_u8 == 3 || mode_u8 == 4) && !is_dc && !cfg.unlock_battery_power_limit {
                    crate::log_warn!(
                        "SLINT_POWER",
                        "Blocked power mode switch to {} on battery/Type-C without unlock_battery_power_limit",
                        mode_u8
                    );
                    super::super::sync::sync_slint_switches_state();
                    super::super::sync::sync_slint_power_targets(cfg.power_mode);
                    return true;
                }

                if let Some(app) = app {
                    let state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                    if let Err(e) = crate::commands::hardware_cmd::set_power_mode(mode_u8, app.clone(), state) {
                        crate::log_warn!("SLINT_POWER", "hardware_cmd::set_power_mode failed: {}", e);
                        super::super::sync::sync_slint_switches_state();
                        super::super::sync::sync_slint_power_targets(cfg.power_mode);
                        return true;
                    }
                } else {
                    let ec = crate::driver::ec_bridge::get_global_ec();
                    let is_declarative = crate::hal::declarative::load_matched_manifest().is_some();
                    let is_tongfang_acpi_channel = crate::driver::wmi_ec::is_acpi_driver_available()
                        || crate::hal::declarative::load_matched_manifest()
                            .map(|m| m.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. })))
                            .unwrap_or(false);

                    if is_tongfang_acpi_channel {
                        ec.lock().set_power_mode(mode_u8);
                    } else if is_thin_and_light || is_declarative {
                        let hal = crate::hal::get_hardware_driver();
                        let hw_mode = match mode_u8 {
                            1 => crate::hal::HardwarePowerMode::Office,
                            2 => crate::hal::HardwarePowerMode::Balance,
                            3 => crate::hal::HardwarePowerMode::Turbo,
                            4 => crate::hal::HardwarePowerMode::Custom,
                            _ => crate::hal::HardwarePowerMode::Balance,
                        };
                        let _ = hal.set_performance_mode(hw_mode);
                        ec.lock().set_power_mode_from_hardware(mode_u8);
                        if mode_u8 == 4 {
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
                        ec.lock().set_power_mode(mode_u8);
                    }
                    cfg.power_mode = mode_u8;
                    if is_dc {
                        cfg.power_mode_ac = mode_u8;
                    } else {
                        cfg.power_mode_battery = mode_u8;
                    }
                    let _ = crate::system::config::save_config(&cfg);
                    crate::update_tray_power_mode(mode_u8);
                    crate::system::osd_window::show_power_mode(mode_u8);
                    crate::system::native_wry::broadcast_event("physical-mode-switched", &serde_json::json!(mode_u8));
                }
                sync_slint_power_state();
                super::super::sync::sync_slint_power_targets(mode_u8);
            }
            true
        }
        "set_battery_limit" => {
            if let Some(limit) = val.get("battery_limit").and_then(|b| b.as_i64()) {
                let limit_u8 = limit as u8;
                if let Some(app) = app {
                    let state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                    let _ = crate::commands::hardware_cmd::set_battery_limit(limit_u8, state);
                } else {
                    let mut cfg = crate::system::config::load_config();
                    cfg.battery_limit = limit_u8;
                    let _ = crate::system::config::save_config(&cfg);
                    let ec = crate::driver::ec_bridge::get_global_ec();
                    ec.lock().apply_battery_limit_with_override(limit_u8, cfg.battery_hardware_limit_enabled);
                }
                sync_slint_power_state();
            }
            true
        }
        "switch_power_scheme" => {
            if let Some(guid) = val.get("guid").and_then(|v| v.as_str()) {
                let _ = crate::commands::power_cmd::set_active_windows_power_scheme(guid.to_string());
                sync_slint_power_schemes(None);
            }
            true
        }
        "refresh_power_schemes" => {
            sync_slint_power_schemes(None);
            true
        }
        "restore_official_power_plan" => {
            let res = crate::commands::power_cmd::restore_official_power_plan();
            let msg = match res {
                Ok(s) => s,
                Err(e) => format!("修复失败: {}", e),
            };
            sync_slint_power_schemes(Some(&msg));
            true
        }
        _ => false,
    }
}
