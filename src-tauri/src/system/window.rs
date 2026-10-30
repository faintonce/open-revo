// OpenRevo - Window Lifecycle & Desktop Drawer Management
// Handles window alignment, tray overflow anti-flicker hack, and window events.

use tauri::{Emitter, Manager, WebviewWindow, Window, WindowEvent};

#[cfg(windows)]
pub fn dismiss_windows_tray_overflow() {
    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, IsWindowVisible, PostMessageW, ShowWindow, SW_HIDE, WM_CLOSE,
    };

    unsafe {
        // Windows 10 & 11 Taskbar notification overflow tray window class name is strictly "NotifyIconOverflowWindow"
        if let Ok(hwnd) = FindWindowW(w!("NotifyIconOverflowWindow"), None) {
            if !hwnd.0.is_null() && IsWindowVisible(hwnd).as_bool() {
                // Post close message and hide to dismiss the popup tray cleanly
                let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}

#[cfg(not(windows))]
pub fn dismiss_windows_tray_overflow() {}

#[cfg(windows)]
pub fn is_point_in_notify_icon_overflow(cursor: windows::Win32::Foundation::POINT) -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect, IsWindowVisible};

    unsafe {
        if let Ok(hwnd) = FindWindowW(w!("NotifyIconOverflowWindow"), None) {
            if !hwnd.0.is_null() && IsWindowVisible(hwnd).as_bool() {
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    return cursor.x >= rect.left
                        && cursor.x <= rect.right
                        && cursor.y >= rect.top
                        && cursor.y <= rect.bottom;
                }
            }
        }
    }
    false
}

#[cfg(not(windows))]
pub fn is_point_in_notify_icon_overflow(_cursor: ()) -> bool {
    false
}

/// Aligns the mini drawer window precisely above the Windows taskbar on the active monitor.
pub fn align_window_to_tray(window: &WebviewWindow) {
    let monitor_opt = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());

    if let Some(monitor) = monitor_opt {
        let screen_size = monitor.size();
        let screen_pos = monitor.position();
        let scale = monitor.scale_factor();
        let (win_w, win_h) = if let Ok(sz) = window.inner_size() {
            (sz.width as i32, sz.height as i32)
        } else {
            let mini_h = crate::commands::window_cmd::get_last_mini_height();
            ((395.0 * scale) as i32, (mini_h * scale) as i32)
        };
        let margin = (16.0 * scale) as i32;
        let taskbar_pad = (56.0 * scale) as i32;
        let x = screen_pos.x + screen_size.width as i32 - win_w - margin;
        let y = screen_pos.y + screen_size.height as i32 - win_h - taskbar_pad;
        crate::log_info!(
            "WINDOW",
            "Positioning window: screen_pos=({}, {}), screen_size=({}, {}), target=({}, {})",
            screen_pos.x,
            screen_pos.y,
            screen_size.width,
            screen_size.height,
            x,
            y
        );
        if x < screen_pos.x || y < screen_pos.y {
            crate::log_warn!(
                "WINDOW",
                "Target coordinates ({}, {}) off-bounds! Centering window instead.",
                x,
                y
            );
            let _ = window.center();
        } else {
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
        }
    } else {
        crate::log_warn!("WINDOW", "No monitor detected! Centering window.");
        let _ = window.center();
    }
}

/// Cooldown timestamp tracking the most recent window wake/show to prevent focus bounce
static LAST_WAKE_TIME: parking_lot::Mutex<Option<std::time::Instant>> = parking_lot::Mutex::new(None);

/// Cooldown timestamp tracking the most recent window drag/move to prevent collapse during movement
static LAST_MOVE_TIME: parking_lot::Mutex<Option<std::time::Instant>> = parking_lot::Mutex::new(None);

/// Atomic generation counter for focus events to prevent race conditions during debounce
static FOCUS_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(windows)]
fn is_foreground_belonging_to_current_app(window_hwnd: Option<isize>) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.0.is_null() {
            return false;
        }

        // 1. Direct HWND match
        if let Some(h) = window_hwnd {
            if fg_hwnd == HWND(h as _) {
                return true;
            }
        }

        // 2. Process ID match (covers WebView2 child hwnd or any sub-windows of OpenRevo)
        let mut fg_pid = 0u32;
        GetWindowThreadProcessId(fg_hwnd, Some(&mut fg_pid));
        if fg_pid != 0 && fg_pid == std::process::id() {
            return true;
        }
    }
    false
}

