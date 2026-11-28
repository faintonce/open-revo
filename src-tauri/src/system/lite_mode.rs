// OpenRevo - Lite Mode (轻量模式)
// Opt-in background WebView2 reclamation for the main UI.
//
// Problem: hiding the main UI never frees its WebView2 runtime. In classic
// mode `CloseRequested` only hides the `main` WebviewWindow, and in Slint mode
// the embedded WebView2 plugin view is only dropped when the user switches tabs
// or collapses the dashboard — not when the whole dashboard is hidden. A single
// idle WebView2 group (7 x msedgewebview2.exe) was measured at 182.4 MB dedicated
// working set / 437.1 MB working set / 410.2 MB commit / 105.7 MB dedicated GPU
// memory, ~10x the 18.5 MB host process, while doing 0% CPU and 0 network I/O.
//
// When lite mode is enabled, arming starts when the main UI becomes hidden and
// the WebView2 surfaces are destroyed after `lite_mode_idle_secs` of continuous
// hidden time. Showing the UI again disarms any pending timer and recreates the
// surfaces on demand through the existing paths (`ensure_window_alive` for
// classic, `attach_or_switch_embedded` for the Slint embedded view).
//
// Deliberately NOT destroyed: desktop widgets / popouts (user-visible), the OSD,
// plugin background services, the Watcher Hub and the telemetry loop. Only the
// main UI's own WebView2 surfaces are reclaimed.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// Minimum idle delay before background WebView2 surfaces are destroyed.
pub const LITE_MODE_IDLE_MIN_SECS: u64 = 60;
/// Maximum idle delay before background WebView2 surfaces are destroyed.
pub const LITE_MODE_IDLE_MAX_SECS: u64 = 3600;
/// Default idle delay (5 minutes): long enough to survive alt-tabbing away and
/// back, short enough that a tray-parked UI does not hold Chromium all day.
pub const LITE_MODE_IDLE_DEFAULT_SECS: u64 = 300;

/// Generation counter: every arm/disarm bumps it, so a sleeper from an older
/// hide can never fire after the UI has been shown again.
static LITE_GENERATION: AtomicU64 = AtomicU64::new(0);
/// Last AppHandle seen by `arm`, for the Tauri-side destroy step. `None` in the
/// pure Slint daemon runtime, where only the native-wry path applies.
static LITE_APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

/// Clamp a user-supplied idle delay into the supported range.
pub fn clamp_idle_secs(secs: u64) -> u64 {
    secs.clamp(LITE_MODE_IDLE_MIN_SECS, LITE_MODE_IDLE_MAX_SECS)
}

pub fn is_enabled() -> bool {
    crate::system::config::load_config().lite_mode
}

pub fn idle_timeout() -> Duration {
    Duration::from_secs(clamp_idle_secs(
        crate::system::config::load_config().lite_mode_idle_secs,
    ))
}

/// Re-check at fire time: is the main UI still hidden? Anything unknown counts
/// as "not hidden" so we never destroy a visible UI.
fn is_main_ui_hidden(app: Option<&AppHandle>) -> bool {
    let cfg = crate::system::config::load_config();
    if cfg.use_classic_webview {
        match app.and_then(|a| a.get_webview_window("main")) {
            // Already destroyed (e.g. by an earlier lite-mode run): nothing to do.
            None => true,
            Some(win) => match (win.is_visible(), win.is_minimized()) {
                (Ok(visible), Ok(minimized)) => !visible || minimized,
                _ => false,
            },
        }
    } else {
        // Slint mode: the companion process exits whenever its window is hidden,
        // so "companion not running" is the host-side hidden signal.
        !crate::system::slint_window::is_slint_main_running()
    }
}

/// Arm the idle timer. Called whenever the main UI becomes hidden. A no-op
/// when lite mode is off.
pub fn arm(app: Option<AppHandle>) {
    if !is_enabled() {
        return;
    }
    if let Some(handle) = app {
        *LITE_APP_HANDLE.lock() = Some(handle);
    }
    let generation = LITE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let timeout = idle_timeout();
    std::thread::Builder::new()
        .name("openrevo-lite-mode".into())
        .spawn(move || {
            std::thread::sleep(timeout);
            // The UI was shown again (or re-armed) since this timer started.
            if LITE_GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            let stored = LITE_APP_HANDLE.lock().clone();
            if !is_main_ui_hidden(stored.as_ref()) {
                return;
            }
            crate::log_info!(
                "LITE_MODE",
                "Main UI hidden for {:?}; destroying background WebView2 surfaces.",
                timeout
            );
            destroy_background_webviews(stored.as_ref());
        })
        .ok();
}

