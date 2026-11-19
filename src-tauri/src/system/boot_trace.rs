//! OpenRevo Boot Tracing & Startup Milestone Monitor Facade
//! Re-exports core boot tracing engine and provides host Tauri commands.

pub use openrevo_core::system::boot_trace::*;
use tauri::Manager;

/// Record that frontend React/Vue application mounted, executed first render, and established two-way IPC handshake.
#[tauri::command]
pub fn frontend_ready(app: tauri::AppHandle) -> Result<u128, String> {
    let total = BOOT_START_INSTANT.get().map(|s| s.elapsed().as_millis()).unwrap_or(0);
    boot_log(
        "FE_READY",
        &format!("Frontend DOM Mounted & Handshake Confirmed (+{}ms). WebView2 message loop fully responsive!", total),
    );
    mark_boot_completed();
    crate::system::webview_orchestrator::WebviewOrchestrator::global().notify_main_ready();

    // Diagnostics & Visibility Override: Auto-reveal main window only if explicitly requested
    // via CLI argument (--visible / --show) or persistent config.json ("launch_visible": true).
    let args: Vec<String> = std::env::args().collect();
    let cli_visible = args.iter().any(|a| a == "--visible" || a == "--show");
    let cfg = crate::system::config::load_config();
    let should_reveal = cli_visible || cfg.launch_visible;

    if should_reveal {
        if let Some(window) = app.get_webview_window("main") {
            crate::log_info!("BOOT", "Diagnostic auto-reveal triggered (CLI --visible or config.json launch_visible): revealing main window");
            crate::system::window::restore_and_sync_window(&window);
        }
    }

    Ok(total)
}

/// Retrieve the current boot log contents for frontend diagnostics view
#[tauri::command]
pub fn get_boot_log() -> Result<String, String> {
    let path = get_boot_log_path();
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read boot log: {}", e))
}
