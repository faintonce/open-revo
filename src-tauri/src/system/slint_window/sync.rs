use tauri::Manager;
use super::{send_slint_ipc_message, SELECTED_MONITOR_INDEX, SLINT_APP_HANDLE};

pub fn sync_slint_telemetry(state: &crate::commands::system_cmd::HardwareState) {
    let vram_used_gb = (state.gpu_vram_used_mb as f64) / 1024.0;
    let vram_total_gb = (state.gpu_vram_mb as f64) / 1024.0;
    let vram_text = if state.gpu_vram_mb > 0 {
        format!("{:.1} / {:.1} GB", vram_used_gb, vram_total_gb)
    } else {
        "- / -".to_string()
    };

    let cpu_watts_text = format!("{:.1} W", state.cpu_power_w);
    let gpu_watts_text = format!("{:.1} W", state.gpu_power_w);

    let sel_idx = SELECTED_MONITOR_INDEX.load(std::sync::atomic::Ordering::Relaxed);
    let (monitor_name, curr_hz, supported_rates) = if !state.monitors.is_empty() {
        let m = if sel_idx < state.monitors.len() { &state.monitors[sel_idx] } else { &state.monitors[0] };
        (m.friendly_name.clone(), m.current_hz, m.supported_rates.clone())
    } else {
        ("京东方 (主屏)".to_string(), state.refresh_rate, vec![60, 144, 240])
    };

    let (rates_row1, rates_row2) = if supported_rates.len() > 5 {
        let mid = (supported_rates.len() + 1) / 2;
        (supported_rates[..mid].to_vec(), supported_rates[mid..].to_vec())
    } else {
        (supported_rates.clone(), Vec::new())
    };

    let (live_battery, live_ac, _) = crate::commands::power_cmd::get_live_power_status();
    let has_secondary_monitor = state.monitors.len() > 1;
    let primary_monitor_name = if !state.monitors.is_empty() { state.monitors[0].friendly_name.clone() } else { "内置主屏".to_string() };
    let secondary_monitor_name = if state.monitors.len() > 1 { state.monitors[1].friendly_name.clone() } else { "".to_string() };

    let (pl1_target, tgp_target, power_target_text) = get_current_power_targets(state.power_mode);

    send_slint_ipc_message(serde_json::json!({
        "action": "telemetry",
        "battery_percent": live_battery,
        "is_ac_online": live_ac,
        "cpu_temp": state.cpu_temp,
        "cpu_mhz": state.cpu_freq,
        "cpu_watts_text": cpu_watts_text,
        "cpu_usage": state.cpu_usage,
        "cpu_name": state.cpu_name,
        "gpu_temp": state.gpu_temp,
        "gpu_mhz": state.gpu_core_freq,
        "gpu_watts_text": gpu_watts_text,
        "gpu_usage": state.gpu_usage,
        "gpu_name": state.gpu_name,
        "cpu_rpm": state.cpu_fan_rpm,
        "gpu_rpm": state.gpu_fan_rpm,
        "vram_text": vram_text,
        "vram_mhz": state.gpu_mem_freq,
        "vram_usage": if state.gpu_vram_mb > 0 { ((state.gpu_vram_used_mb as f64 / state.gpu_vram_mb as f64) * 100.0) as u32 } else { 0 },
        "refresh_rate": curr_hz,
        "monitor_name": monitor_name,
        "rates_row1": rates_row1,
        "rates_row2": rates_row2,
        "has_secondary_monitor": has_secondary_monitor,
        "primary_monitor_name": primary_monitor_name,
        "secondary_monitor_name": secondary_monitor_name,
        "power_mode": state.power_mode,
        "pl1_target": pl1_target,
        "tgp_target": tgp_target,
        "power_target_text": power_target_text,
        "fan_boost": state.fan_boost,
        "model_name": &state.model_name,
        "custom_status_text": &state.model_name,
    }));

    sync_slint_water_state(Some(state.cpu_temp as i32), Some(state.gpu_temp as i32));
}

