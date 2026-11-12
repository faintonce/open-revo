use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use slint::ComponentHandle;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, InvalidateRect, MonitorFromWindow, RedrawWindow, HRGN, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY, RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE,
    RDW_UPDATENOW,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetFocus, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, EnumThreadWindows, GetCursorPos, GetForegroundWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWLP_WNDPROC, GWL_EXSTYLE, SC_MINIMIZE, SC_RESTORE,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE,
    WA_INACTIVE, WM_ACTIVATE, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_MOVE,
    WM_SYSCOMMAND, WM_WINDOWPOSCHANGED, WNDPROC, WS_EX_APPWINDOW,
};

use crate::MainWindow;

#[derive(Clone, Copy, Debug)]
pub struct SafeHwnd(pub isize);
unsafe impl Send for SafeHwnd {}
unsafe impl Sync for SafeHwnd {}

impl SafeHwnd {
    pub fn to_hwnd(&self) -> HWND {
        HWND(self.0 as *mut _)
    }
}

pub(crate) static PREV_WNDPROC: Mutex<WNDPROC> = Mutex::new(None);
pub(crate) static WAS_MINIMIZED: AtomicBool = AtomicBool::new(false);
pub(crate) static SLINT_UI_WEAK: Mutex<Option<slint::Weak<MainWindow>>> = Mutex::new(None);
pub static HWND_HOLDER: Mutex<Option<SafeHwnd>> = Mutex::new(None);
pub static SAVED_FULL_POS: Mutex<Option<(i32, i32)>> = Mutex::new(None);
pub(crate) static LAST_REDRAW_INSTANT: Mutex<Option<Instant>> = Mutex::new(None);

pub static FOCUS_GENERATION: AtomicU64 = AtomicU64::new(0);
pub static LAST_WAKE_TIME: Mutex<Option<Instant>> = Mutex::new(None);
pub static LAST_MOVE_TIME: Mutex<Option<Instant>> = Mutex::new(None);
pub static HOST_PID: AtomicU32 = AtomicU32::new(0);

pub fn set_host_pid(pid: u32) {
    HOST_PID.store(pid, Ordering::SeqCst);
}

pub fn mark_window_woken() {
    FOCUS_GENERATION.fetch_add(1, Ordering::SeqCst);
    *LAST_WAKE_TIME.lock().unwrap() = Some(Instant::now());
}

pub fn mark_window_focused() {
    FOCUS_GENERATION.fetch_add(1, Ordering::SeqCst);
}

pub fn mark_window_moved() {
    *LAST_MOVE_TIME.lock().unwrap() = Some(Instant::now());
}

fn is_foreground_belonging_to_current_app(window_hwnd: HWND) -> bool {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.0.is_null() {
            return false;
        }
        if fg_hwnd == window_hwnd {
            return true;
        }
        let mut fg_pid = 0u32;
        GetWindowThreadProcessId(fg_hwnd, Some(&mut fg_pid));
        if fg_pid != 0 {
            if fg_pid == std::process::id() {
                return true;
            }
            let host_pid = HOST_PID.load(Ordering::SeqCst);
            if host_pid != 0 && fg_pid == host_pid {
                return true;
            }
        }
    }
    false
}

fn is_user_interacting_with_window(window_hwnd: HWND) -> bool {
    unsafe {
        let mut pt = POINT::default();
        let mut rect = RECT::default();
        if GetCursorPos(&mut pt).is_ok() && GetWindowRect(window_hwnd, &mut rect).is_ok() {
            let inside_rect = pt.x >= (rect.left - 12)
                && pt.x <= (rect.right + 12)
                && pt.y >= (rect.top - 12)
                && pt.y <= (rect.bottom + 12);

            let lbutton_down = (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0;
            let rbutton_down = (GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16 & 0x8000) != 0;

            if inside_rect && (lbutton_down || rbutton_down) {
                return true;
            }
        }
    }
    false
}

pub fn handle_window_blur(hwnd: HWND) {
    let (auto_hide, is_expanded) = if let Ok(guard) = SLINT_UI_WEAK.lock() {
        if let Some(weak) = guard.as_ref() {
            if let Some(ui) = weak.upgrade() {
                (ui.get_mini_auto_hide(), ui.get_is_expanded())
            } else {
                (false, false)
            }
        } else {
            (false, false)
        }
    } else {
        (false, false)
    };

    // 4. Panel mode check: Only auto-collapse in Mini Drawer mode, Full Panel stays untouched!
    if is_expanded || !auto_hide {
        return;
    }

    let current_gen = FOCUS_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let safe_hwnd = SafeHwnd(hwnd.0 as isize);

    // Spawn a debounced validation task (160ms) to filter out transient drag/click focus loss
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(160));
        let hwnd = safe_hwnd.to_hwnd();

        // 1. Generation check: Cancel if focus regained or new wake event occurred
        if FOCUS_GENERATION.load(Ordering::SeqCst) != current_gen {
            return;
        }

        // 2. Cooldown check: Cancel if recently woken (within 500ms)
        let is_recent_wake = LAST_WAKE_TIME
            .lock()
            .unwrap()
            .map_or(false, |t| t.elapsed() < Duration::from_millis(500));
        if is_recent_wake {
            return;
        }

        // 3. Movement check: Cancel if recently dragged or moved (within 400ms)
        let is_recent_move = LAST_MOVE_TIME
            .lock()
            .unwrap()
            .map_or(false, |t| t.elapsed() < Duration::from_millis(400));
        if is_recent_move {
            return;
        }

        // 4. Double check panel mode
        let is_expanded_now = if let Ok(guard) = SLINT_UI_WEAK.lock() {
            guard.as_ref().and_then(|w| w.upgrade()).map(|ui| ui.get_is_expanded()).unwrap_or(false)
        } else {
            false
        };
        if is_expanded_now {
            return;
        }

        // 5. Visibility / Minimized check
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                return;
            }
        }

        // 6. Direct focus check: If window is currently focused, do not collapse
        unsafe {
            if GetFocus() == hwnd {
                return;
            }
        }

        // 7. Win32 foreground check: If foreground window belongs to current process or host app, cancel collapse
        if is_foreground_belonging_to_current_app(hwnd) {
            return;
        }

        // 8. Active interaction check: If mouse button is held down over window (dragging/clicking), cancel collapse
        if is_user_interacting_with_window(hwnd) {
            return;
        }

        // All validation passed: True external blur detected. Safely collapse window.
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        crate::ipc::send_action_to_host("record_window_mode", serde_json::json!({ "is_expanded": false }));
        crate::ipc::send_action_to_host("slint_window_hidden", serde_json::json!({}));
        std::thread::sleep(Duration::from_millis(30));
        std::process::exit(0);
    });
}

