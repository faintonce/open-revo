use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use slint::{ComponentHandle, Model};
pub use slint;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, EnumThreadWindows, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect,
    IsIconic, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWLP_WNDPROC, GWL_EXSTYLE,
    SC_MINIMIZE, SC_RESTORE, SM_CXSCREEN, SM_CYSCREEN, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WA_INACTIVE, WM_ACTIVATE,
    WM_SYSCOMMAND, WM_WINDOWPOSCHANGED, WNDPROC, WS_EX_APPWINDOW,
};

pub mod ipc;

slint::include_modules!();

#[derive(Clone, Copy, Debug)]
pub struct SafeHwnd(pub isize);
unsafe impl Send for SafeHwnd {}
unsafe impl Sync for SafeHwnd {}

impl SafeHwnd {
    pub fn to_hwnd(&self) -> HWND {
        HWND(self.0 as *mut _)
    }
}

static PREV_WNDPROC: Mutex<WNDPROC> = Mutex::new(None);
static WAS_MINIMIZED: AtomicBool = AtomicBool::new(false);
static SLINT_UI_WEAK: Mutex<Option<slint::Weak<MainWindow>>> = Mutex::new(None);

fn do_redraw_pass(hwnd: HWND) {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            if w > 100 && h > 100 {
                // Micro-nudge window size to force winit/Slint software renderer
                // to completely invalidate and recreate the backing frame buffer and mark full dirty region
                let _ = SetWindowPos(
                    hwnd,
                    HWND(std::ptr::null_mut()),
                    rect.left,
                    rect.top,
                    w + 1,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
                let _ = SetWindowPos(
                    hwnd,
                    HWND(std::ptr::null_mut()),
                    rect.left,
                    rect.top,
                    w,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
        }
        let _ = InvalidateRect(hwnd, None, true);
    }

    if let Ok(guard) = SLINT_UI_WEAK.lock() {
        if let Some(weak) = guard.as_ref() {
            let _ = weak.upgrade_in_event_loop(|ui| {
                ui.window().request_redraw();
            });
        }
    }
}

fn trigger_full_redraw(hwnd: HWND) {
    do_redraw_pass(hwnd);

    // Re-trigger after 25ms to ensure Windows DWM animation and geometry restoration is fully settled
    let safe_hwnd = SafeHwnd(hwnd.0 as isize);
    slint::Timer::single_shot(Duration::from_millis(25), move || {
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

fn find_thread_hwnd() -> Option<HWND> {
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

fn apply_window_decorations(hwnd: HWND) {
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

fn resize_native_window(hwnd: HWND, target_w: i32, target_h: i32) {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let mut left = rect.left;
            let mut top = rect.top;

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);

            if left + target_w > screen_w {
                left = (screen_w - target_w - 20).max(10);
            }
            if top + target_h > screen_h {
                top = (screen_h - target_h - 40).max(10);
            }

            let _ = SetWindowPos(
                hwnd,
                HWND(std::ptr::null_mut()),
                left,
                top,
                target_w,
                target_h,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

fn apply_monotonic_fan_rule(curve: &mut [i32], idx: usize, target_duty: i32, min_duty: i32) {
    if idx >= curve.len() {
        return;
    }
    let target = target_duty.clamp(min_duty, 100);
    curve[idx] = target;

    // Monotonic Rule 1: Points after current index cannot have lower duty (push higher)
    for j in (idx + 1)..curve.len() {
        if curve[j] < target {
            curve[j] = target;
        }
    }

    // Monotonic Rule 2: Points before current index cannot have higher duty (pull lower)
    for j in 0..idx {
        if curve[j] > target {
            curve[j] = target.max(min_duty);
        }
    }
}

pub fn run_main_standalone() {
    let window = match MainWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[OpenRevo Slint] Failed to create MainWindow: {}", e);
            return;
        }
    };

    // Dynamically inject the package version from CARGO_PKG_VERSION at compile time
    window.set_app_version(format!("v{}", env!("CARGO_PKG_VERSION")).into());

    let hwnd_holder: Arc<Mutex<Option<SafeHwnd>>> = Arc::new(Mutex::new(None));
    let ui_weak = window.as_weak();
    *SLINT_UI_WEAK.lock().unwrap() = Some(ui_weak.clone());

    // Start stdin IPC listener from host process
    ipc::start_ipc_listener(ui_weak.clone());

    // 1. Power mode
    let ui_cb = ui_weak.clone();
    window.on_power_mode_clicked(move |mode| {
        ipc::send_action_to_host("set_power_mode", serde_json::json!({ "mode": mode }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_power_mode(mode);
        }
    });

    window.on_open_custom_tuning(move || {
        ipc::send_action_to_host("open_custom_tuning", serde_json::json!({}));
    });

    // 2. GPU mode
    let ui_cb = ui_weak.clone();
    window.on_gpu_mode_clicked(move |mode| {
        ipc::send_action_to_host("set_gpu_mode", serde_json::json!({ "gpu_mode": mode.as_str() }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_gpu_mode(mode);
        }
    });

    // 3. Refresh Rate & Display Monitor
    let ui_cb = ui_weak.clone();
    window.on_refresh_rate_switched(move |hz| {
        ipc::send_action_to_host("switch_refresh_rate", serde_json::json!({ "hz": hz }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_refresh_rate(hz);
        }
    });

    window.on_select_monitor_index(move |idx| {
        ipc::send_action_to_host("select_monitor_index", serde_json::json!({ "index": idx }));
    });

    // 4. Auto min hz
    let ui_cb = ui_weak.clone();
    window.on_auto_min_hz_toggled(move || {
        ipc::send_action_to_host("toggle_auto_min_hz", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_auto_min_hz(!ui.get_auto_min_hz());
        }
    });

    // 5. Brightness
    let ui_cb = ui_weak.clone();
    window.on_brightness_changed(move |b| {
        ipc::send_action_to_host("set_brightness", serde_json::json!({ "brightness": b }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_brightness(b);
        }
    });

    // 6. Keyboard Lighting
    window.on_kbd_effect_cycled(move || {
        ipc::send_action_to_host("cycle_kbd_effect", serde_json::json!({}));
    });
    window.on_kbd_color_cycled(move || {
        ipc::send_action_to_host("cycle_kbd_color", serde_json::json!({}));
    });
    window.on_kbd_more_clicked(move || {
        ipc::send_action_to_host("open_lighting_studio", serde_json::json!({}));
    });

    // 7. Battery limit
    let ui_cb = ui_weak.clone();
    window.on_battery_limit_clicked(move |limit| {
        ipc::send_action_to_host("set_battery_limit", serde_json::json!({ "battery_limit": limit }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_battery_limit(limit);
        }
    });

    // 8. Switches
    let ui_cb = ui_weak.clone();
    window.on_fan_boost_toggled(move || {
        ipc::send_action_to_host("toggle_fan_boost", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_fan_boost(!ui.get_fan_boost());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_camera_toggled(move || {
        ipc::send_action_to_host("toggle_camera", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_camera_enabled(!ui.get_camera_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_mic_toggled(move || {
        ipc::send_action_to_host("toggle_mic", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_mic_enabled(!ui.get_mic_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_win_lock_toggled(move || {
        ipc::send_action_to_host("toggle_win_lock", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_win_locked(!ui.get_win_locked());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_keep_awake_toggled(move || {
        ipc::send_action_to_host("toggle_keep_awake", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_keep_awake(!ui.get_keep_awake());
        }
    });

    window.on_turn_off_screen_clicked(move || {
        ipc::send_action_to_host("turn_off_screen", serde_json::json!({}));
    });

    // Hardware Switches (Extra)
    let ui_cb = ui_weak.clone();
    window.on_wifi_toggled(move || {
        ipc::send_action_to_host("toggle_wifi", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_wifi_enabled(!ui.get_wifi_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_bt_toggled(move || {
        ipc::send_action_to_host("toggle_bt", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_bt_enabled(!ui.get_bt_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_touchpad_toggled(move || {
        ipc::send_action_to_host("toggle_touchpad", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_touchpad_locked(!ui.get_touchpad_locked());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_osd_toggled(move || {
        ipc::send_action_to_host("toggle_osd", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_osd_enabled(!ui.get_osd_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_fn_lock_toggled(move || {
        ipc::send_action_to_host("toggle_fn_lock", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_fn_locked(!ui.get_fn_locked());
        }
    });

    // 9. Footer & Dashboard Navigation
    let ui_cb = ui_weak.clone();
    window.on_autostart_toggled(move || {
        ipc::send_action_to_host("toggle_autostart", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_autostart(!ui.get_autostart());
        }
    });

    window.on_oem_toggled(move || {
        ipc::send_action_to_host("toggle_oem", serde_json::json!({}));
    });

    let hwnd_expand = hwnd_holder.clone();
    let ui_expand = ui_weak.clone();
    window.on_expand_dashboard_clicked(move || {
        if let Some(h) = *hwnd_expand.lock().unwrap() {
            resize_native_window(h.to_hwnd(), 1120, 760);
        }
        if let Some(ui) = ui_expand.upgrade() {
            ui.set_is_expanded(true);
        }
        ipc::send_action_to_host("expand_dashboard", serde_json::json!({ "is_expanded": true }));
    });

    let hwnd_collapse = hwnd_holder.clone();
    let ui_collapse = ui_weak.clone();
    window.on_collapse_dashboard_clicked(move || {
        if let Some(h) = *hwnd_collapse.lock().unwrap() {
            resize_native_window(h.to_hwnd(), 395, 632);
        }
        if let Some(ui) = ui_collapse.upgrade() {
            ui.set_is_expanded(false);
        }
        ipc::send_action_to_host("collapse_dashboard", serde_json::json!({ "is_expanded": false }));
    });

    let hwnd_fs = hwnd_holder.clone();
    let is_fullscreen = Arc::new(Mutex::new(false));
    let is_fs_clone = is_fullscreen.clone();
    window.on_toggle_fullscreen_clicked(move || {
        if let Some(h) = *hwnd_fs.lock().unwrap() {
            let mut fs = is_fs_clone.lock().unwrap();
            unsafe {
                if *fs {
                    let _ = ShowWindow(h.to_hwnd(), SW_RESTORE);
                    *fs = false;
                } else {
                    let _ = ShowWindow(h.to_hwnd(), SW_MAXIMIZE);
                    *fs = true;
                }
            }
        }
    });

    window.on_open_sponsor_clicked(move || {
        ipc::send_action_to_host("open_sponsor", serde_json::json!({}));
    });

    // Window controls
    let hwnd_min = hwnd_holder.clone();
    window.on_minimize_clicked(move || {
        if let Some(h) = *hwnd_min.lock().unwrap() {
            unsafe {
                WAS_MINIMIZED.store(true, Ordering::SeqCst);
                let _ = ShowWindow(h.to_hwnd(), SW_MINIMIZE);
            }
        }
    });

    let hwnd_close = hwnd_holder.clone();
    window.on_close_clicked(move || {
        if let Some(h) = *hwnd_close.lock().unwrap() {
            unsafe {
                let _ = ShowWindow(h.to_hwnd(), SW_HIDE);
            }
        }
    });

    let ui_theme = ui_weak.clone();
    window.on_toggle_theme_clicked(move || {
        if let Some(ui) = ui_theme.upgrade() {
            let next = (ui.get_theme_mode() + 1) % 3;
            ui.set_theme_mode(next);
            ipc::send_action_to_host("set_theme_mode", serde_json::json!({ "theme_mode": next }));
        }
    });

    // 10. Custom Hardware Tuning Callbacks
    window.on_tuning_back_to_overview(move || {
        ipc::send_action_to_host("tuning_back_to_overview", serde_json::json!({}));
    });
    window.on_tuning_select_preset(move |id| {
        ipc::send_action_to_host("tuning_select_preset", serde_json::json!({ "id": id.as_str() }));
    });
    window.on_tuning_create_preset(move || {
        ipc::send_action_to_host("tuning_create_preset", serde_json::json!({}));
    });
    window.on_tuning_reset_preset(move || {
        ipc::send_action_to_host("tuning_reset_preset", serde_json::json!({}));
    });
    window.on_tuning_rename_preset(move || {
        ipc::send_action_to_host("tuning_rename_preset", serde_json::json!({}));
    });
    window.on_tuning_export_preset(move || {
        ipc::send_action_to_host("tuning_export_preset", serde_json::json!({}));
    });
    window.on_tuning_import_preset(move || {
        ipc::send_action_to_host("tuning_import_preset", serde_json::json!({}));
    });
    window.on_tuning_open_preset_dir(move || {
        ipc::send_action_to_host("tuning_open_preset_dir", serde_json::json!({}));
    });
    window.on_tuning_delete_preset(move || {
        ipc::send_action_to_host("tuning_delete_preset", serde_json::json!({}));
    });

    let ui_tw = ui_weak.clone();
    window.on_tuning_toggle_temp_wall(move || {
        ipc::send_action_to_host("tuning_toggle_temp_wall", serde_json::json!({}));
        if let Some(ui) = ui_tw.upgrade() {
            ui.set_temp_wall_enabled(!ui.get_temp_wall_enabled());
        }
    });
    let ui_tw_val = ui_weak.clone();
    window.on_tuning_change_temp_wall(move |t| {
        ipc::send_action_to_host("tuning_set_temp_wall", serde_json::json!({ "temp_wall": t }));
        if let Some(ui) = ui_tw_val.upgrade() {
            ui.set_temp_wall(t);
        }
    });
    let ui_tw_max = ui_weak.clone();
    window.on_tuning_adjust_temp_wall_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_temp_wall_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_tw_max.upgrade() {
            let next = (ui.get_temp_wall_max() + delta).clamp(85, 105);
            ui.set_temp_wall_max(next);
        }
    });
    let ui_p1 = ui_weak.clone();
    window.on_tuning_change_pl1(move |w| {
        ipc::send_action_to_host("tuning_set_pl1", serde_json::json!({ "watts": w }));
        if let Some(ui) = ui_p1.upgrade() {
            ui.set_pl1(w);
        }
    });
    let ui_p1_max = ui_weak.clone();
    window.on_tuning_adjust_pl1_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_pl1_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_p1_max.upgrade() {
            let next = (ui.get_pl1_max() + delta).clamp(15, 255);
            ui.set_pl1_max(next);
        }
    });
    let ui_p2 = ui_weak.clone();
    window.on_tuning_change_pl2(move |w| {
        ipc::send_action_to_host("tuning_set_pl2", serde_json::json!({ "watts": w }));
        if let Some(ui) = ui_p2.upgrade() {
            ui.set_pl2(w);
        }
    });
    let ui_p2_max = ui_weak.clone();
    window.on_tuning_adjust_pl2_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_pl2_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_p2_max.upgrade() {
            let next = (ui.get_pl2_max() + delta).clamp(15, 255);
            ui.set_pl2_max(next);
        }
    });
    let ui_p4 = ui_weak.clone();
    window.on_tuning_change_pl4(move |w| {
        ipc::send_action_to_host("tuning_set_pl4", serde_json::json!({ "watts": w }));
        if let Some(ui) = ui_p4.upgrade() {
            ui.set_pl4(w);
        }
    });
    let ui_p4_max = ui_weak.clone();
    window.on_tuning_adjust_pl4_max(move |delta| {
        ipc::send_action_to_host("tuning_adjust_pl4_max", serde_json::json!({ "delta": delta }));
        if let Some(ui) = ui_p4_max.upgrade() {
            let next = (ui.get_pl4_max() + delta).clamp(35, 300);
            ui.set_pl4_max(next);
        }
    });
    let ui_oc = ui_weak.clone();
    window.on_tuning_toggle_bios_oc(move || {
        ipc::send_action_to_host("tuning_toggle_bios_oc", serde_json::json!({}));
        if let Some(ui) = ui_oc.upgrade() {
            ui.set_bios_oc_enabled(!ui.get_bios_oc_enabled());
        }
    });

    let ui_sp = ui_weak.clone();
    window.on_tuning_toggle_super_perf(move || {
        ipc::send_action_to_host("tuning_toggle_super_perf", serde_json::json!({}));
        if let Some(ui) = ui_sp.upgrade() {
            ui.set_super_perf(!ui.get_super_perf());
        }
    });
    let ui_co = ui_weak.clone();
    window.on_tuning_change_core_offset(move |o| {
        ipc::send_action_to_host("tuning_set_core_offset", serde_json::json!({ "offset": o }));
        if let Some(ui) = ui_co.upgrade() {
            ui.set_core_offset(o);
        }
    });
    let ui_mo = ui_weak.clone();
    window.on_tuning_change_mem_offset(move |o| {
        ipc::send_action_to_host("tuning_set_mem_offset", serde_json::json!({ "offset": o }));
        if let Some(ui) = ui_mo.upgrade() {
            ui.set_mem_offset(o);
        }
    });
    let ui_db = ui_weak.clone();
    window.on_tuning_toggle_db(move || {
        ipc::send_action_to_host("tuning_toggle_db", serde_json::json!({}));
        if let Some(ui) = ui_db.upgrade() {
            ui.set_db_enabled(!ui.get_db_enabled());
        }
    });
    let ui_dbw = ui_weak.clone();
    window.on_tuning_change_db_watts(move |w| {
        ipc::send_action_to_host("tuning_set_db_watts", serde_json::json!({ "watts": w }));
        if let Some(ui) = ui_dbw.upgrade() {
            ui.set_db_watts(w);
        }
    });

    let ui_sm = ui_weak.clone();
    window.on_tuning_toggle_smooth(move || {
        ipc::send_action_to_host("tuning_toggle_smooth", serde_json::json!({}));
        if let Some(ui) = ui_sm.upgrade() {
            ui.set_smooth_transition(!ui.get_smooth_transition());
        }
    });
    let ui_iso = ui_weak.clone();
    window.on_tuning_toggle_isolated(move || {
        ipc::send_action_to_host("tuning_toggle_isolated", serde_json::json!({}));
        if let Some(ui) = ui_iso.upgrade() {
            ui.set_fan_isolated_output(!ui.get_fan_isolated_output());
        }
    });
    let ui_lnk = ui_weak.clone();
    window.on_tuning_toggle_link(move || {
        ipc::send_action_to_host("tuning_toggle_link", serde_json::json!({}));
        if let Some(ui) = ui_lnk.upgrade() {
            ui.set_fan_linked(!ui.get_fan_linked());
        }
    });
    let ui_tab = ui_weak.clone();
    window.on_tuning_select_fan_tab(move |tab| {
        ipc::send_action_to_host("tuning_select_fan_tab", serde_json::json!({ "tab": tab }));
        if let Some(ui) = ui_tab.upgrade() {
            ui.set_active_fan_tab(tab);
        }
    });

    let cpu_curve_model = std::rc::Rc::new(slint::VecModel::from(vec![20, 30, 45, 55, 65, 75, 85, 100]));
    let gpu_curve_model = std::rc::Rc::new(slint::VecModel::from(vec![20, 30, 45, 55, 65, 75, 85, 100]));
    window.set_cpu_curve(cpu_curve_model.clone().into());
    window.set_gpu_curve(gpu_curve_model.clone().into());

    let fan_debounce_timer = std::rc::Rc::new(std::cell::RefCell::new(slint::Timer::default()));

    let fan_timer_adj = fan_debounce_timer.clone();
    let ui_fan = ui_weak.clone();
    window.on_tuning_adjust_point(move |idx, delta| {
        if let Some(ui) = ui_fan.upgrade() {
            let active_tab = ui.get_active_fan_tab();
            let is_linked = ui.get_fan_linked();

            let mut cpu_pts: Vec<i32> = ui.get_cpu_curve().iter().collect();
            let mut gpu_pts: Vec<i32> = ui.get_gpu_curve().iter().collect();
            let i = idx as usize;

            if active_tab == 0 || is_linked {
                if i < cpu_pts.len() {
                    let cur = cpu_pts[i];
                    apply_monotonic_fan_rule(&mut cpu_pts, i, cur + delta, 0);
                }
            }
            if active_tab == 1 || is_linked {
                if i < gpu_pts.len() {
                    let cur = gpu_pts[i];
                    apply_monotonic_fan_rule(&mut gpu_pts, i, cur + delta, 0);
                }
            }
            if is_linked {
                if active_tab == 0 {
                    gpu_pts = cpu_pts.clone();
                } else {
                    cpu_pts = gpu_pts.clone();
                }
            }

            ui.set_cpu_curve(slint::ModelRc::new(slint::VecModel::from(cpu_pts.clone())));
            ui.set_gpu_curve(slint::ModelRc::new(slint::VecModel::from(gpu_pts.clone())));

            let timer = fan_timer_adj.clone();
            timer.borrow_mut().stop();
            let c_send = cpu_pts.clone();
            let g_send = gpu_pts.clone();
            timer.borrow_mut().start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(150),
                move || {
                    ipc::send_action_to_host("tuning_set_fan_curve", serde_json::json!({
                        "cpu_curve": c_send,
                        "gpu_curve": g_send,
                    }));
                },
            );
        }
    });

    let fan_timer_set = fan_debounce_timer.clone();
    let ui_fan_set = ui_weak.clone();
    window.on_tuning_set_point(move |idx, duty| {
        if let Some(ui) = ui_fan_set.upgrade() {
            let active_tab = ui.get_active_fan_tab();
            let is_linked = ui.get_fan_linked();

            let mut cpu_pts: Vec<i32> = ui.get_cpu_curve().iter().collect();
            let mut gpu_pts: Vec<i32> = ui.get_gpu_curve().iter().collect();
            let i = idx as usize;

            if active_tab == 0 || is_linked {
                if i < cpu_pts.len() {
                    apply_monotonic_fan_rule(&mut cpu_pts, i, duty, 0);
                }
            }
            if active_tab == 1 || is_linked {
                if i < gpu_pts.len() {
                    apply_monotonic_fan_rule(&mut gpu_pts, i, duty, 0);
                }
            }
            if is_linked {
                if active_tab == 0 {
                    gpu_pts = cpu_pts.clone();
                } else {
                    cpu_pts = gpu_pts.clone();
                }
            }

            ui.set_cpu_curve(slint::ModelRc::new(slint::VecModel::from(cpu_pts.clone())));
            ui.set_gpu_curve(slint::ModelRc::new(slint::VecModel::from(gpu_pts.clone())));

            let timer = fan_timer_set.clone();
            timer.borrow_mut().stop();
            let c_send = cpu_pts.clone();
            let g_send = gpu_pts.clone();
            timer.borrow_mut().start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(150),
                move || {
                    ipc::send_action_to_host("tuning_set_fan_curve", serde_json::json!({
                        "cpu_curve": c_send,
                        "gpu_curve": g_send,
                    }));
                },
            );
        }
    });

    window.on_tuning_toggle_bld_boost(move || {
        ipc::send_action_to_host("tuning_toggle_bld_boost", serde_json::json!({}));
    });
    window.on_tuning_change_bld_cap(move |rpm| {
        ipc::send_action_to_host("tuning_change_bld_cap", serde_json::json!({ "rpm": rpm }));
    });
    window.on_tuning_restore_bld_auto(move || {
        ipc::send_action_to_host("tuning_restore_bld_auto", serde_json::json!({}));
    });

    let _ = window.show();
    ipc::send_action_to_host("slint_ready", serde_json::json!({}));

    // Setup window handles and DWM styles
    let hwnd_setup = hwnd_holder.clone();
    let timer = slint::Timer::default();
    let applied = Arc::new(Mutex::new(false));
    let applied_clone = applied.clone();

    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let mut done = applied_clone.lock().unwrap();
            if !*done {
                if let Some(hwnd) = find_thread_hwnd() {
                    apply_window_decorations(hwnd);
                    *hwnd_setup.lock().unwrap() = Some(SafeHwnd(hwnd.0 as isize));
                    *done = true;
                }
            }
        },
    );

    let _ = window.run();
}
