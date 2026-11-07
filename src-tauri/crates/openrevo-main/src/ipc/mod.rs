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

