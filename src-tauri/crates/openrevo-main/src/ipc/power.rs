use crate::{MainWindow, PowerSchemeItem};

pub fn handle(action: &str, val: &serde_json::Value, ui: &MainWindow) -> bool {
    match action {
        "sync_power_settings" => {
            if let Some(bp) = val.get("battery_percent").and_then(|v| v.as_i64()) {
                ui.set_battery_percent(bp as i32);
            }
            if let Some(ac) = val.get("is_ac_online").and_then(|v| v.as_bool()) {
                ui.set_is_ac_online(ac);
            }
            if let Some(dc) = val.get("is_dc_online").and_then(|v| v.as_bool()) {
                ui.set_is_dc_online(dc);
            }
            if let Some(tl) = val.get("is_thin_and_light").and_then(|v| v.as_bool()) {
                ui.set_is_thin_and_light(tl);
            }
            if let Some(bm) = val.get("battery_mode").and_then(|v| v.as_str()) {
                ui.set_battery_mode(bm.into());
            }
            if let Some(bl) = val.get("battery_limit").and_then(|v| v.as_i64()) {
                ui.set_battery_limit(bl as i32);
            }
            if let Some(uc) = val.get("usb_charge_enabled").and_then(|v| v.as_bool()) {
                ui.set_usb_charge_enabled(uc);
            }
            if let Some(ar) = val.get("ac_recovery_enabled").and_then(|v| v.as_bool()) {
                ui.set_ac_recovery_enabled(ar);
            }
            if let Some(hp) = val.get("high_perf_scheme").and_then(|v| v.as_bool()) {
                ui.set_high_perf_scheme(hp);
            }
            if let Some(sg) = val.get("sleep_guard_enabled").and_then(|v| v.as_bool()) {
                ui.set_sleep_guard_enabled(sg);
            }
            if let Some(sdn) = val.get("sleep_disconnect_net").and_then(|v| v.as_bool()) {
                ui.set_sleep_disconnect_net(sdn);
            }
            if let Some(sba) = val.get("sleep_block_away").and_then(|v| v.as_bool()) {
                ui.set_sleep_block_away(sba);
            }
            if let Some(sis) = val.get("sleep_ignore_sysreq").and_then(|v| v.as_bool()) {
                ui.set_sleep_ignore_sysreq(sis);
            }
            if let Some(sdw) = val.get("sleep_disable_wake_timers").and_then(|v| v.as_bool()) {
                ui.set_sleep_disable_wake_timers(sdw);
            }
            if let Some(sds) = val.get("sleep_deep_sleep").and_then(|v| v.as_bool()) {
                ui.set_sleep_deep_sleep(sds);
            }
            if let Some(sh) = val.get("sleep_hibernate_30m").and_then(|v| v.as_bool()) {
                ui.set_sleep_hibernate_30m(sh);
            }
            if let Some(sao) = val.get("sleep_auto_office").and_then(|v| v.as_bool()) {
                ui.set_sleep_auto_office(sao);
            }
            if let Some(en) = val.get("balanced_db_enabled").and_then(|v| v.as_bool()) {
                ui.set_balanced_db_enabled(en);
            }
            if let Some(core) = val.get("oc_core_mhz").and_then(|v| v.as_i64()) {
                ui.set_oc_core_mhz(core as i32);
            }
            if let Some(mem) = val.get("oc_mem_mhz").and_then(|v| v.as_i64()) {
                ui.set_oc_mem_mhz(mem as i32);
            }
            if let Some(ubl) = val.get("unlock_battery_limit").and_then(|v| v.as_bool()) {
                ui.set_unlock_battery_limit(ubl);
            }
            if let Some(wm) = val.get("wake_window_mode").and_then(|v| v.as_str()) {
                ui.set_wake_window_mode(wm.into());
            }
            if let Some(tm) = val.get("theme_mode").and_then(|v| v.as_i64()) {
                ui.set_theme_mode(tm as i32);
            }
            if let Some(mah) = val.get("mini_auto_hide").and_then(|v| v.as_bool()) {
                ui.set_mini_auto_hide(mah);
            }
            if let Some(cw) = val.get("use_classic_webview").and_then(|v| v.as_bool()) {
                ui.set_use_classic_webview(cw);
            }
            if let (Some(x), Some(y)) = (
                val.get("full_window_x").and_then(|v| v.as_i64()),
                val.get("full_window_y").and_then(|v| v.as_i64()),
            ) {
                *crate::window::SAVED_FULL_POS.lock().unwrap() = Some((x as i32, y as i32));
            }
            if let Some(osd_obj) = val.get("osd_config").and_then(|v| v.as_object()) {
                let mut cfg = ui.get_osd_config();
                if let Some(en) = osd_obj.get("enabled").and_then(|v| v.as_bool()) { cfg.enabled = en; }
                if let Some(th) = osd_obj.get("theme").and_then(|v| v.as_str()) { cfg.theme = th.into(); }
                if let Some(sc) = osd_obj.get("scale").and_then(|v| v.as_str()) { cfg.scale = sc.into(); }
                if let Some(pos) = osd_obj.get("position").and_then(|v| v.as_str()) { cfg.position = pos.into(); }
                ui.set_osd_config(cfg);
            }
            if let Some(dd_obj) = val.get("data_dir_info").and_then(|v| v.as_object()) {
                let mut info = ui.get_data_dir_info();
                if let Some(p) = dd_obj.get("path").and_then(|v| v.as_str()) { info.path = p.into(); }
                if let Some(m) = dd_obj.get("mode").and_then(|v| v.as_str()) { info.mode = m.into(); }
                ui.set_data_dir_info(info);
            }
            true
        }
        "sync_power_schemes" => {
            if let Some(arr) = val.get("schemes").and_then(|v| v.as_array()) {
                let mut list = Vec::new();
                let mut active_name = "平衡 (Balanced)".to_string();
                for item in arr {
                    let guid = item.get("guid").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let is_active = item.get("is_active").and_then(|v| v.as_bool()).unwrap_or(false);
                    if is_active {
                        active_name = name.clone();
                    }
                    list.push(PowerSchemeItem {
                        guid: guid.into(),
                        name: name.into(),
                        is_active,
                    });
                }
                ui.set_power_schemes(slint::ModelRc::new(slint::VecModel::from(list)));
                ui.set_active_power_scheme_name(active_name.into());
            }
            if let Some(msg) = val.get("restore_plan_msg").and_then(|v| v.as_str()) {
                ui.set_restore_plan_msg(msg.into());
            }
            true
        }
        _ => false,
    }
}