pub fn sync_slint_tuning_state() {
    let cfg = crate::system::config::load_config();
    let manifest = crate::hal::declarative::load_matched_manifest();
    let is_bld = manifest.as_ref().map(|m| m.id.contains("bld")).unwrap_or(false);
    let (_cpu_name, is_amd) = crate::driver::uefi_nvram::get_cpu_info();
    let cpu_vendor = if is_amd { "amd" } else { "intel" };
    let cpu_tuning_type = if manifest.as_ref().and_then(|m| m.capabilities.cpu_tuning_type.as_deref()) == Some("none") {
        "none"
    } else if is_amd {
        "amd_smu"
    } else {
        "intel_pl"
    };
    let cpu_supports_fppt = manifest.as_ref().and_then(|m| m.capabilities.cpu_supports_fppt).unwrap_or(!is_bld);

    let active_id = {
        let cfg = crate::system::config::load_config();
        if !cfg.active_profile_id.is_empty() {
            cfg.active_profile_id
        } else {
            "preset_balance".to_string()
        }
    };
    let presets = crate::commands::tuning_cmd::get_profiles_list().unwrap_or_default();
    let current_preset = presets.iter().find(|p| p.id == active_id).cloned().or_else(|| presets.first().cloned());
    let profile = current_preset.as_ref().map(|p| &p.profile).unwrap_or(&cfg.custom_profile);
    let preset_name = current_preset.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "均衡电竞".to_string());

    let cpu_curve: Vec<i32> = profile.cpu_fan_curve.iter().map(|p| p.duty as i32).collect();
    let gpu_curve: Vec<i32> = profile.gpu_fan_curve.iter().map(|p| p.duty as i32).collect();

    let base_tgp = crate::system::power_mode::get_cached_factory_power_presets().map(|fp| fp.base_tgp).unwrap_or(150);
    let is_clamped = crate::driver::bld_driver_mgr::is_bld_fan_clamped();

    let preset_ids: Vec<String> = presets.iter().map(|p| p.id.clone()).collect();
    let preset_names: Vec<String> = presets.iter().map(|p| p.name.clone()).collect();

    let bios_status = crate::driver::uefi_nvram::read_bios_advanced_menu_status();
    let is_partial = (bios_status.cpu_oc_enabled || bios_status.mem_oc_enabled) && !bios_status.enabled;
    let is_unsupported = bios_status.hardware_read && !bios_status.supported;
    let bios_oc_disabled = is_amd || is_unsupported;

    let bios_oc_desc = if is_amd {
        "AMD 平台采用 SMU 动态功耗架构，无 Intel 格式 BIOS 超频菜单".to_string()
    } else if is_unsupported {
        "当前机型主板或移动标压CPU未开放BIOS超频菜单".to_string()
    } else {
        "同步解锁 CPU 与内存超频菜单".to_string()
    };

    let (bios_oc_btn_text, bios_oc_status) = if is_amd || is_unsupported {
        ("不支持".to_string(), 3)
    } else if bios_status.enabled {
        ("已开启".to_string(), 1)
    } else if is_partial {
        ("部分开启".to_string(), 2)
    } else {
        ("关闭".to_string(), 0)
    };

    let payload = serde_json::json!({
        "action": "sync_tuning_state",
        "cpu_vendor": cpu_vendor,
        "cpu_tuning_type": cpu_tuning_type,
        "cpu_supports_fppt": cpu_supports_fppt,
        "is_bld": is_bld,
        "active_preset_name": preset_name,
        "preset_ids": preset_ids,
        "preset_names": preset_names,
        "temp_wall": profile.cpu_temp_target,
        "temp_wall_enabled": profile.cpu_temp_target_enabled,
        "temp_wall_max": 95,
        "pl1": profile.cpu_pl1_watts,
        "pl1_max": 230,
        "pl2": profile.cpu_pl2_watts,
        "pl2_max": 230,
        "pl4": profile.cpu_pl4_watts,
        "pl4_max": 275,
        "bios_oc_enabled": bios_status.enabled,
        "bios_oc_desc": bios_oc_desc,
        "bios_oc_btn_text": bios_oc_btn_text,
        "bios_oc_status": bios_oc_status,
        "bios_oc_disabled": bios_oc_disabled,
        "super_perf": profile.gpu_super_perf,
        "core_offset": profile.gpu_core_offset_mhz,
        "mem_offset": profile.gpu_mem_offset_mhz,
        "db_enabled": profile.gpu_db_enabled,
        "db_watts": profile.gpu_db_watts,
        "base_tgp": base_tgp,
        "smooth_transition": profile.fan_ramp_rate,
        "smooth_time": format!("{}ms", profile.fan_ramp_speed_ms),
        "fan_isolated_output": profile.fan_isolated_output,
        "fan_linked": profile.fan_curve_linked,
        "cpu_curve": cpu_curve,
        "gpu_curve": gpu_curve,
        "bld_rpm_cap": 5800,
        "bld_boost": is_clamped,
    });

    send_slint_ipc_message(payload);
    sync_slint_power_targets(cfg.power_mode);
}

