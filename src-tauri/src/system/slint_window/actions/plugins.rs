use tauri::AppHandle;
use super::super::sync::sync_slint_plugins_state;
use super::super::webview_dock::{attach_or_switch_embedded_webview, hide_embedded_webview};

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
                if let Some(app) = app {
                    let app_clone = app.clone();
                    let pid_string = pid.to_string();
                    let res = app.run_on_main_thread(move || {
                        crate::log_info!("PLUGINS", "run_on_main_thread closure executing for pid='{}'", pid_string);
                        if pid_string.is_empty() {
                            hide_embedded_webview(&app_clone);
                        } else {
                            attach_or_switch_embedded_webview(&app_clone, &pid_string);
                        }
                    });
                    crate::log_info!("PLUGINS", "app.run_on_main_thread scheduled: {:?}", res);
                } else {
                    crate::log_error!("PLUGINS", "AppHandle is None in select_plugin_tab handler!");
                }
            }
            true
        }
        "open_custom_widget" => {
            crate::log_info!("PLUGINS", "Received open_custom_widget: val={:?}, app_is_some={}", val, app.is_some());
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                if let Some(app) = app {
                    let app_clone = app.clone();
                    let pid_string = pid.to_string();
                    tauri::async_runtime::spawn(async move {
                        crate::log_info!("PLUGINS", "Calling widget_manager::open_custom_widget for '{}'", pid_string);
                        let res = crate::system::widget_manager::open_custom_widget(app_clone, pid_string).await;
                        crate::log_info!("PLUGINS", "widget_manager::open_custom_widget result: {:?}", res);
                    });
                } else {
                    crate::log_error!("PLUGINS", "AppHandle is None in open_custom_widget handler!");
                }
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
