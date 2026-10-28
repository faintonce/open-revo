use std::io::Write;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

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


/// Initialize the native Slint OSD companion process
pub fn init_osd_window(_app: &AppHandle) {
    let cfg = crate::system::config::load_config();
    if cfg.osd_enabled {
        ensure_companion_process();
    }
}

pub async fn build_osd_window_async(_app: &AppHandle) -> Result<String, String> {
    Ok("osd".into())
}

pub fn update_osd_position(_app: &AppHandle, position_str: &str) {
    let cfg = crate::system::config::load_config();
    send_osd_ipc_message(serde_json::json!({
        "action": "config",
        "theme": cfg.osd_theme,
        "scale": cfg.osd_scale,
        "position": position_str,
    }));
}

pub fn wake_osd_if_enabled(_app: &AppHandle) {
    // Zero latency: triggered instantly via direct stdin message
}

pub fn present_osd_window(_app: &AppHandle) {
    send_osd_ipc_message(serde_json::json!({
        "action": "preview",
    }));
}

pub fn dismiss_osd_window(_app: &AppHandle) {}

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

    let payload = serde_json::json!({
        "enabled": new_enabled,
        "theme": new_theme,
        "scale": new_scale,
        "position": new_position,
    });
    let _ = app.emit("osd-config-changed", &payload);

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
    send_osd_ipc_message(serde_json::json!({
        "action": "config",
        "theme": cfg.osd_theme,
        "scale": cfg.osd_scale,
        "position": cfg.osd_position,
    }));
    send_osd_ipc_message(serde_json::json!({
        "action": "preview",
    }));

    let payload = serde_json::json!({
        "theme": cfg.osd_theme,
        "scale": cfg.osd_scale,
        "position": cfg.osd_position,
    });
    let _ = app.emit("trigger-osd-preview", &payload);
    Ok(())
}