#[cfg(not(windows))]
fn is_foreground_belonging_to_current_app(_window_hwnd: Option<isize>) -> bool {
    false
}

#[cfg(windows)]
fn is_user_interacting_with_window(window_hwnd: Option<isize>) -> bool {
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

    unsafe {
        if let Some(h) = window_hwnd {
            let hwnd = HWND(h as _);
            let mut pt = POINT::default();
            let mut rect = RECT::default();
            if GetCursorPos(&mut pt).is_ok() && GetWindowRect(hwnd, &mut rect).is_ok() {
                // Expanding tolerance rect slightly (+12px) to handle high-speed drag cursor lags
                let inside_rect = pt.x >= (rect.left - 12)
                    && pt.x <= (rect.right + 12)
                    && pt.y >= (rect.top - 12)
                    && pt.y <= (rect.bottom + 12);

                let lbutton_down = (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0;
                let rbutton_down = (GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16 & 0x8000) != 0;

                // If mouse button is held down over/near the window, user is actively dragging or clicking inside
                if inside_rect && (lbutton_down || rbutton_down) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(not(windows))]
fn is_user_interacting_with_window(_window_hwnd: Option<isize>) -> bool {
    false
}

pub fn mark_window_woken() {
    FOCUS_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    *LAST_WAKE_TIME.lock() = Some(std::time::Instant::now());
}

/// Restores the main window, dismisses the tray overflow flyout, and synchronizes mini/expanded layout.
pub fn restore_and_sync_window(window: &WebviewWindow) {
    mark_window_woken();
    #[cfg(windows)]
    {
        dismiss_windows_tray_overflow();
        // Asynchronously dismiss again after 80ms & 200ms to override Windows DWM deferred tray repaint
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(80));
            dismiss_windows_tray_overflow();
            std::thread::sleep(std::time::Duration::from_millis(150));
            dismiss_windows_tray_overflow();
        });
    }

    // Step 1: Determine presentation mode and apply resize/sync while window is still hidden
    let cfg = crate::system::config::load_config();
    let is_expanded = match cfg.wake_window_mode.as_str() {
        "full" => true,
        "mini" => false,
        _ => is_window_expanded(window),
    };
    crate::system::tray::update_tray_window_mode(is_expanded);

    if is_expanded {
        let _ = window.emit("sync-window-mode", true);
        let _ = crate::commands::window_cmd::resize_window(window.app_handle().clone(), true);
    } else {
        let _ = window.emit("reset-to-mini", ());
        let _ = window.emit("sync-window-mode", false);
        let _ = crate::commands::window_cmd::resize_window(window.app_handle().clone(), false);
    }

    // Step 2: Window coordinates and dimensions are now fully prepared behind the scenes. Present to user.
    if window.label() == "main" {
        crate::system::telemetry::set_main_window_visible(true);
    }
    let show_res = window.show();
    let unmin_res = window.unminimize();
    let top_res = window.set_always_on_top(true);
    let focus_res = window.set_focus();
    let _ = window.set_skip_taskbar(false);

    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::{
            AllowSetForegroundWindow, GetWindowRect, IsWindow, IsWindowVisible, SetForegroundWindow,
            ShowWindow, ASFW_ANY, SW_RESTORE, SW_SHOW,
        };

        let _ = AllowSetForegroundWindow(ASFW_ANY);

        if let Ok(hwnd) = window.hwnd() {
            let win_hwnd = windows::Win32::Foundation::HWND(hwnd.0 as _);
            let is_win = IsWindow(win_hwnd).as_bool();
            let is_vis = IsWindowVisible(win_hwnd).as_bool();
            let mut rect = RECT::default();
            let _ = GetWindowRect(win_hwnd, &mut rect);
            crate::log_info!(
                "WINDOW",
                "Win32 HWND: 0x{:X}, IsWindow={}, IsVisible={}, Rect=[{},{},{},{}], show_res={:?}, unmin_res={:?}, top_res={:?}, focus_res={:?}",
                hwnd.0 as usize, is_win, is_vis, rect.left, rect.top, rect.right, rect.bottom,
                show_res, unmin_res, top_res, focus_res
            );

            let _ = ShowWindow(win_hwnd, SW_RESTORE);
            let _ = ShowWindow(win_hwnd, SW_SHOW);
            let _ = SetForegroundWindow(win_hwnd);
        } else {
            crate::log_error!("WINDOW", "Failed to retrieve HWND from WebviewWindow!");
        }
    }

    let _ = window.eval("window.focus && window.focus()");

    // Release always-on-top so the mini window behaves naturally with user focus
    let win_clone = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = win_clone.set_always_on_top(false);
    });
}