pub fn set_slint_ui_weak(weak: slint::Weak<MainWindow>) {
    *SLINT_UI_WEAK.lock().unwrap() = Some(weak);
}

fn do_redraw_pass(hwnd: HWND) {
    unsafe {
        let _ = InvalidateRect(hwnd, None, true);
        let _ = RedrawWindow(
            hwnd,
            None,
            HRGN::default(),
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME | RDW_UPDATENOW,
        );
    }

    if let Ok(guard) = SLINT_UI_WEAK.lock() {
        if let Some(weak) = guard.as_ref() {
            let _ = weak.upgrade_in_event_loop(|ui| {
                ui.set_redraw_tick(ui.get_redraw_tick().wrapping_add(1));
                ui.window().request_redraw();
            });
        }
    }
}

pub fn trigger_full_redraw(hwnd: HWND) {
    {
        let mut last = LAST_REDRAW_INSTANT.lock().unwrap();
        let now = Instant::now();
        if let Some(prev) = *last {
            if now.duration_since(prev) < Duration::from_millis(200) {
                return;
            }
        }
        *last = Some(now);
    }

    do_redraw_pass(hwnd);
}

unsafe extern "system" fn subclass_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_SIZE => {
            use windows::Win32::UI::WindowsAndMessaging::{SIZE_MINIMIZED, SIZE_RESTORED};
            let resize_type = wparam.0 as u32;
            if resize_type == SIZE_MINIMIZED {
                WAS_MINIMIZED.store(true, Ordering::SeqCst);
            } else if resize_type == SIZE_RESTORED {
                let prev = *PREV_WNDPROC.lock().unwrap();
                let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                if WAS_MINIMIZED.swap(false, Ordering::SeqCst) || IsIconic(hwnd).as_bool() {
                    trigger_full_redraw(hwnd);
                }
                return ret;
            }
        }
        WM_SYSCOMMAND => {
            let cmd = (wparam.0 & 0xFFF0) as u32;
            if cmd == SC_MINIMIZE {
                WAS_MINIMIZED.store(true, Ordering::SeqCst);
            } else if cmd == SC_RESTORE {
                let prev = *PREV_WNDPROC.lock().unwrap();
                let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                if WAS_MINIMIZED.swap(false, Ordering::SeqCst) || IsIconic(hwnd).as_bool() {
                    trigger_full_redraw(hwnd);
                }
                return ret;
            }
        }
        WM_ACTIVATE => {
            let active = (wparam.0 & 0xFFFF) as u32;
            if active != WA_INACTIVE {
                mark_window_focused();
                let prev = *PREV_WNDPROC.lock().unwrap();
                let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                // Only trigger full redraw if this activation is recovering from minimized iconic state
                if WAS_MINIMIZED.swap(false, Ordering::SeqCst) || IsIconic(hwnd).as_bool() {
                    trigger_full_redraw(hwnd);
                }
                return ret;
            } else {
                handle_window_blur(hwnd);
            }
        }
        WM_MOVE | WM_ENTERSIZEMOVE => {
            mark_window_moved();
        }
        WM_WINDOWPOSCHANGED => {
            if !IsIconic(hwnd).as_bool() && WAS_MINIMIZED.swap(false, Ordering::SeqCst) {
                let prev = *PREV_WNDPROC.lock().unwrap();
                let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                trigger_full_redraw(hwnd);
                return ret;
            }
        }
        WM_EXITSIZEMOVE => {
            mark_window_moved();
            let is_expanded = if let Ok(guard) = SLINT_UI_WEAK.lock() {
                guard.as_ref().and_then(|w| w.upgrade()).map(|ui| ui.get_is_expanded()).unwrap_or(false)
            } else {
                false
            };
            if is_expanded {
                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
                    let x = rect.left;
                    let y = rect.top;
                    *SAVED_FULL_POS.lock().unwrap() = Some((x, y));
                    crate::ipc::send_action_to_host("save_full_window_pos", serde_json::json!({ "x": x, "y": y }));
                }
            }
        }
        _ => {}
    }

    let prev = *PREV_WNDPROC.lock().unwrap();
    CallWindowProcW(prev, hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn enum_thread_wnd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;

    let mut title = [0u16; 64];
    let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut title);
    let title_str = if len > 0 {
        String::from_utf16_lossy(&title[..len as usize])
    } else {
        String::new()
    };

    // Filter out 0x0 hidden message-only helper windows; accurately match OpenRevo title or desktop dimensions
    if title_str.eq_ignore_ascii_case("openrevo") || (w >= 300 && h >= 400) {
        let ptr = lparam.0 as *mut Option<HWND>;
        *ptr = Some(hwnd);
        return BOOL(0);
    }

    BOOL(1)
}

