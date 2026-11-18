use std::io::{BufRead, Write};
use crate::MainWindow;

pub mod display;
pub mod lighting;
pub mod power;
pub mod system;
pub mod telemetry;
pub mod tuning;
pub mod water;

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

pub(crate) fn parse_hex_color(hex: &str) -> u32 {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        if let Ok(val) = u32::from_str_radix(clean, 16) {
            return 0xff000000 | val;
        }
    }
    0xff6366f1
}

pub(crate) fn update_refresh_rates(val: &serde_json::Value, ui: &MainWindow) {
    let mut all_rates: Vec<i32> = Vec::new();
    if let Some(r1_arr) = val.get("rates_row1").and_then(|v| v.as_array()) {
        let r1: Vec<i32> = r1_arr.iter().filter_map(|r| r.as_i64().map(|v| v as i32)).collect();
        if !r1.is_empty() {
            all_rates.extend(r1.iter().copied());
            ui.set_rates_row1(slint::ModelRc::new(slint::VecModel::from(r1)));
        }
    }
    if let Some(r2_arr) = val.get("rates_row2").and_then(|v| v.as_array()) {
        let r2: Vec<i32> = r2_arr.iter().filter_map(|r| r.as_i64().map(|v| v as i32)).collect();
        all_rates.extend(r2.iter().copied());
        ui.set_rates_row2(slint::ModelRc::new(slint::VecModel::from(r2)));
    }
    if !all_rates.is_empty() {
        all_rates.sort();
        all_rates.dedup();
        let min_rate = *all_rates.first().unwrap();
        let max_rate = *all_rates.last().unwrap();
        ui.set_min_refresh_rate(min_rate);
        ui.set_max_refresh_rate(max_rate);
        let intermediate: Vec<i32> = if all_rates.len() > 2 {
            all_rates[1..all_rates.len() - 1].to_vec()
        } else {
            Vec::new()
        };
        ui.set_intermediate_rates(slint::ModelRc::new(slint::VecModel::from(intermediate)));
    }
    if let Some(min_r) = val.get("min_refresh_rate").and_then(|v| v.as_i64()) {
        ui.set_min_refresh_rate(min_r as i32);
    }
    if let Some(max_r) = val.get("max_refresh_rate").and_then(|v| v.as_i64()) {
        ui.set_max_refresh_rate(max_r as i32);
    }
    if let Some(inter_arr) = val.get("intermediate_rates").and_then(|v| v.as_array()) {
        let inter: Vec<i32> = inter_arr.iter().filter_map(|r| r.as_i64().map(|v| v as i32)).collect();
        ui.set_intermediate_rates(slint::ModelRc::new(slint::VecModel::from(inter)));
    }
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
            let act = action.as_str();
            if system::handle(act, &val, &ui) { return; }
            if power::handle(act, &val, &ui) { return; }
            if display::handle(act, &val, &ui) { return; }
            if telemetry::handle(act, &val, &ui) { return; }
            if tuning::handle(act, &val, &ui) { return; }
            if lighting::handle(act, &val, &ui) { return; }
            if water::handle(act, &val, &ui) { return; }
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