/// Checks whether the window is currently in expanded full panel mode.
pub fn is_window_expanded(window: &WebviewWindow) -> bool {
    let scale = window.scale_factor().unwrap_or(1.0);
    if let Ok(sz) = window.inner_size() {
        (sz.width as f64 / scale) > 600.0
    } else {
        false
    }
}

/// Explicitly switches window to Full Panel mode and updates tray checkmarks.
pub fn switch_to_full_panel(app: &tauri::AppHandle) {
    if let Some(window) = ensure_window_alive(app) {
        crate::system::telemetry::set_main_window_visible(true);
        let _ = crate::commands::window_cmd::resize_window(app.clone(), true);
        let _ = window.emit("sync-window-mode", true);
        crate::system::tray::update_tray_window_mode(true);
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.set_skip_taskbar(false);
    }
}

/// Explicitly switches window to Mini Drawer mode and updates tray checkmarks.
pub fn switch_to_mini_drawer(app: &tauri::AppHandle) {
    if let Some(window) = ensure_window_alive(app) {
        crate::system::telemetry::set_main_window_visible(true);
        let _ = crate::commands::window_cmd::resize_window(app.clone(), false);
        let _ = window.emit("reset-to-mini", ());
        let _ = window.emit("sync-window-mode", false);
        crate::system::tray::update_tray_window_mode(false);
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.set_skip_taskbar(false);
        mark_window_woken();
    }
}

