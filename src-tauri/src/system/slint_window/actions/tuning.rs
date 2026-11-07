use tauri::AppHandle;
use tauri::Manager;
use super::super::dialog::open_file_dialog;
use super::super::mutate_active_preset_profile;
use super::super::sync::{sync_slint_display_state, sync_slint_tuning_state};

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "open_custom_tuning" => {
            crate::log_info!("SLINT_MAIN", "User requested custom tuning sub-panel");
            true
        }
        "tuning_back_to_overview" => {
            crate::log_info!("SLINT_MAIN", "Returned from custom tuning to overview");
            true
        }
        "tuning_toggle_temp_wall" => {
            mutate_active_preset_profile(|p| p.cpu_temp_target_enabled = !p.cpu_temp_target_enabled);
            true
        }
        "tuning_set_temp_wall" => {
            if let Some(t) = val.get("temp_wall").and_then(|v| v.as_i64()) {
                let temp = t as u8;
                mutate_active_preset_profile(|p| p.cpu_temp_target = temp);
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                    app, None, None, None, Some(temp), None, None, None, None, None, None, None, None, None
                );
            }
            true
        }
        "tuning_set_pl1" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u16;
                mutate_active_preset_profile(|p| p.cpu_pl1_watts = watts);
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                    app, Some(watts), None, None, None, None, None, None, None, None, None, None, None, None
                );
            }
            true
        }
        "tuning_set_pl2" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u16;
                mutate_active_preset_profile(|p| p.cpu_pl2_watts = watts);
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                    app, None, Some(watts), None, None, None, None, None, None, None, None, None, None, None
                );
            }
            true
        }
        "tuning_set_pl4" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u16;
                mutate_active_preset_profile(|p| p.cpu_pl4_watts = watts);
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                    app, None, None, Some(watts), None, None, None, None, None, None, None, None, None, None
                );
            }
            true
        }
        "tuning_toggle_bios_oc" => {
            mutate_active_preset_profile(|p| p.bios_oc_enabled = !p.bios_oc_enabled);
            let cfg = crate::system::config::load_config();
            let _ = crate::driver::uefi_nvram::write_bios_advanced_menu_status(cfg.custom_profile.bios_oc_enabled);
            true
        }
        "tuning_toggle_super_perf" => {
            mutate_active_preset_profile(|p| p.gpu_super_perf = !p.gpu_super_perf);
            let cfg = crate::system::config::load_config();
            let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                app, None, None, None, None, None, None, None, None, Some(cfg.custom_profile.gpu_super_perf), None, None, None, None
            );
            true
        }
        "tuning_set_core_offset" => {
            if let Some(o) = val.get("offset").and_then(|v| v.as_i64()) {
                let offset = o as i32;
                mutate_active_preset_profile(|p| p.gpu_core_offset_mhz = offset);
                let cfg = crate::system::config::load_config();
                let mem = cfg.custom_profile.gpu_mem_offset_mhz;
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::set_gpu_clock_offsets(app.clone(), offset, mem);
                } else {
                    let gpu = crate::driver::gpu_bridge::get_global_gpu();
                    gpu.lock().set_clock_offsets(offset, mem);
                }
            }
            true
        }
        "tuning_set_mem_offset" => {
            if let Some(o) = val.get("offset").and_then(|v| v.as_i64()) {
                let offset = o as i32;
                mutate_active_preset_profile(|p| p.gpu_mem_offset_mhz = offset);
                let cfg = crate::system::config::load_config();
                let core = cfg.custom_profile.gpu_core_offset_mhz;
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::set_gpu_clock_offsets(app.clone(), core, offset);
                } else {
                    let gpu = crate::driver::gpu_bridge::get_global_gpu();
                    gpu.lock().set_clock_offsets(core, offset);
                }
            }
            true
        }
        "tuning_toggle_db" => {
            mutate_active_preset_profile(|p| p.gpu_db_enabled = !p.gpu_db_enabled);
            let cfg = crate::system::config::load_config();
            let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                app, None, None, None, None, None, None, Some(cfg.custom_profile.gpu_db_enabled), None, None, None, None, None, None
            );
            true
        }
        "tuning_set_db_watts" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u8;
                mutate_active_preset_profile(|p| p.gpu_db_watts = watts);
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                    app, None, None, None, None, None, None, None, Some(watts), None, None, None, None, None
                );
            }
            true
        }
        "tuning_toggle_smooth" => {
            mutate_active_preset_profile(|p| p.fan_ramp_rate = !p.fan_ramp_rate);
            let cfg = crate::system::config::load_config();
            let _ = crate::commands::tuning_cmd::toggle_fan_ramp_rate(cfg.custom_profile.fan_ramp_rate, Some(cfg.custom_profile.fan_ramp_speed_ms));
            true
        }
        "tuning_toggle_isolated" => {
            mutate_active_preset_profile(|p| p.fan_isolated_output = !p.fan_isolated_output);
            let cfg = crate::system::config::load_config();
            let _ = crate::commands::tuning_cmd::toggle_fan_isolated_output_internal(
                app,
                cfg.custom_profile.fan_isolated_output,
                None,
                None,
            );
            true
        }
        "tuning_toggle_link" => {
            mutate_active_preset_profile(|p| p.fan_curve_linked = !p.fan_curve_linked);
            true
        }
        "tuning_set_fan_curve" => {
            if let (Some(c_arr), Some(g_arr)) = (val.get("cpu_curve").and_then(|v| v.as_array()), val.get("gpu_curve").and_then(|v| v.as_array())) {
                let c_pcts: Vec<u8> = c_arr.iter().filter_map(|x| x.as_i64().map(|v| v as u8)).collect();
                let g_pcts: Vec<u8> = g_arr.iter().filter_map(|x| x.as_i64().map(|v| v as u8)).collect();
                if c_pcts.len() == 8 && g_pcts.len() == 8 {
                    let default_curve = crate::system::config::default_fan_curve();
                    let mut new_cpu_curve = default_curve.clone();
                    let mut new_gpu_curve = default_curve.clone();
                    for (i, p) in new_cpu_curve.iter_mut().enumerate() {
                        p.duty = c_pcts[i];
                    }
                    for (i, p) in new_gpu_curve.iter_mut().enumerate() {
                        p.duty = g_pcts[i];
                    }
                    let c_clone = new_cpu_curve.clone();
                    let g_clone = new_gpu_curve.clone();
                    mutate_active_preset_profile(|p| {
                        p.cpu_fan_curve = c_clone;
                        p.gpu_fan_curve = g_clone;
                    });
                    let app_clone = app.cloned();
                    std::thread::spawn(move || {
                        let _ = crate::commands::tuning_cmd::apply_fan_curve_live_internal(
                            app_clone.as_ref(),
                            new_cpu_curve,
                            new_gpu_curve,
                        );
                    });
                }
            }
            true
        }
        "tuning_toggle_bld_boost" => {
            let is_clamped = crate::driver::bld_driver_mgr::is_bld_fan_clamped();
            let target = if is_clamped { 0 } else { 5800 };
            let _ = crate::commands::tuning_cmd::set_bld_fan_speed_cap_internal(app, target);
            true
        }
        "tuning_change_bld_cap" => {
            if let Some(rpm) = val.get("rpm").and_then(|v| v.as_i64()) {
                let _ = crate::commands::tuning_cmd::set_bld_fan_speed_cap_internal(app, rpm as u16);
            }
            true
        }
        "tuning_restore_bld_auto" => {
            let _ = crate::commands::tuning_cmd::set_bld_fan_speed_cap_internal(app, 0);
            true
        }
        "tuning_select_preset" => {
            if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                if let Some(app) = app {
                    let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                    let _ = crate::commands::tuning_cmd::set_active_profile_id(id.to_string(), ec_state);
                } else {
                    let mut cfg = crate::system::config::load_config();
                    cfg.active_profile_id = id.to_string();
                    let _ = crate::system::config::save_config(&cfg);
                }
                sync_slint_tuning_state();
                sync_slint_display_state();
            }
            true
        }
        "tuning_create_preset" => {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() % 10000).unwrap_or(0);
            let name = format!("极客方案 {}", now);
            let _ = crate::commands::tuning_cmd::create_profile_preset(name, None, None);
            sync_slint_tuning_state();
            sync_slint_display_state();
            true
        }
        "tuning_reset_preset" => {
            let active_id = {
                let cfg = crate::system::config::load_config();
                if !cfg.active_profile_id.is_empty() {
                    cfg.active_profile_id
                } else {
                    "preset_balance".to_string()
                }
            };
            crate::log_info!("TUNING_RESET", "Starting reset for preset: {}", active_id);
            match crate::commands::tuning_cmd::reset_profile_preset(active_id.clone()) {
                Ok(target) => {
                    crate::log_info!("TUNING_RESET", "Successfully reset preset {} to factory baseline", active_id);
                    let target_profile = target.profile.clone();
                    mutate_active_preset_profile(|p| {
                        *p = target_profile.clone();
                    });
                    let _ = crate::commands::tuning_cmd::heal_gpu_power_contract(None, None);
                    let _ = crate::commands::tuning_cmd::heal_cpu_power_contract(None, None, None, None, None);
                    let is_declarative = crate::hal::declarative::load_matched_manifest().is_some();
                    if is_declarative {
                        if let Err(error) = crate::commands::tuning_cmd::sync_custom_profile_to_declarative_hardware(&target.profile) {
                            crate::log_warn!("TUNING_RESET", "Custom profile synchronization failed: {}", error);
                        }
                    }
                }
                Err(e) => {
                    crate::log_error!("TUNING_RESET", "Failed to reset preset {}: {}", active_id, e);
                }
            }
            sync_slint_tuning_state();
            sync_slint_display_state();
            true
        }
        "tuning_import_preset" => {
            tauri::async_runtime::spawn(async move {
                if let Some(file_path) = open_file_dialog("JSON 调教方案", "json") {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let fname = file_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
                        let _ = crate::commands::tuning_cmd::import_profile_preset(content, fname);
                        sync_slint_tuning_state();
                        sync_slint_display_state();
                    }
                }
            });
            true
        }
        "tuning_export_preset" => {
            let active_id = {
                let cfg = crate::system::config::load_config();
                if !cfg.active_profile_id.is_empty() {
                    cfg.active_profile_id
                } else {
                    "preset_balance".to_string()
                }
            };
            let _ = crate::commands::tuning_cmd::export_profile_preset(active_id);
            true
        }
        "tuning_delete_preset" => {
            let active_id = {
                let cfg = crate::system::config::load_config();
                if !cfg.active_profile_id.is_empty() {
                    cfg.active_profile_id
                } else {
                    "preset_balance".to_string()
                }
            };
            let _ = crate::commands::tuning_cmd::delete_profile_preset(active_id);
            sync_slint_tuning_state();
            sync_slint_display_state();
            true
        }
        "tuning_open_preset_dir" => {
            let _ = crate::commands::tuning_cmd::open_profiles_folder();
            true
        }
        _ => false,
    }
}

