pub use slint_interpreter;
use std::path::{Path, PathBuf};
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

/// Run plugin standalone as a floating widget or popout window
pub fn run_widget_standalone(plugin_id: &str) {
    println!("[Slint Widget] Launching standalone widget for plugin: '{}'", plugin_id);
    let Some(plugin_dir) = find_plugin_dir(plugin_id) else {
        eprintln!("[Slint Widget] Plugin directory for '{}' not found!", plugin_id);
        return;
    };

    let manifest_path = plugin_dir.join("manifest.json");
    let mut entry_name = "main.slint".to_string();
    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(e) = val.get("widget").and_then(|w| w.get("entry")).and_then(|s| s.as_str()) {
                entry_name = e.to_string();
            } else if let Some(e) = val.get("tab").and_then(|t| t.get("entry")).and_then(|s| s.as_str()) {
                entry_name = e.to_string();
            } else if let Some(e) = val.get("entry").and_then(|s| s.as_str()) {
                entry_name = e.to_string();
            }
        }
    }

    let entry_file = plugin_dir.join(&entry_name);
    if !entry_file.exists() {
        eprintln!("[Slint Widget] Slint entry file '{:?}' not found!", entry_file);
        return;
    }

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
    let mut entry_name = "main.slint".to_string();
    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(e) = val.get("tab").and_then(|t| t.get("entry")).and_then(|s| s.as_str()) {
                entry_name = e.to_string();
            } else if let Some(e) = val.get("entry").and_then(|s| s.as_str()) {
                entry_name = e.to_string();
            } else if let Some(e) = val.get("widget").and_then(|w| w.get("entry")).and_then(|s| s.as_str()) {
                entry_name = e.to_string();
            }
        }
    }

    let entry_file = plugin_dir.join(&entry_name);
    if !entry_file.exists() {
        eprintln!("[Slint Embedded] Slint entry file '{:?}' not found!", entry_file);
        return;
    }

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

    let _ = instance.show();

    #[cfg(target_os = "windows")]
    unsafe {
        if let Some(child_hwnd) = find_thread_hwnd() {
            let parent_hwnd = HWND(parent_hwnd_raw as *mut _);

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

            // 4. Background watcher thread to listen for parent window close
            let parent_raw = parent_hwnd_raw;
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if !IsWindow(HWND(parent_raw as *mut _)).as_bool() {
                        std::process::exit(0);
                    }
                }
            });
        }
    }

    println!("[Slint Embedded] Running embedded instance for '{}'", plugin_id);
    let _ = instance.run();
}
