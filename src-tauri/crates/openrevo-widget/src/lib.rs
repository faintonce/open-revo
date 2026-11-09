pub use slint_interpreter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use slint::ComponentHandle;
use slint_interpreter::Compiler;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentThreadId;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumThreadWindows, GetWindowLongW, IsWindow, SetParent, SetWindowLongW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOW,
    WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_APPWINDOW, WS_EX_CONTROLPARENT,
    WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_THICKFRAME, WS_VISIBLE,
};

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_thread_wnd_proc(hwnd: HWND, lparam: LPARAM) -> windows::Win32::Foundation::BOOL {
    let target = &mut *(lparam.0 as *mut Option<HWND>);
    if target.is_none() {
        *target = Some(hwnd);
        return windows::Win32::Foundation::BOOL(0);
    }
    windows::Win32::Foundation::BOOL(1)
}

#[cfg(target_os = "windows")]
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

/// Locate plugin root directory across APPDATA, local folders, and plugins_sample
pub fn find_plugin_dir(plugin_id: &str) -> Option<PathBuf> {
    // 1. Check APPDATA Roaming
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("openrevo").join("plugins").join(plugin_id);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Check current working directory plugins/
    let p_curr = Path::new("plugins").join(plugin_id);
    if p_curr.exists() {
        return Some(p_curr);
    }

    // 3. Check sample directories
    let p_sample_slint = Path::new("plugins_sample").join("slint").join(plugin_id);
    if p_sample_slint.exists() {
        return Some(p_sample_slint);
    }
    let p_sample_web = Path::new("plugins_sample").join("webview2").join(plugin_id);
    if p_sample_web.exists() {
        return Some(p_sample_web);
    }

    // 4. Executable relative directory
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p_exe = dir.join("plugins").join(plugin_id);
            if p_exe.exists() {
                return Some(p_exe);
            }
            let p_sample = dir.join("plugins_sample").join("slint").join(plugin_id);
            if p_sample.exists() {
                return Some(p_sample);
            }
        }
    }

    None
}

