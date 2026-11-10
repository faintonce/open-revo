use std::collections::HashMap;
use tauri::AppHandle;
use super::super::sync::sync_slint_plugins_state;
use super::super::webview_dock::hide_embedded_webview;

static ACTIVE_SLINT_CHILD: parking_lot::Mutex<Option<std::process::Child>> = parking_lot::Mutex::new(None);
static ACTIVE_SLINT_WIDGETS: parking_lot::Mutex<Option<HashMap<String, std::process::Child>>> = parking_lot::Mutex::new(None);

#[cfg(target_os = "windows")]
pub fn assign_to_app_job(child: &std::process::Child) {
    use std::sync::OnceLock;
    use windows::Win32::System::JobObjects::{
        CreateJobObjectW, SetInformationJobObject, AssignProcessToJobObject,
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::Foundation::HANDLE;

    static JOB_HANDLE: OnceLock<Option<isize>> = OnceLock::new();
    let job_opt = JOB_HANDLE.get_or_init(|| {
        unsafe {
            if let Ok(job) = CreateJobObjectW(None, None) {
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let res = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if res.is_ok() {
                    Some(job.0 as isize)
                } else {
                    None
                }
            } else {
                None
            }
        }
    });

    if let Some(raw_job) = *job_opt {
        use std::os::windows::io::AsRawHandle;
        let raw_h = child.as_raw_handle();
        unsafe {
            let _ = AssignProcessToJobObject(HANDLE(raw_job as *mut _), HANDLE(raw_h as *mut _));
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn assign_to_app_job(_child: &std::process::Child) {}

fn with_active_slint_widgets<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, std::process::Child>) -> R,
{
    let mut guard = ACTIVE_SLINT_WIDGETS.lock();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

pub fn launch_slint_widget(id: &str) -> Result<(), String> {
    crate::log_info!("PLUGINS", "launch_slint_widget requested for '{}'", id);
    let current_exe = std::env::current_exe().unwrap_or_else(|_| "open-revo.exe".into());
    let mut cmd = std::process::Command::new(current_exe);
    cmd.args(&["--slint-widget", id]);

    match cmd.spawn() {
        Ok(child) => {
            assign_to_app_job(&child);
            with_active_slint_widgets(|widgets| {
                if let Some(mut old) = widgets.insert(id.to_string(), child) {
                    let _ = old.kill();
                    let _ = old.wait();
                }
            });
            crate::log_info!("PLUGINS", "Successfully spawned and registered Slint widget '{}'", id);
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("Failed to spawn Slint widget child for '{}': {:?}", id, e);
            crate::log_error!("PLUGINS", "{}", err_msg);
            Err(err_msg)
        }
    }
}

pub fn close_slint_widget(id: &str) {
    crate::log_info!("PLUGINS", "close_slint_widget requested for '{}'", id);
    with_active_slint_widgets(|widgets| {
        if let Some(mut child) = widgets.remove(id) {
            let _ = child.kill();
            let _ = child.wait();
        }
    });
}

pub fn terminate_all_slint_processes() {
    crate::log_info!("PLUGINS", "Terminating all Slint child processes and widgets...");
    hide_slint_embedded();
    with_active_slint_widgets(|widgets| {
        for (id, mut child) in widgets.drain() {
            let _ = child.kill();
            let _ = child.wait();
            crate::log_info!("PLUGINS", "Killed Slint widget process for '{}'", id);
        }
    });
}

pub fn hide_slint_embedded() {
    let mut guard = ACTIVE_SLINT_CHILD.lock();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
        crate::log_info!("PLUGINS", "Terminated active Slint embedded child process");
    }
}

/// Determine whether the Tab view (embedded in host dashboard) should render using Slint
pub fn is_slint_tab(pid: &str) -> bool {
    let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
    let m_path = p_dir.join("manifest.json");
    if let Ok(content) = std::fs::read_to_string(&m_path) {
        if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
            // 1. Explicit tab config
            if let Some(ref tab) = manifest.tab {
                if let Some(ref t) = tab.plugin_type {
                    if t.eq_ignore_ascii_case("slint") { return true; }
                    if t.eq_ignore_ascii_case("webview2") || t.eq_ignore_ascii_case("web") || t.eq_ignore_ascii_case("html") {
                        return false;
                    }
                }
                if let Some(ref e) = tab.entry {
                    if e.ends_with(".slint") { return true; }
                    if e.ends_with(".html") || e.ends_with(".htm") { return false; }
                }
            }

            // 2. Global plugin_type declaration
            if let Some(ref pt) = manifest.plugin_type {
                if pt.eq_ignore_ascii_case("slint") {
                    if let Some(ref tab) = manifest.tab {
                        if tab.entry.as_deref().map_or(false, |e| e.ends_with(".html") || e.ends_with(".htm")) {
                            return false;
                        }
                    }
                    return true;
                }
                if pt.eq_ignore_ascii_case("webview2") || pt.eq_ignore_ascii_case("web") {
                    return false;
                }
            }

            // 3. Global entry extension
            if let Some(ref e) = manifest.entry {
                if e.ends_with(".slint") { return true; }
                if e.ends_with(".html") || e.ends_with(".htm") { return false; }
            }
        }
    }

    if p_dir.join("tab.slint").exists() {
        return true;
    }
    if p_dir.join("main.slint").exists() && !p_dir.join("index.html").exists() && !p_dir.join("dist").join("index.html").exists() {
        return true;
    }
    false
}

/// Determine whether the Widget view (desktop floating or standalone popout) should render using Slint
pub fn is_slint_widget(pid: &str) -> bool {
    let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
    let m_path = p_dir.join("manifest.json");
    if let Ok(content) = std::fs::read_to_string(&m_path) {
        if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
            // 1. Explicit widget config
            if let Some(ref w) = manifest.widget {
                if let Some(ref t) = w.plugin_type {
                    if t.eq_ignore_ascii_case("slint") { return true; }
                    if t.eq_ignore_ascii_case("webview2") || t.eq_ignore_ascii_case("web") || t.eq_ignore_ascii_case("html") {
                        return false;
                    }
                }
                if let Some(ref e) = w.entry {
                    if e.ends_with(".slint") { return true; }
                    if e.ends_with(".html") || e.ends_with(".htm") { return false; }
                }
            }

            // 2. Check for widget.slint existence in plugin dir
            if p_dir.join("widget.slint").exists() {
                return true;
            }

            // 3. Global plugin_type declaration fallback
            if let Some(ref pt) = manifest.plugin_type {
                if pt.eq_ignore_ascii_case("slint") {
                    return true;
                }
                if pt.eq_ignore_ascii_case("webview2") || pt.eq_ignore_ascii_case("web") {
                    return false;
                }
            }

            // 4. Global entry fallback
            if let Some(ref e) = manifest.entry {
                if e.ends_with(".slint") { return true; }
                if e.ends_with(".html") || e.ends_with(".htm") { return false; }
            }
        }
    }

    if p_dir.join("widget.slint").exists() {
        return true;
    }
    if p_dir.join("main.slint").exists() && !p_dir.join("widget.html").exists() {
        return true;
    }
    false
}

pub fn is_slint_plugin(pid: &str) -> bool {
    is_slint_tab(pid) || is_slint_widget(pid)
}

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "toggle_builtin_module" => {
            if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                if let Some(enabled) = val.get("enabled").and_then(|v| v.as_bool()) {
                    let mut cfg = crate::system::config::load_config();
                    cfg.module_toggles.insert(id.to_string(), enabled);
                    let _ = crate::system::config::save_config(&cfg);

                    if id == "water" {
                        let _ = crate::commands::cooler_cmd::set_water_cooler_enabled(enabled);
                    }
                    sync_slint_plugins_state();
                }
            }
            true
        }
        "toggle_custom_plugin" => {
            if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                if let Some(enabled) = val.get("enabled").and_then(|v| v.as_bool()) {
                    let _ = crate::commands::system_cmd::set_custom_plugin_enabled(id.to_string(), enabled);

                    // If this plugin has a desktop widget or companion service, toggle them dynamically
                    let p_dir = crate::commands::system_cmd::find_plugin_dir(id);
                    let manifest_path = p_dir.join("manifest.json");
                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                            let p_type = manifest.plugin_type.as_deref().unwrap_or_default();
                            let has_widget = manifest.widget.is_some()
                                || p_dir.join("widget.slint").exists()
                                || p_type.eq_ignore_ascii_case("widget")
                                || p_type.eq_ignore_ascii_case("skin");

                            if has_widget {
                                if is_slint_widget(id) {
                                    if enabled {
                                        crate::log_info!("PLUGINS", "Toggled Slint widget '{}' ON -> launching standalone widget", id);
                                        let _ = launch_slint_widget(id);
                                    } else {
                                        crate::log_info!("PLUGINS", "Toggled Slint widget '{}' OFF -> terminating standalone widget", id);
                                        close_slint_widget(id);
                                    }
                                } else {
                                    if enabled {
                                        crate::log_info!("PLUGINS", "Toggled Web widget '{}' ON -> opening desktop widget", id);
                                        crate::system::native_wry::open_popout(id);
                                    } else {
                                        crate::log_info!("PLUGINS", "Toggled Web widget '{}' OFF -> closing desktop widget", id);
                                        crate::system::native_wry::close_popout(id);
                                    }
                                }
                            }

                            if let Some(svc) = manifest.service {
                                if enabled && svc.auto_start {
                                    let target_id = id.to_string();
                                    std::thread::spawn(move || {
                                        let _ = crate::system::plugin_service::start_service(&target_id);
                                    });
                                } else if !enabled && svc.auto_stop {
                                    let _ = crate::system::plugin_service::stop_service(id);
                                }
                            }
                        }
                    }

                    sync_slint_plugins_state();
                }
            }
            true
        }
        "open_plugins_dir" => {
            let _ = crate::commands::system_cmd::open_plugins_dir();
            true
        }
        "reload_plugins" => {
            sync_slint_plugins_state();
            crate::system::native_wry::reload_active_embedded();
            true
        }
        "collapse_dashboard" => {
            crate::log_info!("PLUGINS", "Received collapse_dashboard from Slint UI");
            hide_slint_embedded();
            crate::system::native_wry::hide_embedded();
            super::super::send_slint_ipc_message(serde_json::json!({ "action": "trigger_full_redraw" }));
            if let Some(app) = app {
                let app_clone = app.clone();
                let _ = app.run_on_main_thread(move || {
                    hide_embedded_webview(&app_clone);
                });
            }
            true
        }
        "select_plugin_tab" => {
            crate::log_info!("PLUGINS", "Received select_plugin_tab: val={:?}, app_is_some={}", val, app.is_some());
            if let Some(hwnd) = val.get("hwnd").and_then(|v| v.as_i64()) {
                if hwnd != 0 {
                    super::super::dialog::RECORDED_SLINT_HWND.store(hwnd as isize, std::sync::atomic::Ordering::SeqCst);
                    crate::log_info!("PLUGINS", "Recorded Slint HWND from action payload: 0x{:X}", hwnd);
                }
            }
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                crate::log_info!("PLUGINS", "User switched to plugin tab: '{}'", pid);
                if pid.is_empty() {
                    hide_slint_embedded();
                    crate::system::native_wry::hide_embedded();
                    super::super::send_slint_ipc_message(serde_json::json!({ "action": "trigger_full_redraw" }));
                    if let Some(app) = app {
                        let app_clone = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            hide_embedded_webview(&app_clone);
                        });
                    }
                } else {
                    let parent_hwnd_raw = super::super::dialog::get_openrevo_main_hwnd();
                    if parent_hwnd_raw != 0 {
                        let parent_hwnd = windows::Win32::Foundation::HWND(parent_hwnd_raw as *mut _);
                        let (target_x, target_y, target_w, target_h, _) = super::super::webview_dock::calc_embedded_geometry(parent_hwnd);

                        if is_slint_tab(pid) {
                            crate::log_info!("PLUGINS", "Detected Slint native tab plugin '{}' -> docking via native child process", pid);
                            crate::system::native_wry::hide_embedded();
                            hide_slint_embedded();

                            let current_exe = std::env::current_exe().unwrap_or_else(|_| "open-revo.exe".into());
                            let spawn_res = std::process::Command::new(current_exe)
                                .args(&[
                                    "--slint-widget",
                                    pid,
                                    "--embed",
                                    &parent_hwnd_raw.to_string(),
                                    &target_x.to_string(),
                                    &target_y.to_string(),
                                    &target_w.to_string(),
                                    &target_h.to_string(),
                                ])
                                .spawn();

                            match spawn_res {
                                Ok(child) => {
                                    assign_to_app_job(&child);
                                    crate::log_info!("PLUGINS", "Spawned Slint embedded child process successfully for '{}'", pid);
                                    *ACTIVE_SLINT_CHILD.lock() = Some(child);
                                }
                                Err(e) => {
                                    crate::log_error!("PLUGINS", "Failed to spawn Slint embedded child for '{}': {:?}", pid, e);
                                }
                            }
                        } else {
                            hide_slint_embedded();
                            crate::system::native_wry::attach_or_switch_embedded(
                                parent_hwnd_raw,
                                pid,
                                (target_x, target_y, target_w, target_h),
                            );
                        }

                        // Auto-start companion service if declared and configured with auto_start
                        let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
                        let m_path = p_dir.join("manifest.json");
                        if let Ok(content) = std::fs::read_to_string(&m_path) {
                            if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                                if let Some(svc) = manifest.service {
                                    if svc.auto_start {
                                        let target_pid = pid.to_string();
                                        std::thread::spawn(move || {
                                            let _ = crate::system::plugin_service::start_service(&target_pid);
                                        });
                                    }
                                }
                            }
                        }
                    } else {
                        crate::log_warn!("PLUGINS", "Cannot dock plugin '{}': Slint HWND is 0", pid);
                        super::super::send_slint_ipc_message(serde_json::json!({
                            "action": "set_active_plugin_id",
                            "plugin_id": ""
                        }));
                    }
                }
            }
            true
        }
        "open_custom_widget" => {
            crate::log_info!("PLUGINS", "Received open_custom_widget: val={:?}, app_is_some={}", val, app.is_some());
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                if is_slint_widget(pid) {
                    crate::log_info!("PLUGINS", "Opening standalone Slint widget for plugin: '{}'", pid);
                    let _ = launch_slint_widget(pid);
                } else {
                    crate::log_info!("PLUGINS", "Opening native Wry popout window for plugin: '{}'", pid);
                    crate::system::native_wry::open_popout(pid);
                }

                let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
                let m_path = p_dir.join("manifest.json");
                if let Ok(content) = std::fs::read_to_string(&m_path) {
                    if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                        if let Some(svc) = manifest.service {
                            if svc.auto_start {
                                let target_pid = pid.to_string();
                                std::thread::spawn(move || {
                                    let _ = crate::system::plugin_service::start_service(&target_pid);
                                });
                            }
                        }
                    }
                }
            }
            true
        }
        "open_plugin_dir" => {
            if let Some(pid) = val.get("plugin_id").and_then(|v| v.as_str()) {
                let p_dir = crate::commands::system_cmd::find_plugin_dir(pid);
                let _ = std::process::Command::new("explorer").arg(p_dir).spawn();
            }
            true
        }
        _ => false,
    }
}
