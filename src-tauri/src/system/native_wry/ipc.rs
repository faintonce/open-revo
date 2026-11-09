// OpenRevo - Native Wry Plugin IPC Gateway
// Routes JSON-RPC messages from Wry-hosted plugins directly to OpenRevo commands.

use serde_json::Value;
use tauri::Emitter;

pub async fn dispatch_plugin_ipc(plugin_id: &str, cmd: &str, args: Value) -> Result<Value, String> {
    crate::log_info!("NATIVE_WRY", "IPC call: plugin='{}', cmd='{}'", plugin_id, cmd);

    let get_app = || {
        crate::system::webview_orchestrator::WebviewOrchestrator::global().app()
            .or_else(|| crate::system::slint_window::SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone()))
    };

    match cmd {
        // Plugin storage sandboxed APIs
        "get_plugin_storage" => {
            let res = crate::commands::plugin_storage_cmd::get_plugin_storage(plugin_id.to_string())?;
            Ok(res)
        }
        "set_plugin_storage" => {
            let data = args.get("data").cloned().unwrap_or(args);
            crate::commands::plugin_storage_cmd::set_plugin_storage(plugin_id.to_string(), data)?;
            Ok(Value::Bool(true))
        }
        "read_plugin_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let content = crate::commands::plugin_storage_cmd::read_plugin_file(plugin_id.to_string(), path)?;
            Ok(Value::String(content))
        }
        "write_plugin_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            crate::commands::plugin_storage_cmd::write_plugin_file(plugin_id.to_string(), path, content)?;
            Ok(Value::Bool(true))
        }
        "delete_plugin_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let res = crate::commands::plugin_storage_cmd::delete_plugin_file(plugin_id.to_string(), path)?;
            Ok(Value::Bool(res))
        }
        "list_plugin_files" => {
            let sub_dir = args.get("subDir").or_else(|| args.get("sub_dir")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let list = crate::commands::plugin_storage_cmd::list_plugin_files(plugin_id.to_string(), sub_dir)?;
            serde_json::to_value(list).map_err(|e| e.to_string())
        }

        // Low-level Hardware Gateway
        "raw_mailbox_invoke" => {
            let req = serde_json::from_value(args).map_err(|e| format!("Invalid RawMailboxRequest payload: {}", e))?;
            let res = crate::commands::raw_ec_cmd::raw_mailbox_invoke(req)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "raw_ec_read" => {
            let req = if let Some(sub) = args.get("request") {
                serde_json::from_value(sub.clone()).map_err(|e| e.to_string())?
            } else {
                serde_json::from_value(args).map_err(|e| e.to_string())?
            };
            let res = crate::commands::raw_ec_cmd::raw_ec_read(req)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "raw_ec_write" => {
            let req = if let Some(sub) = args.get("request") {
                serde_json::from_value(sub.clone()).map_err(|e| e.to_string())?
            } else {
                serde_json::from_value(args).map_err(|e| e.to_string())?
            };
            let res = crate::commands::raw_ec_cmd::raw_ec_write(req)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "get_raw_ec_capabilities" => {
            let res = crate::commands::raw_ec_cmd::get_raw_ec_capabilities();
            serde_json::to_value(res).map_err(|e| e.to_string())
        }

        // GPU Tuning
        "get_gpu_tuning_info" => {
            let res = crate::commands::tuning_cmd::get_gpu_tuning_info();
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "set_gpu_clock_offsets" => {
            let core = args.get("core").or_else(|| args.get("coreOffsetMhz")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let memory = args.get("memory").or_else(|| args.get("memOffsetMhz")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if let Some(app) = crate::system::slint_window::SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone()) {
                crate::commands::tuning_cmd::set_gpu_clock_offsets(app, core, memory)?;
            } else {
                crate::driver::nvapi::set_gpu_clock_offsets(core, memory)?;
            }
            Ok(Value::Bool(true))
        }
        "set_gpu_vf_curve_deltas" => {
            let deltas: Vec<i32> = args.get("deltasMhz")
                .or_else(|| args.get("deltas_mhz"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            crate::commands::tuning_cmd::set_gpu_vf_curve_deltas(deltas)?;
            Ok(Value::Bool(true))
        }
        "set_gpu_undervolt_curve" => {
            let max_v = args.get("maxVoltageMv").or_else(|| args.get("max_voltage_mv")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let target_f = args.get("targetFreqMhz").or_else(|| args.get("target_freq_mhz")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            crate::commands::tuning_cmd::set_gpu_undervolt_curve(max_v, target_f)?;
            Ok(Value::Bool(true))
        }
        "reset_gpu_tuning" => {
            if let Some(app) = crate::system::slint_window::SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone()) {
                crate::commands::tuning_cmd::reset_gpu_tuning(app)?;
            } else {
                crate::driver::nvapi::reset_gpu_tuning()?;
            }
            Ok(Value::Bool(true))
        }

        // Shortcuts
        "check_hotkey_conflict" => {
            let combo = args.get("combo").and_then(|v| v.as_str()).unwrap_or_default();
            let res = crate::commands::shortcut_cmd::check_hotkey_conflict(combo.to_string());
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "register_custom_shortcut" => {
            let binding = if let Some(b) = args.get("binding") {
                serde_json::from_value(b.clone()).map_err(|e| e.to_string())?
            } else {
                serde_json::from_value(args).map_err(|e| e.to_string())?
            };
            crate::commands::shortcut_cmd::register_custom_shortcut(binding)?;
            Ok(Value::Bool(true))
        }
        "unregister_custom_shortcut" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let res = crate::commands::shortcut_cmd::unregister_custom_shortcut(id.to_string())?;
            Ok(Value::Bool(res))
        }
        "get_registered_shortcuts" => {
            let res = crate::commands::shortcut_cmd::get_registered_shortcuts();
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "trigger_shortcut_action" => {
            let action_val = args.get("action").cloned().unwrap_or(args.clone());
            let action: crate::system::shortcuts::types::ShortcutAction = serde_json::from_value(action_val)
                .map_err(|e| format!("Invalid ShortcutAction: {}", e))?;
            let osd = args.get("osd").and_then(|v| v.as_bool());
            if let Some(app) = crate::system::slint_window::SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone()) {
                crate::commands::shortcut_cmd::trigger_shortcut_action(app, action, osd)?;
            }
            Ok(Value::Bool(true))
        }

        // Display & Cloud ICC
        "get_cloud_icc_identity" => {
            let res = crate::commands::display_cmd::get_cloud_icc_identity().await?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "fetch_cloud_icc_profile" => {
            let candidate = args.get("candidate").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let method = args.get("method").or_else(|| args.get("mode")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let server_url = args.get("serverUrl").or_else(|| args.get("server_url")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let res = crate::commands::display_cmd::fetch_cloud_icc_profile(candidate, method, server_url).await?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "import_custom_color_profile" => {
            let file_name = args.get("fileName").or_else(|| args.get("file_name")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let file_bytes = args.get("fileBytes").or_else(|| args.get("file_bytes")).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
            let dev = args.get("deviceName").or_else(|| args.get("device_name")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let res = crate::commands::display_cmd::import_custom_color_profile(file_name, file_bytes, dev).await?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "apply_color_profile" => {
            let pid = args.get("profileId").or_else(|| args.get("profile_id")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let dev = args.get("deviceName").or_else(|| args.get("device_name")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let res = crate::commands::display_cmd::apply_color_profile(pid, dev).await?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }

        // Hardware Telemetry & System Controls
        "get_hardware_status" => {
            let ec = crate::driver::ec_bridge::get_global_ec();
            let gpu = crate::driver::gpu_bridge::get_global_gpu();
            let state = crate::commands::system_cmd::collect_hardware_state(&ec, &gpu, true);
            serde_json::to_value(state).map_err(|e| e.to_string())
        }
        "set_fan_boost" => {
            let enable = args.get("enable").and_then(|v| v.as_bool())
                .or_else(|| args.as_bool())
                .unwrap_or(false);
            let ec = crate::driver::ec_bridge::get_global_ec();
            let app = get_app();
            crate::commands::power_cmd::set_fan_boost_internal(enable, app.as_ref(), &ec)?;
            Ok(Value::Bool(true))
        }
        "set_power_mode" => {
            let mode = args.get("mode").and_then(|v| v.as_u64())
                .or_else(|| args.as_u64())
                .unwrap_or(2) as u8;
            let ec = crate::driver::ec_bridge::get_global_ec();
            let app = get_app();
            crate::commands::power_cmd::set_power_mode_internal(mode, app.as_ref(), &ec)?;
            Ok(Value::Bool(true))
        }
        "set_battery_limit" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(80) as u8;
            let ec = crate::driver::ec_bridge::get_global_ec();
            crate::commands::power_cmd::set_battery_limit_internal(limit, &ec)?;
            Ok(Value::Bool(true))
        }
        "get_power_settings" => {
            let ec = crate::driver::ec_bridge::get_global_ec();
            let res = crate::commands::power_cmd::read_power_settings(&ec);
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "widget_ready" => {
            let pid = args.get("pluginId").or_else(|| args.get("plugin_id")).and_then(|v| v.as_str()).unwrap_or(plugin_id);
            crate::system::telemetry::register_consumer(pid, &[
                "cpu_temp".into(), "gpu_temp".into(), "cpu_power".into(), "gpu_power".into(),
                "cpu_fan_rpm".into(), "gpu_fan_rpm".into(), "fps".into(), "frametime_ms".into(),
                "power_mode".into(), "fan_boost".into(), "battery_level".into(), "is_charging".into()
            ]);
            if let Some(app) = get_app() {
                let _ = crate::system::widget_manager::widget_ready(app, pid.to_string());
            }
            Ok(Value::Bool(true))
        }

        // Inter-plugin & Host Event Broadcasting
        "emit" => {
            let event = args.get("event").and_then(|v| v.as_str()).unwrap_or_default();
            let payload = args.get("payload").cloned().unwrap_or(Value::Null);
            if !event.is_empty() {
                if let Some(app) = get_app() {
                    let _ = app.emit(event, &payload);
                }
                super::broadcast_event(event, &payload);
            }
            Ok(Value::Bool(true))
        }

        // Dynamic Telemetry Consumer Registration
        "register_telemetry_consumer" => {
            let pid = args.get("pluginId").or_else(|| args.get("plugin_id")).and_then(|v| v.as_str()).unwrap_or(plugin_id);
            let sensors: Vec<String> = args.get("sensors")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_else(|| vec![
                    "cpu_temp".into(), "gpu_temp".into(), "cpu_power".into(), "gpu_power".into(), "fps".into()
                ]);
            crate::system::telemetry::register_consumer(pid, &sensors);
            Ok(Value::Bool(true))
        }
        "unregister_telemetry_consumer" => {
            let pid = args.get("pluginId").or_else(|| args.get("plugin_id")).and_then(|v| v.as_str()).unwrap_or(plugin_id);
            crate::system::telemetry::unregister_consumer(pid);
            Ok(Value::Bool(true))
        }

        // Watcher Hub APIs for dynamic plugin registration
        "register_watcher" => {
            let def: crate::system::watcher_hub::WatcherDefinition = serde_json::from_value(args)
                .map_err(|e| format!("Invalid WatcherDefinition payload: {}", e))?;
            let ok = crate::system::watcher_hub::register_watcher(def);
            Ok(Value::Bool(ok))
        }
        "unregister_watcher" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let ok = crate::system::watcher_hub::unregister_watcher(id);
            Ok(Value::Bool(ok))
        }
        "get_watchers" => {
            let list = crate::system::watcher_hub::get_all_watchers();
            serde_json::to_value(list).map_err(|e| e.to_string())
        }

        // Plugin Companion Services & Enablement Management
        "start_plugin_service" => {
            let target_pid = args.get("pluginId")
                .or_else(|| args.get("plugin_id"))
                .or_else(|| args.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or(plugin_id);
            let status = crate::system::plugin_service::start_service(target_pid)?;
            serde_json::to_value(status).map_err(|e| e.to_string())
        }
        "stop_plugin_service" => {
            let target_pid = args.get("pluginId")
                .or_else(|| args.get("plugin_id"))
                .or_else(|| args.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or(plugin_id);
            let status = crate::system::plugin_service::stop_service(target_pid)?;
            serde_json::to_value(status).map_err(|e| e.to_string())
        }
        "get_plugin_service_status" => {
            let target_pid = args.get("pluginId")
                .or_else(|| args.get("plugin_id"))
                .or_else(|| args.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or(plugin_id);
            let status = crate::system::plugin_service::get_service_status(target_pid);
            serde_json::to_value(status).map_err(|e| e.to_string())
        }
        "set_custom_plugin_enabled" | "set_plugin_enabled_unified" => {
            let target_pid = args.get("pluginId")
                .or_else(|| args.get("plugin_id"))
                .or_else(|| args.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or(plugin_id);
            let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let res = crate::commands::system_cmd::set_custom_plugin_enabled(target_pid.to_string(), enabled)?;

            // If widget or skin, open or close the popout window and manage its companion service
            let p_dir = crate::commands::system_cmd::find_plugin_dir(target_pid);
            let manifest_path = p_dir.join("manifest.json");
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                    let p_type = manifest.plugin_type.as_deref().unwrap_or_default();
                    if p_type.eq_ignore_ascii_case("widget") || p_type.eq_ignore_ascii_case("skin") {
                        if enabled {
                            crate::system::native_wry::open_popout(target_pid);
                            if let Some(svc) = manifest.service {
                                if svc.auto_start {
                                    let _ = crate::system::plugin_service::start_service(target_pid);
                                }
                            }
                        } else {
                            crate::system::native_wry::close_popout(target_pid);
                            let _ = crate::system::plugin_service::stop_service(target_pid);
                        }
                    }
                }
            }
            crate::system::slint_window::sync::sync_slint_plugins_state();
            Ok(Value::Bool(res))
        }

        // System
        "close_widget" | "close_custom_widget" => {
            let pid = args.get("pluginId").or_else(|| args.get("plugin_id")).and_then(|v| v.as_str()).unwrap_or(plugin_id);
            crate::system::telemetry::unregister_consumer(pid);
            super::close_popout(pid);
            let mut cfg = crate::system::config::load_config();
            cfg.custom_plugin_toggles.insert(pid.to_string(), false);
            let _ = crate::system::config::save_config(&cfg);
            crate::system::slint_window::sync::sync_slint_plugins_state();
            Ok(Value::Bool(true))
        }
        "get_running_widgets" => {
            let list = super::get_running_popouts();
            Ok(serde_json::to_value(list).unwrap_or(Value::Array(vec![])))
        }
        "open_plugins_dir" => {
            crate::commands::system_cmd::open_plugins_dir()?;
            Ok(Value::Bool(true))
        }
        "get_app_config" => {
            let res = crate::commands::system_cmd::get_app_config();
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "get_live_power_status" => {
            let (percent, is_ac, is_charging) = crate::commands::power_cmd::get_live_power_status();
            Ok(serde_json::json!({
                "battery_percent": percent,
                "is_ac_online": is_ac,
                "is_charging": is_charging
            }))
        }

        // AI Gateway & Hub APIs (Native Wry / Slint host compatibility)
        "ai_get_config" => {
            let cfg = crate::commands::ai_cmd::ai_get_config()?;
            serde_json::to_value(cfg).map_err(|e| e.to_string())
        }
        "ai_save_config" => {
            let cfg_val = args.get("config").cloned().unwrap_or(args);
            let cfg = serde_json::from_value(cfg_val).map_err(|e| format!("配置格式错误: {}", e))?;
            crate::commands::ai_cmd::ai_save_config(cfg)?;
            Ok(Value::Bool(true))
        }
        "ai_fetch_models" => {
            let base_url = args.get("baseUrl").or_else(|| args.get("base_url")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let api_key = args.get("apiKey").or_else(|| args.get("api_key")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let list = crate::commands::ai_cmd::ai_fetch_models(base_url, api_key)?;
            serde_json::to_value(list).map_err(|e| e.to_string())
        }
        "ai_chat_send" => {
            let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let session_id = args.get("sessionId").or_else(|| args.get("session_id")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let enable_tools = args.get("enableTools").or_else(|| args.get("enable_tools")).and_then(|v| v.as_bool());
            let config_override: Option<crate::system::ai_gateway::types::AiGatewayConfig> = args.get("configOverride")
                .or_else(|| args.get("config_override"))
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let sid = crate::commands::ai_cmd::ai_chat_send(prompt, session_id, enable_tools, config_override)?;
            Ok(Value::String(sid))
        }
        "ai_get_system_skills" => {
            let skills = crate::commands::ai_cmd::ai_get_system_skills()?;
            Ok(skills)
        }
        "write_clipboard" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let res = crate::commands::market_cmd::write_clipboard(text)?;
            Ok(Value::Bool(res))
        }

        other => {
            crate::log_warn!("NATIVE_WRY", "Unhandled IPC command: '{}'", other);
            Err(format!("Command '{}' not supported in native Wry container", other))
        }
    }
}
