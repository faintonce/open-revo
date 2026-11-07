use tauri::AppHandle;
use super::super::dialog::RECORDED_SLINT_HWND;
use super::super::send_slint_ipc_message;
use super::super::sync::{
    sync_slint_display_state, sync_slint_lighting_state, sync_slint_plugins_state,
    sync_slint_power_state, sync_slint_tuning_state, sync_slint_water_state,
};

pub fn handle(action: &str, val: &serde_json::Value, _app: Option<&AppHandle>) -> bool {
    match action {
        "save_full_window_pos" => {
            if let (Some(x), Some(y)) = (
                val.get("x").and_then(|v| v.as_i64()),
                val.get("y").and_then(|v| v.as_i64()),
            ) {
                crate::commands::window_cmd::save_full_window_pos(x as i32, y as i32);
            }
            true
        }
        "sync_slint_hwnd" => {
            if let Some(hwnd) = val.get("hwnd").and_then(|v| v.as_i64()) {
                if hwnd != 0 {
                    RECORDED_SLINT_HWND.store(hwnd as isize, std::sync::atomic::Ordering::SeqCst);
                    crate::log_info!("SLINT_MAIN", "Synchronized Slint main HWND: 0x{:X}", hwnd);
                }
            }
            true
        }
        "slint_ready" => {
            if let Some(hwnd) = val.get("hwnd").and_then(|v| v.as_i64()) {
                if hwnd != 0 {
                    RECORDED_SLINT_HWND.store(hwnd as isize, std::sync::atomic::Ordering::SeqCst);
                    crate::log_info!("SLINT_MAIN", "Companion reported ready with HWND: 0x{:X}", hwnd);
                }
            } else {
                crate::log_info!("SLINT_MAIN", "Companion reported ready, sending full states");
            }
            sync_slint_tuning_state();
            sync_slint_display_state();
            sync_slint_power_state();
            sync_slint_lighting_state();
            sync_slint_water_state(None, None);
            sync_slint_plugins_state();
            true
        }
        "window_moved" => {
            if let Some(hwnd) = val.get("hwnd").and_then(|v| v.as_i64()) {
                if hwnd != 0 {
                    RECORDED_SLINT_HWND.store(hwnd as isize, std::sync::atomic::Ordering::SeqCst);
                }
            }
            true
        }
        "expand_dashboard" => {
            crate::log_info!("SLINT_MAIN", "Dashboard expanded to full native view");
            let mut cfg = crate::system::config::load_config();
            cfg.last_window_expanded = true;
            let _ = crate::system::config::save_config(&cfg);
            crate::system::tray::update_tray_window_mode(true);
            true
        }
        "collapse_dashboard" => {
            crate::log_info!("SLINT_MAIN", "Dashboard collapsed to mini drawer view");
            let mut cfg = crate::system::config::load_config();
            cfg.last_window_expanded = false;
            let _ = crate::system::config::save_config(&cfg);
            crate::system::tray::update_tray_window_mode(false);
            true
        }
        "record_window_mode" => {
            if let Some(is_exp) = val.get("is_expanded").and_then(|v| v.as_bool()) {
                let mut cfg = crate::system::config::load_config();
                cfg.last_window_expanded = is_exp;
                let _ = crate::system::config::save_config(&cfg);
            }
            true
        }
        "set_wake_window_mode" => {
            if let Some(m) = val.get("mode").and_then(|v| v.as_str()) {
                let _ = crate::commands::window_cmd::set_wake_window_mode(m.to_string());
            }
            true
        }
        "set_mini_auto_hide" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                let _ = crate::commands::window_cmd::set_mini_auto_hide(en);
            }
            true
        }
        "set_classic_webview" => {
            if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                let mut cfg = crate::system::config::load_config();
                cfg.use_classic_webview = en;
                crate::system::config::save_config(&cfg);
                crate::log_info!("SLINT_MAIN", "use_classic_webview updated to: {}", en);
            }
            true
        }
        "restart_openrevo" => {
            crate::log_info!("SLINT_MAIN", "Restart requested from Slint UI");
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe).spawn();
            }
            std::process::exit(0);
        }
        "open_user_data_dir" => {
            let _ = crate::commands::system_cmd::open_user_data_dir();
            true
        }
        "clear_user_data_dir" => {
            let _ = crate::commands::system_cmd::clear_user_data_dir();
            true
        }
        "migrate_user_data_dir" => {
            if let Some(p) = val.get("target_path").and_then(|v| v.as_str()) {
                let _ = crate::commands::system_cmd::migrate_user_data_dir(p.to_string());
            }
            true
        }
        "check_for_updates" => {
            tauri::async_runtime::spawn(async move {
                match crate::updater::check_for_updates_cmd(None).await {
                    Ok(res) => {
                        let size_mb = format!("{:.2}", (res.size as f64) / (1024.0 * 1024.0));
                        send_slint_ipc_message(serde_json::json!({
                            "action": "sync_update_result",
                            "update": {
                                "has_update": res.has_update,
                                "current_version": res.current_version,
                                "latest_version": res.latest_version,
                                "display_version": res.display_version,
                                "notes": res.notes,
                                "chosen_source": res.chosen_source,
                                "asset_type": res.asset_type,
                                "filename": res.filename,
                                "download_url": res.download_url,
                                "size_mb": size_mb,
                                "sha256": res.sha256,
                            }
                        }));
                    }
                    Err(e) => {
                        crate::log_warn!("UPDATER", "Check update failed: {}", e);
                        send_slint_ipc_message(serde_json::json!({
                            "action": "sync_update_result",
                            "update": { "has_update": false }
                        }));
                    }
                }
            });
            true
        }
        _ => false,
    }
}