pub fn sync_slint_display_state() {
    let tuning_state = crate::commands::display_cmd::get_display_tuning_state();
    let calib_state = crate::system::color_calibration::get_color_calibration_state(None);

    let monitors = crate::system::display::get_all_monitors();
    let sel_idx = SELECTED_MONITOR_INDEX.load(std::sync::atomic::Ordering::Relaxed);
    let (monitor_name, curr_hz, supported_rates) = if !monitors.is_empty() {
        let m = if sel_idx < monitors.len() { &monitors[sel_idx] } else { &monitors[0] };
        (m.friendly_name.clone(), m.current_hz, m.supported_rates.clone())
    } else {
        ("京东方 (主屏)".to_string(), 240, vec![60, 144, 240])
    };

    let (rates_row1, rates_row2) = if supported_rates.len() > 4 {
        let mid = (supported_rates.len() + 1) / 2;
        (supported_rates[..mid].to_vec(), supported_rates[mid..].to_vec())
    } else {
        (supported_rates.clone(), Vec::new())
    };
    let has_secondary_monitor = monitors.len() > 1;
    let primary_monitor_name = if !monitors.is_empty() { monitors[0].friendly_name.clone() } else { "内置主屏".to_string() };
    let secondary_monitor_name = if monitors.len() > 1 { monitors[1].friendly_name.clone() } else { "".to_string() };

    let mut profiles_list: Vec<String> = Vec::new();
    let mut profiles_ids: Vec<String> = Vec::new();
    for p in &calib_state.profiles {
        profiles_list.push(p.name.clone());
        profiles_ids.push(p.id.clone());
    }

    send_slint_ipc_message(serde_json::json!({
        "action": "sync_display_state",
        "tuning_enabled": tuning_state.enabled,
        "active_preset_id": tuning_state.active_preset.id,
        "active_preset_name": tuning_state.active_preset.name,
        "tuning_temp_k": tuning_state.active_preset.color_temp_k,
        "tuning_saturation": (tuning_state.active_preset.saturation * 100.0).round() as i64,
        "tuning_contrast": (tuning_state.active_preset.contrast * 100.0).round() as i64,
        "tuning_gamma": (tuning_state.active_preset.gamma * 100.0).round() as i64,
        "tuning_gain_r": (tuning_state.active_preset.gain_r * 100.0).round() as i64,
        "tuning_gain_g": (tuning_state.active_preset.gain_g * 100.0).round() as i64,
        "tuning_gain_b": (tuning_state.active_preset.gain_b * 100.0).round() as i64,
        "tuning_is_modified": false,
        "active_color_profile": calib_state.active_profile_name,
        "active_color_profile_id": calib_state.active_profile_id,
        "color_profiles_list": profiles_list,
        "color_profiles_ids": profiles_ids,
        "monitor_name": monitor_name,
        "refresh_rate": curr_hz,
        "rates_row1": rates_row1,
        "rates_row2": rates_row2,
        "has_secondary_monitor": has_secondary_monitor,
        "primary_monitor_name": primary_monitor_name,
        "secondary_monitor_name": secondary_monitor_name,
    }));
}

