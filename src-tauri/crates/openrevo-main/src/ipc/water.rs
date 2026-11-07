use crate::MainWindow;

pub fn handle(action: &str, val: &serde_json::Value, ui: &MainWindow) -> bool {
    match action {
        "sync_water_cooler_state" => {
            ui.set_is_water_connecting(false);
            ui.set_is_water_pairing(false);

            if let Some(conn) = val.get("connected").and_then(|v| v.as_bool()) {
                ui.set_is_water_connected(conn);
            }
            if let Some(flow) = val.get("is_flow_ok").and_then(|v| v.as_bool()) {
                ui.set_water_flow_ok(flow);
            }
            if let Some(ms) = val.get("meter_state").and_then(|v| v.as_i64()) {
                ui.set_water_meter_state(ms as i32);
            }
            if let Some(tier) = val.get("pump_tier").and_then(|v| v.as_i64()) {
                ui.set_water_pump_tier(tier as i32);
            }
            if let Some(volt) = val.get("pump_voltage").and_then(|v| v.as_i64()) {
                ui.set_water_pump_voltage(volt as i32);
            }
            if let Some(fan) = val.get("fan_duty").and_then(|v| v.as_i64()) {
                ui.set_water_fan_duty(fan as i32);
            }
            if let Some(prpm) = val.get("pump_rpm").and_then(|v| v.as_i64()) {
                ui.set_water_pump_rpm(prpm as i32);
            }
            if let Some(frpm) = val.get("fan_rpm").and_then(|v| v.as_i64()) {
                ui.set_water_fan_rpm(frpm as i32);
            }
            if let Some(prim) = val.get("is_priming").and_then(|v| v.as_bool()) {
                ui.set_is_water_priming(prim);
            }
            if let Some(mac) = val.get("mac_address").and_then(|v| v.as_str()) {
                ui.set_water_mac_address(mac.into());
            }
            if let Some(strat) = val.get("strategy").and_then(|v| v.as_str()) {
                ui.set_water_strategy(strat.into());
            }
            let is_curve_dragging = ui.get_is_water_curve_dragging();
            if !is_curve_dragging {
                if let Some(st) = val.get("pump_start_temp").and_then(|v| v.as_i64()) {
                    ui.set_water_pump_start_temp(st as i32);
                }
                if let Some(tb) = val.get("pump_turbo_temp").and_then(|v| v.as_i64()) {
                    ui.set_water_pump_turbo_temp(tb as i32);
                }

                // Curve points (guaranteed exactly 6 points)
                if let Some(t_arr) = val.get("curve_temps").and_then(|v| v.as_array()) {
                    let mut temps: Vec<i32> = t_arr.iter().filter_map(|t| t.as_i64().map(|v| v as i32)).collect();
                    if !temps.is_empty() {
                        while temps.len() < 6 {
                            let last = *temps.last().unwrap_or(&95);
                            temps.push((last + 3).min(100));
                        }
                        if temps.len() > 6 {
                            temps.truncate(6);
                        }
                        ui.set_water_curve_temps(slint::ModelRc::new(slint::VecModel::from(temps)));
                    }
                }
                if let Some(d_arr) = val.get("curve_duties").and_then(|v| v.as_array()) {
                    let mut duties: Vec<i32> = d_arr.iter().filter_map(|d| d.as_i64().map(|v| v as i32)).collect();
                    if !duties.is_empty() {
                        while duties.len() < 6 {
                            let last = *duties.last().unwrap_or(&100);
                            duties.push(last.min(100));
                        }
                        if duties.len() > 6 {
                            duties.truncate(6);
                        }
                        ui.set_water_curve_duties(slint::ModelRc::new(slint::VecModel::from(duties)));
                    }
                }
            }

            if let Some(lfd) = val.get("live_fan_duty").and_then(|v| v.as_i64()) {
                ui.set_water_live_fan_duty(lfd as i32);
            }

            // LED
            if let Some(lon) = val.get("led_on").and_then(|v| v.as_bool()) {
                ui.set_water_led_on(lon);
            }
            if let Some(lm) = val.get("led_mode").and_then(|v| v.as_i64()) {
                ui.set_water_led_mode(lm as i32);
            }
            if let Some(lr) = val.get("led_r").and_then(|v| v.as_i64()) {
                ui.set_water_led_r(lr as i32);
            }
            if let Some(lg) = val.get("led_g").and_then(|v| v.as_i64()) {
                ui.set_water_led_g(lg as i32);
            }
            if let Some(lb) = val.get("led_b").and_then(|v| v.as_i64()) {
                ui.set_water_led_b(lb as i32);
            }
            true
        }
        "sync_water_cooler_devices" => {
            ui.set_is_water_scanning_devices(false);
            if let Some(err) = val.get("error_msg").and_then(|v| v.as_str()) {
                ui.set_water_modal_error_msg(err.into());
            } else {
                ui.set_water_modal_error_msg("".into());
            }
            if let Some(devs) = val.get("devices").and_then(|v| v.as_array()) {
                let mut list = Vec::new();
                for d in devs {
                    let name = d.get("name").and_then(|v| v.as_str()).unwrap_or("水冷魔盒").to_string();
                    let mac = d.get("mac_address").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let rssi = d.get("rssi").and_then(|v| v.as_i64()).unwrap_or(-70) as i32;
                    let is_paired = d.get("is_paired").and_then(|v| v.as_bool()).unwrap_or(false);
                    let is_current = d.get("is_current").and_then(|v| v.as_bool()).unwrap_or(false);
                    list.push(crate::WaterCoolerDeviceItem {
                        name: name.into(),
                        mac_address: mac.into(),
                        rssi,
                        is_paired,
                        is_current,
                    });
                }
                ui.set_water_devices(slint::ModelRc::new(slint::VecModel::from(list)));
            }
            true
        }
        _ => false,
    }
}