/// Safely retrieves the main WebviewWindow instance if alive.
pub fn ensure_window_alive(app: &tauri::AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// Toggles between hiding and restoring the mini drawer window.
pub fn show_or_toggle_drawer(app: &tauri::AppHandle) {
    if let Some(window) = ensure_window_alive(app) {
        let is_min = window.is_minimized().unwrap_or(false);
        let is_vis = window.is_visible().unwrap_or(false);
        if is_vis && !is_min {
            let _ = window.hide();
            crate::system::telemetry::set_main_window_visible(false);
        } else {
            restore_and_sync_window(&window);
        }
    }
}

/// Setup helper to initialize window dimensions, position, and saved coordinate state on boot.
pub fn init_main_window(window: &WebviewWindow) {
    crate::log_info!("WINDOW", "Startup: 'main' WebviewWindow successfully initialized");
    let cfg = crate::system::config::load_config();
    if let (Some(x), Some(y)) = (cfg.full_window_x, cfg.full_window_y) {
        crate::commands::hardware_cmd::update_full_window_pos(x, y);
    }

    if cfg.wake_window_mode == "full" {
        let _ = crate::commands::window_cmd::resize_window(window.app_handle().clone(), true);
    } else {
        let mini_h = crate::commands::window_cmd::get_last_mini_height();
        let _ = window.set_size(tauri::LogicalSize::new(395.0, mini_h));
        align_window_to_tray(window);
    }
}

/// Focus lost guard hook placeholder for future auto-collapse policies.
pub fn setup_window_focus_lost_handler(_window: &WebviewWindow) {
    // Focus loss hook reserved for optional auto-hide behavior
}

/// Centralized window event processor handling move coordinates persistence and close cancellation.
pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    match event {
        WindowEvent::Moved(pos) => {
            *LAST_MOVE_TIME.lock() = Some(std::time::Instant::now());
            let label = window.label();
            if label.starts_with("widget_") {
                let plugin_id = label.trim_start_matches("widget_").replace('_', "-");
                crate::system::widget_manager::queue_widget_position(plugin_id, pos.x, pos.y);
                return;
            }
            let scale = window.scale_factor().unwrap_or(1.0);
            if let Ok(sz) = window.inner_size() {
                if (sz.width as f64 / scale) > 600.0 {
                    crate::commands::hardware_cmd::update_full_window_pos(pos.x, pos.y);
                }
            }
        }
        WindowEvent::Focused(focused) => {
            if *focused {
                FOCUS_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            } else if window.label() == "main" {
                let cfg = crate::system::config::load_config();
                if cfg.mini_auto_hide_on_blur {
                    let current_gen = FOCUS_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    let win = window.clone();

                    // Spawn a debounced validation task (160ms) to filter out transient drag/click focus loss
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(160));

                        // 1. Generation check: Cancel if focus regained or new wake event occurred
                        if FOCUS_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != current_gen {
                            return;
                        }

                        // 2. Cooldown check: Cancel if recently woken (within 500ms)
                        let is_recent_wake = LAST_WAKE_TIME
                            .lock()
                            .map_or(false, |t| t.elapsed() < std::time::Duration::from_millis(500));
                        if is_recent_wake {
                            return;
                        }

                        // 3. Movement check: Cancel if recently dragged or moved (within 400ms)
                        let is_recent_move = LAST_MOVE_TIME
                            .lock()
                            .map_or(false, |t| t.elapsed() < std::time::Duration::from_millis(400));
                        if is_recent_move {
                            return;
                        }

                        // 4. Panel mode check: Only auto-collapse in Mini Drawer mode, Full Panel stays untouched!
                        let scale = win.scale_factor().unwrap_or(1.0);
                        let is_expanded = if let Ok(sz) = win.inner_size() {
                            (sz.width as f64 / scale) > 600.0
                        } else {
                            false
                        };
                        if is_expanded {
                            return;
                        }

                        // 5. Visibility / Minimized check
                        if !win.is_visible().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
                            return;
                        }

                        // 6. Direct focus check: If window is currently focused, do not collapse
                        if win.is_focused().unwrap_or(false) {
                            return;
                        }

                        // 7. Win32 HWND resolution
                        let win_hwnd = win.app_handle().get_webview_window(win.label())
                            .and_then(|w| w.hwnd().ok())
                            .map(|h| h.0 as isize);

                        // 8. Win32 foreground check: If foreground window belongs to current process, cancel collapse
                        if is_foreground_belonging_to_current_app(win_hwnd) {
                            return;
                        }

                        // 9. Active interaction check: If mouse button is held down over window (dragging/clicking), cancel collapse
                        if is_user_interacting_with_window(win_hwnd) {
                            return;
                        }

                        // All validation passed: True external blur detected. Safely collapse window.
                        let _ = win.hide();
                        crate::system::telemetry::set_main_window_visible(false);
                        crate::log_info!(
                            "WINDOW",
                            "Mini drawer automatically collapsed on blur (focus confirmed transferred outside)"
                        );
                    });
                }
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            let label = window.label();
            if label.starts_with("widget_") {
                // Secondary widget or standalone pop-out tab window: allow standard OS/user termination!
                // Do NOT prevent close here. Let OS & Tauri destroy the window naturally.
                let plugin_id = label.trim_start_matches("widget_").replace('_', "-");
                let app = window.app_handle().clone();
                crate::system::widget_manager::cleanup_widget_on_close(&app, &plugin_id);
                crate::log_info!("WINDOW", "Widget window '{}' close granted and cleaned up smoothly.", label);
                return;
            }

            api.prevent_close();
            let scale = window.scale_factor().unwrap_or(1.0);
            if let Ok(sz) = window.inner_size() {
                if (sz.width as f64 / scale) > 600.0 {
                    if let Ok(pos) = window.outer_position() {
                        crate::commands::hardware_cmd::save_full_window_pos(pos.x, pos.y);
                    }
                }
            }
            let _ = window.hide();
            if window.label() == "main" {
                crate::system::telemetry::set_main_window_visible(false);
            }
        }
        _ => {}
    }
}
