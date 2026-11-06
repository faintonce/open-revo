use tauri::AppHandle;
use super::super::sync::sync_slint_plugins_state;
use super::super::webview_dock::hide_embedded_webview;

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "toggle_builtin_module" => {
            if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                if let Some(enabled) = val.get("enabled").and_then(|v| v.as_bool()) {
                    let mut cfg = crate::system::config::load_config();
                    cfg.module_toggles.insert(id.to_string(), enabled);
                    let _ = crate::system::config::save_config(&cfg);

                    if id == "water" {
                        let _ = crate::commands::cooler_cmd::set_water_cooler_enabled(enabled);
                    }
                    sync_slint_plugins_state();
                }
            }
            true
        }
        "toggle_custom_plugin" => {
            if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                if let Some(enabled) = val.get("enabled").and_then(|v| v.as_bool()) {
                    let _ = crate::commands::system_cmd::set_custom_plugin_enabled(id.to_string(), enabled);
                    sync_slint_plugins_state();
                }
            }
            true
        }
        "open_plugins_dir" => {
            let _ = crate::commands::system_cmd::open_plugins_dir();
            true
        }
        "reload_plugins" => {
            sync_slint_plugins_state();
            true
        }
        "collapse_dashboard" => {
            crate::log_info!("PLUGINS", "Received collapse_dashboard from Slint UI");
            crate::system::native_wry::hide_embedded();
            if let Some(app) = app {
                let app_clone = app.clone();
                let _ = app.run_on_main_thread(move || {
                    hide_embedded_webview(&app_clone);
                });
            }
            true
        }
        "select_plugin_tab" => {
            crate::log_info!("PLUGINS", "Received select_plugin_tab: val={:?}, app_is_some={}", val, app.is_some());
            if let Some(hwnd) = val.get("hwnd").and_then(|v| v.as_i64()) {
                if hwnd != 0 {
                    super::super::dialog::RECORDED_SLINT_HWND.store(hwnd as isize, std::sync::atomic::Ordering::SeqCst);
                    crate::log_info!("PLUGINS", "Recorded Slint HWND from action payload: 0x{:X}", hwnd);
                }
            }
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                crate::log_info!("PLUGINS", "User switched to plugin tab: '{}'", pid);
                if pid.is_empty() {
                    crate::system::native_wry::hide_embedded();
                    if let Some(app) = app {
                        let app_clone = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            hide_embedded_webview(&app_clone);
                        });
                    }
                } else {
                    let parent_hwnd_raw = super::super::dialog::get_openrevo_main_hwnd();
                    if parent_hwnd_raw != 0 {
                        let parent_hwnd = windows::Win32::Foundation::HWND(parent_hwnd_raw as *mut _);
                        let (target_x, target_y, target_w, target_h, _) = super::super::webview_dock::calc_embedded_geometry(parent_hwnd);
                        crate::system::native_wry::attach_or_switch_embedded(
                            parent_hwnd_raw,
                            pid,
                            (target_x, target_y, target_w, target_h),
                        );
                    } else {
                        crate::log_warn!("PLUGINS", "Cannot dock plugin '{}': Slint HWND is 0", pid);
                    }
                }
            }
            true
        }
        "open_custom_widget" => {
            crate::log_info!("PLUGINS", "Received open_custom_widget: val={:?}, app_is_some={}", val, app.is_some());
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                crate::log_info!("PLUGINS", "Opening native Wry popout window for plugin: '{}'", pid);
                crate::system::native_wry::open_popout(pid);
            }
            true
        }
        "open_plugin_dir" => {
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
                let _ = std::process::Command::new("explorer").arg(p_dir).spawn();
            }
            true
        }
        _ => false,
    }
}
