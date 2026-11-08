use tauri::AppHandle;
use super::super::sync::sync_slint_plugins_state;
use super::super::webview_dock::hide_embedded_webview;

static ACTIVE_SLINT_CHILD: parking_lot::Mutex<Option<std::process::Child>> = parking_lot::Mutex::new(None);

pub fn hide_slint_embedded() {
    let mut guard = ACTIVE_SLINT_CHILD.lock();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
        crate::log_info!("PLUGINS", "Terminated active Slint embedded child process");
    }
}

pub fn is_slint_plugin(pid: &str) -> bool {
    let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
    let m_path = p_dir.join("manifest.json");
    if let Ok(content) = std::fs::read_to_string(&m_path) {
        if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
            let is_slint_type = manifest.plugin_type.as_deref().map_or(false, |t| t.eq_ignore_ascii_case("slint"));
            let is_slint_entry = manifest.tab.as_ref().and_then(|t| t.entry.as_deref())
                .or(manifest.entry.as_deref())
                .map_or(false, |e| e.ends_with(".slint"));
            if is_slint_type || is_slint_entry {
                return true;
            }
        }
    }
    if p_dir.join("main.slint").exists() {
        return true;
    }
    false
}

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

                    // If this plugin has a desktop widget or companion service, toggle them dynamically
                    let p_dir = crate::commands::system_cmd::find_plugin_dir(id);
                    let manifest_path = p_dir.join("manifest.json");
                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                            let p_type = manifest.plugin_type.as_deref().unwrap_or_default();
                            let has_widget = manifest.widget.is_some() || p_type.eq_ignore_ascii_case("widget") || p_type.eq_ignore_ascii_case("skin");
                            if has_widget {
                                if enabled {
                                    crate::log_info!("PLUGINS", "Toggled widget '{}' ON -> opening desktop widget", id);
                                    crate::system::native_wry::open_popout(id);
                                } else {
                                    crate::log_info!("PLUGINS", "Toggled widget '{}' OFF -> closing desktop widget", id);
                                    crate::system::native_wry::close_popout(id);
                                }
                            }

                            if let Some(svc) = manifest.service {
                                if enabled && svc.auto_start {
                                    let target_id = id.to_string();
                                    std::thread::spawn(move || {
                                        let _ = crate::system::plugin_service::start_service(&target_id);
                                    });
                                } else if !enabled && svc.auto_stop {
                                    let _ = crate::system::plugin_service::stop_service(id);
                                }
                            }
                        }
                    }

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
            hide_slint_embedded();
            crate::system::native_wry::hide_embedded();
            super::super::send_slint_ipc_message(serde_json::json!({ "action": "trigger_full_redraw" }));
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
                    hide_slint_embedded();
                    crate::system::native_wry::hide_embedded();
                    super::super::send_slint_ipc_message(serde_json::json!({ "action": "trigger_full_redraw" }));
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

                        if is_slint_plugin(pid) {
                            crate::log_info!("PLUGINS", "Detected Slint native plugin '{}' -> docking via native child process", pid);
                            crate::system::native_wry::hide_embedded();
                            hide_slint_embedded();

                            let current_exe = std::env::current_exe().unwrap_or_else(|_| "open-revo.exe".into());
                            let spawn_res = std::process::Command::new(current_exe)
                                .args(&[
                                    "--slint-widget",
                                    pid,
                                    "--embed",
                                    &parent_hwnd_raw.to_string(),
                                    &target_x.to_string(),
                                    &target_y.to_string(),
                                    &target_w.to_string(),
                                    &target_h.to_string(),
                                ])
                                .spawn();

                            match spawn_res {
                                Ok(child) => {
                                    crate::log_info!("PLUGINS", "Spawned Slint embedded child process successfully for '{}'", pid);
                                    *ACTIVE_SLINT_CHILD.lock() = Some(child);
                                }
                                Err(e) => {
                                    crate::log_error!("PLUGINS", "Failed to spawn Slint embedded child for '{}': {:?}", pid, e);
                                }
                            }
                        } else {
                            hide_slint_embedded();
                            crate::system::native_wry::attach_or_switch_embedded(
                                parent_hwnd_raw,
                                pid,
                                (target_x, target_y, target_w, target_h),
                            );
                        }

                        // Auto-start companion service if declared and configured with auto_start
                        let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
                        let m_path = p_dir.join("manifest.json");
                        if let Ok(content) = std::fs::read_to_string(&m_path) {
                            if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                                if let Some(svc) = manifest.service {
                                    if svc.auto_start {
                                        let target_pid = pid.to_string();
                                        std::thread::spawn(move || {
                                            let _ = crate::system::plugin_service::start_service(&target_pid);
                                        });
                                    }
                                }
                            }
                        }
                    } else {
                        crate::log_warn!("PLUGINS", "Cannot dock plugin '{}': Slint HWND is 0", pid);
                        super::super::send_slint_ipc_message(serde_json::json!({
                            "action": "set_active_plugin_id",
                            "plugin_id": ""
                        }));
                    }
                }
            }
            true
        }
        "open_custom_widget" => {
            crate::log_info!("PLUGINS", "Received open_custom_widget: val={:?}, app_is_some={}", val, app.is_some());
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                if is_slint_plugin(pid) {
                    crate::log_info!("PLUGINS", "Opening standalone Slint widget for plugin: '{}'", pid);
                    let current_exe = std::env::current_exe().unwrap_or_else(|_| "open-revo.exe".into());
                    let _ = std::process::Command::new(current_exe)
                        .args(&["--slint-widget", pid])
                        .spawn();
                } else {
                    crate::log_info!("PLUGINS", "Opening native Wry popout window for plugin: '{}'", pid);
                    crate::system::native_wry::open_popout(pid);
                }

                let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
                let m_path = p_dir.join("manifest.json");
                if let Ok(content) = std::fs::read_to_string(&m_path) {
                    if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                        if let Some(svc) = manifest.service {
                            if svc.auto_start {
                                let target_pid = pid.to_string();
                                std::thread::spawn(move || {
                                    let _ = crate::system::plugin_service::start_service(&target_pid);
                                });
                            }
                        }
                    }
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
