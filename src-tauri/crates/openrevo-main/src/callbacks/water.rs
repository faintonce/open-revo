use slint::Model;
use crate::ipc;
use crate::MainWindow;

pub fn register(window: &MainWindow, ui_weak: slint::Weak<MainWindow>) {
    // Water Cooler (BLE) Callbacks
    let ui_cb = ui_weak.clone();
    window.on_connect_water_cooler(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_water_connecting(true);
            let is_custom = ui.get_water_strategy().as_str() == "custom";
            let pump_tier = ui.get_water_pump_tier();
            let fan_duty = ui.get_water_fan_duty();
            ipc::send_action_to_host("start_water_cooler", serde_json::json!({
                "pump_v": if is_custom { Some(pump_tier) } else { None },
                "fan_d": if is_custom { Some(fan_duty) } else { None },
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_disconnect_water_cooler(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_water_connecting(true);
            ipc::send_action_to_host("stop_water_cooler", serde_json::json!({}));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_pair_water_cooler(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_water_pairing(true);
            ipc::send_action_to_host("pair_water_cooler", serde_json::json!({}));
        }
    });

    window.on_fill_water_cooler(move || {
        ipc::send_action_to_host("fill_water_cooler", serde_json::json!({}));
    });

    window.on_reset_water_cooler(move || {
        ipc::send_action_to_host("reset_water_cooler", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_update_water_strategy(move |strat| {
        if let Some(ui) = ui_cb.upgrade() {
            let s = strat.as_str();
            ui.set_water_strategy(strat.clone());
            if s != "smart" && s != "custom" {
                let target_tier = if s == "performance" { 3 } else { 2 };
                let target_fan = if s == "performance" { 100 } else if s == "balance" { 65 } else { 40 };
                ui.set_water_pump_tier(target_tier);
                ui.set_water_fan_duty(target_fan);
            }
            ipc::send_action_to_host("set_water_cooler_strategy", serde_json::json!({
                "strategy": s,
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_water_cooler_speed(move |tier, fan| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_water_strategy("custom".into());
            ui.set_water_pump_tier(tier);
            ui.set_water_fan_duty(fan);
            ipc::send_action_to_host("set_water_cooler_speed", serde_json::json!({
                "pump_v": tier,
                "fan_d": fan,
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_water_curve_point(move |idx, new_temp, new_duty| {
        if let Some(ui) = ui_cb.upgrade() {
            let i = idx as usize;
            let mut temps: Vec<i32> = ui.get_water_curve_temps().iter().collect();
            let mut duties: Vec<i32> = ui.get_water_curve_duties().iter().collect();
            if i < temps.len() && i < duties.len() {
                let mut clamped_temp = new_temp.clamp(30, 100);
                if i > 0 {
                    clamped_temp = clamped_temp.max(temps[i - 1] + 2);
                }
                if i < temps.len() - 1 {
                    clamped_temp = clamped_temp.min(temps[i + 1] - 2);
                }
                let clamped_duty = new_duty.clamp(0, 100);

                if temps[i] == clamped_temp && duties[i] == clamped_duty {
                    return;
                }

                temps[i] = clamped_temp;
                duties[i] = clamped_duty;

                ui.set_water_curve_temps(slint::ModelRc::new(slint::VecModel::from(temps)));
                ui.set_water_curve_duties(slint::ModelRc::new(slint::VecModel::from(duties)));
            }
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_commit_water_curve(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let temps: Vec<i32> = ui.get_water_curve_temps().iter().collect();
            let duties: Vec<i32> = ui.get_water_curve_duties().iter().collect();
            let mut curve_pts = Vec::new();
            for i in 0..temps.len().min(duties.len()) {
                let volt = if duties[i] >= 90 { 3 } else if duties[i] == 0 { 0 } else { 2 };
                curve_pts.push(serde_json::json!({
                    "temp": temps[i],
                    "fan_duty": duties[i],
                    "pump_voltage": volt,
                }));
            }
            ipc::send_action_to_host("set_water_cooler_strategy", serde_json::json!({
                "strategy": ui.get_water_strategy().as_str(),
                "curve": curve_pts,
            }));
        }
    });

    window.on_water_pump_thresholds_changing(move |_st, _tb| {
        // UI already live-updates via two-way binding
    });

    window.on_commit_water_pump_thresholds(move |st, tb| {
        ipc::send_action_to_host("set_water_cooler_pump_thresholds", serde_json::json!({
            "start_temp": st,
            "turbo_temp": tb,
        }));
    });

    let ui_cb = ui_weak.clone();
    window.on_apply_water_curve_template(move |tpl| {
        if let Some(ui) = ui_cb.upgrade() {
            let (temps, duties, st, tb) = match tpl.as_str() {
                "quiet" => (vec![50, 65, 75, 85, 92, 98], vec![0, 25, 40, 55, 70, 90], 65, 95),
                "aggressive" => (vec![40, 45, 55, 70, 85, 90], vec![20, 40, 60, 80, 95, 100], 45, 80),
                "linear" => (vec![35, 45, 60, 75, 85, 95], vec![10, 25, 45, 65, 80, 100], 45, 85),
                _ => (vec![45, 55, 65, 80, 90, 95], vec![0, 35, 50, 60, 75, 100], 55, 90),
            };

            ui.set_water_curve_template(tpl);
            ui.set_water_curve_temps(slint::ModelRc::new(slint::VecModel::from(temps.clone())));
            ui.set_water_curve_duties(slint::ModelRc::new(slint::VecModel::from(duties.clone())));
            ui.set_water_pump_start_temp(st);
            ui.set_water_pump_turbo_temp(tb);

            let mut curve_pts = Vec::new();
            for i in 0..temps.len() {
                let volt = if duties[i] >= 90 { 3 } else if duties[i] == 0 { 0 } else { 2 };
                curve_pts.push(serde_json::json!({
                    "temp": temps[i],
                    "fan_duty": duties[i],
                    "pump_voltage": volt,
                }));
            }
            ipc::send_action_to_host("set_water_cooler_strategy", serde_json::json!({
                "strategy": ui.get_water_strategy().as_str(),
                "curve": curve_pts,
            }));
            ipc::send_action_to_host("set_water_cooler_pump_thresholds", serde_json::json!({
                "start_temp": st,
                "turbo_temp": tb,
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_water_led_enabled(move |en| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_water_led_on(en);
            ipc::send_action_to_host("set_water_cooler_led", serde_json::json!({
                "on": en,
                "mode": ui.get_water_led_mode(),
                "r": ui.get_water_led_r(),
                "g": ui.get_water_led_g(),
                "b": ui.get_water_led_b(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_water_led_mode(move |m| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_water_led_mode(m);
            ui.set_water_led_on(true);
            ipc::send_action_to_host("set_water_cooler_led", serde_json::json!({
                "on": true,
                "mode": m,
                "r": ui.get_water_led_r(),
                "g": ui.get_water_led_g(),
                "b": ui.get_water_led_b(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_water_led_color(move |r, g, b| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_water_led_r(r);
            ui.set_water_led_g(g);
            ui.set_water_led_b(b);
            ui.set_water_led_on(true);
            ipc::send_action_to_host("set_water_cooler_led", serde_json::json!({
                "on": true,
                "mode": ui.get_water_led_mode(),
                "r": r,
                "g": g,
                "b": b,
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_open_water_device_selector(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_water_device_modal_open(true);
            ui.set_is_water_scanning_devices(true);
            ipc::send_action_to_host("scan_water_cooler_devices", serde_json::json!({}));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_close_water_device_selector(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_water_device_modal_open(false);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_refresh_water_devices(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_water_scanning_devices(true);
            ipc::send_action_to_host("scan_water_cooler_devices", serde_json::json!({}));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_select_water_device(move |mac| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_water_device_modal_open(false);
            ui.set_is_water_connecting(true);
            ipc::send_action_to_host("select_water_cooler", serde_json::json!({
                "mac": mac.as_str(),
            }));
        }
    });
}

