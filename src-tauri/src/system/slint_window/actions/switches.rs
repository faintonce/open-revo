use tauri::AppHandle;
use tauri::Manager;

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "toggle_fan_boost" => {
            if let Some(app) = app {
                let cfg = crate::system::config::load_config();
                let next = !cfg.fan_boost;
                let state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                let _ = crate::commands::hardware_cmd::set_fan_boost(app.clone(), next, state);
            }
            true
        }
        "toggle_camera" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.camera_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "camera".into(), next);
            }
            true
        }
        "toggle_mic" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.mic_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "mic".into(), next);
            }
            true
        }
        "toggle_win_lock" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.win_key_locked;
            let _ = crate::commands::hardware_cmd::set_win_key_locked(next);
            true
        }
        "toggle_keep_awake" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.keep_awake_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "keep_awake".into(), next);
            }
            true
        }
        "toggle_autostart" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.autostart;
            let _ = crate::commands::hardware_cmd::set_autostart_enabled(next);
            true
        }
        "toggle_touchpad" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.touchpad;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "touchpad".into(), next);
            }
            true
        }
        "toggle_osd" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.osd;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "osd".into(), next);
            }
            true
        }
        "toggle_fn_lock" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.fn_lock;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "fn_lock".into(), next);
            }
            true
        }
        "toggle_wifi" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.wifi;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "wifi".into(), next);
            }
            true
        }
        "toggle_bt" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.bluetooth;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "bluetooth".into(), next);
            }
            true
        }
        "set_balanced_db" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                let mut cfg = crate::system::config::load_config();
                cfg.balanced_gpu_dynamic_boost = en;
                let _ = crate::system::config::save_config(&cfg);
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
            }
            true
        }
        "set_osd_scale" => {
            if let Some(s) = val.get("scale").and_then(|v| v.as_str()) {
                let mut cfg = crate::system::config::load_config();
                cfg.osd_scale = s.to_string();
                let _ = crate::system::config::save_config(&cfg);
            }
            true
        }
        "set_osd_position" => {
            if let Some(p) = val.get("position").and_then(|v| v.as_str()) {
                let mut cfg = crate::system::config::load_config();
                cfg.osd_position = p.to_string();
                let _ = crate::system::config::save_config(&cfg);
            }
            true
        }
        "set_osd_enabled" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                if let Some(app) = app {
                    let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "osd".into(), en);
                }
            }
            true
        }
        "trigger_osd_preview" => {
            if let Some(app) = app {
                let _ = crate::system::osd_window::trigger_osd_preview(app.clone());
            }
            true
        }
        "set_unlock_battery_limit" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                if let Some(app) = app {
                    let ec_bridge = app.state::<crate::driver::ec_bridge::SharedEcBridge>();
                    let _ = crate::commands::system_cmd::set_unlock_battery_power_limit(en, app.clone(), ec_bridge);
                }
            }
            true
        }
        _ => false,
    }
}