/// Resolve an existing Slint entry file path by prioritizing declared entries and checking candidates
fn resolve_slint_entry(plugin_dir: &Path, candidates: &[Option<String>], fallbacks: &[&str]) -> Option<PathBuf> {
    for cand in candidates.iter().flatten() {
        if cand.ends_with(".slint") {
            let p = plugin_dir.join(cand);
            if p.exists() {
                return Some(p);
            }
        }
    }
    for fb in fallbacks {
        let p = plugin_dir.join(fb);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Run plugin standalone as a floating widget or popout window
pub fn run_widget_standalone(plugin_id: &str) {
    println!("[Slint Widget] Launching standalone widget for plugin: '{}'", plugin_id);
    let Some(plugin_dir) = find_plugin_dir(plugin_id) else {
        eprintln!("[Slint Widget] Plugin directory for '{}' not found!", plugin_id);
        return;
    };

    let manifest_path = plugin_dir.join("manifest.json");
    let mut cand_entries = Vec::new();
    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            cand_entries.push(val.get("widget").and_then(|w| w.get("entry")).and_then(|s| s.as_str()).map(|s| s.to_string()));
            cand_entries.push(val.get("entry").and_then(|s| s.as_str()).map(|s| s.to_string()));
            cand_entries.push(val.get("tab").and_then(|t| t.get("entry")).and_then(|s| s.as_str()).map(|s| s.to_string()));
        }
    }

    let Some(entry_file) = resolve_slint_entry(&plugin_dir, &cand_entries, &["widget.slint", "main.slint", "tab.slint"]) else {
        eprintln!("[Slint Widget] No valid Slint entry file found for standalone widget in '{:?}'", plugin_dir);
        return;
    };

    let compiler = Compiler::default();
    let comp_res = spin_on::spin_on(compiler.build_from_path(&entry_file));
    for diag in comp_res.diagnostics() {
        eprintln!("[Slint Widget Compiler] {:?}", diag);
    }

    let Some(comp_name) = comp_res.component_names().next() else {
        eprintln!("[Slint Widget] No export component found in '{:?}'", entry_file);
        return;
    };

    let Some(def) = comp_res.component(&comp_name) else {
        eprintln!("[Slint Widget] Failed to retrieve component '{}'", comp_name);
        return;
    };

    let Ok(instance) = def.create() else {
        eprintln!("[Slint Widget] Failed to instantiate component '{}'", comp_name);
        return;
    };

    // Bind universal hardware microkernel callbacks (Raw EC, Fan Boost, Power Mode, Battery Health)
    bind_openrevo_hardware_callbacks(&instance);

    let _ = instance.show();

    #[cfg(target_os = "windows")]
    if let Some(hwnd) = find_thread_hwnd() {
        unsafe {
            let dark_mode: i32 = 1;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }

    println!("[Slint Widget] Running standalone instance for '{}'", plugin_id);
    let _ = instance.run();
}

/// Run plugin embedded as a native Win32 child window into the host parent window
pub fn run_widget_embedded(plugin_id: &str, parent_hwnd_raw: isize, rect: (i32, i32, i32, i32)) {
    println!(
        "[Slint Embedded] Launching embedded view for plugin '{}' into parent 0x{:X} rect={:?}",
        plugin_id, parent_hwnd_raw, rect
    );

    let Some(plugin_dir) = find_plugin_dir(plugin_id) else {
        eprintln!("[Slint Embedded] Plugin directory for '{}' not found!", plugin_id);
        return;
    };

    let manifest_path = plugin_dir.join("manifest.json");
    let mut cand_entries = Vec::new();
    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            cand_entries.push(val.get("tab").and_then(|t| t.get("entry")).and_then(|s| s.as_str()).map(|s| s.to_string()));
            cand_entries.push(val.get("entry").and_then(|s| s.as_str()).map(|s| s.to_string()));
            cand_entries.push(val.get("widget").and_then(|w| w.get("entry")).and_then(|s| s.as_str()).map(|s| s.to_string()));
        }
    }

    let Some(entry_file) = resolve_slint_entry(&plugin_dir, &cand_entries, &["tab.slint", "main.slint", "widget.slint"]) else {
        eprintln!("[Slint Embedded] No valid Slint entry file found for embedded tab in '{:?}'", plugin_dir);
        return;
    };

    let compiler = Compiler::default();
    let comp_res = spin_on::spin_on(compiler.build_from_path(&entry_file));
    for diag in comp_res.diagnostics() {
        eprintln!("[Slint Embedded Compiler] {:?}", diag);
    }

    let Some(comp_name) = comp_res.component_names().next() else {
        eprintln!("[Slint Embedded] No export component found in '{:?}'", entry_file);
        return;
    };

    let Some(def) = comp_res.component(&comp_name) else {
        eprintln!("[Slint Embedded] Failed to retrieve component '{}'", comp_name);
        return;
    };

    let Ok(instance) = def.create() else {
        eprintln!("[Slint Embedded] Failed to instantiate component '{}'", comp_name);
        return;
    };

    // Bind universal hardware microkernel callbacks (Raw EC, Fan Boost, Power Mode, Battery Health)
    bind_openrevo_hardware_callbacks(&instance);

    // Show instance first to trigger Winit window creation
    let _ = instance.show();

    // Setup repeated timer to reliably capture child HWND and apply Win32 parent attachment
    let timer = slint::Timer::default();
    let applied = Arc::new(Mutex::new(false));
    let applied_clone = applied.clone();
    let parent_raw = parent_hwnd_raw;

    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(10),
        move || {
            let mut done = applied_clone.lock().unwrap();
            if !*done {
                #[cfg(target_os = "windows")]
                if let Some(child_hwnd) = find_thread_hwnd() {
                    unsafe {
                        let parent_hwnd = HWND(parent_raw as *mut _);
                        if parent_raw != 0 && IsWindow(parent_hwnd).as_bool() {
                            // 1. Strip top-level chrome styles and convert to child window
                            let style = GetWindowLongW(child_hwnd, GWL_STYLE) as u32;
                            let remove_mask = WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0;
                            let new_style = ((style & !remove_mask) | WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0 | WS_CLIPCHILDREN.0) as i32;
                            let _ = SetWindowLongW(child_hwnd, GWL_STYLE, new_style);

                            let ex_style = GetWindowLongW(child_hwnd, GWL_EXSTYLE) as u32;
                            let remove_ex = WS_EX_APPWINDOW.0 | WS_EX_TOOLWINDOW.0;
                            let new_ex_style = ((ex_style & !remove_ex) | WS_EX_CONTROLPARENT.0) as i32;
                            let _ = SetWindowLongW(child_hwnd, GWL_EXSTYLE, new_ex_style);

                            // 2. Attach into host parent window
                            let _ = SetParent(child_hwnd, parent_hwnd);

                            // 3. Position and size to fit the container space
                            let _ = SetWindowPos(
                                child_hwnd,
                                None,
                                rect.0,
                                rect.1,
                                rect.2,
                                rect.3,
                                SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                            );
                            let _ = ShowWindow(child_hwnd, SW_SHOW);

                            println!("[Slint Embedded] Child HWND 0x{:X} successfully docked into parent 0x{:X}", child_hwnd.0 as usize, parent_raw);
                            *done = true;
                        }
                    }
                }
            }
        },
    );

    // Background watcher thread to listen for parent window close
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(500));
            #[cfg(target_os = "windows")]
            unsafe {
                if parent_raw != 0 && !IsWindow(HWND(parent_raw as *mut _)).as_bool() {
                    std::process::exit(0);
                }
            }
        }
    });

    println!("[Slint Embedded] Running embedded instance for '{}'", plugin_id);
    let _ = instance.run();
}

