use tauri::AppHandle;
use tauri::Manager;
use super::super::dialog::open_file_dialog;
use super::super::sync::sync_slint_display_state;
use super::super::{send_slint_ipc_message, LAST_CLOUD_ICC_HIT, LAST_CLOUD_ICC_IDENTITY, SELECTED_MONITOR_INDEX};

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "set_gpu_mode" => {
            if let Some(gm) = val.get("gpu_mode").and_then(|m| m.as_str()) {
                if let Some(app) = app {
                    let gpu_state: tauri::State<crate::driver::gpu_bridge::SharedGpuBridge> = app.state();
                    let _ = crate::commands::hardware_cmd::set_gpu_mode(app.clone(), gm.to_string(), None, gpu_state);
                }
            }
            true
        }
        "switch_refresh_rate" => {
            if let Some(hz) = val.get("hz").and_then(|h| h.as_i64()) {
                let sel_idx = SELECTED_MONITOR_INDEX.load(std::sync::atomic::Ordering::Relaxed);
                let monitors = crate::system::display::get_all_monitors();
                if sel_idx < monitors.len() && !monitors[sel_idx].is_primary && !monitors[sel_idx].is_internal {
                    let _ = crate::commands::display_cmd::set_display_monitor_refresh_rate(
                        monitors[sel_idx].device_name.clone(),
                        hz as u32,
                    );
                } else {
                    let _ = crate::commands::hardware_cmd::switch_refresh_rate(hz as u32);
                }
            }
            true
        }
        "select_monitor_index" => {
            if let Some(idx) = val.get("index").and_then(|i| i.as_u64()) {
                SELECTED_MONITOR_INDEX.store(idx as usize, std::sync::atomic::Ordering::Relaxed);
                sync_slint_display_state();
            }
            true
        }
        "toggle_auto_min_hz" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.auto_min_refresh_on_battery;
            let _ = crate::commands::hardware_cmd::set_auto_min_refresh_on_battery(next);
            true
        }
        "set_brightness" => {
            if let Some(b) = val.get("brightness").and_then(|b| b.as_i64()) {
                if let Some(app) = app {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = crate::commands::hardware_cmd::set_display_brightness(app_clone, b as u8).await;
                    });
                }
            }
            true
        }
        "turn_off_screen" => {
            let _ = crate::commands::hardware_cmd::turn_off_screen();
            true
        }
        "set_display_tuning_enabled" => {
            let enabled = val.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
            let _ = crate::commands::display_cmd::set_display_tuning_enabled(enabled);
            sync_slint_display_state();
            true
        }
        "toggle_display_tuning" => {
            let state = crate::commands::display_cmd::get_display_tuning_state();
            let _ = crate::commands::display_cmd::set_display_tuning_enabled(!state.enabled);
            sync_slint_display_state();
            true
        }
        "apply_display_tuning_preset" => {
            if let Some(preset_id) = val.get("preset_id").and_then(|p| p.as_str()) {
                let state = crate::commands::display_cmd::get_display_tuning_state();
                if let Some(target) = state.presets.into_iter().find(|p| p.id == preset_id) {
                    let _ = crate::commands::display_cmd::apply_display_color_preset(target);
                }
            }
            sync_slint_display_state();
            true
        }
        "update_display_tuning_slider" => {
            if let (Some(key), Some(raw_val)) = (val.get("key").and_then(|k| k.as_str()), val.get("value").and_then(|v| v.as_i64())) {
                let state = crate::commands::display_cmd::get_display_tuning_state();
                let mut active = state.active_preset.clone();
                match key {
                    "temp_k" => active.color_temp_k = raw_val as u32,
                    "saturation" => active.saturation = (raw_val as f32) / 100.0,
                    "contrast" => active.contrast = (raw_val as f32) / 100.0,
                    "gamma" => active.gamma = (raw_val as f32) / 100.0,
                    "gain_r" => active.gain_r = (raw_val as f32) / 100.0,
                    "gain_g" => active.gain_g = (raw_val as f32) / 100.0,
                    "gain_b" => active.gain_b = (raw_val as f32) / 100.0,
                    _ => {}
                }
                let _ = crate::system::display_tuning::apply_color_preset(&active);
                let _ = crate::commands::display_cmd::apply_display_color_preset(active);
            }
            true
        }
        "reset_display_tuning_preset" => {
            let state = crate::commands::display_cmd::get_display_tuning_state();
            let _ = crate::commands::display_cmd::reset_color_preset_to_default(state.active_preset.id);
            sync_slint_display_state();
            true
        }
        "import_display_tuning_file" => {
            tauri::async_runtime::spawn(async move {
                if let Some(file_path) = open_file_dialog("JSON 调色预设", "json") {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let fname = file_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
                        let _ = crate::commands::display_cmd::import_color_preset_content(content, fname);
                        sync_slint_display_state();
                    }
                }
            });
            true
        }
        "export_display_tuning_file" => {
            tauri::async_runtime::spawn(async move {
                let state = crate::commands::display_cmd::get_display_tuning_state();
                let default_name = format!("{}.json", state.active_preset.id);
                if let Some(dest) = crate::commands::tuning_cmd::save_file_dialog(&default_name, "JSON 调色预设", "json") {
                    if let Ok(json) = serde_json::to_string_pretty(&state.active_preset) {
                        let _ = std::fs::write(&dest, json);
                    }
                }
            });
            true
        }
        "import_color_profile" => {
            tauri::async_runtime::spawn(async move {
                if let Some(file_path) = open_file_dialog("ICC/ICM 校色文件", "icc;*.icm") {
                    if let Ok(bytes) = std::fs::read(&file_path) {
                        let file_name = file_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("imported.icc")
                            .to_string();
                        let _ = crate::commands::display_cmd::import_custom_color_profile(file_name, bytes, None).await;
                        sync_slint_display_state();
                    }
                }
            });
            true
        }
        "open_cloud_icc" | "refresh_cloud_icc_identity" => {
            tauri::async_runtime::spawn(async move {
                let id_res = tauri::async_runtime::spawn_blocking(crate::system::cloud_icc::get_cloud_icc_identity).await;
                if let Ok(Ok(id)) = id_res {
                    let main_mon = id.monitors.iter().find(|m| m.is_internal).or_else(|| id.monitors.first());
                    let panel_pnp = main_mon.map(|m| m.pnp.clone()).unwrap_or_else(|| "未检出".into());
                    let panel_sn = main_mon.map(|m| if m.panel_serial.is_empty() { "(无独立面板SN)".to_string() } else { m.panel_serial.clone() }).unwrap_or_else(|| "(无独立面板SN)".into());
                    let mac_str = if id.mac_addresses.is_empty() { "未检出".into() } else { id.mac_addresses.join(" / ") };

                    *LAST_CLOUD_ICC_IDENTITY.lock() = Some(id.clone());

                    send_slint_ipc_message(serde_json::json!({
                        "action": "sync_cloud_icc_identity",
                        "panel_pnp": panel_pnp,
                        "panel_sn": panel_sn,
                        "bios_sn": id.bios_serial,
                        "mac_address": mac_str,
                        "arch_name": id.arch_name,
                        "recommended_method": id.recommended_method
                    }));
                }
            });
            true
        }
        "start_cloud_icc_search" => {
            let mode = val.get("mode").and_then(|v| v.as_str()).unwrap_or("auto").to_string();
            tauri::async_runtime::spawn(async move {
                send_slint_ipc_message(serde_json::json!({
                    "action": "sync_cloud_icc_searching",
                    "is_searching": true,
                    "status": "正在连接云端校色数据库..."
                }));

                let id = {
                    let guard = LAST_CLOUD_ICC_IDENTITY.lock();
                    guard.clone()
                };

                let id = match id {
                    Some(i) => i,
                    None => {
                        let res = tauri::async_runtime::spawn_blocking(crate::system::cloud_icc::get_cloud_icc_identity).await;
                        match res {
                            Ok(Ok(i)) => {
                                *LAST_CLOUD_ICC_IDENTITY.lock() = Some(i.clone());
                                i
                            }
                            _ => {
                                send_slint_ipc_message(serde_json::json!({
                                    "action": "sync_cloud_icc_hit",
                                    "hit_found": false,
                                    "toast": "未能获取本机硬件特征指纹"
                                }));
                                return;
                            }
                        }
                    }
                };

                let mut candidates: Vec<(String, String)> = Vec::new();
                if mode == "50" {
                    for c in &id.candidates_method1 { candidates.push((c.clone(), "50".into())); }
                } else if mode == "40" {
                    for c in &id.candidates_method2 { candidates.push((c.clone(), "40".into())); }
                } else {
                    for c in &id.candidates_method1 { candidates.push((c.clone(), "50".into())); }
                    for c in &id.candidates_method2 { candidates.push((c.clone(), "40".into())); }
                    for c in &id.candidates_fallback { candidates.push((c.clone(), "auto".into())); }
                }

                let mut hit_opt = None;
                for (cand, meth) in candidates {
                    let probe_res = crate::commands::display_cmd::fetch_cloud_icc_profile(cand.clone(), Some(meth), None).await;
                    if let Ok(hit) = probe_res {
                        if !hit.icc_profiles.is_empty() {
                            hit_opt = Some(hit);
                            break;
                        }
                    }
                }

                if let Some(hit) = hit_opt {
                    let is_uniwill = hit.server_url.contains("uniwill");
                    let count = hit.icc_profiles.len();
                    let meta = format!(
                        "包大小: {:.1} KB · 节点: {} · 内含 {} 个有效色彩配置",
                        (hit.file_size as f64) / 1024.0,
                        if is_uniwill { "Uniwill (40系)" } else { "Aistone (50系)" },
                        count
                    );
                    let profile_names: Vec<String> = hit.icc_profiles.iter().map(|p| p.file_name.clone()).collect();
                    *LAST_CLOUD_ICC_HIT.lock() = Some(hit.clone());

                    send_slint_ipc_message(serde_json::json!({
                        "action": "sync_cloud_icc_hit",
                        "hit_found": true,
                        "file_name": hit.file_name,
                        "meta": meta,
                        "profiles": profile_names
                    }));
                } else {
                    *LAST_CLOUD_ICC_HIT.lock() = None;
                    send_slint_ipc_message(serde_json::json!({
                        "action": "sync_cloud_icc_hit",
                        "hit_found": false,
                        "toast": "所有候选路径均未命中，当前机型可能未在服务器留存出厂专属校色"
                    }));
                }
            });
            true
        }
        "apply_cloud_icc_profile" => {
            let idx = val.get("index").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            tauri::async_runtime::spawn(async move {
                let target = {
                    let guard = LAST_CLOUD_ICC_HIT.lock();
                    guard.as_ref().and_then(|h| h.icc_profiles.get(idx).cloned())
                };

                if let Some(p) = target {
                    let import_res = crate::commands::display_cmd::import_custom_color_profile(p.file_name.clone(), p.file_bytes, None).await;
                    if let Ok(st) = import_res {
                        let _ = crate::commands::display_cmd::apply_color_profile(st.active_profile_id, None).await;
                        sync_slint_display_state();
                        send_slint_ipc_message(serde_json::json!({
                            "action": "cloud_icc_toast",
                            "toast": format!("✔ 已成功应用原厂校色 [{}] 至当前显示器！", p.file_name)
                        }));
                    }
                }
            });
            true
        }
        "apply_all_cloud_icc_profiles" => {
            tauri::async_runtime::spawn(async move {
                let profiles = {
                    let guard = LAST_CLOUD_ICC_HIT.lock();
                    guard.as_ref().map(|h| h.icc_profiles.clone()).unwrap_or_default()
                };

                if !profiles.is_empty() {
                    let mut last_id = None;
                    for p in &profiles {
                        if let Ok(st) = crate::commands::display_cmd::import_custom_color_profile(p.file_name.clone(), p.file_bytes.clone(), None).await {
                            last_id = Some(st.active_profile_id);
                        }
                    }
                    if let Some(id) = last_id {
                        let _ = crate::commands::display_cmd::apply_color_profile(id, None).await;
                    }
                    sync_slint_display_state();
                    send_slint_ipc_message(serde_json::json!({
                        "action": "cloud_icc_toast",
                        "toast": format!("🎉 全部 {} 个原厂色彩配置已全量导入并同步至显卡色彩管理！", profiles.len())
                    }));
                }
            });
            true
        }
        "apply_color_profile" => {
            let prof_id = val.get("profile_id").or_else(|| val.get("profile_name")).and_then(|p| p.as_str()).unwrap_or("default_native").to_string();
            tauri::async_runtime::spawn(async move {
                let _ = crate::commands::display_cmd::apply_color_profile(prof_id, None).await;
                sync_slint_display_state();
            });
            true
        }
        _ => false,
    }
}
