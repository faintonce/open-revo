use tauri::AppHandle;
use tauri::Manager;
use super::super::dialog::open_file_dialog;
use super::super::mutate_active_preset_profile;
use super::super::sync::{sync_slint_display_state, sync_slint_tuning_state};

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "open_custom_tuning" => {
            crate::system::telemetry::set_main_window_visible(true);
            crate::log_info!("SLINT_MAIN", "User requested custom tuning sub-panel");
            let cfg = crate::system::config::load_config();
            sync_slint_tuning_state();
            sync_slint_display_state();
            if cfg.power_mode == 4 {
                std::thread::spawn(move || {
                    let _ = crate::commands::tuning_cmd::heal_gpu_power_contract(None, None);
                    sync_slint_tuning_state();
                });
            }
            true
        }
        "tuning_back_to_overview" => {
            crate::log_info!("SLINT_MAIN", "Returned from custom tuning to overview");
            true
        }
        "tuning_toggle_temp_wall" => {
            mutate_active_preset_profile(|p| p.cpu_temp_target_enabled = !p.cpu_temp_target_enabled);
            let cfg = crate::system::config::load_config();
            let bounds = super::super::sync::compute_tuning_bounds(&cfg);
            let effective_temp_max = bounds
                .temp_wall_max
                .max((cfg.custom_profile.cpu_temp_target as i32).min(bounds.max_temp_wall));
            let effective_temp = if cfg.custom_profile.cpu_temp_target_enabled {
                (cfg.custom_profile.cpu_temp_target as i32).clamp(bounds.temp_wall_min, effective_temp_max) as u8
            } else {
                effective_temp_max as u8
            };
            let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                app, None, None, None, Some(effective_temp), None, None, None, None, None, None, None, None, None
            );
            true
        }
        "tuning_set_temp_wall" => {
            if let Some(t) = val.get("temp_wall").and_then(|v| v.as_i64()) {
                let cfg = crate::system::config::load_config();
                let bounds = super::super::sync::compute_tuning_bounds(&cfg);
                let effective_temp_max = bounds
                    .temp_wall_max
                    .max((t as i32).min(bounds.max_temp_wall));
                let temp = (t as i32).clamp(bounds.temp_wall_min, effective_temp_max) as u8;
                mutate_active_preset_profile(|p| p.cpu_temp_target = temp);
                if cfg.custom_profile.cpu_temp_target_enabled {
                    let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                        app, None, None, None, Some(temp), None, None, None, None, None, None, None, None, None
                    );
                }
            }
            true
        }
        "tuning_set_pl1" | "tuning_set_pl2" | "tuning_set_pl4" => {
            apply_pl_limit_update(app, action, val);
            true
        }
        "tuning_toggle_bios_oc" => {
            let bios_status = crate::driver::uefi_nvram::read_bios_advanced_menu_status();
            if bios_status.hardware_read && !bios_status.supported {
                crate::log_warn!("BIOS_OC", "Hardware does not support BIOS OC menu, blocked");
            } else {
                let next_val = !bios_status.enabled;
                crate::log_info!("BIOS_OC", "User toggling BIOS advanced menu to {}", next_val);
                match crate::driver::uefi_nvram::write_bios_advanced_menu_status(next_val) {
                    Ok(new_status) => {
                        crate::log_info!("BIOS_OC", "Successfully wrote BIOS advanced menu to {}, readback enabled={}", next_val, new_status.enabled);
                        mutate_active_preset_profile(|p| p.bios_oc_enabled = new_status.enabled);
                    }
                    Err(e) => {
                        crate::log_error!("BIOS_OC", "Failed to write BIOS advanced menu: {}", e);
                    }
                }
            }
            sync_slint_tuning_state();
            true
        }
        "tuning_toggle_super_perf" => {
            let (is_mode_locked, supports_ctgp, base_tgp_max_watts, max_db_watts) = probe_gpu_tuning_context();
            if is_mode_locked {
                mutate_active_preset_profile(|p| p.gpu_super_perf = true);
                sync_slint_tuning_state();
                return true;
            }
            mutate_active_preset_profile(|p| p.gpu_super_perf = !p.gpu_super_perf);
            let cfg = crate::system::config::load_config();
            let bounds = super::super::sync::compute_tuning_bounds(&cfg);
            let enabled = cfg.custom_profile.gpu_super_perf;
            let db_enabled = if enabled { true } else { cfg.custom_profile.gpu_db_enabled };
            let db_watts = if enabled {
                max_db_watts as u8
            } else {
                cfg.custom_profile.gpu_db_watts.min(max_db_watts as u8)
            };
            let core_offset = if enabled {
                cfg.custom_profile.gpu_core_offset_mhz.clamp(0, bounds.gpu_core_offset_max)
            } else {
                0
            };
            let mem_offset = if enabled {
                cfg.custom_profile.gpu_mem_offset_mhz.clamp(-bounds.gpu_mem_offset_max, bounds.gpu_mem_offset_max)
            } else {
                0
            };
            let ctgp_enabled = if supports_ctgp {
                Some(if enabled { false } else { cfg.custom_profile.gpu_ctgp_enabled })
            } else {
                None
            };
            let ctgp_watts = if supports_ctgp {
                Some(if enabled {
                    base_tgp_max_watts as u8
                } else if cfg.custom_profile.gpu_ctgp_watts > 0 {
                    cfg.custom_profile.gpu_ctgp_watts
                } else {
                    base_tgp_max_watts as u8
                })
            } else {
                None
            };
            let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                app,
                None,
                None,
                None,
                None,
                ctgp_enabled,
                ctgp_watts,
                Some(db_enabled),
                Some(db_watts),
                Some(enabled),
                Some(core_offset),
                Some(mem_offset),
                None,
                None,
            );
            true
        }
        "tuning_set_core_offset" => {
            if let Some(o) = val.get("offset").and_then(|v| v.as_i64()) {
                let (is_mode_locked, _, _, _) = probe_gpu_tuning_context();
                let cfg = crate::system::config::load_config();
                if !is_mode_locked && !cfg.custom_profile.gpu_super_perf {
                    return true;
                }
                let bounds = super::super::sync::compute_tuning_bounds(&cfg);
                let offset = (o as i32).clamp(0, bounds.gpu_core_offset_max);
                mutate_active_preset_profile(|p| p.gpu_core_offset_mhz = offset);
                let mem = cfg
                    .custom_profile
                    .gpu_mem_offset_mhz
                    .clamp(-bounds.gpu_mem_offset_max, bounds.gpu_mem_offset_max);
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
                let (is_mode_locked, _, _, _) = probe_gpu_tuning_context();
                let cfg = crate::system::config::load_config();
                if !is_mode_locked && !cfg.custom_profile.gpu_super_perf {
                    return true;
                }
                let bounds = super::super::sync::compute_tuning_bounds(&cfg);
                let offset = (o as i32).clamp(-bounds.gpu_mem_offset_max, bounds.gpu_mem_offset_max);
                mutate_active_preset_profile(|p| p.gpu_mem_offset_mhz = offset);
                let core = cfg
                    .custom_profile
                    .gpu_core_offset_mhz
                    .clamp(0, bounds.gpu_core_offset_max);
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
        "tuning_set_ctgp" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u8;
                mutate_active_preset_profile(|p| {
                    p.gpu_ctgp_watts = watts;
                    p.gpu_ctgp_enabled = true;
                });
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
                    app, None, None, None, None, Some(true), Some(watts), None, None, None, None, None, None, None
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
            mutate_active_preset_profile(|p| {
                let next = !p.fan_isolated_output;
                p.fan_isolated_output = next;
                p.fan_curve_enabled = next;
            });
            let cfg = crate::system::config::load_config();
            let is_bld = crate::hal::declarative::load_matched_manifest()
                .map(|m| m.id.contains("bld"))
                .unwrap_or(false);
            if cfg.power_mode == 4 || is_bld {
                let _ = crate::commands::tuning_cmd::toggle_fan_isolated_output_internal(
                    app,
                    cfg.custom_profile.fan_isolated_output,
                    None,
                    None,
                );
            }
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
                activate_preset_profile(app, id);
                sync_slint_tuning_state();
                sync_slint_display_state();
            }
            true
        }
        "tuning_create_preset" => {
            let cfg = crate::system::config::load_config();
            let active_id = super::super::sync::active_profile_id_or_default(&cfg);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() % 10000)
                .unwrap_or(0);
            let name = format!("极客方案 {}", now);
            if let Ok(created) = crate::commands::tuning_cmd::create_profile_preset(
                name,
                Some(active_id),
                Some(cfg.custom_profile),
            ) {
                activate_preset_profile(app, &created.id);
            }
            sync_slint_tuning_state();
            sync_slint_display_state();
            true
        }
        "tuning_reset_preset" => {
            let cfg = crate::system::config::load_config();
            let active_id = super::super::sync::active_profile_id_or_default(&cfg);
            crate::log_info!("TUNING_RESET", "Starting reset for preset: {}", active_id);
            match crate::commands::tuning_cmd::reset_profile_preset(active_id.clone()) {
                Ok(target) => {
                    crate::log_info!("TUNING_RESET", "Successfully reset preset {} to factory baseline", active_id);
                    let target_profile = target.profile;
                    if target_profile.fan_isolated_output || target_profile.fan_curve_enabled {
                        let app_clone = app.cloned();
                        let c_clone = target_profile.cpu_fan_curve;
                        let g_clone = target_profile.gpu_fan_curve;
                        std::thread::spawn(move || {
                            let _ = crate::commands::tuning_cmd::apply_fan_curve_live_internal(
                                app_clone.as_ref(),
                                c_clone,
                                g_clone,
                            );
                        });
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
            let app_clone = app.cloned();
            tauri::async_runtime::spawn(async move {
                if let Some(file_path) = open_file_dialog("JSON 调教方案", "json") {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let fname = file_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
                        if let Ok(imported) = crate::commands::tuning_cmd::import_profile_preset(content, fname) {
                            activate_preset_profile(app_clone.as_ref(), &imported.id);
                            sync_slint_tuning_state();
                            sync_slint_display_state();
                        }
                    }
                }
            });
            true
        }
        "tuning_export_preset" => {
            let cfg = crate::system::config::load_config();
            let active_id = super::super::sync::active_profile_id_or_default(&cfg);
            let _ = crate::commands::tuning_cmd::export_profile_preset(active_id);
            true
        }
        "tuning_delete_preset" => {
            let cfg = crate::system::config::load_config();
            let active_id = super::super::sync::active_profile_id_or_default(&cfg);
            if crate::commands::tuning_cmd::delete_profile_preset(active_id).is_ok() {
                activate_preset_profile(app, "preset_silent_beast");
            }
            sync_slint_tuning_state();
            sync_slint_display_state();
            true
        }
        "tuning_open_preset_dir" => {
            let _ = crate::commands::tuning_cmd::open_profiles_folder();
            true
        }
        "tuning_adjust_pl1_max" | "tuning_adjust_pl2_max" | "tuning_adjust_pl4_max" | "tuning_adjust_temp_wall_max" => {
            if let Some(delta) = val.get("delta").and_then(|v| v.as_i64()) {
                adjust_slider_max(app, action, delta as i32);
            }
            true
        }
        "tuning_select_fan_tab" => {
            if let Some(tab) = val.get("tab").and_then(|v| v.as_i64()) {
                crate::log_info!("SLINT_MAIN", "Tuning fan tab selected: {}", tab);
            }
            true
        }
        "tuning_rename_preset" => {
            let cfg = crate::system::config::load_config();
            let active_id = super::super::sync::active_profile_id_or_default(&cfg);
            if let Some(new_name) = val.get("new_name").and_then(|v| v.as_str()) {
                let _ = crate::commands::tuning_cmd::rename_profile_preset(active_id, new_name.to_string());
                sync_slint_tuning_state();
            } else {
                crate::log_info!("SLINT_MAIN", "tuning_rename_preset called for active preset: {}", active_id);
            }
            true
        }
        _ => false,
    }
}

