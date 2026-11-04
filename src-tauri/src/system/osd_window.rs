use std::io::Write;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

static OSD_STDIN: Mutex<Option<ChildStdin>> = Mutex::new(None);

/// Ensure the native companion OSD process is spawned and acquire direct stdin handle
pub fn ensure_companion_process() {
    let mut lock = match OSD_STDIN.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };

    if lock.is_some() {
        return;
    }

    if let Ok(exe_path) = std::env::current_exe() {
        crate::log_info!("OSD", "Spawning native OSD companion with direct piped stdin: {:?}", exe_path);
        let mut cmd = Command::new(exe_path);
        cmd.arg("--osd-companion");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());


        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let cfg = crate::system::config::load_config();
                    let line = format!("{}\n", serde_json::json!({
                        "action": "config",
                        "theme": cfg.osd_theme,
                        "scale": cfg.osd_scale,
                        "position": cfg.osd_position,
                    }));
                    let _ = stdin.write_all(line.as_bytes());
                    let _ = stdin.flush();
                    *lock = Some(stdin);
                    crate::log_info!("OSD", "Native OSD companion stdin pipe established & initial config synced: theme={}, scale={}, pos={}", cfg.osd_theme, cfg.osd_scale, cfg.osd_position);
                }
            }
            Err(e) => {
                crate::log_error!("OSD", "Failed to spawn native OSD companion: {}", e);
            }
        }
    }
}

static LAST_MESSAGE: Mutex<Option<(String, std::time::Instant)>> = Mutex::new(None);

/// Send a JSON instruction directly to the companion child process via anonymous stdin pipe
pub fn send_osd_ipc_message(payload: serde_json::Value) {
    let cfg = crate::system::config::load_config();
    if !cfg.osd_enabled {
        return;
    }

    let line = format!("{}\n", payload);
    {
        let mut last = match LAST_MESSAGE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some((ref prev_msg, prev_time)) = *last {
            if prev_msg == &line && prev_time.elapsed() < std::time::Duration::from_millis(250) {
                return;
            }
        }
        *last = Some((line.clone(), std::time::Instant::now()));
    }

    ensure_companion_process();

    let mut lock = match OSD_STDIN.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };

    if let Some(ref mut stdin) = *lock {
        if let Err(e) = stdin.write_all(line.as_bytes()) {
            crate::log_error!("OSD", "Failed writing to companion stdin: {}", e);
            *lock = None; // Invalidate broken pipe
        } else {
            let _ = stdin.flush();
        }
    }
}


/// Initialize the OSD overlay subsystem (Slint native companion or classic WebView2 orchestrator)
pub fn init_osd_window(_app: &AppHandle) {
    let cfg = crate::system::config::load_config();
    if cfg.osd_enabled {
        if !cfg.use_classic_webview {
            ensure_companion_process();
        } else {
            crate::system::webview_orchestrator::WebviewOrchestrator::global().enqueue_osd();
        }
    }
}

