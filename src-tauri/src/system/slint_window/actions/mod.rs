use tauri::AppHandle;
use super::SLINT_APP_HANDLE;

pub mod display;
pub mod lighting;
pub mod plugins;
pub mod power;
pub mod switches;
pub mod tuning;
pub mod water;
pub mod window;

pub fn handle_slint_incoming_action(line: &str, app: Option<&AppHandle>) {
    let fallback_app = SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone());
    let app = app.or(fallback_app.as_ref());

    let val: serde_json::Value = match serde_json::from_str(line.trim()) {
        Ok(v) => v,
        Err(_) => return,
    };

    let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("");
    crate::log_info!("SLINT_MAIN", "Received action from Slint UI: {}", action);

    if window::handle(action, &val, app) { return; }
    if power::handle(action, &val, app) { return; }
    if switches::handle(action, &val, app) { return; }
    if display::handle(action, &val, app) { return; }
    if tuning::handle(action, &val, app) { return; }
    if lighting::handle(action, &val, app) { return; }
    if water::handle(action, &val, app) { return; }
    if plugins::handle(action, &val, app) { return; }
}

