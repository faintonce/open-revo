use std::collections::HashMap;
use std::sync::OnceLock;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::system_cmd::RawPluginManifest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WidgetRuntimeInfo {
    pub plugin_id: String,
    pub label: String,
    pub width: f64,
    pub height: f64,
}

fn get_active_widgets() -> &'static Mutex<HashMap<String, WidgetRuntimeInfo>> {
    static WIDGETS: OnceLock<Mutex<HashMap<String, WidgetRuntimeInfo>>> = OnceLock::new();
    WIDGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Convert plugin_id to a valid Tauri window label
pub fn format_widget_label(plugin_id: &str) -> String {
    format!("widget_{}", plugin_id.replace('-', "_"))
}

/// Open an independent, sandbox-isolated Webview window for a custom widget plugin
#[tauri::command]
pub async fn open_custom_widget(app: AppHandle, plugin_id: String) -> Result<String, String> {
    let label = format_widget_label(&plugin_id);

    // 1. Persist plugin toggle state
    let mut cfg = crate::system::config::load_config();
    cfg.custom_plugin_toggles.insert(plugin_id.clone(), true);
    crate::system::config::save_config(&cfg);

    // 2. Fast-path: if window already exists (e.g. was hidden), restore and show it instantly
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.unminimize();
        crate::log_info!("WIDGET", "Restored and unhid existing widget window '{}'", label);
        return Ok(label);
    }

    // 3. Delegate to WebviewOrchestrator queue for safe staggered creation with circuit-breaker protection
    crate::system::webview_orchestrator::WebviewOrchestrator::global().enqueue_widget(plugin_id.clone());

    // 4. Await orchestrator window instantiation (up to 3000ms)
    for _ in 0..60 {
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.show();
            let _ = win.unminimize();
            return Ok(label);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Err(format!("Timed out awaiting widget '{}' creation from orchestrator", plugin_id))
}

