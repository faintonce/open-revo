use std::io::{BufRead, Write};
use crate::MainWindow;
use slint::ComponentHandle;

pub fn start_ipc_listener(ui_weak: slint::Weak<MainWindow>) {
    std::thread::Builder::new()
        .name("openrevo-main-stdin-listener".into())
        .spawn(move || {
            let stdin = std::io::stdin();
            for line_res in stdin.lock().lines() {
                match line_res {
                    Ok(line) => {
                        let line = line.trim();
                        if !line.is_empty() {
                            handle_ipc_message(line, &ui_weak);
                        }
                    }
                    Err(e) => {
                        eprintln!("[OpenRevo Slint] IPC stdin error / EOF: {}", e);
                        break;
                    }
                }
            }
            eprintln!("[OpenRevo Slint] Host process terminated, exiting");
            std::process::exit(0);
        })
        .expect("Failed to spawn Main Slint stdin listener");
}

fn handle_ipc_message(line: &str, ui_weak: &slint::Weak<MainWindow>) {
    let val: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return,
    };

    let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("").to_string();
    let ui_weak = ui_weak.clone();

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_weak.upgrade() {
            match action.as_str() {
                "sync_state" => {
                    if let Some(mode) = val.get("power_mode").and_then(|v| v.as_i64()) {
                        ui.set_power_mode(mode as i32);
                    }
                    if let Some(gm) = val.get("gpu_mode").and_then(|v| v.as_str()) {
                        ui.set_gpu_mode(gm.into());
                    }
                    if let Some(hz) = val.get("refresh_rate").and_then(|v| v.as_i64()) {
                        ui.set_refresh_rate(hz as i32);
                    }
                    if let Some(amh) = val.get("auto_min_hz").and_then(|v| v.as_bool()) {
                        ui.set_auto_min_hz(amh);
                    }
                    if let Some(br) = val.get("brightness").and_then(|v| v.as_i64()) {
                        ui.set_brightness(br as i32);
                    }
                    if let Some(eff) = val.get("kbd_effect").and_then(|v| v.as_str()) {
                        ui.set_kbd_effect(eff.into());
                    }
                    if let Some(bl) = val.get("battery_limit").and_then(|v| v.as_i64()) {
                        ui.set_battery_limit(bl as i32);
                    }
                    if let Some(ac) = val.get("is_ac_online").and_then(|v| v.as_bool()) {
                        ui.set_is_ac_online(ac);
                    }
                    if let Some(fb) = val.get("fan_boost").and_then(|v| v.as_bool()) {
                        ui.set_fan_boost(fb);
                    }
                    if let Some(bp) = val.get("battery_percent").and_then(|v| v.as_i64()) {
                        ui.set_battery_percent(bp as i32);
                    }
                    if let Some(ac) = val.get("is_ac_online").and_then(|v| v.as_bool()) {
                        ui.set_is_ac_online(ac);
                    }
                    if let Some(hz) = val.get("refresh_rate").and_then(|v| v.as_i64()) {
                        ui.set_refresh_rate(hz as i32);
                    }
                    if let Some(mon) = val.get("monitor_name").and_then(|v| v.as_str()) {
                        ui.set_monitor_name(mon.into());
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
                    if let Some(cam) = val.get("camera_enabled").and_then(|v| v.as_bool()) {
                        ui.set_camera_enabled(cam);
                    }
                    if let Some(mic) = val.get("mic_enabled").and_then(|v| v.as_bool()) {
                        ui.set_mic_enabled(mic);
                    }
                    if let Some(wl) = val.get("win_locked").and_then(|v| v.as_bool()) {
                        ui.set_win_locked(wl);
                    }
                    if let Some(ka) = val.get("keep_awake").and_then(|v| v.as_bool()) {
                        ui.set_keep_awake(ka);
                    }
                    if let Some(as_en) = val.get("autostart").and_then(|v| v.as_bool()) {
                        ui.set_autostart(as_en);
                    }
                    if let Some(tl) = val.get("touchpad_locked").and_then(|v| v.as_bool()) {
                        ui.set_touchpad_locked(tl);
                    }
                    if let Some(osd) = val.get("osd_enabled").and_then(|v| v.as_bool()) {
                        ui.set_osd_enabled(osd);
                    }
                    if let Some(fn_l) = val.get("fn_locked").and_then(|v| v.as_bool()) {
                        ui.set_fn_locked(fn_l);
                    }
                    if let Some(ver) = val.get("app_version").and_then(|v| v.as_str()) {
                        ui.set_app_version(ver.into());
                    }
                    if let Some(wifi) = val.get("wifi_enabled").and_then(|v| v.as_bool()) {
                        ui.set_wifi_enabled(wifi);
                    }
                    if let Some(bt) = val.get("bt_enabled").and_then(|v| v.as_bool()) {
                        ui.set_bt_enabled(bt);
                    }
                    if let Some(mon) = val.get("monitor_name").and_then(|v| v.as_str()) {
                        ui.set_monitor_name(mon.into());
                    }
                    if let Some(text) = val.get("custom_status_text").or_else(|| val.get("model_name")).and_then(|v| v.as_str()) {
                        ui.set_custom_status_text(text.into());
                    }
                    if let Some(en) = val.get("tuning_enabled").and_then(|v| v.as_bool()) {
                        ui.set_tuning_enabled(en);
                    }
                    if let Some(prof_name) = val.get("active_color_profile").and_then(|v| v.as_str()) {
                        ui.set_active_color_profile(prof_name.into());
                    }
                    if let Some(bp) = val.get("battery_percent").and_then(|v| v.as_i64()) {
                        ui.set_battery_percent(bp as i32);
                    }
                    if let Some(bm) = val.get("battery_mode").and_then(|v| v.as_str()) {
                        ui.set_battery_mode(bm.into());
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
                }
                "sync_power_settings" => {
                    if let Some(bp) = val.get("battery_percent").and_then(|v| v.as_i64()) {
                        ui.set_battery_percent(bp as i32);
                    }
                    if let Some(ac) = val.get("is_ac_online").and_then(|v| v.as_bool()) {
                        ui.set_is_ac_online(ac);
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
                }
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
                    if let Some(hz) = val.get("refresh_rate").and_then(|v| v.as_i64()) {
                        ui.set_refresh_rate(hz as i32);
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
                }
                "telemetry" => {
                    if let Some(ct) = val.get("cpu_temp").and_then(|v| v.as_i64()) {
                        ui.set_cpu_temp(ct as i32);
                    }
                    if let Some(cm) = val.get("cpu_mhz").and_then(|v| v.as_i64()) {
                        ui.set_cpu_mhz(cm as i32);
                    }
                    if let Some(cw) = val.get("cpu_watts_text").and_then(|v| v.as_str()) {
                        ui.set_cpu_watts_text(cw.into());
                    }
                    if let Some(cu) = val.get("cpu_usage").and_then(|v| v.as_i64()) {
                        ui.set_cpu_usage(cu as i32);
                    }
                    if let Some(cn) = val.get("cpu_name").and_then(|v| v.as_str()) {
                        ui.set_cpu_name(cn.into());
                    }

                    if let Some(gt) = val.get("gpu_temp").and_then(|v| v.as_i64()) {
                        ui.set_gpu_temp(gt as i32);
                    }
                    if let Some(gm) = val.get("gpu_mhz").and_then(|v| v.as_i64()) {
                        ui.set_gpu_mhz(gm as i32);
                    }
                    if let Some(gw) = val.get("gpu_watts_text").and_then(|v| v.as_str()) {
                        ui.set_gpu_watts_text(gw.into());
                    }
                    if let Some(gu) = val.get("gpu_usage").and_then(|v| v.as_i64()) {
                        ui.set_gpu_usage(gu as i32);
                    }
                    if let Some(gn) = val.get("gpu_name").and_then(|v| v.as_str()) {
                        ui.set_gpu_name(gn.into());
                    }

                    if let Some(cr) = val.get("cpu_rpm").and_then(|v| v.as_i64()) {
                        ui.set_cpu_rpm(cr as i32);
                    }
                    if let Some(gr) = val.get("gpu_rpm").and_then(|v| v.as_i64()) {
                        ui.set_gpu_rpm(gr as i32);
                    }

                    if let Some(vt) = val.get("vram_text").and_then(|v| v.as_str()) {
                        ui.set_vram_text(vt.into());
                    }
                    if let Some(vm) = val.get("vram_mhz").and_then(|v| v.as_i64()) {
                        ui.set_vram_mhz(vm as i32);
                    }
                    if let Some(vu) = val.get("vram_usage").and_then(|v| v.as_i64()) {
                        ui.set_vram_usage(vu as i32);
                    }
                    if let Some(pm) = val.get("power_mode").and_then(|v| v.as_i64()) {
                        ui.set_power_mode(pm as i32);
                    }
                    if let Some(fb) = val.get("fan_boost").and_then(|v| v.as_bool()) {
                        ui.set_fan_boost(fb);
                    }
                    if let Some(bp) = val.get("battery_percent").and_then(|v| v.as_i64()) {
                        ui.set_battery_percent(bp as i32);
                    }
                    if let Some(ac) = val.get("is_ac_online").and_then(|v| v.as_bool()) {
                        ui.set_is_ac_online(ac);
                    }
                    if let Some(hz) = val.get("refresh_rate").and_then(|v| v.as_i64()) {
                        ui.set_refresh_rate(hz as i32);
                    }
                    if let Some(mon) = val.get("monitor_name").and_then(|v| v.as_str()) {
                        ui.set_monitor_name(mon.into());
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
                }
                "show" => {
                    let _ = ui.show();
                }
                "hide" => {
                    let _ = ui.hide();
                }
                "open_tuning_view" => {
                    ui.set_is_in_tuning_view(true);
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
                }
                "sync_cloud_icc_searching" => {
                    if let Some(s) = val.get("is_searching").and_then(|v| v.as_bool()) {
                        ui.set_cloud_icc_is_searching(s);
                    }
                    if let Some(st) = val.get("status").and_then(|v| v.as_str()) {
                        ui.set_cloud_icc_status(st.into());
                    }
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
                }
                "cloud_icc_toast" => {
                    if let Some(t) = val.get("toast").and_then(|v| v.as_str()) {
                        ui.set_cloud_icc_toast(t.into());
                    }
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
                }
                _ => {}
            }
        }
    });
}

pub fn send_action_to_host(action: &str, payload: serde_json::Value) {
    let mut obj = payload;
    if let Some(m) = obj.as_object_mut() {
        m.insert("action".into(), serde_json::Value::String(action.into()));
    }
    let line = format!("{}\n", obj);
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.flush();
}
