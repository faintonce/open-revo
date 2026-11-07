use crate::MainWindow;

pub fn handle(action: &str, val: &serde_json::Value, ui: &MainWindow) -> bool {
    match action {
        "open_tuning_view" => {
            ui.set_is_in_tuning_view(true);
            true
        }
        "sync_tuning_state" => {
            if let Some(cv) = val.get("cpu_vendor").and_then(|v| v.as_str()) {
                ui.set_cpu_vendor(cv.into());
            }
            if let Some(ctt) = val.get("cpu_tuning_type").and_then(|v| v.as_str()) {
                ui.set_cpu_tuning_type(ctt.into());
            }
            if let Some(fppt) = val.get("cpu_supports_fppt").and_then(|v| v.as_bool()) {
                ui.set_cpu_supports_fppt(fppt);
            }
            if let Some(bld) = val.get("is_bld").and_then(|v| v.as_bool()) {
                ui.set_is_bld(bld);
            }
            if let Some(preset) = val.get("active_preset_name").and_then(|v| v.as_str()) {
                ui.set_active_preset_name(preset.into());
            }
            if let Some(names_arr) = val.get("preset_names").and_then(|v| v.as_array()) {
                let names: Vec<slint::SharedString> = names_arr.iter().filter_map(|x| x.as_str().map(|s| s.into())).collect();
                if !names.is_empty() {
                    ui.set_preset_names(slint::ModelRc::new(slint::VecModel::from(names)));
                }
            }
            if let Some(ids_arr) = val.get("preset_ids").and_then(|v| v.as_array()) {
                let ids: Vec<slint::SharedString> = ids_arr.iter().filter_map(|x| x.as_str().map(|s| s.into())).collect();
                if !ids.is_empty() {
                    ui.set_preset_ids(slint::ModelRc::new(slint::VecModel::from(ids)));
                }
            }
            if let Some(tw) = val.get("temp_wall").and_then(|v| v.as_i64()) {
                ui.set_temp_wall(tw as i32);
            }
            if let Some(twe) = val.get("temp_wall_enabled").and_then(|v| v.as_bool()) {
                ui.set_temp_wall_enabled(twe);
            }
            if let Some(twm) = val.get("temp_wall_max").and_then(|v| v.as_i64()) {
                ui.set_temp_wall_max(twm as i32);
            }
            if let Some(p1) = val.get("pl1").and_then(|v| v.as_i64()) {
                ui.set_pl1(p1 as i32);
            }
            if let Some(p1m) = val.get("pl1_max").and_then(|v| v.as_i64()) {
                ui.set_pl1_max(p1m as i32);
            }
            if let Some(p2) = val.get("pl2").and_then(|v| v.as_i64()) {
                ui.set_pl2(p2 as i32);
            }
            if let Some(p2m) = val.get("pl2_max").and_then(|v| v.as_i64()) {
                ui.set_pl2_max(p2m as i32);
            }
            if let Some(p4) = val.get("pl4").and_then(|v| v.as_i64()) {
                ui.set_pl4(p4 as i32);
            }
            if let Some(p4m) = val.get("pl4_max").and_then(|v| v.as_i64()) {
                ui.set_pl4_max(p4m as i32);
            }
            if let Some(boc) = val.get("bios_oc_enabled").and_then(|v| v.as_bool()) {
                ui.set_bios_oc_enabled(boc);
            }

            if let Some(sp) = val.get("super_perf").and_then(|v| v.as_bool()) {
                ui.set_super_perf(sp);
            }
            if let Some(co) = val.get("core_offset").and_then(|v| v.as_i64()) {
                ui.set_core_offset(co as i32);
            }
            if let Some(mo) = val.get("mem_offset").and_then(|v| v.as_i64()) {
                ui.set_mem_offset(mo as i32);
            }
            if let Some(dbe) = val.get("db_enabled").and_then(|v| v.as_bool()) {
                ui.set_db_enabled(dbe);
            }
            if let Some(dbw) = val.get("db_watts").and_then(|v| v.as_i64()) {
                ui.set_db_watts(dbw as i32);
            }
            if let Some(tgp) = val.get("base_tgp").and_then(|v| v.as_i64()) {
                ui.set_base_tgp(tgp as i32);
            }

            if let Some(st) = val.get("smooth_transition").and_then(|v| v.as_bool()) {
                ui.set_smooth_transition(st);
            }
            if let Some(stm) = val.get("smooth_time").and_then(|v| v.as_str()) {
                ui.set_smooth_time(stm.into());
            }
            if let Some(ifo) = val.get("fan_isolated_output").and_then(|v| v.as_bool()) {
                ui.set_fan_isolated_output(ifo);
            }
            if let Some(fl) = val.get("fan_linked").and_then(|v| v.as_bool()) {
                ui.set_fan_linked(fl);
            }
            if let Some(c_arr) = val.get("cpu_curve").and_then(|v| v.as_array()) {
                let c: Vec<i32> = c_arr.iter().filter_map(|x| x.as_i64().map(|v| v as i32)).collect();
                if c.len() == 8 {
                    ui.set_cpu_curve(slint::ModelRc::new(slint::VecModel::from(c)));
                }
            }
            if let Some(g_arr) = val.get("gpu_curve").and_then(|v| v.as_array()) {
                let g: Vec<i32> = g_arr.iter().filter_map(|x| x.as_i64().map(|v| v as i32)).collect();
                if g.len() == 8 {
                    ui.set_gpu_curve(slint::ModelRc::new(slint::VecModel::from(g)));
                }
            }
            if let Some(bld_cap) = val.get("bld_rpm_cap").and_then(|v| v.as_i64()) {
                ui.set_bld_rpm_cap(bld_cap as i32);
            }
            if let Some(bld_b) = val.get("bld_boost").and_then(|v| v.as_bool()) {
                ui.set_bld_boost(bld_b);
            }
            true
        }
        _ => false,
    }
}

