use std::sync::{Arc, Mutex};
use std::time::Duration;
use slint::ComponentHandle;
use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

pub use slint;

pub mod callbacks;
pub mod curve_helpers;
pub mod ipc;
pub mod window;

pub use window::SafeHwnd;

slint::include_modules!();

pub fn run_main_standalone() {
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "user32")]
        extern "system" {
            fn SetProcessDpiAwarenessContext(value: isize) -> i32;
        }
        const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

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
    window::set_slint_ui_weak(ui_weak.clone());

    for arg in std::env::args() {
        if let Some(pid_str) = arg.strip_prefix("--host-pid=") {
            if let Ok(pid) = pid_str.parse::<u32>() {
                window::set_host_pid(pid);
            }
        }
    }

    // Start stdin IPC listener from host process
    ipc::start_ipc_listener(ui_weak.clone());

    let start_expanded = std::env::args().any(|a| a == "--expanded");
    if start_expanded {
        window.set_is_expanded(true);
    }

    // Register all callbacks by domain
    callbacks::register_all(&window, ui_weak.clone(), hwnd_holder.clone());

    let _ = window.show();

    // Synchronously capture HWND and configure visibility/position immediately
    let hwnd_setup = hwnd_holder.clone();
    let mut initial_hwnd_val = 0;
    if let Some(hwnd) = window::find_thread_hwnd() {
        window::apply_window_decorations(hwnd);
        let safe = SafeHwnd(hwnd.0 as isize);
        *hwnd_setup.lock().unwrap() = Some(safe);
        *window::HWND_HOLDER.lock().unwrap() = Some(safe);
        initial_hwnd_val = hwnd.0 as isize;
        if start_expanded {
            let saved = *window::SAVED_FULL_POS.lock().unwrap();
            window::expand_native_window(hwnd, 1120, 760, saved);
        } else {
            window::align_mini_drawer_to_tray(hwnd, 395, 632);
        }
        if !std::env::args().any(|a| a == "--visible") {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        } else {
            window::mark_window_woken();
        }
    }

    ipc::send_action_to_host("slint_ready", serde_json::json!({ "hwnd": initial_hwnd_val }));

    // Setup window handles and DWM styles fallback timer
    let timer = slint::Timer::default();
    let applied = Arc::new(Mutex::new(false));
    let applied_clone = applied.clone();

    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let mut done = applied_clone.lock().unwrap();
            if !*done {
                if let Some(hwnd) = window::find_thread_hwnd() {
                    window::apply_window_decorations(hwnd);
                    let safe = SafeHwnd(hwnd.0 as isize);
                    *hwnd_setup.lock().unwrap() = Some(safe);
                    *window::HWND_HOLDER.lock().unwrap() = Some(safe);
                    if start_expanded {
                        let saved = *window::SAVED_FULL_POS.lock().unwrap();
                        window::expand_native_window(hwnd, 1120, 760, saved);
                    } else {
                        window::align_mini_drawer_to_tray(hwnd, 395, 632);
                    }
                    if !std::env::args().any(|a| a == "--visible") {
                        unsafe {
                            let _ = ShowWindow(hwnd, SW_HIDE);
                        }
                    }
                    ipc::send_action_to_host("sync_slint_hwnd", serde_json::json!({ "hwnd": hwnd.0 as isize }));
                    *done = true;
                }
            }
        },
    );

    let _ = window.run();
}