fn probe_gpu_tuning_context() -> (bool, bool, u32, u32) {
    let manifest = crate::hal::declarative::load_matched_manifest();
    let is_bld = manifest.as_ref().map(|m| m.id.contains("bld")).unwrap_or(false);
    let base_tgp = crate::system::power_mode::get_cached_factory_power_presets()
        .map(|fp| fp.base_tgp)
        .unwrap_or(150);
    let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    let gpu_power_tuning_mode = manifest
        .as_ref()
        .and_then(|m| m.capabilities.gpu_power_tuning_mode.clone())
        .unwrap_or_else(|| if is_bld { "mode_locked".to_string() } else { "none".to_string() });
    let is_mode_locked = gpu_power_tuning_mode == "mode_locked";
    let supports_ctgp = !is_mode_locked && gpu_caps.supports_ctgp;
    let base_tgp_max_watts = if gpu_caps.base_tgp_max_watts > 0 {
        gpu_caps.base_tgp_max_watts
    } else {
        base_tgp as u32
    };
    let max_db_watts = if gpu_caps.max_db_watts > 0 { gpu_caps.max_db_watts } else { 25 };
    (is_mode_locked, supports_ctgp, base_tgp_max_watts, max_db_watts)
}

fn apply_pl_limit_update(app: Option<&tauri::AppHandle>, action: &str, val: &serde_json::Value) {
    let cfg = crate::system::config::load_config();
    let bounds = super::super::sync::compute_tuning_bounds(&cfg);
    let cur = &cfg.custom_profile;
    let mut p1 = val.get("pl1").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(cur.cpu_pl1_watts as i32);
    let mut p2 = val.get("pl2").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(cur.cpu_pl2_watts as i32);
    let mut p4 = val.get("pl4").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(cur.cpu_pl4_watts as i32);
    let min_pl4 = bounds.min_pl + bounds.pl4_margin;

    match action {
        "tuning_set_pl1" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                p1 = w as i32;
            }
            p1 = p1.clamp(bounds.min_pl, bounds.pl1_max);
            p2 = p2.max(p1).clamp(bounds.min_pl, bounds.pl2_max);
            p4 = p4.max(p2 + bounds.pl4_margin).clamp(min_pl4, bounds.pl4_max);
            p2 = p2.min((p4 - bounds.pl4_margin).max(bounds.min_pl));
            p1 = p1.min(p2);
        }
        "tuning_set_pl2" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                p2 = w as i32;
            }
            p2 = p2.clamp(bounds.min_pl, bounds.pl2_max);
            p1 = p1.min(p2).clamp(bounds.min_pl, bounds.pl1_max);
            p4 = p4.max(p2 + bounds.pl4_margin).clamp(min_pl4, bounds.pl4_max);
            p2 = p2.min((p4 - bounds.pl4_margin).max(bounds.min_pl));
            p1 = p1.min(p2);
        }
        _ => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                p4 = w as i32;
            }
            p4 = p4.clamp(min_pl4, bounds.pl4_max);
            let max_allowed_pl2 = (p4 - bounds.pl4_margin).max(bounds.min_pl);
            p2 = p2.min(max_allowed_pl2).clamp(bounds.min_pl, bounds.pl2_max);
            p1 = p1.min(p2).clamp(bounds.min_pl, bounds.pl1_max);
        }
    }

    mutate_active_preset_profile(|p| {
        p.cpu_pl1_watts = p1 as u16;
        p.cpu_pl2_watts = p2 as u16;
        p.cpu_pl4_watts = p4 as u16;
    });
    let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
        app,
        Some(p1 as u16),
        Some(p2 as u16),
        Some(p4 as u16),
        None, None, None, None, None, None, None, None, None, None,
    );
}