pub fn sync_slint_power_state() {
    let cfg = crate::system::config::load_config();
    sync_slint_power_targets(cfg.power_mode);

    let app_opt = SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone());
    if let Some(app) = app_opt {
        let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
        let info = crate::commands::power_cmd::read_power_settings(ec_state.inner());
        send_slint_ipc_message(serde_json::json!({
            "action": "sync_power_settings",
            "battery_percent": info.battery_percent,
            "is_ac_online": info.is_ac_online,
            "is_dc_online": crate::commands::power_cmd::is_dc_round_plugged_in(),
            "is_thin_and_light": crate::commands::power_cmd::is_thin_and_light_or_pure_igpu(),
            "unlock_battery_limit": cfg.unlock_battery_power_limit,
            "battery_mode": info.battery_mode,
            "battery_limit": info.battery_limit,
            "usb_charge_enabled": info.usb_charge_enabled,
            "ac_recovery_enabled": info.ac_recovery_enabled,
            "high_perf_scheme": info.high_perf_scheme,
            "sleep_guard_enabled": info.sleep_guard_enabled,
            "sleep_disconnect_net": info.sleep_disconnect_net,
            "sleep_block_away": info.sleep_block_away,
            "sleep_ignore_sysreq": info.sleep_ignore_sysreq,
            "sleep_disable_wake_timers": info.sleep_disable_wake_timers,
            "sleep_deep_sleep": info.sleep_deep_sleep,
            "sleep_hibernate_30m": info.sleep_hibernate_30m,
            "sleep_auto_office": info.sleep_auto_office,
        }));
    } else {
        let cfg = crate::system::config::load_config();
        let (percent, is_ac, _) = crate::commands::power_cmd::get_live_power_status();
        let mode = if cfg.battery_limit >= 95 {
            "long_life"
        } else if cfg.battery_limit <= 65 {
            "station"
        } else {
            "balance"
        };
        send_slint_ipc_message(serde_json::json!({
            "action": "sync_power_settings",
            "battery_percent": percent,
            "is_ac_online": is_ac,
            "is_dc_online": crate::commands::power_cmd::is_dc_round_plugged_in(),
            "is_thin_and_light": crate::commands::power_cmd::is_thin_and_light_or_pure_igpu(),
            "unlock_battery_limit": cfg.unlock_battery_power_limit,
            "battery_mode": mode,
            "battery_limit": cfg.battery_limit,
            "usb_charge_enabled": cfg.usb_charge_enabled,
            "ac_recovery_enabled": cfg.ac_recovery_enabled,
            "high_perf_scheme": cfg.high_perf_scheme,
            "sleep_guard_enabled": cfg.sleep_guard_enabled,
            "sleep_disconnect_net": cfg.sleep_disconnect_net,
            "sleep_block_away": cfg.sleep_block_away,
            "sleep_ignore_sysreq": cfg.sleep_ignore_sysreq,
            "sleep_disable_wake_timers": cfg.sleep_disable_wake_timers,
            "sleep_deep_sleep": cfg.sleep_deep_sleep,
            "sleep_hibernate_30m": cfg.sleep_hibernate_30m,
            "sleep_auto_office": cfg.sleep_auto_office,
        }));
    }
}

pub fn sync_slint_switches_state() {
    let switches = crate::system::device_switches::get_switches_state();
    let cfg = crate::system::config::load_config();
    let model_alias = crate::system::model_engine::get_active_alias();
    send_slint_ipc_message(serde_json::json!({
        "action": "sync_state",
        "camera_enabled": switches.camera,
        "mic_enabled": switches.mic,
        "win_locked": cfg.win_key_locked,
        "keep_awake": switches.keep_awake,
        "autostart": cfg.autostart,
        "touchpad_locked": !switches.touchpad,
        "osd_enabled": switches.osd,
        "fn_locked": switches.fn_lock,
        "wifi_enabled": switches.wifi,
        "bt_enabled": switches.bluetooth,
        "fan_boost": cfg.fan_boost,
        "power_mode": cfg.power_mode,
        "is_dc_online": crate::commands::power_cmd::is_dc_round_plugged_in(),
        "is_thin_and_light": crate::commands::power_cmd::is_thin_and_light_or_pure_igpu(),
        "unlock_battery_limit": cfg.unlock_battery_power_limit,
        "theme_mode": cfg.theme_mode,
        "model_name": &model_alias,
        "custom_status_text": &model_alias,
    }));
}

