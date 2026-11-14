use crate::curve_helpers::apply_monotonic_fan_rule;
use crate::ipc;
use crate::MainWindow;
use slint::Model;

pub fn register(window: &MainWindow, ui_weak: slint::Weak<MainWindow>) {
    window.on_open_custom_tuning(move || {
        ipc::send_action_to_host("open_custom_tuning", serde_json::json!({}));
    });

    window.on_tuning_back_to_overview(move || {
        ipc::send_action_to_host("tuning_back_to_overview", serde_json::json!({}));
    });
    window.on_tuning_select_preset(move |id| {
        ipc::send_action_to_host("tuning_select_preset", serde_json::json!({ "id": id.as_str() }));
    });
    window.on_tuning_create_preset(move || {
        ipc::send_action_to_host("tuning_create_preset", serde_json::json!({}));
    });
    window.on_tuning_reset_preset(move || {
        ipc::send_action_to_host("tuning_reset_preset", serde_json::json!({}));
    });
    window.on_tuning_rename_preset(move || {
        ipc::send_action_to_host("tuning_rename_preset", serde_json::json!({}));
    });
    window.on_tuning_export_preset(move || {
        ipc::send_action_to_host("tuning_export_preset", serde_json::json!({}));
    });
    window.on_tuning_import_preset(move || {
        ipc::send_action_to_host("tuning_import_preset", serde_json::json!({}));
    });
    window.on_tuning_open_preset_dir(move || {
        ipc::send_action_to_host("tuning_open_preset_dir", serde_json::json!({}));
    });
    window.on_tuning_delete_preset(move || {
        ipc::send_action_to_host("tuning_delete_preset", serde_json::json!({}));
    });

    let ui_tw = ui_weak.clone();
    window.on_tuning_toggle_temp_wall(move || {
        ipc::send_action_to_host("tuning_toggle_temp_wall", serde_json::json!({}));
        if let Some(ui) = ui_tw.upgrade() {
            ui.set_temp_wall_enabled(!ui.get_temp_wall_enabled());
        }
    });
    let ui_tw_val = ui_weak.clone();
    window.on_tuning_change_temp_wall(move |t| {
        ipc::send_action_to_host("tuning_set_temp_wall", serde_json::json!({ "temp_wall": t }));
        if let Some(ui) = ui_tw_val.upgrade() {
            ui.set_temp_wall(t);
        }
    });
    let ui_tw_max = ui_weak.clone();
    window.on_tuning_adjust_temp_wall_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_temp_wall_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_tw_max.upgrade() {
            let next = (ui.get_temp_wall_max() + delta).clamp(ui.get_temp_wall_min(), ui.get_max_temp_wall());
            ui.set_temp_wall_max(next);
            if ui.get_temp_wall() > next {
                ui.set_temp_wall(next);
            }
        }
    });
    let ui_p1 = ui_weak.clone();
    window.on_tuning_change_pl1(move |w| {
        if let Some(ui) = ui_p1.upgrade() {
            let (p1, p2, p4) = update_coupled_pl(&ui, "pl1", w);
            ipc::send_action_to_host("tuning_set_pl1", serde_json::json!({ "watts": p1, "pl1": p1, "pl2": p2, "pl4": p4 }));
        } else {
            ipc::send_action_to_host("tuning_set_pl1", serde_json::json!({ "watts": w }));
        }
    });
    let ui_p1_max = ui_weak.clone();
    window.on_tuning_adjust_pl1_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_pl1_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_p1_max.upgrade() {
            let next = (ui.get_pl1_max() + delta).clamp(ui.get_min_max_pl1(), ui.get_max_max_pl1());
            ui.set_pl1_max(next);
            if ui.get_pl1() > next {
                update_coupled_pl(&ui, "pl1", next);
            }
        }
    });
    let ui_p2 = ui_weak.clone();
    window.on_tuning_change_pl2(move |w| {
        if let Some(ui) = ui_p2.upgrade() {
            let (p1, p2, p4) = update_coupled_pl(&ui, "pl2", w);
            ipc::send_action_to_host("tuning_set_pl2", serde_json::json!({ "watts": p2, "pl1": p1, "pl2": p2, "pl4": p4 }));
        } else {
            ipc::send_action_to_host("tuning_set_pl2", serde_json::json!({ "watts": w }));
        }
    });
    let ui_p2_max = ui_weak.clone();
    window.on_tuning_adjust_pl2_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_pl2_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_p2_max.upgrade() {
            let next = (ui.get_pl2_max() + delta).clamp(ui.get_min_max_pl2(), ui.get_max_max_pl2());
            ui.set_pl2_max(next);
            if ui.get_pl2() > next {
                update_coupled_pl(&ui, "pl2", next);
            }
        }
    });
    let ui_p4 = ui_weak.clone();
    window.on_tuning_change_pl4(move |w| {
        if let Some(ui) = ui_p4.upgrade() {
            let (p1, p2, p4) = update_coupled_pl(&ui, "pl4", w);
            ipc::send_action_to_host("tuning_set_pl4", serde_json::json!({ "watts": p4, "pl1": p1, "pl2": p2, "pl4": p4 }));
        } else {
            ipc::send_action_to_host("tuning_set_pl4", serde_json::json!({ "watts": w }));
        }
    });
    let ui_p4_max = ui_weak.clone();
    window.on_tuning_adjust_pl4_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_pl4_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_p4_max.upgrade() {
            let next = (ui.get_pl4_max() + delta).clamp(ui.get_min_max_pl4(), ui.get_max_max_pl4());
            ui.set_pl4_max(next);
            if ui.get_pl4() > next {
                update_coupled_pl(&ui, "pl4", next);
            }
        }
    });
    let ui_oc = ui_weak.clone();
    window.on_tuning_toggle_bios_oc(move || {
        ipc::send_action_to_host("tuning_toggle_bios_oc", serde_json::json!({}));
        if let Some(ui) = ui_oc.upgrade() {
            ui.set_bios_oc_btn_text("写入中...".into());
        }
    });

    let ui_sp = ui_weak.clone();
    window.on_tuning_toggle_super_perf(move || {
        if let Some(ui) = ui_sp.upgrade() {
            if ui.get_is_mode_locked() {
                ui.set_super_perf(true);
                return;
            }
            ui.set_super_perf(!ui.get_super_perf());
        }
        ipc::send_action_to_host("tuning_toggle_super_perf", serde_json::json!({}));
    });
    let ui_co = ui_weak.clone();
    window.on_tuning_change_core_offset(move |o| {
        if let Some(ui) = ui_co.upgrade() {
            if !ui.get_is_mode_locked() && !ui.get_super_perf() {
                return;
            }
            let clamped = o.clamp(0, ui.get_gpu_core_offset_max());
            ui.set_core_offset(clamped);
            ipc::send_action_to_host("tuning_set_core_offset", serde_json::json!({ "offset": clamped }));
        } else {
            ipc::send_action_to_host("tuning_set_core_offset", serde_json::json!({ "offset": o }));
        }
    });
    let ui_mo = ui_weak.clone();
    window.on_tuning_change_mem_offset(move |o| {
        if let Some(ui) = ui_mo.upgrade() {
            if !ui.get_is_mode_locked() && !ui.get_super_perf() {
                return;
            }
            let max = ui.get_gpu_mem_offset_max();
            let clamped = o.clamp(-max, max);
            ui.set_mem_offset(clamped);
            ipc::send_action_to_host("tuning_set_mem_offset", serde_json::json!({ "offset": clamped }));
        } else {
            ipc::send_action_to_host("tuning_set_mem_offset", serde_json::json!({ "offset": o }));
        }
    });
    let ui_db = ui_weak.clone();
    window.on_tuning_toggle_db(move || {
        ipc::send_action_to_host("tuning_toggle_db", serde_json::json!({}));
        if let Some(ui) = ui_db.upgrade() {
            ui.set_db_enabled(!ui.get_db_enabled());
        }
    });
    let ui_dbw = ui_weak.clone();
    window.on_tuning_change_db_watts(move |w| {
        ipc::send_action_to_host("tuning_set_db_watts", serde_json::json!({ "watts": w }));
        if let Some(ui) = ui_dbw.upgrade() {
            ui.set_db_watts(w);
        }
    });

    let ui_ctgp = ui_weak.clone();
    window.on_tuning_change_ctgp_watts(move |w| {
        ipc::send_action_to_host("tuning_set_ctgp", serde_json::json!({ "watts": w }));
        if let Some(ui) = ui_ctgp.upgrade() {
            ui.set_ctgp_watts(w);
            ui.set_ctgp_enabled(true);
        }
    });

    let ui_sm = ui_weak.clone();
    window.on_tuning_toggle_smooth(move || {
        ipc::send_action_to_host("tuning_toggle_smooth", serde_json::json!({}));
        if let Some(ui) = ui_sm.upgrade() {
            ui.set_smooth_transition(!ui.get_smooth_transition());
        }
    });
    let ui_smt = ui_weak.clone();
    window.on_tuning_change_smooth_time(move |st| {
        let ms_str = st.as_str().trim_end_matches("ms");
        if let Ok(ms) = ms_str.parse::<i64>() {
            ipc::send_action_to_host("tuning_set_smooth_time", serde_json::json!({ "speed_ms": ms }));
        }
        if let Some(ui) = ui_smt.upgrade() {
            ui.set_smooth_time(st);
        }
    });
    let ui_iso = ui_weak.clone();
    window.on_tuning_toggle_isolated(move || {
        ipc::send_action_to_host("tuning_toggle_isolated", serde_json::json!({}));
        if let Some(ui) = ui_iso.upgrade() {
            ui.set_fan_isolated_output(!ui.get_fan_isolated_output());
        }
    });
    let ui_lnk = ui_weak.clone();
    window.on_tuning_toggle_link(move || {
        ipc::send_action_to_host("tuning_toggle_link", serde_json::json!({}));
        if let Some(ui) = ui_lnk.upgrade() {
            let next_linked = !ui.get_fan_linked();
            ui.set_fan_linked(next_linked);
            if next_linked {
                let tab = ui.get_active_fan_tab();
                if tab == 0 {
                    let c: Vec<i32> = ui.get_cpu_curve().iter().collect();
                    ui.set_gpu_curve(slint::ModelRc::new(slint::VecModel::from(c.clone())));
                    ipc::send_action_to_host("tuning_set_fan_curve", serde_json::json!({
                        "cpu_curve": c.clone(),
                        "gpu_curve": c,
                    }));
                } else {
                    let g: Vec<i32> = ui.get_gpu_curve().iter().collect();
                    ui.set_cpu_curve(slint::ModelRc::new(slint::VecModel::from(g.clone())));
                    ipc::send_action_to_host("tuning_set_fan_curve", serde_json::json!({
                        "cpu_curve": g.clone(),
                        "gpu_curve": g,
                    }));
                }
            }
        }
    });
    let ui_tab = ui_weak.clone();
    window.on_tuning_select_fan_tab(move |tab| {
        ipc::send_action_to_host("tuning_select_fan_tab", serde_json::json!({ "tab": tab }));
        if let Some(ui) = ui_tab.upgrade() {
            ui.set_active_fan_tab(tab);
        }
    });

    let cpu_curve_model = std::rc::Rc::new(slint::VecModel::from(vec![20, 30, 45, 55, 65, 75, 85, 100]));
    let gpu_curve_model = std::rc::Rc::new(slint::VecModel::from(vec![20, 30, 45, 55, 65, 75, 85, 100]));
    window.set_cpu_curve(cpu_curve_model.clone().into());
    window.set_gpu_curve(gpu_curve_model.clone().into());

    let fan_debounce_timer = std::rc::Rc::new(std::cell::RefCell::new(slint::Timer::default()));

    let fan_timer_adj = fan_debounce_timer.clone();
    let ui_fan = ui_weak.clone();
    window.on_tuning_adjust_point(move |idx, delta| {
        if let Some(ui) = ui_fan.upgrade() {
            let active_tab = ui.get_active_fan_tab();
            let is_linked = ui.get_fan_linked();
            let min_duty = ui.get_fan_min_duty();

            let mut cpu_pts: Vec<i32> = ui.get_cpu_curve().iter().collect();
            let mut gpu_pts: Vec<i32> = ui.get_gpu_curve().iter().collect();
            let i = idx as usize;

            if active_tab == 0 || is_linked {
                if i < cpu_pts.len() {
                    let cur = cpu_pts[i];
                    apply_monotonic_fan_rule(&mut cpu_pts, i, cur + delta, min_duty);
                }
            }
            if active_tab == 1 || is_linked {
                if i < gpu_pts.len() {
                    let cur = gpu_pts[i];
                    apply_monotonic_fan_rule(&mut gpu_pts, i, cur + delta, min_duty);
                }
            }
            if is_linked {
                if active_tab == 0 {
                    gpu_pts = cpu_pts.clone();
                } else {
                    cpu_pts = gpu_pts.clone();
                }
            }

            ui.set_cpu_curve(slint::ModelRc::new(slint::VecModel::from(cpu_pts.clone())));
            ui.set_gpu_curve(slint::ModelRc::new(slint::VecModel::from(gpu_pts.clone())));

            let timer = fan_timer_adj.clone();
            timer.borrow_mut().stop();
            let c_send = cpu_pts.clone();
            let g_send = gpu_pts.clone();
            timer.borrow_mut().start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(150),
                move || {
                    ipc::send_action_to_host("tuning_set_fan_curve", serde_json::json!({
                        "cpu_curve": c_send,
                        "gpu_curve": g_send,
                    }));
                },
            );
        }
    });

    let fan_timer_set = fan_debounce_timer.clone();
    let ui_fan_set = ui_weak.clone();
    window.on_tuning_set_point(move |idx, duty| {
        if let Some(ui) = ui_fan_set.upgrade() {
            let active_tab = ui.get_active_fan_tab();
            let is_linked = ui.get_fan_linked();
            let min_duty = ui.get_fan_min_duty();

            let mut cpu_pts: Vec<i32> = ui.get_cpu_curve().iter().collect();
            let mut gpu_pts: Vec<i32> = ui.get_gpu_curve().iter().collect();
            let i = idx as usize;

            if active_tab == 0 || is_linked {
                if i < cpu_pts.len() {
                    apply_monotonic_fan_rule(&mut cpu_pts, i, duty, min_duty);
                }
            }
            if active_tab == 1 || is_linked {
                if i < gpu_pts.len() {
                    apply_monotonic_fan_rule(&mut gpu_pts, i, duty, min_duty);
                }
            }
            if is_linked {
                if active_tab == 0 {
                    gpu_pts = cpu_pts.clone();
                } else {
                    cpu_pts = gpu_pts.clone();
                }
            }

            ui.set_cpu_curve(slint::ModelRc::new(slint::VecModel::from(cpu_pts.clone())));
            ui.set_gpu_curve(slint::ModelRc::new(slint::VecModel::from(gpu_pts.clone())));

            let timer = fan_timer_set.clone();
            timer.borrow_mut().stop();
            let c_send = cpu_pts.clone();
            let g_send = gpu_pts.clone();
            timer.borrow_mut().start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(150),
                move || {
                    ipc::send_action_to_host("tuning_set_fan_curve", serde_json::json!({
                        "cpu_curve": c_send,
                        "gpu_curve": g_send,
                    }));
                },
            );
        }
    });

    window.on_tuning_toggle_bld_boost(move || {
        ipc::send_action_to_host("tuning_toggle_bld_boost", serde_json::json!({}));
    });
    window.on_tuning_change_bld_cap(move |rpm| {
        ipc::send_action_to_host("tuning_change_bld_cap", serde_json::json!({ "rpm": rpm }));
    });
    window.on_tuning_restore_bld_auto(move || {
        ipc::send_action_to_host("tuning_restore_bld_auto", serde_json::json!({}));
    });
}

