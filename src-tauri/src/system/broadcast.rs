// OpenRevo - Universal Bi-directional Event Broadcast Hub
// Bridges events across Tauri Webviews, Slint UI, Native Wry Webview2 surfaces, and Background Watchers.

use std::sync::OnceLock;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};

static BROADCAST_APP: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

fn get_app_cell() -> &'static Mutex<Option<AppHandle>> {
    BROADCAST_APP.get_or_init(|| Mutex::new(None))
}

pub fn bind_broadcast_app(app: AppHandle) {
    let mut cell = get_app_cell().lock();
    *cell = Some(app);
}

pub fn get_broadcast_app() -> Option<AppHandle> {
    get_app_cell().lock().clone()
}

/// Unified broadcast function: delivers an event to all surfaces:
/// 1. Tauri WebView windows (main, osd, etc.) via app.emit
/// 2. Native Wry instances (embedded Slint tabs and detached popouts) via evaluate_script / postMessage
pub fn broadcast_all(event: &str, payload: &serde_json::Value) {
    if let Some(app) = get_broadcast_app() {
        let _ = app.emit(event, payload);
    }
    crate::system::native_wry::broadcast_event(event, payload);
}

