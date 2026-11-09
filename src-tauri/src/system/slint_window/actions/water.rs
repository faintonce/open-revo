use tauri::AppHandle;
use super::super::sync::sync_slint_water_state;
use super::super::send_slint_ipc_message;

pub fn handle(action: &str, val: &serde_json::Value, _app: Option<&AppHandle>) -> bool {
    match action {
        "start_water_cooler" => {
            let pump_v = val.get("pump_v").and_then(|v| v.as_i64()).map(|v| v as u8);
            let fan_d = val.get("fan_d").and_then(|v| v.as_i64()).map(|v| v as u8);
            let _ = crate::commands::cooler_cmd::start_water_cooler(pump_v, fan_d);
            sync_slint_water_state(None, None);
            true
        }
        "stop_water_cooler" => {
            let _ = crate::commands::cooler_cmd::stop_water_cooler();
            sync_slint_water_state(None, None);
            true
        }
        "pair_water_cooler" => {
            std::thread::spawn(|| {
                let _ = crate::commands::cooler_cmd::pair_water_cooler();
                sync_slint_water_state(None, None);
            });
            true
        }
        "fill_water_cooler" => {
            let _ = crate::commands::cooler_cmd::fill_water_cooler();
            sync_slint_water_state(None, None);
            true
        }
        "reset_water_cooler" => {
            let _ = crate::commands::cooler_cmd::reset_water_cooler();
            sync_slint_water_state(None, None);
            true
        }
        "set_water_cooler_strategy" => {
            let strat = val.get("strategy").and_then(|v| v.as_str()).unwrap_or("smart").to_string();
            let curve = if let Some(arr) = val.get("curve").and_then(|v| v.as_array()) {
                let mut pts = Vec::new();
                for item in arr {
                    let temp = item.get("temp").and_then(|v| v.as_i64()).unwrap_or(55) as u8;
                    let fan_duty = item.get("fan_duty").and_then(|v| v.as_i64()).unwrap_or(50) as u8;
                    let pump_voltage = item.get("pump_voltage").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
                    pts.push(crate::system::config::CoolerCurvePoint {
                        temp,
                        fan_duty,
                        pump_voltage,
                    });
                }
                Some(pts)
            } else {
                None
            };
            let _ = crate::commands::cooler_cmd::set_water_cooler_strategy(strat, curve);
            sync_slint_water_state(None, None);
            true
        }
        "set_water_cooler_speed" => {
            let pump_v = val.get("pump_v").and_then(|v| v.as_i64()).map(|v| v as u8);
            let fan_d = val.get("fan_d").and_then(|v| v.as_i64()).map(|v| v as u8);
            let _ = crate::commands::cooler_cmd::set_water_cooler_speed(pump_v, fan_d);
            sync_slint_water_state(None, None);
            true
        }
        "set_water_cooler_pump_thresholds" => {
            let st = val.get("start_temp").and_then(|v| v.as_i64()).unwrap_or(55) as u8;
            let tb = val.get("turbo_temp").and_then(|v| v.as_i64()).unwrap_or(90) as u8;
            let _ = crate::commands::cooler_cmd::set_water_cooler_pump_thresholds(st, tb);
            sync_slint_water_state(None, None);
            true
        }
        "set_water_cooler_led" => {
            let on = val.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
            let mode = val.get("mode").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
            let r = val.get("r").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
            let g = val.get("g").and_then(|v| v.as_i64()).unwrap_or(128) as u8;
            let b = val.get("b").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
            let _ = crate::commands::cooler_cmd::set_water_cooler_led(on, r, g, b, mode);
            sync_slint_water_state(None, None);
            true
        }
        "scan_water_cooler_devices" => {
            std::thread::spawn(|| {
                match crate::commands::cooler_cmd::scan_water_cooler_devices() {
                    Ok(devs) => {
                        let current_mac = crate::driver::ble_cooler::get_global_cooler().get_telemetry().mac_address;
                        let json_devs: Vec<serde_json::Value> = devs.into_iter().map(|d| {
                            let is_curr = !current_mac.is_empty() && d.mac_address.eq_ignore_ascii_case(&current_mac);
                            serde_json::json!({
                                "name": d.name,
                                "mac_address": d.mac_address,
                                "rssi": d.rssi,
                                "is_paired": d.is_paired,
                                "is_current": is_curr,
                            })
                        }).collect();
                        send_slint_ipc_message(serde_json::json!({
                            "action": "sync_water_cooler_devices",
                            "devices": json_devs,
                        }));
                    }
                    Err(e) => {
                        send_slint_ipc_message(serde_json::json!({
                            "action": "sync_water_cooler_devices",
                            "devices": [],
                            "error_msg": e,
                        }));
                    }
                }
            });
            true
        }
        "select_water_cooler" => {
            if let Some(mac) = val.get("mac").and_then(|v| v.as_str()) {
                let _ = crate::commands::cooler_cmd::select_water_cooler(mac.to_string());
                sync_slint_water_state(None, None);
            }
            true
        }
        _ => false,
    }
}