pub fn sync_slint_lighting_state() {
    let lm = crate::driver::ite_lighting::get_global_lighting();
    let st = lm.state.lock().clone();
    send_slint_ipc_message(serde_json::json!({
        "action": "sync_lighting_state",
        "sleep_minutes": st.sleep_minutes,
        "ceremony_enabled": st.ceremony_enabled,
        "kb_enabled": st.kb_enabled,
        "kb_effect": if st.kb_engine == "streamer" { st.streamer_effect } else { st.kb_effect },
        "kb_brightness": st.kb_brightness,
        "kb_speed": st.kb_speed,
        "kb_direction": st.kb_direction,
        "kb_color": st.kb_color,
        "kb_engine": st.kb_engine,
        "kb_fps": st.kb_fps,
        "lb_enabled": st.lb_enabled,
        "lb_effect": st.lb_effect,
        "lb_brightness": st.lb_brightness,
        "lb_speed": st.lb_speed,
        "lb_color": st.lb_color,
        "hinge_enabled": st.hinge_enabled,
        "hinge_effect": st.hinge_effect,
        "hinge_brightness": st.hinge_brightness,
        "hinge_speed": st.hinge_speed,
        "hinge_color": st.hinge_color,
        "logo_enabled": st.logo_enabled,
        "logo_effect": st.logo_effect,
        "logo_brightness": st.logo_brightness,
        "logo_color": st.logo_color,
    }));

    let hw = crate::commands::lighting_cmd::get_keyboard_hardware_info_internal(&lm);
    let is_four_zone = hw.is_four_zone || hw.hardware_type == "FourZoneRgb";
    let is_single_zone = hw.hardware_type == "SingleZoneRgb";
    let is_single_color = hw.hardware_type == "SingleColor";
    send_slint_ipc_message(serde_json::json!({
        "action": "sync_lighting_hardware_info",
        "has_front_lightbar": hw.has_front_lightbar || hw.has_ite7001,
        "has_hinge": hw.has_hinge,
        "has_logo": hw.has_logo,
        "is_ec_logo": hw.is_ec_logo,
        "description": hw.description,
        "hardware_type": hw.hardware_type,
        "is_four_zone": is_four_zone,
        "is_single_zone": is_single_zone,
        "is_single_color": is_single_color,
    }));
}

pub fn sync_slint_water_state(opt_cpu: Option<i32>, opt_gpu: Option<i32>) {
    let cooler = crate::driver::ble_cooler::get_global_cooler();
    let tel = cooler.get_telemetry();

    let (curve_temps, curve_duties) = if !tel.curve_points.is_empty() {
        let mut pts = tel.curve_points.clone();
        pts.sort_by_key(|p| p.temp);
        while pts.len() < 6 {
            let last = pts.last().cloned().unwrap();
            pts.push(crate::system::config::CoolerCurvePoint {
                temp: (last.temp + 4).min(100),
                fan_duty: last.fan_duty,
                pump_voltage: last.pump_voltage,
            });
        }
        if pts.len() > 6 {
            pts.truncate(6);
        }
        let t: Vec<i32> = pts.iter().map(|p| p.temp as i32).collect();
        let d: Vec<i32> = pts.iter().map(|p| p.fan_duty as i32).collect();
        (t, d)
    } else {
        (vec![45, 55, 65, 80, 90, 95], vec![0, 35, 50, 60, 75, 100])
    };

    let cpu = opt_cpu.unwrap_or(55);
    let gpu = opt_gpu.unwrap_or(45);
    let max_temp = cpu.max(gpu);

    let live_fan = if tel.strategy == "quiet" {
        40
    } else if tel.strategy == "balance" {
        65
    } else if tel.strategy == "performance" {
        100
    } else if tel.strategy == "custom" {
        tel.fan_duty as i32
    } else {
        // "smart" interpolation
        if max_temp < tel.pump_start_temp as i32 {
            0
        } else if tel.curve_points.is_empty() {
            40
        } else if max_temp <= tel.curve_points[0].temp as i32 {
            tel.curve_points[0].fan_duty as i32
        } else if max_temp >= tel.curve_points.last().unwrap().temp as i32 {
            tel.curve_points.last().unwrap().fan_duty as i32
        } else {
            let mut interp = tel.curve_points[0].fan_duty as i32;
            for i in 0..tel.curve_points.len() - 1 {
                let t1 = tel.curve_points[i].temp as i32;
                let t2 = tel.curve_points[i + 1].temp as i32;
                if max_temp >= t1 && max_temp <= t2 {
                    let d1 = tel.curve_points[i].fan_duty as i32;
                    let d2 = tel.curve_points[i + 1].fan_duty as i32;
                    if t2 > t1 {
                        interp = d1 + (max_temp - t1) * (d2 - d1) / (t2 - t1);
                    } else {
                        interp = d1;
                    }
                    break;
                }
            }
            interp
        }
    };

    send_slint_ipc_message(serde_json::json!({
        "action": "sync_water_cooler_state",
        "connected": tel.connected,
        "is_flow_ok": tel.is_flow_ok,
        "meter_state": tel.meter_state,
        "pump_voltage": tel.pump_voltage,
        "pump_tier": tel.pump_tier,
        "fan_duty": tel.fan_duty,
        "pump_rpm": tel.pump_rpm,
        "fan_rpm": if tel.connected { (live_fan as f64 * 32.0) as i32 } else { 0 },
        "is_priming": tel.is_priming,
        "led_on": tel.led_on,
        "led_r": tel.led_r,
        "led_g": tel.led_g,
        "led_b": tel.led_b,
        "led_mode": tel.led_mode,
        "mac_address": tel.mac_address,
        "strategy": tel.strategy,
        "pump_start_temp": tel.pump_start_temp,
        "pump_turbo_temp": tel.pump_turbo_temp,
        "curve_temps": curve_temps,
        "curve_duties": curve_duties,
        "live_fan_duty": live_fan,
    }));
}

