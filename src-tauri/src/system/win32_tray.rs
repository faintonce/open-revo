// OpenRevo - Pure Win32 System Tray & Event Loop for Standalone Slint Mode
// Eliminates 100% of Tauri/Tao/WebView2 background runtime memory footprint (~20-25MB saved).

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, AppendMenuW, ChangeWindowMessageFilterEx, CheckMenuItem,
    CheckMenuRadioItem, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DispatchMessageW, GetCursorPos, GetMessageW, KillTimer, LoadIconW, PostMessageW,
    PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow, SetTimer,
    TrackPopupMenu, TranslateMessage, ASFW_ANY, CW_USEDEFAULT, HICON, HMENU, IDI_APPLICATION,
    MF_BYCOMMAND, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, MSGFLT_ALLOW,
    TPM_BOTTOMALIGN, TPM_RIGHTALIGN, WINDOW_EX_STYLE, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WM_TIMER, WM_USER, WNDCLASSEXW,
    WS_OVERLAPPEDWINDOW,
};

use crate::driver::ec_bridge::SharedEcBridge;

pub const WM_TRAY_ICON: u32 = WM_USER + 101;
pub const WM_TRAY_WAKE_INSTANCE: u32 = WM_USER + 102;

// Menu Command IDs (5000+ to avoid hardware vault address collision)
const ID_MODE_FULL_PANEL: usize = 5001;
const ID_MODE_MINI_DRAWER: usize = 5002;
const ID_TURN_OFF_SCREEN: usize = 5003;

const ID_MODE_OFFICE: usize = 5101;
const ID_MODE_BALANCE: usize = 5102;
const ID_MODE_BEAST: usize = 5103;
const ID_MODE_CUSTOM: usize = 5104;

const ID_TOGGLE_FAN: usize = 5201;
const ID_TAKEOVER_TOGGLE: usize = 5202;

const ID_NOTICE: usize = 5301;
const ID_QUIT: usize = 5302;

static GLOBAL_EC_BRIDGE: std::sync::OnceLock<SharedEcBridge> = std::sync::OnceLock::new();
static WM_TASKBAR_CREATED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static TRAY_ICON_HANDLE: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

const TIMER_ID_TRAY_RETRY: usize = 1;
const TIMER_ID_TRAY_HEALTH: usize = 2;
const TRAY_RETRY_INTERVAL_MS: u32 = 2000;
const TRAY_HEALTH_INTERVAL_MS: u32 = 15000;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn build_notify_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let tip = to_wide("OpenRevo 控制中枢");
    let mut tip_arr = [0u16; 128];
    let copy_len = tip.len().min(127);
    tip_arr[..copy_len].copy_from_slice(&tip[..copy_len]);

    let h_icon = HICON(TRAY_ICON_HANDLE.load(std::sync::atomic::Ordering::Relaxed) as *mut core::ffi::c_void);

    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAY_ICON,
        hIcon: h_icon,
        szTip: tip_arr,
        ..Default::default()
    }
}

unsafe fn ensure_tray_icon_registered(hwnd: HWND) -> bool {
    let nid = build_notify_icon_data(hwnd);
    if Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
        return true;
    }
    Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool()
}

unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_created_msg = WM_TASKBAR_CREATED.load(std::sync::atomic::Ordering::Relaxed);
    if taskbar_created_msg != 0 && msg == taskbar_created_msg {
        crate::log_info!("TRAY", "TaskbarCreated broadcast received. Re-registering Win32 tray icon...");
        if ensure_tray_icon_registered(hwnd) {
            let _ = KillTimer(hwnd, TIMER_ID_TRAY_RETRY);
        } else {
            let _ = SetTimer(hwnd, TIMER_ID_TRAY_RETRY, TRAY_RETRY_INTERVAL_MS, None);
        }
        return LRESULT(0);
    }

    match msg {
        WM_TRAY_ICON => {
            let event = lparam.0 as u32;
            match event {
                WM_LBUTTONDOWN => {
                    let _ = AllowSetForegroundWindow(ASFW_ANY);
                    crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
                        "action": "tray_click_guard"
                    }));
                }
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    let _ = AllowSetForegroundWindow(ASFW_ANY);
                    crate::system::slint_window::show_slint_window();
                }
                WM_RBUTTONUP => {
                    show_tray_context_menu(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_TRAY_WAKE_INSTANCE => {
            crate::log_info!("TRAY", "Single-instance wake message received. Showing Slint window.");
            crate::system::slint_window::show_slint_window();
            LRESULT(0)
        }
        WM_TIMER => {
            match wparam.0 {
                TIMER_ID_TRAY_RETRY => {
                    if ensure_tray_icon_registered(hwnd) {
                        crate::log_info!("TRAY", "Win32 tray icon registered after retry.");
                        let _ = KillTimer(hwnd, TIMER_ID_TRAY_RETRY);
                    }
                }
                TIMER_ID_TRAY_HEALTH => {
                    let nid = build_notify_icon_data(hwnd);
                    if !Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool() {
                        crate::log_warn!("TRAY", "Win32 tray icon missing from shell; re-registering...");
                        if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                            let _ = SetTimer(hwnd, TIMER_ID_TRAY_RETRY, TRAY_RETRY_INTERVAL_MS, None);
                        }
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xFFFF) as usize;
            handle_tray_menu_command(hwnd, cmd_id);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(hwnd, TIMER_ID_TRAY_RETRY);
            let _ = KillTimer(hwnd, TIMER_ID_TRAY_HEALTH);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_tray_context_menu(hwnd: HWND) {
    let hmenu = match CreatePopupMenu() {
        Ok(m) => m,
        Err(_) => return,
    };

    let cfg = crate::system::config::load_config();
    let current_mode = if let Some(ec) = GLOBAL_EC_BRIDGE.get() {
        ec.lock().get_power_mode()
    } else {
        cfg.power_mode
    };
    let current_fan = if let Some(ec) = GLOBAL_EC_BRIDGE.get() {
        ec.lock().is_fan_boost()
    } else {
        cfg.fan_boost
    };

    // 1. Presentation Modes
    let full_w = to_wide("🖥️ 完整面板");
    let mini_w = to_wide("🪟 迷你抽屉");
    let screen_off_w = to_wide("🌙 关闭屏幕 (3秒防误触)");

    let _ = AppendMenuW(hmenu, MF_STRING, ID_MODE_FULL_PANEL, PCWSTR(full_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, ID_MODE_MINI_DRAWER, PCWSTR(mini_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, ID_TURN_OFF_SCREEN, PCWSTR(screen_off_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());

    if cfg.last_window_expanded {
        let _ = CheckMenuItem(hmenu, ID_MODE_FULL_PANEL as u32, (MF_BYCOMMAND | MF_CHECKED).0);
    } else {
        let _ = CheckMenuItem(hmenu, ID_MODE_MINI_DRAWER as u32, (MF_BYCOMMAND | MF_CHECKED).0);
    }

    // 2. Power Modes
    let office_w = to_wide("办公模式 (Office)");
    let balance_w = to_wide("均衡模式 (Balance)");
    let beast_w = to_wide("狂暴模式 (Beast)");
    let custom_w = to_wide("极客调校 (Custom)");

    let _ = AppendMenuW(hmenu, MF_STRING, ID_MODE_OFFICE, PCWSTR(office_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, ID_MODE_BALANCE, PCWSTR(balance_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, ID_MODE_BEAST, PCWSTR(beast_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, ID_MODE_CUSTOM, PCWSTR(custom_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());

    let target_radio = match current_mode {
        1 => ID_MODE_OFFICE,
        2 => ID_MODE_BALANCE,
        3 => ID_MODE_BEAST,
        4 => ID_MODE_CUSTOM,
        _ => ID_MODE_BALANCE,
    };
    let _ = CheckMenuRadioItem(
        hmenu,
        ID_MODE_OFFICE as u32,
        ID_MODE_CUSTOM as u32,
        target_radio as u32,
        MF_BYCOMMAND.0,
    );

    // 3. Hardware Controls
    let fan_w = to_wide("一键强冷 (Fan Boost)");
    let oem_w = to_wide("🔄 接管/恢复 官方后台服务");

    let _ = AppendMenuW(hmenu, MF_STRING, ID_TOGGLE_FAN, PCWSTR(fan_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, ID_TAKEOVER_TOGGLE, PCWSTR(oem_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());

    if current_fan {
        let _ = CheckMenuItem(hmenu, ID_TOGGLE_FAN as u32, (MF_BYCOMMAND | MF_CHECKED).0);
    }

    // 4. Meta & Quit
    let version_str = format!("🛡️ PRO v{}", env!("CARGO_PKG_VERSION"));
    let version_w = to_wide(&version_str);
    let quit_w = to_wide("退出 OpenRevo");

    let _ = AppendMenuW(hmenu, MF_STRING | MF_GRAYED, ID_NOTICE, PCWSTR(version_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, ID_QUIT, PCWSTR(quit_w.as_ptr()));

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(
        hmenu,
        TPM_RIGHTALIGN | TPM_BOTTOMALIGN,
        pt.x,
        pt.y,
        0,
        hwnd,
        None,
    );
    let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(hmenu);
}

fn handle_tray_menu_command(_hwnd: HWND, cmd_id: usize) {
    match cmd_id {
        ID_MODE_FULL_PANEL => {
            let mut cfg = crate::system::config::load_config();
            cfg.last_window_expanded = true;
            let _ = crate::system::config::save_config(&cfg);
            crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
                "action": "expand_dashboard",
                "is_expanded": true
            }));
            crate::system::slint_window::show_slint_window();
        }
        ID_MODE_MINI_DRAWER => {
            let mut cfg = crate::system::config::load_config();
            cfg.last_window_expanded = false;
            let _ = crate::system::config::save_config(&cfg);
            crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
                "action": "collapse_dashboard",
                "is_expanded": false
            }));
            crate::system::slint_window::show_slint_window();
        }
        ID_TURN_OFF_SCREEN => {
            let _ = crate::system::switches::turn_off_screen();
        }
        ID_MODE_OFFICE => set_power_mode_standalone(1),
        ID_MODE_BALANCE => set_power_mode_standalone(2),
        ID_MODE_BEAST => set_power_mode_standalone(3),
        ID_MODE_CUSTOM => set_power_mode_standalone(4),
        ID_TOGGLE_FAN => {
            if let Some(ec) = GLOBAL_EC_BRIDGE.get() {
                let next = !ec.lock().is_fan_boost();
                ec.lock().set_fan_boost(next);
                let mut cfg = crate::system::config::load_config();
                cfg.fan_boost = next;
                let _ = crate::system::config::save_config(&cfg);
                crate::system::osd_window::show_fan_boost(next);
                crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
                    "action": "sync_state",
                    "fan_boost": next
                }));
            }
        }
        ID_TAKEOVER_TOGGLE => {
            let is_running = crate::system::process::is_oem_service_running();
            if is_running {
                let _ = crate::system::service_mgr::pause_oem_services();
            } else {
                let _ = crate::system::service_mgr::resume_oem_services();
            }
        }
        ID_QUIT => {
            crate::log_info!("TRAY", "Win32 tray quit triggered. Performing safe shutdown...");
            crate::system::osd_window::shutdown_osd_process();
            crate::system::slint_window::shutdown_slint_process();
            crate::system::tray::safe_hardware_shutdown_on_exit();
            crate::system::plugin_service::shutdown_all_plugin_services();
            crate::remove_core_pid_file();
            unsafe {
                let nid = build_notify_icon_data(_hwnd);
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                let _ = PostQuitMessage(0);
            }
            std::process::exit(0);
        }
        _ => {}
    }
}

fn set_power_mode_standalone(mode: u8) {
    let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
    let mut cfg = crate::system::config::load_config();
    if (mode == 3 || mode == 4) && !is_dc && !cfg.unlock_battery_power_limit {
        crate::log_warn!("TRAY", "Blocked Turbo/Custom mode: Running on battery and unlock limit not enabled.");
        return;
    }

    if let Some(ec) = GLOBAL_EC_BRIDGE.get() {
        ec.lock().set_power_mode(mode);
    }
    cfg.power_mode = mode;
    if is_dc {
        cfg.power_mode_ac = mode;
    } else {
        cfg.power_mode_battery = mode;
    }
    let _ = crate::system::config::save_config(&cfg);
    crate::system::osd_window::show_power_mode(mode);
    crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
        "action": "sync_state",
        "power_mode": mode
    }));
}

#[link(name = "shell32")]
extern "system" {
    fn ExtractIconW(hinst: HINSTANCE, lpszexename: PCWSTR, niconindex: u32) -> HICON;
}

fn load_best_tray_icon(h_inst: HINSTANCE) -> HICON {
    // 1. Try ExtractIconW on current running executable (Index 0 is the embedded application icon)
    if let Ok(exe) = std::env::current_exe() {
        let exe_wide = to_wide(&exe.to_string_lossy());
        let ico = unsafe { ExtractIconW(h_inst, PCWSTR(exe_wide.as_ptr()), 0) };
        if !ico.0.is_null() && (ico.0 as isize) > 1 {
            return ico;
        }
    }

    // 2. Try loading from common Windows/Tauri resource identifiers
    for res_id in [1, 32512, 128] {
        if let Ok(ico) = unsafe { LoadIconW(h_inst, PCWSTR(res_id as *const u16)) } {
            if !ico.0.is_null() {
                return ico;
            }
        }
    }

    // 3. Fallback to standard application icon
    unsafe { LoadIconW(HINSTANCE(std::ptr::null_mut()), IDI_APPLICATION).unwrap_or(HICON(std::ptr::null_mut())) }
}

/// Runs the native Win32 system tray message loop on the main thread (blocking until exit)
pub fn run_tray_loop(ec_instance: SharedEcBridge) {
    let _ = GLOBAL_EC_BRIDGE.set(ec_instance);
    unsafe {
        let h_inst = GetModuleHandleW(None).unwrap_or_default();
        let class_name = w!("OpenRevo_Tray_WndClass");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(tray_window_proc),
            hInstance: h_inst.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        let _ = RegisterClassExW(&wc);

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("OpenRevo_Tray_Window"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND(std::ptr::null_mut()),
            HMENU(std::ptr::null_mut()),
            h_inst,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                crate::log_error!("TRAY", "Failed to create tray message window: {:?}", e);
                return;
            }
        };

        let h_icon = load_best_tray_icon(h_inst.into());
        TRAY_ICON_HANDLE.store(h_icon.0 as isize, std::sync::atomic::Ordering::Relaxed);

        let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        if taskbar_created != 0 {
            WM_TASKBAR_CREATED.store(taskbar_created, std::sync::atomic::Ordering::Relaxed);
            // Allow TaskbarCreated from medium-integrity explorer.exe when running elevated (UIPI bypass for shell broadcast)
            let _ = ChangeWindowMessageFilterEx(hwnd, taskbar_created, MSGFLT_ALLOW, None);
        }
        let _ = ChangeWindowMessageFilterEx(hwnd, WM_TRAY_WAKE_INSTANCE, MSGFLT_ALLOW, None);

        if ensure_tray_icon_registered(hwnd) {
            crate::log_info!("TRAY", "Native Win32 system tray registered successfully. Message loop running.");
        } else {
            crate::log_warn!("TRAY", "Initial Win32 tray registration failed (shell may not be ready); scheduling retry.");
            let _ = SetTimer(hwnd, TIMER_ID_TRAY_RETRY, TRAY_RETRY_INTERVAL_MS, None);
        }
        let _ = SetTimer(hwnd, TIMER_ID_TRAY_HEALTH, TRAY_HEALTH_INTERVAL_MS, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let nid = build_notify_icon_data(hwnd);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        crate::remove_core_pid_file();
    }
}
