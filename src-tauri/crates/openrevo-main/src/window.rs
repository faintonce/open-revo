use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use slint::ComponentHandle;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, InvalidateRect, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, EnumThreadWindows, GetWindowLongPtrW, GetWindowRect,
    IsIconic, SetWindowLongPtrW, SetWindowPos,
    GWLP_WNDPROC, GWL_EXSTYLE, SC_MINIMIZE, SC_RESTORE,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER,
    WA_INACTIVE, WM_ACTIVATE, WM_EXITSIZEMOVE, WM_SYSCOMMAND,
    WM_WINDOWPOSCHANGED, WNDPROC, WS_EX_APPWINDOW,
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

pub fn set_slint_ui_weak(weak: slint::Weak<MainWindow>) {
    *SLINT_UI_WEAK.lock().unwrap() = Some(weak);
}

fn do_redraw_pass(hwnd: HWND) {
    unsafe {
        let _ = InvalidateRect(hwnd, None, true);
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
    do_redraw_pass(hwnd);

    let safe_hwnd = SafeHwnd(hwnd.0 as isize);
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        do_redraw_pass(safe_hwnd.to_hwnd());
    });
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        do_redraw_pass(safe_hwnd.to_hwnd());
    });
    slint::Timer::single_shot(Duration::from_millis(300), move || {
        do_redraw_pass(safe_hwnd.to_hwnd());
    });
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
                WAS_MINIMIZED.store(false, Ordering::SeqCst);
                let prev = *PREV_WNDPROC.lock().unwrap();
                let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                trigger_full_redraw(hwnd);
                return ret;
            }
        }
        WM_SYSCOMMAND => {
            let cmd = (wparam.0 & 0xFFF0) as u32;
            if cmd == SC_MINIMIZE {
                WAS_MINIMIZED.store(true, Ordering::SeqCst);
            } else if cmd == SC_RESTORE {
                WAS_MINIMIZED.store(false, Ordering::SeqCst);
                let prev = *PREV_WNDPROC.lock().unwrap();
                let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                trigger_full_redraw(hwnd);
                return ret;
            }
        }
        WM_ACTIVATE => {
            let active = (wparam.0 & 0xFFFF) as u32;
            if active != WA_INACTIVE {
                if WAS_MINIMIZED.swap(false, Ordering::SeqCst) || IsIconic(hwnd).as_bool() {
                    let prev = *PREV_WNDPROC.lock().unwrap();
                    let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                    trigger_full_redraw(hwnd);
                    return ret;
                }
            }
        }
        WM_WINDOWPOSCHANGED => {
            if WAS_MINIMIZED.load(Ordering::SeqCst) && !IsIconic(hwnd).as_bool() {
                WAS_MINIMIZED.store(false, Ordering::SeqCst);
                let prev = *PREV_WNDPROC.lock().unwrap();
                let ret = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
                trigger_full_redraw(hwnd);
                return ret;
            }
        }
        WM_EXITSIZEMOVE => {
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

extern "system" fn enum_thread_wnd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let ptr = lparam.0 as *mut Option<HWND>;
        *ptr = Some(hwnd);
        BOOL(0)
    }
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

pub fn align_mini_drawer_to_tray(hwnd: HWND, target_w: i32, target_h: i32) {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            let rc = mi.rcWork;
            let mut rect = RECT::default();
            let (real_w, real_h) = if GetWindowRect(hwnd, &mut rect).is_ok() && (rect.right - rect.left) > 100 {
                (rect.right - rect.left, rect.bottom - rect.top)
            } else {
                (target_w, target_h)
            };
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

pub fn expand_native_window(hwnd: HWND, target_w: i32, target_h: i32, saved_pos: Option<(i32, i32)>) {
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