pub fn sync_slint_plugins_state() {
    let toggles = crate::system::config::get_module_toggles();

    // 1. Sync Builtin Module Toggles
    send_slint_ipc_message(serde_json::json!({
        "action": "sync_state",
        "module_toggles": {
            "overview": toggles.get("overview").copied().unwrap_or(true),
            "gpu": toggles.get("gpu").copied().unwrap_or(true),
            "tuning": toggles.get("tuning").copied().unwrap_or(true),
            "lighting": toggles.get("lighting").copied().unwrap_or(true),
            "water": toggles.get("water").copied().unwrap_or(true),
        }
    }));

    // 2. Scan Disk and Sync Custom Plugins
    if let Ok(plugins) = crate::commands::system_cmd::get_custom_plugins() {
        let list: Vec<serde_json::Value> = plugins.into_iter().map(|p| {
            let tag = match p.plugin_type.as_str() {
                "service" => "后台守护".to_string(),
                "app" => "面板/挂件".to_string(),
                "tab" => "独立面板".to_string(),
                "widget" => "桌面组件".to_string(),
                "skin" => "主题皮肤".to_string(),
                "background" => "壁纸扩展".to_string(),
                other => other.to_string(),
            };
            let min_ver = p.min_host_version.map(|v| format!("v{}+", v)).unwrap_or_default();
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "version": if p.version.starts_with('v') { p.version } else { format!("v{}", p.version) },
                "author": p.author,
                "description": p.description,
                "plugin_type": p.plugin_type,
                "enabled": p.enabled,
                "tag": tag,
                "min_version": min_ver,
                "dir_name": p.dir_name,
            })
        }).collect();

        send_slint_ipc_message(serde_json::json!({
            "action": "sync_custom_plugins",
            "plugins": list,
        }));
    }
}

pub fn sync_slint_power_schemes(restore_msg: Option<&str>) {
    if let Ok(schemes) = crate::commands::power_cmd::get_windows_power_schemes() {
        let list: Vec<serde_json::Value> = schemes.into_iter().map(|s| {
            serde_json::json!({
                "guid": s.guid,
                "name": s.name,
                "is_active": s.is_active,
            })
        }).collect();
        send_slint_ipc_message(serde_json::json!({
            "action": "sync_power_schemes",
            "schemes": list,
            "restore_plan_msg": restore_msg.unwrap_or(""),
        }));
    }
}

