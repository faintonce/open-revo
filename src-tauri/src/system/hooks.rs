use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetAsyncKeyState, GetKeyState, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_CAPITAL,
    VK_CONTROL, VK_LWIN, VK_MENU, VK_NUMLOCK, VK_RWIN, VK_SCROLL, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

extern "system" {
    fn GetCurrentThreadId() -> u32;
}

static WIN_KEY_LOCKED: AtomicBool = AtomicBool::new(false);
static WIN_PHYSICALLY_DOWN: AtomicBool = AtomicBool::new(false);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static HOOK_STATE_LOCK: Mutex<()> = Mutex::new(());
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

// Tracked toggle states (Caps Lock, Num Lock, Scroll Lock)
static CAPS_STATE: AtomicBool = AtomicBool::new(false);
static NUM_STATE: AtomicBool = AtomicBool::new(false);
static SCROLL_STATE: AtomicBool = AtomicBool::new(false);

static LAST_CAPS_PRESS: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_NUM_PRESS: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_SCROLL_PRESS: Mutex<Option<Instant>> = Mutex::new(None);

/// Query true live lock key toggle state safely
unsafe fn query_hardware_lock_state(vk: u16) -> bool {
    (GetKeyState(vk as i32) & 1) != 0
}

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param.0 as u32;
        let kbd_struct = *(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk = kbd_struct.vkCode as u16;

        // Bypass dummy mask key (0xE8) from interfering with interactive lighting or lock logic
        if vk == 0xE8 {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // 1. Win Key Lock Software Interception
        if WIN_KEY_LOCKED.load(Ordering::SeqCst) {
            if vk == VK_LWIN.0 || vk == VK_RWIN.0 {
                let is_injected = (kbd_struct.flags.0 & 0x10) != 0;
                if !is_injected {
                    // Industry-standard MenuMaskKey technique (AutoHotkey / PowerToys):
                    // When Windows detects a standalone Win key release without any intervening keys,
                    // the Windows Shell (explorer.exe) triggers the Start Menu from the kernel input path,
                    // completely bypassing normal window message queues even if WH_KEYBOARD_LL returns 1!
                    // Furthermore, once popped up, subsequent Win presses cannot dismiss it because the hook eats them.
                    // Emitting unassigned dummy virtual-key 0xE8 down+up breaks the standalone modifier sequence,
                    // guaranteeing 100% that the Start Menu will NEVER be invoked, while swallowing Win key completely.
                    if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                        if !WIN_PHYSICALLY_DOWN.swap(true, Ordering::SeqCst) {
                            keybd_event(0xE8, 0, KEYBD_EVENT_FLAGS(0), 0);
                            keybd_event(0xE8, 0, KEYEVENTF_KEYUP, 0);
                        }
                    } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                        WIN_PHYSICALLY_DOWN.store(false, Ordering::SeqCst);
                        keybd_event(0xE8, 0, KEYBD_EVENT_FLAGS(0), 0);
                        keybd_event(0xE8, 0, KEYEVENTF_KEYUP, 0);
                    }
                }
                return LRESULT(1);
            }
        }

        // 0. Interactive lighting keystroke ingestion (0ms lock-free event push)
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            crate::driver::streamer::tracker::push_interactive_key_event(vk as i32);

            // 0.1 Universal Shortcut Bus (Route 2: Hardware & System Shortcut Dispatch)
            if let Some(app) = APP_HANDLE.get() {
                let is_ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
                let is_alt = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
                let is_shift = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
                let is_win = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0
                    || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0;

                if crate::system::shortcuts::check_match_and_trigger(app, vk, is_ctrl, is_alt, is_shift, is_win) {
                    return LRESULT(1);
                }
            }
        }

        // 2. Lock Keys Notification (Caps Lock, Num Lock, Scroll Lock)
        // Trigger on WM_KEYDOWN/WM_SYSKEYDOWN with 180ms debounce to filter typematic auto-repeats.
        // At this hook callback point, GetKeyState returns the state BEFORE the key toggle takes effect;
        // checking `(GetKeyState(vk) & 1) == 0` guarantees the exact new state after this toggle.
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            if let Some(app) = APP_HANDLE.get() {
                if vk == VK_CAPITAL.0 {
                    let mut last = LAST_CAPS_PRESS.lock();
                    let should_fire = match *last {
                        Some(t) => t.elapsed() >= std::time::Duration::from_millis(180),
                        None => true,
                    };
                    if should_fire {
                        *last = Some(Instant::now());
                        drop(last);

                        let new_val = (GetKeyState(VK_CAPITAL.0 as i32) & 1) == 0;
                        CAPS_STATE.store(new_val, Ordering::SeqCst);
                        crate::log_info!("HOOKS", "Caps Lock toggled to {}", new_val);
                        crate::system::osd_window::show_key_lock_toast("caps", new_val);
                        crate::system::osd_window::wake_osd_if_enabled(app);
                        let _ = app.emit("caps-lock-changed", new_val);
                        let _ = app.emit_to("main", "caps-lock-changed", new_val);
                        let _ = app.emit_to("osd", "caps-lock-changed", new_val);
                    }
                } else if vk == VK_NUMLOCK.0 {
                    let mut last = LAST_NUM_PRESS.lock();
                    let should_fire = match *last {
                        Some(t) => t.elapsed() >= std::time::Duration::from_millis(180),
                        None => true,
                    };
                    if should_fire {
                        *last = Some(Instant::now());
                        drop(last);

                        let new_val = (GetKeyState(VK_NUMLOCK.0 as i32) & 1) == 0;
                        NUM_STATE.store(new_val, Ordering::SeqCst);
                        crate::log_info!("HOOKS", "Num Lock toggled to {}", new_val);
                        crate::system::osd_window::show_key_lock_toast("num", new_val);
                        crate::system::osd_window::wake_osd_if_enabled(app);
                        let _ = app.emit("num-lock-changed", new_val);
                        let _ = app.emit_to("main", "num-lock-changed", new_val);
                        let _ = app.emit_to("osd", "num-lock-changed", new_val);
                    }
                } else if vk == VK_SCROLL.0 {
                    let mut last = LAST_SCROLL_PRESS.lock();
                    let should_fire = match *last {
                        Some(t) => t.elapsed() >= std::time::Duration::from_millis(180),
                        None => true,
                    };
                    if should_fire {
                        *last = Some(Instant::now());
                        drop(last);

                        let new_val = (GetKeyState(VK_SCROLL.0 as i32) & 1) == 0;
                        SCROLL_STATE.store(new_val, Ordering::SeqCst);
                        crate::log_info!("HOOKS", "Scroll Lock toggled to {}", new_val);
                        crate::system::osd_window::show_key_lock_toast("scroll", new_val);
                        crate::system::osd_window::wake_osd_if_enabled(app);
                        let _ = app.emit("scroll-lock-changed", new_val);
                        let _ = app.emit_to("main", "scroll-lock-changed", new_val);
                        let _ = app.emit_to("osd", "scroll-lock-changed", new_val);
                    }
                }
            }
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}

