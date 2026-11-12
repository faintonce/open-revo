use crate::MainWindow;

pub fn handle(action: &str, val: &serde_json::Value, ui: &MainWindow) -> bool {
    match action {
        "sync_display_state" => {
            if let Some(en) = val.get("tuning_enabled").and_then(|v| v.as_bool()) {
                ui.set_tuning_enabled(en);
            }
            if let Some(id) = val.get("active_preset_id").and_then(|v| v.as_str()) {
                ui.set_active_preset_id(id.into());
            }
            if let Some(name) = val.get("active_preset_name").and_then(|v| v.as_str()) {
                ui.set_active_display_preset_name(name.into());
            }
            if let Some(temp) = val.get("tuning_temp_k").and_then(|v| v.as_i64()) {
                ui.set_tuning_temp_k(temp as i32);
            }
            if let Some(sat) = val.get("tuning_saturation").and_then(|v| v.as_i64()) {
                ui.set_tuning_saturation(sat as i32);
            }
            if let Some(con) = val.get("tuning_contrast").and_then(|v| v.as_i64()) {
                ui.set_tuning_contrast(con as i32);
            }
            if let Some(gam) = val.get("tuning_gamma").and_then(|v| v.as_i64()) {
                ui.set_tuning_gamma(gam as i32);
            }
            if let Some(r) = val.get("tuning_gain_r").and_then(|v| v.as_i64()) {
                ui.set_tuning_gain_r(r as i32);
            }
            if let Some(g) = val.get("tuning_gain_g").and_then(|v| v.as_i64()) {
                ui.set_tuning_gain_g(g as i32);
            }
            if let Some(b) = val.get("tuning_gain_b").and_then(|v| v.as_i64()) {
                ui.set_tuning_gain_b(b as i32);
            }
            if let Some(m) = val.get("tuning_is_modified").and_then(|v| v.as_bool()) {
                ui.set_tuning_is_modified(m);
            }
            if let Some(t) = val.get("tuning_toast").and_then(|v| v.as_str()) {
                ui.set_tuning_toast(t.into());
            }

            if let Some(prof_name) = val.get("active_color_profile").and_then(|v| v.as_str()) {
                ui.set_active_color_profile(prof_name.into());
            }
            if let Some(prof_id) = val.get("active_color_profile_id").and_then(|v| v.as_str()) {
                ui.set_active_color_profile_id(prof_id.into());
            }
            if let Some(p_list) = val.get("color_profiles_list").and_then(|v| v.as_array()) {
                let names: Vec<slint::SharedString> = p_list.iter().filter_map(|s| s.as_str().map(|str_val| str_val.into())).collect();
                if !names.is_empty() {
                    ui.set_color_profiles_list(slint::ModelRc::new(slint::VecModel::from(names)));
                }
            }
            if let Some(p_ids) = val.get("color_profiles_ids").and_then(|v| v.as_array()) {
                let ids: Vec<slint::SharedString> = p_ids.iter().filter_map(|s| s.as_str().map(|str_val| str_val.into())).collect();
                if !ids.is_empty() {
                    ui.set_color_profiles_ids(slint::ModelRc::new(slint::VecModel::from(ids)));
                }
            }

            if let Some(mon) = val.get("monitor_name").and_then(|v| v.as_str()) {
                ui.set_monitor_name(mon.into());
            }
            if let Some(sup) = val.get("miniled_supported").and_then(|v| v.as_bool()) {
                ui.set_miniled_supported(sup);
            }
            if let Some(en) = val.get("miniled_enabled").and_then(|v| v.as_bool()) {
                ui.set_miniled_enabled(en);
            }
            if let Some(hz) = val.get("refresh_rate").and_then(|v| v.as_i64()) {
                ui.set_refresh_rate(hz as i32);
            }
            if let Some(b) = val.get("brightness").and_then(|v| v.as_i64()) {
                ui.set_brightness(b as i32);
            }
            if let Some(r1_arr) = val.get("rates_row1").and_then(|v| v.as_array()) {
                let r1: Vec<i32> = r1_arr.iter().filter_map(|r| r.as_i64().map(|v| v as i32)).collect();
                if !r1.is_empty() {
                    ui.set_rates_row1(slint::ModelRc::new(slint::VecModel::from(r1)));
                }
            }
            if let Some(r2_arr) = val.get("rates_row2").and_then(|v| v.as_array()) {
                let r2: Vec<i32> = r2_arr.iter().filter_map(|r| r.as_i64().map(|v| v as i32)).collect();
                ui.set_rates_row2(slint::ModelRc::new(slint::VecModel::from(r2)));
            }
            if let Some(h) = val.get("has_secondary_monitor").and_then(|v| v.as_bool()) {
                ui.set_has_secondary_monitor(h);
            }
            if let Some(p) = val.get("primary_monitor_name").and_then(|v| v.as_str()) {
                ui.set_primary_monitor_name(p.into());
            }
            if let Some(s) = val.get("secondary_monitor_name").and_then(|v| v.as_str()) {
                ui.set_secondary_monitor_name(s.into());
            }
            true
        }
        "sync_cloud_icc_identity" => {
            if let Some(pnp) = val.get("panel_pnp").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_panel_pnp(pnp.into());
            }
            if let Some(sn) = val.get("panel_sn").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_panel_sn(sn.into());
            }
            if let Some(bsn) = val.get("bios_sn").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_bios_sn(bsn.into());
            }
            if let Some(mac) = val.get("mac_address").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_mac(mac.into());
            }
            if let Some(arch) = val.get("arch_name").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_arch(arch.into());
            }
            if let Some(rec) = val.get("recommended_method").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_recommended(rec.into());
                ui.set_cloud_icc_mode(rec.into());
            }
            ui.set_cloud_icc_status("就绪".into());
            true
        }
        "sync_cloud_icc_searching" => {
            if let Some(s) = val.get("is_searching").and_then(|v| v.as_bool()) {
                ui.set_cloud_icc_is_searching(s);
            }
            if let Some(st) = val.get("status").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_status(st.into());
            }
            true
        }
        "sync_cloud_icc_hit" => {
            ui.set_cloud_icc_is_searching(false);
            let hit_found = val.get("hit_found").and_then(|v| v.as_bool()).unwrap_or(false);
            ui.set_cloud_icc_hit_found(hit_found);
            if hit_found {
                if let Some(fn_str) = val.get("file_name").and_then(|v| v.as_str()) {
                    ui.set_cloud_icc_hit_file_name(fn_str.into());
                }
                if let Some(meta) = val.get("meta").and_then(|v| v.as_str()) {
                    ui.set_cloud_icc_hit_meta(meta.into());
                }
                if let Some(profs) = val.get("profiles").and_then(|v| v.as_array()) {
                    let list: Vec<slint::SharedString> = profs.iter().filter_map(|s| s.as_str().map(|v| v.into())).collect();
                    ui.set_cloud_icc_hit_profiles(slint::ModelRc::new(slint::VecModel::from(list)));
                }
                ui.set_cloud_icc_selected_profile_idx(0);
                ui.set_cloud_icc_status("已成功命中原厂校色包".into());
            } else {
                if let Some(toast) = val.get("toast").and_then(|v| v.as_str()) {
                    ui.set_cloud_icc_toast(toast.into());
                }
                ui.set_cloud_icc_status("未检索到匹配的出厂校色".into());
            }
            true
        }
        "cloud_icc_toast" => {
            if let Some(t) = val.get("toast").and_then(|v| v.as_str()) {
                ui.set_cloud_icc_toast(t.into());
            }
            true
        }
        _ => false,
    }
}