pub fn compute_power_targets(
    power_mode: u8,
    presets: &crate::system::power_mode::FactoryPowerPresets,
    balanced_gpu_dynamic_boost: bool,
    custom_profile: Option<&crate::system::config::CustomProfileConfig>,
    is_pure_igpu: bool,
) -> (i32, i32, String) {
    let is_custom = power_mode == 4;

    if is_custom {
        let pl1 = custom_profile
            .map(|p| p.cpu_pl1_watts as i32)
            .filter(|&v| v > 0)
            .unwrap_or_else(|| if presets.balance_pl1 > 0 { presets.balance_pl1 as i32 } else { 75 });

        let tgp = if is_pure_igpu {
            0
        } else if let Some(cp) = custom_profile {
            if cp.gpu_ctgp_enabled && cp.gpu_ctgp_watts > 0 {
                cp.gpu_ctgp_watts as i32
            } else {
                let base = if presets.base_tgp > 0 { presets.base_tgp as i32 } else { 150 };
                let db = if cp.gpu_db_enabled { cp.gpu_db_watts as i32 } else { 0 };
                base + db
            }
        } else if presets.base_tgp > 0 {
            presets.base_tgp as i32 + presets.max_db as i32
        } else {
            175
        };

        let label = "当前设定";
        let text = if tgp > 0 {
            format!("{} PL1: {}W | TGP: {}W", label, pl1, tgp)
        } else if !is_pure_igpu {
            format!("{} PL1: {}W | TGP: --", label, pl1)
        } else {
            format!("{} PL1: {}W", label, pl1)
        };

        return (pl1, tgp, text);
    }

    let is_hw = presets.hardware_read;
    let label = if is_hw { "原厂目标" } else { "参考值" };

    let (pl1, tgp) = match power_mode {
        1 => {
            let p1 = if presets.office_pl1 > 0 { presets.office_pl1 as i32 } else if is_pure_igpu { 35 } else { 45 };
            let tg = if is_pure_igpu { 0 } else if presets.base_tgp > 0 { presets.base_tgp as i32 } else { 150 };
            (p1, tg)
        }
        2 => {
            let p1 = if presets.balance_pl1 > 0 { presets.balance_pl1 as i32 } else if is_pure_igpu { 45 } else { 75 };
            let tg = if is_pure_igpu {
                0
            } else {
                let base = if presets.base_tgp > 0 { presets.base_tgp as i32 } else { 150 };
                let db = if balanced_gpu_dynamic_boost {
                    if presets.max_db > 0 { presets.max_db as i32 } else { 25 }
                } else {
                    0
                };
                base + db
            };
            (p1, tg)
        }
        3 => {
            let p1 = if presets.turbo_pl1 > 0 { presets.turbo_pl1 as i32 } else if is_pure_igpu { 54 } else { 205 };
            let tg = if is_pure_igpu {
                0
            } else {
                let base = if presets.base_tgp > 0 { presets.base_tgp as i32 } else { 150 };
                let db = if presets.max_db > 0 { presets.max_db as i32 } else { 25 };
                base + db
            };
            (p1, tg)
        }
        _ => (75, 175),
    };

    let text = if tgp > 0 {
        format!("{} PL1: {}W | TGP: {}W", label, pl1, tgp)
    } else if !is_pure_igpu {
        format!("{} PL1: {}W | TGP: --", label, pl1)
    } else {
        format!("{} PL1: {}W", label, pl1)
    };

    (pl1, tgp, text)
}

pub fn get_current_power_targets(power_mode: u8) -> (i32, i32, String) {
    let cfg = crate::system::config::load_config();
    let is_pure_igpu = crate::commands::power_cmd::is_thin_and_light_or_pure_igpu()
        || !crate::driver::gpu_bridge::is_nvidia_dgpu_available();
    let presets = crate::system::power_mode::get_factory_power_presets(!is_pure_igpu);
    let active_id = if !cfg.active_profile_id.is_empty() {
        cfg.active_profile_id.clone()
    } else {
        "preset_balance".to_string()
    };
    let profiles = crate::commands::tuning_cmd::get_profiles_list().unwrap_or_default();
    let matched = profiles.iter().find(|p| p.id == active_id).or_else(|| profiles.first());
    let custom_profile = matched.map(|p| &p.profile).unwrap_or(&cfg.custom_profile);

    compute_power_targets(
        power_mode,
        &presets,
        cfg.balanced_gpu_dynamic_boost,
        Some(custom_profile),
        is_pure_igpu,
    )
}

pub fn sync_slint_power_targets(power_mode: u8) {
    let (pl1, tgp, text) = get_current_power_targets(power_mode);
    send_slint_ipc_message(serde_json::json!({
        "action": "power_targets",
        "power_mode": power_mode,
        "pl1_target": pl1,
        "tgp_target": tgp,
        "power_target_text": text,
    }));
}