/// Internal asynchronous widget window creation routine executed exclusively by WebviewOrchestrator worker
pub async fn spawn_widget_window_internal(app: &AppHandle, plugin_id: &str) -> Result<String, String> {
    let label = format_widget_label(plugin_id);

    let plugins_dir = crate::system::config::get_config_dir().join("plugins").join(plugin_id);
    let manifest_path = plugins_dir.join("manifest.json");

    if !manifest_path.exists() {
        return Err(format!("Plugin directory or manifest.json not found: {:?}", manifest_path));
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
    let manifest: RawPluginManifest = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Invalid manifest.json schema: {}", e))?;

    let (is_compat, notice) = crate::commands::system_cmd::check_version_compatibility(manifest.min_host_version.as_deref());
    if !is_compat {
        return Err(notice.unwrap_or_else(|| "当前 OpenRevo 版本低于该挂件最低要求版本".into()));
    }

    let p_type = manifest.plugin_type.as_deref().unwrap_or_default();
    let is_tab_plugin = p_type.eq_ignore_ascii_case("tab");

    let win_cfg = manifest.window.clone().unwrap_or_else(|| {
        if is_tab_plugin {
            crate::commands::system_cmd::PluginWindowConfig {
                width: 1024.0,
                height: 720.0,
                transparent: false,
                always_on_top: false,
                resizable: true,
                decorations: true,
                skip_taskbar: false,
            }
        } else {
            crate::commands::system_cmd::PluginWindowConfig {
                width: 240.0,
                height: 320.0,
                transparent: true,
                always_on_top: true,
                resizable: false,
                decorations: false,
                skip_taskbar: true,
            }
        }
    });

    // 1. Register fine-grained telemetry sensor subscription
    if let Some(telemetry_cfg) = &manifest.telemetry {
        if telemetry_cfg.enabled {
            crate::system::telemetry::register_consumer(plugin_id, &telemetry_cfg.sensors);
        }
    } else {
        // Default: If telemetry is omitted, register basic CPU/GPU metrics
        crate::system::telemetry::register_consumer(plugin_id, &["cpu_temp".into(), "cpu_fan_rpm".into()]);
    }

    // 2. Record in runtime state
    {
        let mut active = get_active_widgets().lock();
        active.insert(
            plugin_id.to_string(),
            WidgetRuntimeInfo {
                plugin_id: plugin_id.to_string(),
                label: label.clone(),
                width: win_cfg.width,
                height: win_cfg.height,
            },
        );
    }

    // 3. Fast-path: if already alive, present immediately
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.unminimize();
        return Ok(label);
    }

    // Windows 11 26H2 / WebView2 Release Guard:
    // Pure "index.html" prevents 404 resource lookup failure caused by query strings in embedded asset packs.
    let target_url = WebviewUrl::App("index.html".into());
    let init_script = format!(
        "window.__OPENREVO_WIDGET_ID__ = '{}'; if (!window.location.hash.startsWith('#/widget')) {{ window.location.hash = '#/widget/{}'; }}",
        plugin_id, plugin_id
    );

    let has_decorations = win_cfg.decorations;
    let window_title = manifest.name.unwrap_or_else(|| plugin_id.to_string());

    let mut builder = WebviewWindowBuilder::new(app, &label, target_url)
        .title(format!("OpenRevo - {}", window_title))
        .inner_size(win_cfg.width, win_cfg.height)
        .resizable(win_cfg.resizable)
        .decorations(has_decorations)
        .transparent(win_cfg.transparent)
        .shadow(has_decorations)
        .always_on_top(win_cfg.always_on_top)
        .skip_taskbar(win_cfg.skip_taskbar)
        .initialization_script(&init_script)
        .visible(false); // Conceal during initial DOM mount to avoid white flash

    if is_tab_plugin {
        builder = builder.min_inner_size(600.0, 480.0);
    }

    let cfg = crate::system::config::load_config();
    let mut positioned = false;
    if let Some(&(saved_x, saved_y)) = cfg.widget_positions.get(plugin_id) {
        let is_valid = if let Ok(monitors) = app.available_monitors() {
            monitors.iter().any(|m| {
                let m_pos = m.position();
                let m_sz = m.size();
                saved_x >= m_pos.x && saved_x < (m_pos.x + m_sz.width as i32)
                    && saved_y >= m_pos.y && saved_y < (m_pos.y + m_sz.height as i32)
            })
        } else {
            true
        };
        if is_valid {
            builder = builder.position(saved_x as f64, saved_y as f64);
            positioned = true;
        } else {
            crate::log_warn!(
                "WIDGET",
                "Saved widget position ({}, {}) is out of bounds on current monitors. Centering widget instead.",
                saved_x,
                saved_y
            );
        }
    }

    if !positioned {
        builder = builder.center();
    }

    match builder.build() {
        Ok(win) => {
            // Watchdog fallback (600ms): if frontend widget_ready event is somehow missed, show gracefully
            let win_fallback = win.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(600));
                if let Ok(is_vis) = win_fallback.is_visible() {
                    if !is_vis {
                        let _ = win_fallback.show();
                        let _ = win_fallback.unminimize();
                    }
                }
            });

            crate::log_info!(
                "WIDGET",
                "Launched widget window '{}' ({}x{}) successfully for plugin '{}'",
                label,
                win_cfg.width,
                win_cfg.height,
                plugin_id
            );

            Ok(label)
        }
        Err(e) => {
            crate::system::telemetry::unregister_consumer(plugin_id);
            let mut active = get_active_widgets().lock();
            active.remove(plugin_id);
            Err(format!("Failed to build widget window '{}': {}", label, e))
        }
    }
}

/// Signal from frontend that the widget DOM has mounted and first frame rendered, presenting seamlessly
#[tauri::command]
pub fn widget_ready(app: AppHandle, plugin_id: String) -> Result<bool, String> {
    let label = format_widget_label(&plugin_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.unminimize();
        crate::log_info!("WIDGET", "Widget '{}' DOM mounted and rendered, window presented seamlessly.", plugin_id);
    }
    Ok(true)
}