pub async fn build_osd_window_async(app: &AppHandle) -> Result<String, String> {
    if app.get_webview_window("osd").is_some() {
        return Ok("osd".into());
    }

    let cfg = crate::system::config::load_config();
    let target_url = WebviewUrl::App("index.html".into());
    let init_script = "window.__OPENREVO_OSD_WINDOW__ = true; if (!window.location.hash.startsWith('#/osd')) { window.location.hash = '#/osd'; }";

    let builder = WebviewWindowBuilder::new(
        app,
        "osd",
        target_url,
    )
    .title("OpenRevo OSD")
    .inner_size(520.0, 96.0)
    .position(36.0, 36.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .initialization_script(init_script)
    .visible(false);

    match builder.build() {
        Ok(win) => {
            let _ = win.set_ignore_cursor_events(true);

            #[cfg(windows)]
            unsafe {
                use windows::Win32::Foundation::HWND;
                use windows::Win32::UI::WindowsAndMessaging::{
                    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE,
                    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
                };

                if let Ok(hwnd) = win.hwnd() {
                    let win_hwnd = HWND(hwnd.0 as _);
                    let cur_ex = GetWindowLongPtrW(win_hwnd, GWL_EXSTYLE);
                    let target_ex = cur_ex
                        | (WS_EX_NOACTIVATE.0 as isize)
                        | (WS_EX_TOOLWINDOW.0 as isize)
                        | (WS_EX_TOPMOST.0 as isize);

                    let _ = SetWindowLongPtrW(win_hwnd, GWL_EXSTYLE, target_ex);

                    update_osd_position(app, &cfg.osd_position);
                }
            }
            Ok("osd".into())
        }
        Err(e) => Err(format!("Failed to build OSD window: {}", e)),
    }
}

pub fn update_osd_position(app: &AppHandle, position_str: &str) {
    let cfg = crate::system::config::load_config();
    if !cfg.use_classic_webview {
        send_osd_ipc_message(serde_json::json!({
            "action": "config",
            "theme": cfg.osd_theme,
            "scale": cfg.osd_scale,
            "position": position_str,
        }));
    } else if let Some(win) = app.get_webview_window("osd") {
        let monitor_opt = win
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| win.primary_monitor().ok().flatten());

        if let Some(monitor) = monitor_opt {
            let scale_factor = monitor.scale_factor();
            let screen_w = monitor.size().width as f64 / scale_factor;
            let screen_h = monitor.size().height as f64 / scale_factor;
            let win_w = 520.0;
            let win_h = 96.0;

            let mon_pos = monitor.position().to_logical::<f64>(scale_factor);

            let (rel_x, rel_y) = match position_str {
                "bottom-center" | "bottom" => {
                    ((screen_w - win_w) / 2.0, screen_h - win_h - 70.0)
                }
                "center" => {
                    ((screen_w - win_w) / 2.0, (screen_h - win_h) / 2.0)
                }
                _ => (36.0, 36.0), // Default: "top-left"
            };

            let final_x = mon_pos.x + rel_x;
            let final_y = mon_pos.y + rel_y;

            let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(final_x, final_y)));

            crate::log_info!(
                "OSD",
                "OSD window repositioned to '{}' at logical ({:.1}, {:.1})",
                position_str,
                final_x,
                final_y
            );
        }
    }
}

pub fn wake_osd_if_enabled(app: &AppHandle) {
    let cfg = crate::system::config::load_config();
    if cfg.osd_enabled && cfg.use_classic_webview {
        present_osd_window(app);
    }
}