pub fn find_thread_hwnd() -> Option<HWND> {
    unsafe {
        let tid = GetCurrentThreadId();
        let mut target_hwnd: Option<HWND> = None;
        let _ = EnumThreadWindows(
            tid,
            Some(enum_thread_wnd_proc),
            LPARAM(&mut target_hwnd as *mut _ as isize),
        );
        target_hwnd
    }
}

pub fn apply_window_decorations(hwnd: HWND) {
    unsafe {
        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        ex_style |= WS_EX_APPWINDOW.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);

        // Enable DWM rounded corners
        let preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as _,
            std::mem::size_of_val(&preference) as u32,
        );

        // Extend frame into client area for proper DWM shadows
        let margins = MARGINS {
            cxLeftWidth: 1,
            cxRightWidth: 1,
            cyTopHeight: 1,
            cyBottomHeight: 1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        // Subclass window procedure to capture taskbar minimize/restore lifecycle
        let mut prev_guard = PREV_WNDPROC.lock().unwrap();
        if prev_guard.is_none() {
            let prev_addr = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            if prev_addr != 0 {
                let prev_fn: WNDPROC = std::mem::transmute(prev_addr);
                *prev_guard = prev_fn;
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, subclass_wnd_proc as *const () as usize as isize);
            }
        }
    }
}

#[link(name = "user32")]
extern "system" {
    fn GetDpiForWindow(hwnd: HWND) -> u32;
}

pub fn get_window_scale_factor(hwnd: HWND) -> f64 {
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        if dpi > 0 {
            return dpi as f64 / 96.0;
        }
    }
    if let Ok(guard) = SLINT_UI_WEAK.lock() {
        if let Some(weak) = guard.as_ref() {
            if let Some(ui) = weak.upgrade() {
                let s = ui.window().scale_factor();
                if s > 0.1 {
                    return s as f64;
                }
            }
        }
    }
    1.0
}

pub fn align_mini_drawer_to_tray(hwnd: HWND, target_logical_w: i32, target_logical_h: i32) {
    let scale = get_window_scale_factor(hwnd);
    let target_w = (target_logical_w as f64 * scale).round() as i32;
    let target_h = (target_logical_h as f64 * scale).round() as i32;

    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            let rc = mi.rcWork;
            let real_w = target_w;
            let real_h = target_h;
            let margin = 16;
            let x = rc.right - real_w - margin;
            let y = rc.bottom - real_h - margin;

            let _ = SetWindowPos(
                hwnd,
                HWND(std::ptr::null_mut()),
                x,
                y,
                real_w,
                real_h,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

pub fn expand_native_window(hwnd: HWND, target_logical_w: i32, target_logical_h: i32, saved_pos: Option<(i32, i32)>) {
    let scale = get_window_scale_factor(hwnd);
    let target_w = (target_logical_w as f64 * scale).round() as i32;
    let target_h = (target_logical_h as f64 * scale).round() as i32;

    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            let rc = mi.rcWork;
            let screen_w = rc.right - rc.left;
            let screen_h = rc.bottom - rc.top;

            let (target_x, target_y) = if let Some((sx, sy)) = saved_pos {
                let overlap_x = (sx + target_w).min(rc.right) - sx.max(rc.left);
                let overlap_y = (sy + target_h).min(rc.bottom) - sy.max(rc.top);
                if overlap_x >= 200 && overlap_y >= 100 {
                    (sx, sy)
                } else {
                    (rc.left + (screen_w - target_w) / 2, rc.top + (screen_h - target_h) / 2)
                }
            } else {
                (rc.left + (screen_w - target_w) / 2, rc.top + (screen_h - target_h) / 2)
            };

            let _ = SetWindowPos(
                hwnd,
                HWND(std::ptr::null_mut()),
                target_x,
                target_y,
                target_w,
                target_h,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