pub fn is_win_key_locked() -> bool {
    WIN_KEY_LOCKED.load(Ordering::SeqCst)
}

/// Initialize the permanent low-level keyboard hook with its dedicated message pump
pub fn init_hooks(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());

    // Query initial hardware toggle states
    unsafe {
        let init_caps = query_hardware_lock_state(VK_CAPITAL.0);
        let init_num = query_hardware_lock_state(VK_NUMLOCK.0);
        let init_scroll = query_hardware_lock_state(VK_SCROLL.0);

        CAPS_STATE.store(init_caps, Ordering::SeqCst);
        NUM_STATE.store(init_num, Ordering::SeqCst);
        SCROLL_STATE.store(init_scroll, Ordering::SeqCst);

        crate::log_info!(
            "HOOKS",
            "Keyboard lock states initialized: Caps={}, Num={}, Scroll={}",
            init_caps,
            init_num,
            init_scroll
        );
    }

    crate::system::shortcuts::init_shortcuts();

    ensure_hook_running();
}

fn ensure_hook_running() {
    let _guard = HOOK_STATE_LOCK.lock();
    let current_tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
    if current_tid != 0 {
        return;
    }

    thread::spawn(|| {
        unsafe {
            let tid = GetCurrentThreadId();
            HOOK_THREAD_ID.store(tid, Ordering::SeqCst);

            let hook = match SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                HINSTANCE(std::ptr::null_mut()),
                0,
            ) {
                Ok(h) => h,
                Err(e) => {
                    crate::log_error!("HOOKS", "Failed to install low-level keyboard hook: {:?}", e);
                    HOOK_THREAD_ID.store(0, Ordering::SeqCst);
                    return;
                }
            };

            crate::log_info!("HOOKS", "Low-level keyboard hook installed successfully (TID: {})", tid);

            // Win32 Message Pump for WH_KEYBOARD_LL
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
                if msg.message == WM_QUIT {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(hook);
            HOOK_THREAD_ID.store(0, Ordering::SeqCst);
            crate::log_info!("HOOKS", "Low-level keyboard hook unhooked");
        }
    });
}

pub fn set_win_key_locked(locked: bool) {
    WIN_KEY_LOCKED.store(locked, Ordering::SeqCst);
    if !locked {
        WIN_PHYSICALLY_DOWN.store(false, Ordering::SeqCst);
    }
    ensure_hook_running();
}

