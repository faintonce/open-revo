use crate::MainWindow;

pub fn handle(action: &str, val: &serde_json::Value, ui: &MainWindow) -> bool {
    if action != "telemetry" {
        return false;
    }

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