use openrevo_core::system::raw_gateway::{
    detect_auto_raw_protocol, execute_raw_read, execute_raw_write,
    get_gateway_capabilities, RawEcProtocol, RawEcReadRequest, RawEcWriteRequest,
};

/// Binds universal hardware control and raw EC communication callbacks onto a Slint component instance.
/// This allows declarative `.slint` plugins (both standalone widgets and embedded tabs) to interact
/// directly with OpenRevo's hardware microkernel without requiring WebView2 or Tauri IPC.
pub fn bind_openrevo_hardware_callbacks(instance: &slint_interpreter::ComponentInstance) {
    // 1. raw_ec_read: (addr: int) -> int OR (protocol: string, addr: int) -> int
    let _ = instance.set_callback("raw_ec_read", move |args: &[slint_interpreter::Value]| -> slint_interpreter::Value {
        let (protocol, addr) = if args.len() == 1 {
            let addr = args[0].clone().try_into().unwrap_or(0i32) as u16;
            (detect_auto_raw_protocol(), addr)
        } else if args.len() >= 2 {
            let proto_str = match &args[0] {
                slint_interpreter::Value::String(s) => s.as_str().to_string(),
                _ => String::new(),
            };
            let addr = args[1].clone().try_into().unwrap_or(0i32) as u16;
            let proto = serde_json::from_value::<RawEcProtocol>(serde_json::Value::String(proto_str))
                .unwrap_or_else(|_| detect_auto_raw_protocol());
            (proto, addr)
        } else {
            return slint_interpreter::Value::Number(-1.0);
        };

        match execute_raw_read(RawEcReadRequest { protocol, address: addr }) {
            Ok(val) => slint_interpreter::Value::Number(val as f64),
            Err(e) => {
                eprintln!("[Slint HW Gateway] raw_ec_read failed: {}", e);
                slint_interpreter::Value::Number(-1.0)
            }
        }
    });

    // 2. raw_ec_write: (addr: int, val: int) -> bool OR (protocol: string, addr: int, val: int) -> bool
    let _ = instance.set_callback("raw_ec_write", move |args: &[slint_interpreter::Value]| -> slint_interpreter::Value {
        let (protocol, addr, val) = if args.len() == 2 {
            let addr = args[0].clone().try_into().unwrap_or(0i32) as u16;
            let val = args[1].clone().try_into().unwrap_or(0i32) as u8;
            (detect_auto_raw_protocol(), addr, val)
        } else if args.len() >= 3 {
            let proto_str = match &args[0] {
                slint_interpreter::Value::String(s) => s.as_str().to_string(),
                _ => String::new(),
            };
            let addr = args[1].clone().try_into().unwrap_or(0i32) as u16;
            let val = args[2].clone().try_into().unwrap_or(0i32) as u8;
            let proto = serde_json::from_value::<RawEcProtocol>(serde_json::Value::String(proto_str))
                .unwrap_or_else(|_| detect_auto_raw_protocol());
            (proto, addr, val)
        } else {
            return slint_interpreter::Value::Bool(false);
        };

        match execute_raw_write(RawEcWriteRequest { protocol, address: addr, value: val }) {
            Ok(success) => slint_interpreter::Value::Bool(success),
            Err(e) => {
                eprintln!("[Slint HW Gateway] raw_ec_write failed: {}", e);
                slint_interpreter::Value::Bool(false)
            }
        }
    });

    // 3. set_fan_boost: (enable: bool) -> bool
    let _ = instance.set_callback("set_fan_boost", move |args: &[slint_interpreter::Value]| -> slint_interpreter::Value {
        let enable = args.get(0).and_then(|v| v.clone().try_into().ok()).unwrap_or(false);
        let hal = openrevo_core::hal::get_hardware_driver();
        let ok = hal.set_fan_boost(enable).is_ok();
        slint_interpreter::Value::Bool(ok)
    });

    // 4. set_power_mode: (mode: int) -> bool (1: Office, 2: Balance, 3: Turbo, 4: Custom)
    let _ = instance.set_callback("set_power_mode", move |args: &[slint_interpreter::Value]| -> slint_interpreter::Value {
        let raw_mode = args.get(0).and_then(|v| v.clone().try_into().ok()).unwrap_or(2i32);
        let hal_mode = match raw_mode {
            1 => openrevo_core::hal::HardwarePowerMode::Office,
            2 => openrevo_core::hal::HardwarePowerMode::Balance,
            3 => openrevo_core::hal::HardwarePowerMode::Turbo,
            4 => openrevo_core::hal::HardwarePowerMode::Custom,
            _ => openrevo_core::hal::HardwarePowerMode::Balance,
        };
        let hal = openrevo_core::hal::get_hardware_driver();
        let ok = hal.set_performance_mode(hal_mode).is_ok();
        slint_interpreter::Value::Bool(ok)
    });

    // 5. set_battery_health: (mode: int) -> bool (1: 100%, 2: 80%, 3: 60%)
    let _ = instance.set_callback("set_battery_health", move |args: &[slint_interpreter::Value]| -> slint_interpreter::Value {
        let raw_mode = args.get(0).and_then(|v| v.clone().try_into().ok()).unwrap_or(2i32);
        let health_mode = match raw_mode {
            1 => openrevo_core::hal::BatteryHealthMode::LongLife,
            2 => openrevo_core::hal::BatteryHealthMode::Balance,
            3 => openrevo_core::hal::BatteryHealthMode::Station,
            _ => openrevo_core::hal::BatteryHealthMode::Balance,
        };
        let hal = openrevo_core::hal::get_hardware_driver();
        let ok = hal.set_battery_health(health_mode).is_ok();
        slint_interpreter::Value::Bool(ok)
    });

    // 6. get_raw_ec_capabilities: () -> string (JSON array of available protocols)
    let _ = instance.set_callback("get_raw_ec_capabilities", move |_args: &[slint_interpreter::Value]| -> slint_interpreter::Value {
        let caps = get_gateway_capabilities();
        let json_str = serde_json::to_string(&caps.available_protocols).unwrap_or_else(|_| "[]".to_string());
        slint_interpreter::Value::String(json_str.into())
    });
}