fn activate_preset_profile(app: Option<&tauri::AppHandle>, id: &str) {
    if let Some(app) = app {
        let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
        let _ = crate::commands::tuning_cmd::set_active_profile_id(id.to_string(), ec_state);
        return;
    }
    let mut cfg = crate::system::config::load_config();
    cfg.active_profile_id = id.to_string();
    let profiles = crate::system::config::list_profiles();
    if let Some(target) = profiles.into_iter().find(|p| p.id == id) {
        cfg.custom_profile = target.profile.clone();
        let _ = crate::system::config::save_config(&cfg);
        if crate::hal::declarative::load_matched_manifest().is_some() {
            crate::commands::tuning_cmd::sync_custom_profile_to_declarative_hardware(&target.profile);
        } else {
            crate::driver::ec_bridge::get_global_ec().lock().set_power_mode(4);
        }
    } else {
        let _ = crate::system::config::save_config(&cfg);
    }
}

fn adjust_slider_max(app: Option<&tauri::AppHandle>, action: &str, delta: i32) {
    let mut cfg = crate::system::config::load_config();
    let bounds = super::super::sync::compute_tuning_bounds(&cfg);
    let min_pl4 = bounds.min_pl + bounds.pl4_margin;
    let mut p1 = cfg.custom_profile.cpu_pl1_watts as i32;
    let mut p2 = cfg.custom_profile.cpu_pl2_watts as i32;
    let mut p4 = cfg.custom_profile.cpu_pl4_watts as i32;
    let mut tw = cfg.custom_profile.cpu_temp_target as i32;
    let mut pl_changed = false;
    let mut tw_changed = false;

    match action {
        "tuning_adjust_pl1_max" => {
            let next_max = (bounds.pl1_max + delta).clamp(bounds.min_max_pl1, bounds.max_max_pl1);
            cfg.cpu_pl1_max = Some(next_max.clamp(15, 255) as u16);
            if p1 > next_max {
                p1 = next_max.clamp(bounds.min_pl, next_max);
                pl_changed = true;
            }
        }
        "tuning_adjust_pl2_max" => {
            let next_max = (bounds.pl2_max + delta).clamp(bounds.min_max_pl2, bounds.max_max_pl2);
            cfg.cpu_pl2_max = Some(next_max.clamp(15, 255) as u16);
            if p2 > next_max {
                p2 = next_max.clamp(bounds.min_pl, next_max);
                p1 = p1.min(p2);
                pl_changed = true;
            }
        }
        "tuning_adjust_pl4_max" => {
            let next_max = (bounds.pl4_max + delta).clamp(bounds.min_max_pl4, bounds.max_max_pl4);
            cfg.cpu_pl4_max = Some(next_max.clamp(15, 255) as u16);
            if p4 > next_max {
                p4 = next_max.clamp(min_pl4, next_max);
                let max_pl2 = (p4 - bounds.pl4_margin).max(bounds.min_pl);
                p2 = p2.min(max_pl2);
                p1 = p1.min(p2);
                pl_changed = true;
            }
        }
        "tuning_adjust_temp_wall_max" => {
            let next_max = (bounds.temp_wall_max + delta).clamp(bounds.temp_wall_min, bounds.max_temp_wall);
            cfg.cpu_temp_target_max = next_max.clamp(60, 120) as u8;
            if tw > next_max {
                tw = next_max;
                tw_changed = true;
            }
        }
        _ => {}
    }

    let _ = crate::system::config::save_config(&cfg);
    if pl_changed || tw_changed {
        mutate_active_preset_profile(|p| {
            if pl_changed {
                p.cpu_pl1_watts = p1 as u16;
                p.cpu_pl2_watts = p2 as u16;
                p.cpu_pl4_watts = p4 as u16;
            }
            if tw_changed {
                p.cpu_temp_target = tw as u8;
            }
        });
        let _ = crate::commands::tuning_cmd::apply_live_custom_tweak_internal(
            app,
            if pl_changed { Some(p1 as u16) } else { None },
            if pl_changed { Some(p2 as u16) } else { None },
            if pl_changed { Some(p4 as u16) } else { None },
            if tw_changed && cfg.custom_profile.cpu_temp_target_enabled { Some(tw as u8) } else { None },
            None, None, None, None, None, None, None, None, None,
        );
    }
    sync_slint_tuning_state();
}