fn update_coupled_pl(ui: &MainWindow, kind: &str, raw: i32) -> (i32, i32, i32) {
    let min_pl = ui.get_min_pl();
    let margin = ui.get_pl4_margin().max(1);
    let min_pl4 = min_pl + margin;
    let p1_max = ui.get_pl1_max();
    let p2_max = ui.get_pl2_max();
    let p4_max = ui.get_pl4_max();

    let mut p1 = ui.get_pl1();
    let mut p2 = ui.get_pl2();
    let mut p4 = ui.get_pl4();

    match kind {
        "pl1" => {
            p1 = raw.clamp(min_pl, p1_max);
            p2 = p2.max(p1).clamp(min_pl, p2_max);
            p4 = p4.max(p2 + margin).clamp(min_pl4, p4_max);
            p2 = p2.min((p4 - margin).max(min_pl));
            p1 = p1.min(p2);
        }
        "pl2" => {
            p2 = raw.clamp(min_pl, p2_max);
            p1 = p1.min(p2).clamp(min_pl, p1_max);
            p4 = p4.max(p2 + margin).clamp(min_pl4, p4_max);
            p2 = p2.min((p4 - margin).max(min_pl));
            p1 = p1.min(p2);
        }
        _ => {
            p4 = raw.clamp(min_pl4, p4_max);
            let max_allowed_pl2 = (p4 - margin).max(min_pl);
            p2 = p2.min(max_allowed_pl2).clamp(min_pl, p2_max);
            p1 = p1.min(p2).clamp(min_pl, p1_max);
        }
    }

    ui.set_pl1(p1);
    ui.set_pl2(p2);
    ui.set_pl4(p4);
    (p1, p2, p4)
}