/// Cancel any pending idle timer. Called whenever the main UI is shown again.
pub fn disarm() {
    LITE_GENERATION.fetch_add(1, Ordering::SeqCst);
}

/// Destroy the main UI's background WebView2 surfaces, and nothing else.
/// Safe to call when nothing exists: every step is a no-op then.
pub fn destroy_background_webviews(app: Option<&AppHandle>) {
    // 1. Slint embedded WebView2 plugin view (native wry child). This also
    // remembers the plugin id so it can be re-attached when the UI returns.
    crate::system::native_wry::destroy_embedded_for_lite_mode();

    let Some(handle) = app else {
        return;
    };

    // 2. Legacy Tauri embedded dock (`embedded_plugin_host`). Its attach path is
    // currently unused, but destroy defensively in case a window exists.
    crate::system::slint_window::destroy_embedded_webview(handle);

    // 3. Classic mode: the whole main UI *is* a WebView2 window. Destroy it;
    // the next tray click recreates it via `ensure_window_alive`.
    if crate::system::config::load_config().use_classic_webview {
        if let Some(win) = handle.get_webview_window("main") {
            let _ = win.destroy();
            crate::log_info!(
                "LITE_MODE",
                "Classic 'main' WebviewWindow destroyed; it will be recreated on next tray click."
            );
        }
    }
}

/// Re-attach the embedded WebView2 plugin view destroyed by lite mode, if any.
/// Called when the Slint main window is shown (or expanded) again. Best effort:
/// if the Slint HWND is not resolvable yet, or the window is only a mini drawer
/// (which has no plugin area), the plugin id is kept for the next show/expand.
pub fn restore_embedded_if_needed() {
    if !is_enabled() {
        return;
    }
    let Some(plugin_id) = crate::system::native_wry::take_lite_mode_embedded_plugin() else {
        return;
    };
    #[cfg(target_os = "windows")]
    {
        let parent_hwnd_raw = crate::system::slint_window::get_openrevo_main_hwnd();
        if parent_hwnd_raw == 0 {
            crate::system::native_wry::remember_lite_mode_embedded_plugin(&plugin_id);
            crate::log_info!(
                "LITE_MODE",
                "Slint main HWND not ready; deferred lite-mode restore for '{}'.",
                plugin_id
            );
            return;
        }
        // Only restore into an expanded dashboard; a mini drawer has no plugin area.
        let cfg = crate::system::config::load_config();
        let expanded = match cfg.wake_window_mode.as_str() {
            "full" => true,
            "mini" => false,
            _ => cfg.last_window_expanded,
        };
        if !expanded {
            crate::system::native_wry::remember_lite_mode_embedded_plugin(&plugin_id);
            crate::log_info!(
                "LITE_MODE",
                "Mini drawer shown; deferred lite-mode restore for '{}' until expanded.",
                plugin_id
            );
            return;
        }
        let parent_hwnd = windows::Win32::Foundation::HWND(parent_hwnd_raw as *mut _);
        let (x, y, w, h, _) = crate::system::slint_window::calc_embedded_geometry(parent_hwnd);
        crate::system::native_wry::attach_or_switch_embedded(
            parent_hwnd_raw,
            &plugin_id,
            (x, y, w, h),
        );
        crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
            "action": "set_active_plugin_id",
            "plugin_id": plugin_id,
        }));
        crate::log_info!(
            "LITE_MODE",
            "Re-attached lite-mode plugin '{}' after main window shown.",
            plugin_id
        );
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = plugin_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_delay_is_clamped_to_supported_range() {
        assert_eq!(clamp_idle_secs(0), LITE_MODE_IDLE_MIN_SECS);
        assert_eq!(clamp_idle_secs(30), LITE_MODE_IDLE_MIN_SECS);
        assert_eq!(clamp_idle_secs(300), 300);
        assert_eq!(clamp_idle_secs(3600), LITE_MODE_IDLE_MAX_SECS);
        assert_eq!(clamp_idle_secs(u64::MAX), LITE_MODE_IDLE_MAX_SECS);
    }

    #[test]
    fn default_idle_delay_is_five_minutes() {
        assert_eq!(LITE_MODE_IDLE_DEFAULT_SECS, 300);
        assert_eq!(
            clamp_idle_secs(LITE_MODE_IDLE_DEFAULT_SECS),
            LITE_MODE_IDLE_DEFAULT_SECS
        );
    }
}
