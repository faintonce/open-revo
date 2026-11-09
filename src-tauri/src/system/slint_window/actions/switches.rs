use tauri::AppHandle;
use tauri::Manager;

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "toggle_fan_boost" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.fan_boost;
            if let Some(app) = app {
                let state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                let _ = crate::commands::hardware_cmd::set_fan_boost(app.clone(), next, state);
            } else {
                let support_flags = crate::driver::wmi_ec::get_hardware_support_flags();
                if support_flags.has_fan_boost {
                    let ec = crate::driver::ec_bridge::get_global_ec();
                    ec.lock().set_fan_boost(next);
                    let mut cfg = crate::system::config::load_config();
                    cfg.fan_boost = next;
                    let _ = crate::system::config::save_config(&cfg);
                    crate::update_tray_fan_boost(next);
                    crate::system::osd_window::show_fan_boost(next);
                    crate::system::native_wry::broadcast_event("fan-boost-changed", &serde_json::json!(next));
                }
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_camera" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.camera_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "camera".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("camera", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_mic" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.mic_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "mic".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("mic", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_win_lock" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.win_key_locked;
            let _ = crate::commands::hardware_cmd::set_win_key_locked(next);
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_keep_awake" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.keep_awake_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "keep_awake".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("keep_awake", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_autostart" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.autostart;
            let _ = crate::commands::hardware_cmd::set_autostart_enabled(next);
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_touchpad" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.touchpad;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "touchpad".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("touchpad", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_osd" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.osd;
            let app_handle = app.cloned().or_else(|| crate::system::slint_window::SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone()));
            if let Some(a) = app_handle {
                let _ = crate::commands::switches_cmd::set_device_switch(a, "osd".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("osd", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_fn_lock" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.fn_lock;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "fn_lock".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("fn_lock", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_wifi" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.wifi;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "wifi".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("wifi", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "toggle_bt" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.bluetooth;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "bluetooth".into(), next);
            } else {
                let _ = crate::system::device_switches::set_switch("bluetooth", next);
            }
            super::super::sync::sync_slint_switches_state();
            true
        }
        "set_balanced_db" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                let mut cfg = crate::system::config::load_config();
                cfg.balanced_gpu_dynamic_boost = en;
                let _ = crate::system::config::save_config(&cfg);
                super::super::sync::sync_slint_power_targets(cfg.power_mode);
            }
            true
        }
        "set_gpu_oc_core" => {
            if let Some(m) = val.get("mhz").and_then(|v| v.as_i64()) {
                let mut cfg = crate::system::config::load_config();
                cfg.gpu_oc_core_mhz = m.clamp(0, 250) as i32;
                let _ = crate::system::config::save_config(&cfg);
            }
            true
        }
        "set_gpu_oc_mem" => {
            if let Some(m) = val.get("mhz").and_then(|v| v.as_i64()) {
                let mut cfg = crate::system::config::load_config();
                cfg.gpu_oc_mem_mhz = m.clamp(0, 1000) as i32;
                let _ = crate::system::config::save_config(&cfg);
            }
            true
        }
        "set_osd_theme" => {
            if let Some(t) = val.get("theme").and_then(|v| v.as_str()) {
                let mut cfg = crate::system::config::load_config();
                cfg.osd_theme = t.to_string();
                let _ = crate::system::config::save_config(&cfg);
                crate::system::osd_window::send_osd_ipc_message(serde_json::json!({
                    "action": "config",
                    "theme": cfg.osd_theme,
                    "scale": cfg.osd_scale,
                    "position": cfg.osd_position,
                }));
            }
            true
        }
        "set_osd_scale" => {
            if let Some(s) = val.get("scale").and_then(|v| v.as_str()) {
                let mut cfg = crate::system::config::load_config();
                cfg.osd_scale = s.to_string();
                let _ = crate::system::config::save_config(&cfg);
                crate::system::osd_window::send_osd_ipc_message(serde_json::json!({
                    "action": "config",
                    "theme": cfg.osd_theme,
                    "scale": cfg.osd_scale,
                    "position": cfg.osd_position,
                }));
            }
            true
        }
        "set_osd_position" => {
            if let Some(p) = val.get("position").and_then(|v| v.as_str()) {
                let mut cfg = crate::system::config::load_config();
                cfg.osd_position = p.to_string();
                let _ = crate::system::config::save_config(&cfg);
                crate::system::osd_window::send_osd_ipc_message(serde_json::json!({
                    "action": "config",
                    "theme": cfg.osd_theme,
                    "scale": cfg.osd_scale,
                    "position": cfg.osd_position,
                }));
            }
            true
        }
        "set_osd_enabled" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                let app_handle = app.cloned().or_else(|| crate::system::slint_window::SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone()));
                if let Some(a) = app_handle {
                    let _ = crate::commands::switches_cmd::set_device_switch(a, "osd".into(), en);
                } else {
                    let _ = crate::system::device_switches::set_switch("osd", en);
                }
            }
            true
        }
        "trigger_osd_preview" => {
            if let Some(app) = app {
                let _ = crate::system::osd_window::trigger_osd_preview(app.clone());
            } else {
                let cfg = crate::system::config::load_config();
                crate::system::osd_window::send_osd_ipc_message(serde_json::json!({
                    "action": "preview",
                    "theme": cfg.osd_theme,
                    "scale": cfg.osd_scale,
                    "position": cfg.osd_position,
                }));
            }
            true
        }
        "set_unlock_battery_limit" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                let mut cfg = crate::system::config::load_config();
                cfg.unlock_battery_power_limit = en;
                let _ = crate::system::config::save_config(&cfg);
                if let Some(app) = app {
                    let ec_bridge = app.state::<crate::driver::ec_bridge::SharedEcBridge>();
                    let _ = crate::commands::system_cmd::set_unlock_battery_power_limit(en, app.clone(), ec_bridge);
                } else {
                    let ec = crate::driver::ec_bridge::get_global_ec();
                    let cur_mode = cfg.power_mode;
                    if !en && (cur_mode == 3 || cur_mode == 4) && !crate::commands::power_cmd::is_dc_round_plugged_in() && !crate::commands::power_cmd::is_thin_and_light_or_pure_igpu() {
                        ec.lock().set_power_mode(2);
                        cfg.power_mode = 2;
                        let _ = crate::system::config::save_config(&cfg);
                        crate::update_tray_power_mode(2);
                        crate::system::osd_window::show_power_mode(2);
                    }
                }
                super::super::sync::sync_slint_switches_state();
                let fresh_cfg = crate::system::config::load_config();
                super::super::sync::sync_slint_power_targets(fresh_cfg.power_mode);
            }
            true
        }
        _ => false,
    }
}
