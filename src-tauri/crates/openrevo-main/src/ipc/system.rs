use crate::{MainWindow, UpdateResultData};

pub fn handle(action: &str, val: &serde_json::Value, ui: &MainWindow) -> bool {
    match action {
        "sync_state" => {
            if let Some(mode) = val.get("power_mode").and_then(|v| v.as_i64()) {
                ui.set_power_mode(mode as i32);
            }
            if let Some(pl1) = val.get("pl1_target").and_then(|v| v.as_i64()) {
                ui.set_pl1_target(pl1 as i32);
            }
            if let Some(tgp) = val.get("tgp_target").and_then(|v| v.as_i64()) {
                ui.set_tgp_target(tgp as i32);
            }
            if let Some(txt) = val.get("power_target_text").and_then(|v| v.as_str()) {
                ui.set_power_target_text(txt.into());
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
            if let Some(dc) = val.get("is_dc_online").and_then(|v| v.as_bool()) {
                ui.set_is_dc_online(dc);
            }
            if let Some(tl) = val.get("is_thin_and_light").and_then(|v| v.as_bool()) {
                ui.set_is_thin_and_light(tl);
            }
            if let Some(fb) = val.get("fan_boost").and_then(|v| v.as_bool()) {
                ui.set_fan_boost(fb);
            }
            if let Some(bp) = val.get("battery_percent").and_then(|v| v.as_i64()) {
                ui.set_battery_percent(bp as i32);
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
            if let Some(h) = val.get("has_secondary_monitor").and_then(|v| v.as_bool()) {
                ui.set_has_secondary_monitor(h);
            }
            if let Some(p) = val.get("primary_monitor_name").and_then(|v| v.as_str()) {
                ui.set_primary_monitor_name(p.into());
            }
            if let Some(s) = val.get("secondary_monitor_name").and_then(|v| v.as_str()) {
                ui.set_secondary_monitor_name(s.into());
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
                let mut cfg = ui.get_osd_config();
                cfg.enabled = osd;
                ui.set_osd_config(cfg);
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
            if let Some(text) = val.get("custom_status_text").or_else(|| val.get("model_name")).and_then(|v| v.as_str()) {
                ui.set_custom_status_text(text.into());
            }
            if let Some(toggles) = val.get("module_toggles").and_then(|v| v.as_object()) {
                if let Some(en) = toggles.get("overview").and_then(|v| v.as_bool()) {
                    ui.set_is_overview_enabled(en);
                }
                if let Some(en) = toggles.get("gpu").and_then(|v| v.as_bool()) {
                    ui.set_is_gpu_enabled(en);
                }
                if let Some(en) = toggles.get("tuning").and_then(|v| v.as_bool()) {
                    ui.set_is_tuning_enabled(en);
                }
                if let Some(en) = toggles.get("lighting").and_then(|v| v.as_bool()) {
                    ui.set_is_lighting_enabled(en);
                }
                if let Some(en) = toggles.get("water").and_then(|v| v.as_bool()) {
                    ui.set_is_water_enabled(en);
                }
            }
            if let Some(en) = val.get("tuning_enabled").and_then(|v| v.as_bool()) {
                ui.set_tuning_enabled(en);
            }
            if let Some(prof_name) = val.get("active_color_profile").and_then(|v| v.as_str()) {
                ui.set_active_color_profile(prof_name.into());
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
            if let Some(exp) = val.get("is_expanded").and_then(|v| v.as_bool()) {
                ui.set_is_expanded(exp);
                if let Some(h) = *crate::window::HWND_HOLDER.lock().unwrap() {
                    let hwnd = h.to_hwnd();
                    if exp {
                        let saved = *crate::window::SAVED_FULL_POS.lock().unwrap();
                        crate::window::expand_native_window(hwnd, 1120, 760, saved);
                    } else {
                        crate::window::align_mini_drawer_to_tray(hwnd, 395, 632);
                    }
                }
            }
            if let Some(mah) = val.get("mini_auto_hide").and_then(|v| v.as_bool()) {
                ui.set_mini_auto_hide(mah);
            }
            if let Some(cw) = val.get("use_classic_webview").and_then(|v| v.as_bool()) {
                ui.set_use_classic_webview(cw);
            }
            if let Some(ch) = val.get("update_channel").and_then(|v| v.as_str()) {
                ui.set_update_channel(ch.into());
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
        "show" => {
            if let Some(exp) = val.get("is_expanded").and_then(|v| v.as_bool()) {
                ui.set_is_expanded(exp);
            }
            let mut hwnd_opt = *crate::window::HWND_HOLDER.lock().unwrap();
            if hwnd_opt.is_none() {
                if let Some(h) = crate::window::find_thread_hwnd() {
                    let safe = crate::window::SafeHwnd(h.0 as isize);
                    *crate::window::HWND_HOLDER.lock().unwrap() = Some(safe);
                    hwnd_opt = Some(safe);
                }
            }
            if let Some(h) = hwnd_opt {
                let hwnd = h.to_hwnd();
                if ui.get_is_expanded() {
                    let saved = *crate::window::SAVED_FULL_POS.lock().unwrap();
                    crate::window::expand_native_window(hwnd, 1120, 760, saved);
                } else {
                    crate::window::align_mini_drawer_to_tray(hwnd, 395, 632);
                }
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
                    };
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                    crate::window::trigger_full_redraw(hwnd);
                }
            }
            true
        }
        "hide" | "quit" | "shutdown" | "exit" => {
            if let Some(h) = *crate::window::HWND_HOLDER.lock().unwrap() {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
                    let _ = ShowWindow(h.to_hwnd(), SW_HIDE);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
            std::process::exit(0);
        }
        "toggle" => {
            let mut hwnd_opt = *crate::window::HWND_HOLDER.lock().unwrap();
            if hwnd_opt.is_none() {
                if let Some(h) = crate::window::find_thread_hwnd() {
                    let safe = crate::window::SafeHwnd(h.0 as isize);
                    *crate::window::HWND_HOLDER.lock().unwrap() = Some(safe);
                    hwnd_opt = Some(safe);
                }
            }
            if let Some(h) = hwnd_opt {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_HIDE,
                        SW_RESTORE, SW_SHOW,
                    };
                    let hwnd = h.to_hwnd();
                    if IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                        std::thread::sleep(std::time::Duration::from_millis(30));
                        std::process::exit(0);
                    } else {
                        if ui.get_is_expanded() {
                            let saved = *crate::window::SAVED_FULL_POS.lock().unwrap();
                            crate::window::expand_native_window(hwnd, 1120, 760, saved);
                        } else {
                            crate::window::align_mini_drawer_to_tray(hwnd, 395, 632);
                        }
                        let _ = ShowWindow(hwnd, SW_RESTORE);
                        let _ = ShowWindow(hwnd, SW_SHOW);
                        let _ = SetForegroundWindow(hwnd);
                        crate::window::trigger_full_redraw(hwnd);
                    }
                }
            }
            true
        }
        "expand_dashboard" => {
            ui.set_is_expanded(true);
            if let Some(h) = *crate::window::HWND_HOLDER.lock().unwrap() {
                let saved = *crate::window::SAVED_FULL_POS.lock().unwrap();
                crate::window::expand_native_window(h.to_hwnd(), 1120, 760, saved);
            }
            true
        }
        "collapse_dashboard" => {
            ui.set_is_expanded(false);
            if let Some(h) = *crate::window::HWND_HOLDER.lock().unwrap() {
                crate::window::align_mini_drawer_to_tray(h.to_hwnd(), 395, 632);
            }
            true
        }
        "trigger_full_redraw" => {
            if let Some(h) = *crate::window::HWND_HOLDER.lock().unwrap() {
                crate::window::trigger_full_redraw(h.to_hwnd());
            }
            true
        }
        "sync_custom_plugins" => {
            if let Some(arr) = val.get("plugins").and_then(|v| v.as_array()) {
                let mut list = Vec::new();
                let mut tab_items = Vec::new();
                for p in arr {
                    let id = p.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let version = p.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let author = p.get("author").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let description = p.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let plugin_type = p.get("plugin_type").and_then(|v| v.as_str()).unwrap_or("service").to_string();
                    let enabled = p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                    let has_tab = p.get("has_tab").and_then(|v| v.as_bool()).unwrap_or(false)
                        || plugin_type == "tab"
                        || plugin_type == "app";
                    let tag = p.get("tag").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let min_version = p.get("min_version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let dir_name = p.get("dir_name").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    if enabled && has_tab {
                        tab_items.push(crate::PluginItemData {
                            id: id.clone().into(),
                            name: name.clone().into(),
                        });
                    }

                    list.push(crate::CustomPluginItem {
                        id: id.into(),
                        name: name.into(),
                        version: version.into(),
                        author: author.into(),
                        description: description.into(),
                        plugin_type: plugin_type.into(),
                        enabled,
                        tag: tag.into(),
                        min_version: min_version.into(),
                        dir_name: dir_name.into(),
                    });
                }
                ui.set_plugin_items(slint::ModelRc::new(slint::VecModel::from(tab_items)));
                ui.set_custom_plugins(slint::ModelRc::new(slint::VecModel::from(list)));
            }
            true
        }
        "sync_update_channel" => {
            if let Some(ch) = val.get("channel").and_then(|v| v.as_str()) {
                ui.set_update_channel(ch.into());
                ui.set_update_feedback("".into());
                ui.set_update_feedback_type("".into());
            }
            true
        }
        "sync_update_result" => {
            ui.set_is_checking_update(false);
            if let Some(msg) = val.get("message").and_then(|v| v.as_str()) {
                ui.set_update_feedback(msg.into());
            }
            if let Some(st) = val.get("status").and_then(|v| v.as_str()) {
                ui.set_update_feedback_type(st.into());
            }
            if let Some(up) = val.get("update").and_then(|v| v.as_object()) {
                let has_update = up.get("has_update").and_then(|v| v.as_bool()).unwrap_or(false);
                let cur_ver = up.get("current_version").and_then(|v| v.as_str()).unwrap_or_default();
                let lat_ver = up.get("latest_version").and_then(|v| v.as_str()).unwrap_or_default();
                let disp_ver = up.get("display_version").and_then(|v| v.as_str()).unwrap_or(lat_ver);
                let notes = up.get("notes").and_then(|v| v.as_str()).unwrap_or_default();
                let src = up.get("chosen_source").and_then(|v| v.as_str()).unwrap_or("github");
                let asset_type = up.get("asset_type").and_then(|v| v.as_str()).unwrap_or("installer");
                let fname = up.get("filename").and_then(|v| v.as_str()).unwrap_or_default();
                let durl = up.get("download_url").and_then(|v| v.as_str()).unwrap_or_default();
                let size_mb = up.get("size_mb").and_then(|v| v.as_str()).unwrap_or_default();
                let sha = up.get("sha256").and_then(|v| v.as_str()).unwrap_or_default();

                let res = UpdateResultData {
                    has_update,
                    current_version: cur_ver.into(),
                    latest_version: lat_ver.into(),
                    display_version: disp_ver.into(),
                    notes: notes.into(),
                    chosen_source: src.into(),
                    asset_type: asset_type.into(),
                    filename: fname.into(),
                    download_url: durl.into(),
                    size_mb: size_mb.into(),
                    sha256: sha.into(),
                    downloading: false,
                    progress_pct: 0.0,
                    progress_downloaded_mb: "0.00".into(),
                    progress_total_mb: size_mb.into(),
                    progress_speed_mb: "0.00".into(),
                };
                ui.set_update_data(res);
                if has_update {
                    ui.set_show_update_modal(true);
                }
            }
            true
        }
        "sync_update_progress" => {
            let mut d = ui.get_update_data();
            d.downloading = true;
            if let Some(pct) = val.get("percentage").and_then(|v| v.as_f64()) {
                d.progress_pct = (pct / 100.0) as f32;
            }
            if let Some(spd) = val.get("speed_mb").and_then(|v| v.as_str()) {
                d.progress_speed_mb = spd.into();
            }
            if let Some(dn) = val.get("downloaded_mb").and_then(|v| v.as_str()) {
                d.progress_downloaded_mb = dn.into();
            }
            if let Some(tot) = val.get("total_mb").and_then(|v| v.as_str()) {
                d.progress_total_mb = tot.into();
            }
            ui.set_update_data(d);
            true
        }
        "set_active_plugin_id" => {
            let pid = val.get("plugin_id").and_then(|v| v.as_str()).unwrap_or("");
            ui.set_active_plugin_id(pid.into());
            if pid.is_empty() {
                ui.set_active_tab(5);
            }
            true
        }
        _ => false,
    }
}