/// Debounced coordinate persistence: avoids flooding synchronous disk I/O on every single pixel moved
static PENDING_POSITIONS: std::sync::LazyLock<parking_lot::Mutex<HashMap<String, (i32, i32)>>> =
    std::sync::LazyLock::new(|| parking_lot::Mutex::new(HashMap::new()));
static SAVE_DEBOUNCE_TIMER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn queue_widget_position(plugin_id: String, x: i32, y: i32) {
    {
        let mut map = PENDING_POSITIONS.lock();
        map.insert(plugin_id, (x, y));
    }
    if !SAVE_DEBOUNCE_TIMER.swap(true, std::sync::atomic::Ordering::SeqCst) {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(600));
            SAVE_DEBOUNCE_TIMER.store(false, std::sync::atomic::Ordering::SeqCst);
            let positions = {
                let mut map = PENDING_POSITIONS.lock();
                std::mem::take(&mut *map)
            };
            if !positions.is_empty() {
                let mut cfg = crate::system::config::load_config();
                for (id, pos) in positions {
                    cfg.widget_positions.insert(id, pos);
                }
                crate::system::config::save_config(&cfg);
            }
        });
    }
}

/// Clean up runtime telemetry and state when a widget or standalone window closes
pub fn cleanup_widget_on_close(app: &AppHandle, plugin_id: &str) {
    let mut active = get_active_widgets().lock();
    active.remove(plugin_id);

    // Unregister telemetry consumer to restore low-power heartbeat
    crate::system::telemetry::unregister_consumer(plugin_id);

    // Persist plugin toggle state
    let mut cfg = crate::system::config::load_config();
    cfg.custom_plugin_toggles.insert(plugin_id.to_string(), false);
    crate::system::config::save_config(&cfg);

    let _ = app.emit("widget-closed", plugin_id);
    let _ = app.emit("widget-status-changed", serde_json::json!({
        "plugin_id": plugin_id,
        "active": false
    }));
    crate::log_info!("WIDGET", "Cleaned up widget runtime state and notified listeners for '{}'", plugin_id);
}

/// Close and destroy an active widget or standalone tab window, freeing all telemetry resources
#[tauri::command]
pub fn close_custom_widget(app: AppHandle, plugin_id: String) -> Result<bool, String> {
    let label = format_widget_label(&plugin_id);

    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.destroy();
        crate::log_info!("WIDGET", "Widget window '{}' destroyed cleanly.", label);
    }

    cleanup_widget_on_close(&app, &plugin_id);
    Ok(true)
}

/// Get all currently active widget plugin IDs
#[tauri::command]
pub fn get_running_widgets() -> Result<Vec<String>, String> {
    let active = get_active_widgets().lock();
    Ok(active.keys().cloned().collect())
}

/// Automatically restore and spawn enabled widget windows on OpenRevo boot via orchestrator queue
pub fn restore_active_widgets_on_boot(_app: &AppHandle) {
    let cfg = crate::system::config::load_config();
    let plugins_dir = crate::system::config::get_config_dir().join("plugins");

    for (plugin_id, enabled) in cfg.custom_plugin_toggles.iter() {
        if *enabled {
            let manifest_path = plugins_dir.join(plugin_id).join("manifest.json");
            if manifest_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                        let p_type = manifest.plugin_type.unwrap_or_default();
                        if p_type == "widget" || p_type == "skin" {
                            crate::log_info!(
                                "WIDGET",
                                "Auto-restoring enabled widget '{}' on startup via orchestrator queue...",
                                plugin_id
                            );
                            crate::system::webview_orchestrator::WebviewOrchestrator::global()
                                .enqueue_widget(plugin_id.clone());
                        }
                    }
                }
            }
        }
    }
}
