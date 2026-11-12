use tauri::AppHandle;
use super::super::dialog::RECORDED_SLINT_HWND;
use super::super::send_slint_ipc_message;
use super::super::sync::{
    sync_slint_display_state, sync_slint_lighting_state, sync_slint_plugins_state,
    sync_slint_power_state, sync_slint_switches_state, sync_slint_tuning_state, sync_slint_water_state,
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
            crate::system::telemetry::set_main_window_visible(true);
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
            sync_slint_switches_state();
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
        "open_lighting_studio" => {
            crate::log_info!("SLINT_MAIN", "Opening lighting studio directly");
            let mut cfg = crate::system::config::load_config();
            cfg.last_window_expanded = true;
            let _ = crate::system::config::save_config(&cfg);
            crate::system::tray::update_tray_window_mode(true);
            true
        }
        "set_theme_mode" => {
            if let Some(mode) = val.get("theme_mode").and_then(|v| v.as_i64()) {
                crate::log_info!("SLINT_MAIN", "Persisting user theme_mode: {}", mode);
                let mut cfg = crate::system::config::load_config();
                cfg.theme_mode = mode as u8;
                let _ = crate::system::config::save_config(&cfg);
            }
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
        "slint_window_shown" => {
            crate::system::telemetry::set_main_window_visible(true);
            if let Ok(b) = crate::system::display::get_brightness() {
                send_slint_ipc_message(serde_json::json!({
                    "action": "sync_state",
                    "brightness": b,
                }));
            }
            true
        }
        "slint_window_hidden" => {
            crate::system::telemetry::set_main_window_visible(false);
            crate::log_info!("SLINT_MAIN", "Mini drawer companion hidden / auto-collapsed on blur");
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
                if en {
                    crate::system::osd_window::shutdown_osd_process();
                }
                crate::log_info!("SLINT_MAIN", "use_classic_webview updated to: {}", en);
            }
            true
        }
        "restart_openrevo" => {
            crate::log_info!("SLINT_MAIN", "Restart requested from Slint UI");
            crate::system::tray::safe_hardware_shutdown_on_exit();
            if let Ok(exe) = std::env::current_exe() {
                let mut cmd = std::process::Command::new(&exe);
                cmd.arg(format!("--relaunch-wait={}", std::process::id()))
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
                    cmd.creation_flags(0x0000_0008 | 0x0000_0200);
                }
                if let Err(e) = cmd.spawn() {
                    crate::log_error!("SLINT_MAIN", "Failed to spawn relaunch process {:?}: {}", exe, e);
                }
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
        "change_update_channel" => {
            if let Some(ch) = val.get("channel").and_then(|v| v.as_str()) {
                let _ = crate::commands::system_cmd::set_update_channel(ch.to_string());
                send_slint_ipc_message(serde_json::json!({
                    "action": "sync_update_channel",
                    "channel": ch
                }));
            }
            true
        }
        "check_for_updates" => {
            let channel = val.get("channel").and_then(|v| v.as_str()).map(|s| s.to_string());
            let current_ch = channel.clone().unwrap_or_else(|| crate::system::config::load_config().update_channel);
            tauri::async_runtime::spawn(async move {
                match crate::updater::check_for_updates_cmd(channel, None).await {
                    Ok(res) => {
                        let size_mb = format!("{:.2}", (res.size as f64) / (1024.0 * 1024.0));
                        let status = if res.has_update { "found" } else { "latest" };
                        let message = if res.has_update {
                            format!("发现新版本 {}", res.display_version)
                        } else {
                            format!("当前已是最新版本 ({})", res.current_version)
                        };
                        send_slint_ipc_message(serde_json::json!({
                            "action": "sync_update_result",
                            "status": status,
                            "message": message,
                            "update": {
                                "has_update": res.has_update,
                                "channel": res.channel,
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
                        let is_404 = e.contains("404");
                        let (status, message) = if is_404 && current_ch.eq_ignore_ascii_case("beta") {
                            ("not_found", "暂无预览版更新 (preview.json 尚未发布)".to_string())
                        } else if is_404 {
                            ("not_found", "未找到版本清单文件 (404 Not Found)".to_string())
                        } else {
                            ("error", "检查更新失败: 网络超时或连接受阻".to_string())
                        };
                        send_slint_ipc_message(serde_json::json!({
                            "action": "sync_update_result",
                            "status": status,
                            "message": message,
                            "update": { "has_update": false }
                        }));
                    }
                }
            });
            true
        }
        "open_sponsor" => {
            crate::log_info!("SLINT_MAIN", "Sponsor view opened from Slint UI");
            let mut cfg = crate::system::config::load_config();
            cfg.last_window_expanded = true;
            let _ = crate::system::config::save_config(&cfg);
            crate::system::tray::update_tray_window_mode(true);
            true
        }
        "start_download_update" => {
            let app = _app.cloned();
            let val = val.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(app) = app {
                    let download_url = val.get("download_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let expected_sha256 = val.get("expected_sha256").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let filename = val.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let is_installer = val.get("is_installer").and_then(|v| v.as_bool()).unwrap_or(false);

                    if !download_url.is_empty() && !expected_sha256.is_empty() && !filename.is_empty() {
                        crate::log_info!("SLINT_MAIN", "Starting update download from Slint: {}", filename);
                        if let Err(e) = crate::updater::download_and_install_update_cmd(
                            app, download_url, expected_sha256, filename, is_installer
                        ).await {
                            crate::log_error!("UPDATER", "Slint update download/install failed: {}", e);
                            send_slint_ipc_message(serde_json::json!({
                                "action": "sync_update_result",
                                "status": "error",
                                "message": format!("升级失败: {}", e),
                                "update": { "has_update": false }
                            }));
                        }
                    } else {
                        crate::log_warn!("UPDATER", "start_download_update received empty parameters");
                    }
                } else {
                    crate::log_warn!("UPDATER", "start_download_update called without valid AppHandle");
                }
            });
            true
        }
        "open_log_file" => {
            let _ = crate::commands::system_cmd::open_log_file();
            true
        }
        "set_log_level" => {
            if let Some(level) = val.get("level").and_then(|v| v.as_i64()) {
                let _ = crate::commands::system_cmd::set_log_level(level as u8);
            }
            true
        }
        _ => false,
    }
}