pub fn present_osd_window(app: &AppHandle) {
    let cfg = crate::system::config::load_config();
    if !cfg.use_classic_webview {
        send_osd_ipc_message(serde_json::json!({
            "action": "preview",
        }));
    } else if let Some(win) = app.get_webview_window("osd") {
        #[cfg(windows)]
        unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, HWND_TOPMOST,
                SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
            };
            if let Ok(hwnd) = win.hwnd() {
                let win_hwnd = HWND(hwnd.0 as _);
                let _ = SetWindowPos(
                    win_hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
        let _ = win.set_ignore_cursor_events(true);
    } else {
        crate::system::webview_orchestrator::WebviewOrchestrator::global().enqueue_osd();
    }
}

pub fn dismiss_osd_window(app: &AppHandle) {
    let cfg = crate::system::config::load_config();
    if cfg.use_classic_webview {
        if let Some(win) = app.get_webview_window("osd") {
            #[cfg(windows)]
            unsafe {
                use windows::Win32::Foundation::HWND;
                use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
                if let Ok(hwnd) = win.hwnd() {
                    let _ = ShowWindow(HWND(hwnd.0 as _), SW_HIDE);
                }
            }
        }
    }
}

pub fn show_power_mode(mode: u8) {
    send_osd_ipc_message(serde_json::json!({
        "action": "mode",
        "value": mode,
    }));
}

pub fn show_fan_boost(enabled: bool) {
    send_osd_ipc_message(serde_json::json!({
        "action": "fan",
        "value": enabled,
    }));
}

pub fn show_key_lock_toast(name: &str, locked: bool) {
    send_osd_ipc_message(serde_json::json!({
        "action": "key_lock",
        "name": name,
        "locked": locked,
    }));
}

pub fn show_device_switch(name: &str, enabled: bool) {
    send_osd_ipc_message(serde_json::json!({
        "action": "device_switch",
        "name": name,
        "enabled": enabled,
    }));
}

pub fn show_keyboard_brightness(level: u8) {
    send_osd_ipc_message(serde_json::json!({
        "action": "kb_brightness",
        "level": level,
    }));
}

pub fn show_keyboard_sleep_timeout(seconds: u32) {
    send_osd_ipc_message(serde_json::json!({
        "action": "kb_sleep",
        "seconds": seconds,
    }));
}

#[tauri::command]
pub fn show_osd_window(app: AppHandle) {
    present_osd_window(&app);
}

#[tauri::command]
pub fn hide_osd_window(app: AppHandle) {
    dismiss_osd_window(&app);
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct OsdConfigParam {
    pub enabled: Option<bool>,
    pub theme: Option<String>,
    pub scale: Option<String>,
    pub position: Option<String>,
}

#[tauri::command]
pub fn get_osd_config() -> Result<serde_json::Value, String> {
    let cfg = crate::system::config::load_config();
    Ok(serde_json::json!({
        "enabled": cfg.osd_enabled,
        "theme": cfg.osd_theme,
        "scale": cfg.osd_scale,
        "position": cfg.osd_position,
    }))
}

fn handle_set_osd(
    app: &AppHandle,
    config: Option<OsdConfigParam>,
    enabled: Option<bool>,
    theme: Option<String>,
    scale: Option<String>,
    position: Option<String>,
) -> Result<(), String> {
    let mut cfg = crate::system::config::load_config();

    let new_enabled = config
        .as_ref()
        .and_then(|c| c.enabled)
        .or(enabled)
        .unwrap_or(cfg.osd_enabled);

    let new_theme = config
        .as_ref()
        .and_then(|c| c.theme.clone())
        .or(theme)
        .unwrap_or(cfg.osd_theme);

    let new_scale = config
        .as_ref()
        .and_then(|c| c.scale.clone())
        .or(scale)
        .unwrap_or(cfg.osd_scale);

    let new_position = config
        .as_ref()
        .and_then(|c| c.position.clone())
        .or(position)
        .unwrap_or(cfg.osd_position);

    cfg.osd_enabled = new_enabled;
    cfg.osd_theme = new_theme.clone();
    cfg.osd_scale = new_scale.clone();
    cfg.osd_position = new_position.clone();
    crate::system::config::save_config(&cfg);

    if !cfg.use_classic_webview {
        send_osd_ipc_message(serde_json::json!({
            "action": "config",
            "theme": new_theme.clone(),
            "scale": new_scale.clone(),
            "position": new_position.clone(),
        }));

        if new_enabled {
            send_osd_ipc_message(serde_json::json!({
                "action": "preview",
            }));
        }
    } else {
        update_osd_position(app, &new_position);
        if new_enabled {
            present_osd_window(app);
        }
    }

    let payload = serde_json::json!({
        "enabled": new_enabled,
        "theme": new_theme,
        "scale": new_scale,
        "position": new_position,
    });
    let _ = app.emit("osd-config-changed", &payload);
    let _ = app.emit_to("osd", "osd-config-changed", &payload);

    crate::log_info!(
        "OSD",
        "Native OSD config saved: enabled={}, theme={}, scale={}, position={}",
        new_enabled,
        new_theme,
        new_scale,
        new_position
    );
    Ok(())
}

#[tauri::command]
pub fn set_osd_config(
    app: AppHandle,
    config: Option<OsdConfigParam>,
    enabled: Option<bool>,
    theme: Option<String>,
    scale: Option<String>,
    position: Option<String>,
) -> Result<(), String> {
    handle_set_osd(&app, config, enabled, theme, scale, position)
}

#[tauri::command]
pub fn save_osd_config(
    app: AppHandle,
    config: Option<OsdConfigParam>,
    enabled: Option<bool>,
    theme: Option<String>,
    scale: Option<String>,
    position: Option<String>,
) -> Result<(), String> {
    handle_set_osd(&app, config, enabled, theme, scale, position)
}

#[tauri::command]
pub fn trigger_osd_preview(app: AppHandle) -> Result<(), String> {
    let cfg = crate::system::config::load_config();
    if !cfg.use_classic_webview {
        send_osd_ipc_message(serde_json::json!({
            "action": "config",
            "theme": cfg.osd_theme,
            "scale": cfg.osd_scale,
            "position": cfg.osd_position,
        }));
        send_osd_ipc_message(serde_json::json!({
            "action": "preview",
        }));
    } else {
        update_osd_position(&app, &cfg.osd_position);
        present_osd_window(&app);
    }

    let payload = serde_json::json!({
        "theme": cfg.osd_theme,
        "scale": cfg.osd_scale,
        "position": cfg.osd_position,
    });
    let _ = app.emit("trigger-osd-preview", &payload);
    let _ = app.emit_to("osd", "trigger-osd-preview", &payload);
    Ok(())
}
