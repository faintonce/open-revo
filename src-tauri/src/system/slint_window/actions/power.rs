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
                let Ok(mode) = u8::try_from(mode) else {
                    crate::log_warn!("SLINT", "Invalid performance mode: {}", mode);
                    return true;
                };
                let ec = crate::driver::ec_bridge::get_global_ec();
                if let Err(error) = crate::commands::power_cmd::set_power_mode_internal(mode, app, &ec) {
                    crate::log_warn!("SLINT", "Performance mode change failed: {}", error);
                    let current = ec.lock().get_power_mode();
                    crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
                        "action": "sync_state", "power_mode": current
                    }));
                }
                sync_slint_power_state();
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
