pub static IS_FILE_DIALOG_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub fn get_openrevo_main_hwnd() -> isize {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextW, IsWindowVisible,
    };

    unsafe {
        // 1. If foreground window is titled OpenRevo and visible, fast-path
        let fg = GetForegroundWindow();
        if !fg.0.is_null() && IsWindowVisible(fg).as_bool() {
            let mut title = [0u16; 64];
            let len = GetWindowTextW(fg, &mut title);
            if len > 0 {
                let s = String::from_utf16_lossy(&title[..len as usize]);
                if s == "OpenRevo" {
                    return fg.0 as isize;
                }
            }
        }

        // 2. Enumerate top-level windows to find visible window titled "OpenRevo"
        unsafe extern "system" fn enum_wnd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if IsWindowVisible(hwnd).as_bool() {
                let mut title = [0u16; 64];
                let len = GetWindowTextW(hwnd, &mut title);
                if len > 0 {
                    let s = String::from_utf16_lossy(&title[..len as usize]);
                    if s == "OpenRevo" {
                        let out_ptr = lparam.0 as *mut isize;
                        *out_ptr = hwnd.0 as isize;
                        return BOOL(0); // Stop enumeration
                    }
                }
            }
            BOOL(1) // Continue enumeration
        }

        let mut found_hwnd: isize = 0;
        let _ = EnumWindows(Some(enum_wnd_proc), LPARAM(&mut found_hwnd as *mut _ as isize));
        if found_hwnd != 0 {
            return found_hwnd;
        }

        // 3. Fallback to foreground window if non-null
        if !fg.0.is_null() {
            return fg.0 as isize;
        }

        0
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_openrevo_main_hwnd() -> isize {
    0
}

#[cfg(target_os = "windows")]
pub fn open_file_dialog(filter_name: &str, filter_ext: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, SetForegroundWindow};

    // Concurrency protection: ignore re-entrant clicks when dialog is already visible
    if IS_FILE_DIALOG_ACTIVE.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst).is_err() {
        crate::log_warn!("DIALOG", "File dialog is already active; ignoring duplicate invocation");
        return None;
    }

    struct DialogGuard;
    impl Drop for DialogGuard {
        fn drop(&mut self) {
            IS_FILE_DIALOG_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let _guard = DialogGuard;

    let hwnd_owner = get_openrevo_main_hwnd();
    if hwnd_owner != 0 {
        unsafe {
            let hwnd = HWND(hwnd_owner as *mut _);
            let _ = SetForegroundWindow(hwnd);
            let _ = BringWindowToTop(hwnd);
        }
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwndOwner: isize,
        hInstance: isize,
        lpstrFilter: *const u16,
        lpstrCustomFilter: *mut u16,
        nMaxCustFilter: u32,
        nFilterIndex: u32,
        lpstrFile: *mut u16,
        nMaxFile: u32,
        lpstrFileTitle: *mut u16,
        nMaxFileTitle: u32,
        lpstrInitialDir: *const u16,
        lpstrTitle: *const u16,
        Flags: u32,
        nFileOffset: u16,
        nFileExtension: u16,
        lpstrDefExt: *const u16,
        lCustData: isize,
        lpfnHook: usize,
        lpTemplateName: *const u16,
        pvReserved: *mut std::ffi::c_void,
        dwReserved: u32,
        FlagsEx: u32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    }

    let mut file_buf = [0u16; 1024];

    let spec = if filter_ext.starts_with("*.") {
        filter_ext.to_string()
    } else if filter_ext.contains(';') {
        filter_ext.split(';')
            .map(|s| {
                let trimmed = s.trim();
                if trimmed.starts_with("*.") {
                    trimmed.to_string()
                } else {
                    format!("*.{}", trimmed.trim_start_matches('.'))
                }
            })
            .collect::<Vec<_>>()
            .join(";")
    } else {
        format!("*.{}", filter_ext.trim_start_matches('.'))
    };

    let filter_str = format!("{} ({})\0{}\0All Files (*.*)\0*.*\0\0", filter_name, spec, spec);
    let filter_w: Vec<u16> = filter_str.encode_utf16().collect();
    let title_w: Vec<u16> = OsStr::new("选择文件").encode_wide().chain(std::iter::once(0)).collect();

    // OFN_PATHMUSTEXIST (0x800) | OFN_FILEMUSTEXIST (0x1000) | OFN_NOCHANGEDIR (0x8) | OFN_EXPLORER (0x80000) | OFN_ENABLESIZING (0x800000)
    let flags = 0x00000800 | 0x00001000 | 0x00000008 | 0x00080000 | 0x00800000;

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd_owner,
        hInstance: 0,
        lpstrFilter: filter_w.as_ptr(),
        lpstrCustomFilter: std::ptr::null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: 1,
        lpstrFile: file_buf.as_mut_ptr(),
        nMaxFile: file_buf.len() as u32,
        lpstrFileTitle: std::ptr::null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: std::ptr::null(),
        lpstrTitle: title_w.as_ptr(),
        Flags: flags,
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: std::ptr::null(),
        lCustData: 0,
        lpfnHook: 0,
        lpTemplateName: std::ptr::null(),
        pvReserved: std::ptr::null_mut(),
        dwReserved: 0,
        FlagsEx: 0,
    };

    let res = unsafe { GetOpenFileNameW(&mut ofn) };
    if res != 0 {
        let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
        use std::os::windows::ffi::OsStringExt;
        let path_str = std::ffi::OsString::from_wide(&file_buf[..len]);
        Some(std::path::PathBuf::from(path_str))
    } else {
        None
    }
}

use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

static SLINT_STDIN: Mutex<Option<ChildStdin>> = Mutex::new(None);

static LAST_CLOUD_ICC_IDENTITY: parking_lot::Mutex<Option<crate::system::cloud_icc::CloudIccIdentity>> = parking_lot::Mutex::new(None);
static LAST_CLOUD_ICC_HIT: parking_lot::Mutex<Option<crate::system::cloud_icc::CloudIccFetchHit>> = parking_lot::Mutex::new(None);
static SELECTED_MONITOR_INDEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Ensure the native Slint main window companion process is spawned
pub fn ensure_slint_main_process(app: Option<&AppHandle>) {
    let cfg = crate::system::config::load_config();
    if !cfg.main_slint_enabled {
        return;
    }

    let mut lock = match SLINT_STDIN.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };

    if lock.is_some() {
        return;
    }

    if let Ok(exe_path) = std::env::current_exe() {
        crate::log_info!("SLINT_MAIN", "Spawning native Slint Main Window companion: {:?}", exe_path);
        let mut cmd = Command::new(exe_path);
        cmd.arg("--slint-main");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    // Send initial state sync
                    let switches = crate::system::device_switches::get_switches_state();
                    let line = format!("{}\n", serde_json::json!({
                        "action": "sync_state",
                        "app_version": format!("v{}", env!("CARGO_PKG_VERSION")),
                        "power_mode": cfg.power_mode,
                        "gpu_mode": cfg.gpu_mode,
                        "refresh_rate": cfg.refresh_rate,
                        "auto_min_hz": cfg.auto_min_refresh_on_battery,
                        "brightness": 76,
                        "battery_limit": cfg.battery_limit,
                        "fan_boost": cfg.fan_boost,
                        "camera_enabled": switches.camera,
                        "mic_enabled": switches.mic,
                        "win_locked": cfg.win_key_locked,
                        "keep_awake": switches.keep_awake,
                        "autostart": cfg.autostart,
                        "touchpad_locked": !switches.touchpad,
                        "osd_enabled": switches.osd,
                        "fn_locked": switches.fn_lock,
                        "wifi_enabled": switches.wifi,
                        "bt_enabled": switches.bluetooth,
                    }));
                    let _ = stdin.write_all(line.as_bytes());
                    let _ = stdin.flush();
                    *lock = Some(stdin);
                }
                // Release lock before calling sync_slint_tuning_state to avoid self-deadlock
                drop(lock);

                // Immediately sync tuning state to Slint companion
                sync_slint_tuning_state();
    sync_slint_display_state();

                // Listen for stdout from companion
                if let Some(stdout) = child.stdout.take() {
                    let app_opt = app.cloned();
                    std::thread::Builder::new()
                        .name("slint-companion-stdout-reader".into())
                        .spawn(move || {
                            let reader = BufReader::new(stdout);
                            for line_res in reader.lines() {
                                if let Ok(line) = line_res {
                                    handle_slint_incoming_action(&line, app_opt.as_ref());
                                }
                            }
                        })
                        .ok();
                }
            }
            Err(e) => {
                crate::log_error!("SLINT_MAIN", "Failed to spawn Slint Main Window: {}", e);
            }
        }
    }
}

fn mutate_active_preset_profile<F>(f: F)
where
    F: FnOnce(&mut crate::system::config::CustomProfileConfig),
{
    let mut cfg = crate::system::config::load_config();
    let active_id = if !cfg.active_profile_id.is_empty() {
        cfg.active_profile_id.clone()
    } else {
        "preset_balance".to_string()
    };
    f(&mut cfg.custom_profile);

    // 同步到当前的预设文件
    let profiles = crate::system::config::list_profiles();
    if let Some(mut target) = profiles.into_iter().find(|p| p.id == active_id) {
        target.profile = cfg.custom_profile.clone();
        let _ = crate::system::config::save_profile_file(&target);
    }

    let _ = crate::system::config::save_config(&cfg);
}

fn handle_slint_incoming_action(line: &str, app: Option<&AppHandle>) {
    let val: serde_json::Value = match serde_json::from_str(line.trim()) {
        Ok(v) => v,
        Err(_) => return,
    };

    let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("");
    crate::log_info!("SLINT_MAIN", "Received action from Slint UI: {}", action);

    match action {
        "slint_ready" => {
            crate::log_info!("SLINT_MAIN", "Companion reported ready, sending full tuning state");
            sync_slint_tuning_state();
    sync_slint_display_state();
        }
        "set_power_mode" => {
            if let Some(mode) = val.get("mode").and_then(|m| m.as_i64()) {
                if let Some(app) = app {
                    let state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                    let _ = crate::commands::hardware_cmd::set_power_mode(mode as u8, app.clone(), state);
                }
            }
        }
        "open_custom_tuning" => {
            crate::log_info!("SLINT_MAIN", "Requested opening custom hardware tuning view");
            sync_slint_tuning_state();
    sync_slint_display_state();
            send_slint_ipc_message(serde_json::json!({
                "action": "open_tuning_view"
            }));
            if let Some(app) = app {
                let _ = app.emit("open-custom-tuning-modal", serde_json::json!({}));
            }
        }
        "set_gpu_mode" => {
            if let Some(gm) = val.get("gpu_mode").and_then(|m| m.as_str()) {
                if let Some(app) = app {
                    let gpu_state: tauri::State<crate::driver::gpu_bridge::SharedGpuBridge> = app.state();
                    let _ = crate::commands::hardware_cmd::set_gpu_mode(app.clone(), gm.to_string(), None, gpu_state);
                }
            }
        }
        "switch_refresh_rate" => {
            if let Some(hz) = val.get("hz").and_then(|h| h.as_i64()) {
                let sel_idx = SELECTED_MONITOR_INDEX.load(std::sync::atomic::Ordering::Relaxed);
                let monitors = crate::system::display::get_all_monitors();
                if sel_idx < monitors.len() && !monitors[sel_idx].is_primary && !monitors[sel_idx].is_internal {
                    let _ = crate::commands::display_cmd::set_display_monitor_refresh_rate(
                        monitors[sel_idx].device_name.clone(),
                        hz as u32,
                    );
                } else {
                    let _ = crate::commands::hardware_cmd::switch_refresh_rate(hz as u32);
                }
            }
        }
        "select_monitor_index" => {
            if let Some(idx) = val.get("index").and_then(|i| i.as_u64()) {
                SELECTED_MONITOR_INDEX.store(idx as usize, std::sync::atomic::Ordering::Relaxed);
                sync_slint_display_state();
            }
        }
        "toggle_auto_min_hz" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.auto_min_refresh_on_battery;
            let _ = crate::commands::hardware_cmd::set_auto_min_refresh_on_battery(next);
        }
        "set_brightness" => {
            if let Some(b) = val.get("brightness").and_then(|b| b.as_i64()) {
                if let Some(app) = app {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = crate::commands::hardware_cmd::set_display_brightness(app_clone, b as u8).await;
                    });
                }
            }
        }
        "set_battery_limit" => {
            if let Some(limit) = val.get("battery_limit").and_then(|b| b.as_i64()) {
                if let Some(app) = app {
                    let state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                    let _ = crate::commands::hardware_cmd::set_battery_limit(limit as u8, state);
                }
            }
        }
        "toggle_fan_boost" => {
            if let Some(app) = app {
                let cfg = crate::system::config::load_config();
                let next = !cfg.fan_boost;
                let state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                let _ = crate::commands::hardware_cmd::set_fan_boost(app.clone(), next, state);
            }
        }
        "toggle_camera" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.camera_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "camera".into(), next);
            }
        }
        "toggle_mic" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.mic_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "mic".into(), next);
            }
        }
        "toggle_win_lock" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.win_key_locked;
            let _ = crate::commands::hardware_cmd::set_win_key_locked(next);
        }
        "toggle_keep_awake" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.keep_awake_enabled;
            if let Some(app) = app {
                let _ = crate::commands::hardware_cmd::set_device_switch(app.clone(), "keep_awake".into(), next);
            }
        }
        "turn_off_screen" => {
            let _ = crate::commands::hardware_cmd::turn_off_screen();
        }
        "set_display_tuning_enabled" => {
            let enabled = val.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
            let _ = crate::commands::display_cmd::set_display_tuning_enabled(enabled);
            sync_slint_display_state();
        }
        "toggle_display_tuning" => {
            let state = crate::commands::display_cmd::get_display_tuning_state();
            let _ = crate::commands::display_cmd::set_display_tuning_enabled(!state.enabled);
            sync_slint_display_state();
        }
        "apply_display_tuning_preset" => {
            if let Some(preset_id) = val.get("preset_id").and_then(|p| p.as_str()) {
                let state = crate::commands::display_cmd::get_display_tuning_state();
                if let Some(target) = state.presets.into_iter().find(|p| p.id == preset_id) {
                    let _ = crate::commands::display_cmd::apply_display_color_preset(target);
                }
            }
            sync_slint_display_state();
        }
        "update_display_tuning_slider" => {
            if let (Some(key), Some(raw_val)) = (val.get("key").and_then(|k| k.as_str()), val.get("value").and_then(|v| v.as_i64())) {
                let state = crate::commands::display_cmd::get_display_tuning_state();
                let mut active = state.active_preset.clone();
                match key {
                    "temp_k" => active.color_temp_k = raw_val as u32,
                    "saturation" => active.saturation = (raw_val as f32) / 100.0,
                    "contrast" => active.contrast = (raw_val as f32) / 100.0,
                    "gamma" => active.gamma = (raw_val as f32) / 100.0,
                    "gain_r" => active.gain_r = (raw_val as f32) / 100.0,
                    "gain_g" => active.gain_g = (raw_val as f32) / 100.0,
                    "gain_b" => active.gain_b = (raw_val as f32) / 100.0,
                    _ => {}
                }
                let _ = crate::system::display_tuning::apply_color_preset(&active);
                let _ = crate::commands::display_cmd::apply_display_color_preset(active);
            }
        }
        "reset_display_tuning_preset" => {
            let state = crate::commands::display_cmd::get_display_tuning_state();
            let _ = crate::commands::display_cmd::reset_color_preset_to_default(state.active_preset.id);
            sync_slint_display_state();
        }
        "import_display_tuning_file" => {
            tauri::async_runtime::spawn(async move {
                if let Some(file_path) = open_file_dialog("JSON 调色预设", "json") {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let fname = file_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
                        let _ = crate::commands::display_cmd::import_color_preset_content(content, fname);
                        sync_slint_display_state();
                    }
                }
            });
        }
        "export_display_tuning_file" => {
            tauri::async_runtime::spawn(async move {
                let state = crate::commands::display_cmd::get_display_tuning_state();
                let default_name = format!("{}.json", state.active_preset.id);
                if let Some(dest) = crate::commands::tuning_cmd::save_file_dialog(&default_name, "JSON 调色预设", "json") {
                    if let Ok(json) = serde_json::to_string_pretty(&state.active_preset) {
                        let _ = std::fs::write(&dest, json);
                    }
                }
            });
        }
        "import_color_profile" => {
            tauri::async_runtime::spawn(async move {
                if let Some(file_path) = open_file_dialog("ICC/ICM 校色文件", "icc;*.icm") {
                    if let Ok(bytes) = std::fs::read(&file_path) {
                        let file_name = file_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("imported.icc")
                            .to_string();
                        let _ = crate::commands::display_cmd::import_custom_color_profile(file_name, bytes, None).await;
                        sync_slint_display_state();
                    }
                }
            });
        }
        "open_cloud_icc" | "refresh_cloud_icc_identity" => {
            tauri::async_runtime::spawn(async move {
                let id_res = tauri::async_runtime::spawn_blocking(crate::system::cloud_icc::get_cloud_icc_identity).await;
                if let Ok(Ok(id)) = id_res {
                    let main_mon = id.monitors.iter().find(|m| m.is_internal).or_else(|| id.monitors.first());
                    let panel_pnp = main_mon.map(|m| m.pnp.clone()).unwrap_or_else(|| "未检出".into());
                    let panel_sn = main_mon.map(|m| if m.panel_serial.is_empty() { "(无独立面板SN)".to_string() } else { m.panel_serial.clone() }).unwrap_or_else(|| "(无独立面板SN)".into());
                    let mac_str = if id.mac_addresses.is_empty() { "未检出".into() } else { id.mac_addresses.join(" / ") };

                    *LAST_CLOUD_ICC_IDENTITY.lock() = Some(id.clone());

                    send_slint_ipc_message(serde_json::json!({
                        "action": "sync_cloud_icc_identity",
                        "panel_pnp": panel_pnp,
                        "panel_sn": panel_sn,
                        "bios_sn": id.bios_serial,
                        "mac_address": mac_str,
                        "arch_name": id.arch_name,
                        "recommended_method": id.recommended_method
                    }));
                }
            });
        }
        "start_cloud_icc_search" => {
            let mode = val.get("mode").and_then(|v| v.as_str()).unwrap_or("auto").to_string();
            tauri::async_runtime::spawn(async move {
                send_slint_ipc_message(serde_json::json!({
                    "action": "sync_cloud_icc_searching",
                    "is_searching": true,
                    "status": "正在连接云端校色数据库..."
                }));

                let id = {
                    let guard = LAST_CLOUD_ICC_IDENTITY.lock();
                    guard.clone()
                };

                let id = match id {
                    Some(i) => i,
                    None => {
                        let res = tauri::async_runtime::spawn_blocking(crate::system::cloud_icc::get_cloud_icc_identity).await;
                        match res {
                            Ok(Ok(i)) => {
                                *LAST_CLOUD_ICC_IDENTITY.lock() = Some(i.clone());
                                i
                            }
                            _ => {
                                send_slint_ipc_message(serde_json::json!({
                                    "action": "sync_cloud_icc_hit",
                                    "hit_found": false,
                                    "toast": "未能获取本机硬件特征指纹"
                                }));
                                return;
                            }
                        }
                    }
                };

                let mut candidates: Vec<(String, String)> = Vec::new();
                if mode == "50" {
                    for c in &id.candidates_method1 { candidates.push((c.clone(), "50".into())); }
                } else if mode == "40" {
                    for c in &id.candidates_method2 { candidates.push((c.clone(), "40".into())); }
                } else {
                    for c in &id.candidates_method1 { candidates.push((c.clone(), "50".into())); }
                    for c in &id.candidates_method2 { candidates.push((c.clone(), "40".into())); }
                    for c in &id.candidates_fallback { candidates.push((c.clone(), "auto".into())); }
                }

                let mut hit_opt = None;
                for (cand, meth) in candidates {
                    let probe_res = crate::commands::display_cmd::fetch_cloud_icc_profile(cand.clone(), Some(meth), None).await;
                    if let Ok(hit) = probe_res {
                        if !hit.icc_profiles.is_empty() {
                            hit_opt = Some(hit);
                            break;
                        }
                    }
                }

                if let Some(hit) = hit_opt {
                    let is_uniwill = hit.server_url.contains("uniwill");
                    let count = hit.icc_profiles.len();
                    let meta = format!(
                        "包大小: {:.1} KB · 节点: {} · 内含 {} 个有效色彩配置",
                        (hit.file_size as f64) / 1024.0,
                        if is_uniwill { "Uniwill (40系)" } else { "Aistone (50系)" },
                        count
                    );
                    let profile_names: Vec<String> = hit.icc_profiles.iter().map(|p| p.file_name.clone()).collect();
                    *LAST_CLOUD_ICC_HIT.lock() = Some(hit.clone());

                    send_slint_ipc_message(serde_json::json!({
                        "action": "sync_cloud_icc_hit",
                        "hit_found": true,
                        "file_name": hit.file_name,
                        "meta": meta,
                        "profiles": profile_names
                    }));
                } else {
                    *LAST_CLOUD_ICC_HIT.lock() = None;
                    send_slint_ipc_message(serde_json::json!({
                        "action": "sync_cloud_icc_hit",
                        "hit_found": false,
                        "toast": "所有候选路径均未命中，当前机型可能未在服务器留存出厂专属校色"
                    }));
                }
            });
        }
        "apply_cloud_icc_profile" => {
            let idx = val.get("index").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            tauri::async_runtime::spawn(async move {
                let target = {
                    let guard = LAST_CLOUD_ICC_HIT.lock();
                    guard.as_ref().and_then(|h| h.icc_profiles.get(idx).cloned())
                };

                if let Some(p) = target {
                    let import_res = crate::commands::display_cmd::import_custom_color_profile(p.file_name.clone(), p.file_bytes, None).await;
                    if let Ok(st) = import_res {
                        let _ = crate::commands::display_cmd::apply_color_profile(st.active_profile_id, None).await;
                        sync_slint_display_state();
                        send_slint_ipc_message(serde_json::json!({
                            "action": "cloud_icc_toast",
                            "toast": format!("✔ 已成功应用原厂校色 [{}] 至当前显示器！", p.file_name)
                        }));
                    }
                }
            });
        }
        "apply_all_cloud_icc_profiles" => {
            tauri::async_runtime::spawn(async move {
                let profiles = {
                    let guard = LAST_CLOUD_ICC_HIT.lock();
                    guard.as_ref().map(|h| h.icc_profiles.clone()).unwrap_or_default()
                };

                if !profiles.is_empty() {
                    let mut last_id = None;
                    for p in &profiles {
                        if let Ok(st) = crate::commands::display_cmd::import_custom_color_profile(p.file_name.clone(), p.file_bytes.clone(), None).await {
                            last_id = Some(st.active_profile_id);
                        }
                    }
                    if let Some(id) = last_id {
                        let _ = crate::commands::display_cmd::apply_color_profile(id, None).await;
                    }
                    sync_slint_display_state();
                    send_slint_ipc_message(serde_json::json!({
                        "action": "cloud_icc_toast",
                        "toast": format!("🎉 全部 {} 个原厂色彩配置已全量导入并同步至显卡色彩管理！", profiles.len())
                    }));
                }
            });
        }
        "apply_color_profile" => {
            let prof_id = val.get("profile_id").or_else(|| val.get("profile_name")).and_then(|p| p.as_str()).unwrap_or("default_native").to_string();
            tauri::async_runtime::spawn(async move {
                let _ = crate::commands::display_cmd::apply_color_profile(prof_id, None).await;
                sync_slint_display_state();
            });
        }
        "toggle_autostart" => {
            let cfg = crate::system::config::load_config();
            let next = !cfg.autostart;
            let _ = crate::commands::hardware_cmd::set_autostart_enabled(next);
        }
        "toggle_touchpad" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.touchpad;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "touchpad".into(), next);
            }
        }
        "toggle_osd" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.osd;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "osd".into(), next);
            }
        }
        "toggle_fn_lock" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.fn_lock;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "fn_lock".into(), next);
            }
        }
        "toggle_wifi" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.wifi;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "wifi".into(), next);
            }
        }
        "toggle_bt" => {
            let state = crate::system::device_switches::get_switches_state();
            let next = !state.bluetooth;
            if let Some(app) = app {
                let _ = crate::commands::switches_cmd::set_device_switch(app.clone(), "bluetooth".into(), next);
            }
        }
        "expand_dashboard" => {
            // Expanded natively inside Slint
            crate::log_info!("SLINT_MAIN", "Dashboard expanded to full native view");
        }

        // Custom Tuning Actions
        "tuning_back_to_overview" => {
            crate::log_info!("SLINT_MAIN", "Returned from custom tuning to overview");
        }
        "tuning_toggle_temp_wall" => {
            mutate_active_preset_profile(|p| p.cpu_temp_target_enabled = !p.cpu_temp_target_enabled);
        }
        "tuning_set_temp_wall" => {
            if let Some(t) = val.get("temp_wall").and_then(|v| v.as_i64()) {
                let temp = t as u8;
                mutate_active_preset_profile(|p| p.cpu_temp_target = temp);
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::apply_live_custom_tweak(
                        app.clone(), None, None, None, Some(temp), None, None, None, None, None, None, None, None, None
                    );
                }
            }
        }
        "tuning_set_pl1" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u16;
                mutate_active_preset_profile(|p| p.cpu_pl1_watts = watts);
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::apply_live_custom_tweak(
                        app.clone(), Some(watts), None, None, None, None, None, None, None, None, None, None, None, None
                    );
                }
            }
        }
        "tuning_set_pl2" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u16;
                mutate_active_preset_profile(|p| p.cpu_pl2_watts = watts);
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::apply_live_custom_tweak(
                        app.clone(), None, Some(watts), None, None, None, None, None, None, None, None, None, None, None
                    );
                }
            }
        }
        "tuning_set_pl4" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u16;
                mutate_active_preset_profile(|p| p.cpu_pl4_watts = watts);
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::apply_live_custom_tweak(
                        app.clone(), None, None, Some(watts), None, None, None, None, None, None, None, None, None, None
                    );
                }
            }
        }
        "tuning_toggle_bios_oc" => {
            mutate_active_preset_profile(|p| p.bios_oc_enabled = !p.bios_oc_enabled);
            let cfg = crate::system::config::load_config();
            let _ = crate::driver::uefi_nvram::write_bios_advanced_menu_status(cfg.custom_profile.bios_oc_enabled);
        }
        "tuning_toggle_super_perf" => {
            mutate_active_preset_profile(|p| p.gpu_super_perf = !p.gpu_super_perf);
            let cfg = crate::system::config::load_config();
            if let Some(app) = app {
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak(
                    app.clone(), None, None, None, None, None, None, None, None, Some(cfg.custom_profile.gpu_super_perf), None, None, None, None
                );
            }
        }
        "tuning_set_core_offset" => {
            if let Some(o) = val.get("offset").and_then(|v| v.as_i64()) {
                let offset = o as i32;
                mutate_active_preset_profile(|p| p.gpu_core_offset_mhz = offset);
                let cfg = crate::system::config::load_config();
                let mem = cfg.custom_profile.gpu_mem_offset_mhz;
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::set_gpu_clock_offsets(app.clone(), offset, mem);
                }
            }
        }
        "tuning_set_mem_offset" => {
            if let Some(o) = val.get("offset").and_then(|v| v.as_i64()) {
                let offset = o as i32;
                mutate_active_preset_profile(|p| p.gpu_mem_offset_mhz = offset);
                let cfg = crate::system::config::load_config();
                let core = cfg.custom_profile.gpu_core_offset_mhz;
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::set_gpu_clock_offsets(app.clone(), core, offset);
                }
            }
        }
        "tuning_toggle_db" => {
            mutate_active_preset_profile(|p| p.gpu_db_enabled = !p.gpu_db_enabled);
            let cfg = crate::system::config::load_config();
            if let Some(app) = app {
                let _ = crate::commands::tuning_cmd::apply_live_custom_tweak(
                    app.clone(), None, None, None, None, None, None, Some(cfg.custom_profile.gpu_db_enabled), None, None, None, None, None, None
                );
            }
        }
        "tuning_set_db_watts" => {
            if let Some(w) = val.get("watts").and_then(|v| v.as_i64()) {
                let watts = w as u8;
                mutate_active_preset_profile(|p| p.gpu_db_watts = watts);
                if let Some(app) = app {
                    let _ = crate::commands::tuning_cmd::apply_live_custom_tweak(
                        app.clone(), None, None, None, None, None, None, None, Some(watts), None, None, None, None, None
                    );
                }
            }
        }
        "tuning_toggle_smooth" => {
            mutate_active_preset_profile(|p| p.fan_ramp_rate = !p.fan_ramp_rate);
            let cfg = crate::system::config::load_config();
            let _ = crate::commands::tuning_cmd::toggle_fan_ramp_rate(cfg.custom_profile.fan_ramp_rate, Some(cfg.custom_profile.fan_ramp_speed_ms));
        }
        "tuning_toggle_isolated" => {
            mutate_active_preset_profile(|p| p.fan_isolated_output = !p.fan_isolated_output);
            let cfg = crate::system::config::load_config();
            if let Some(app) = app {
                let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                let _ = crate::commands::tuning_cmd::toggle_fan_isolated_output(app.clone(), ec_state, cfg.custom_profile.fan_isolated_output, None, None);
            }
        }
        "tuning_toggle_link" => {
            mutate_active_preset_profile(|p| p.fan_curve_linked = !p.fan_curve_linked);
        }
        "tuning_set_fan_curve" => {
            if let (Some(c_arr), Some(g_arr)) = (val.get("cpu_curve").and_then(|v| v.as_array()), val.get("gpu_curve").and_then(|v| v.as_array())) {
                let c_pcts: Vec<u8> = c_arr.iter().filter_map(|x| x.as_i64().map(|v| v as u8)).collect();
                let g_pcts: Vec<u8> = g_arr.iter().filter_map(|x| x.as_i64().map(|v| v as u8)).collect();
                if c_pcts.len() == 8 && g_pcts.len() == 8 {
                    let default_curve = crate::system::config::default_fan_curve();
                    let mut new_cpu_curve = default_curve.clone();
                    let mut new_gpu_curve = default_curve.clone();
                    for (i, p) in new_cpu_curve.iter_mut().enumerate() {
                        p.duty = c_pcts[i];
                    }
                    for (i, p) in new_gpu_curve.iter_mut().enumerate() {
                        p.duty = g_pcts[i];
                    }
                    let c_clone = new_cpu_curve.clone();
                    let g_clone = new_gpu_curve.clone();
                    mutate_active_preset_profile(|p| {
                        p.cpu_fan_curve = c_clone;
                        p.gpu_fan_curve = g_clone;
                    });
                    if let Some(app) = app {
                        let app_clone = app.clone();
                        std::thread::spawn(move || {
                            let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app_clone.state();
                            let _ = crate::commands::tuning_cmd::apply_fan_curve_live(app_clone.clone(), ec_state, new_cpu_curve, new_gpu_curve);
                        });
                    }
                }
            }
        }
        "tuning_toggle_bld_boost" => {
            if let Some(app) = app {
                let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                let is_clamped = crate::driver::bld_driver_mgr::is_bld_fan_clamped();
                let target = if is_clamped { 0 } else { 5800 };
                let _ = crate::commands::tuning_cmd::set_bld_fan_speed_cap(app.clone(), ec_state, target);
            }
        }
        "tuning_change_bld_cap" => {
            if let Some(rpm) = val.get("rpm").and_then(|v| v.as_i64()) {
                if let Some(app) = app {
                    let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                    let _ = crate::commands::tuning_cmd::set_bld_fan_speed_cap(app.clone(), ec_state, rpm as u16);
                }
            }
        }
        "tuning_restore_bld_auto" => {
            if let Some(app) = app {
                let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                let _ = crate::commands::tuning_cmd::set_bld_fan_speed_cap(app.clone(), ec_state, 0);
            }
        }
        "tuning_select_preset" => {
            if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                if let Some(app) = app {
                    let ec_state: tauri::State<crate::driver::ec_bridge::SharedEcBridge> = app.state();
                    let _ = crate::commands::tuning_cmd::set_active_profile_id(id.to_string(), ec_state);
                } else {
                    let mut cfg = crate::system::config::load_config();
                    cfg.active_profile_id = id.to_string();
                    let _ = crate::system::config::save_config(&cfg);
                }
                sync_slint_tuning_state();
    sync_slint_display_state();
            }
        }
        "tuning_create_preset" => {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() % 10000).unwrap_or(0);
            let name = format!("极客方案 {}", now);
            let _ = crate::commands::tuning_cmd::create_profile_preset(name, None, None);
            sync_slint_tuning_state();
    sync_slint_display_state();
        }
        "tuning_reset_preset" => {
            let active_id = {
                let cfg = crate::system::config::load_config();
                if !cfg.active_profile_id.is_empty() {
                    cfg.active_profile_id
                } else {
                    "preset_balance".to_string()
                }
            };
            crate::log_info!("TUNING_RESET", "Starting reset for preset: {}", active_id);
            match crate::commands::tuning_cmd::reset_profile_preset(active_id.clone()) {
                Ok(target) => {
                    crate::log_info!("TUNING_RESET", "Successfully reset preset {} to factory baseline", active_id);
                    let target_profile = target.profile.clone();
                    mutate_active_preset_profile(|p| {
                        *p = target_profile.clone();
                    });
                    // 承担清除 CPU 和 GPU 异常锁功耗、打破 55W Limp Mode、清除 105C / 0x00 锁死温度墙
                    let _ = crate::commands::tuning_cmd::heal_gpu_power_contract(None, None);
                    let _ = crate::commands::tuning_cmd::heal_cpu_power_contract(None, None, None, None, None);
                    let is_declarative = crate::hal::declarative::load_matched_manifest().is_some();
                    if is_declarative {
                        crate::commands::tuning_cmd::sync_custom_profile_to_declarative_hardware(&target.profile);
                    }
                }
                Err(e) => {
                    crate::log_error!("TUNING_RESET", "Failed to reset preset {}: {}", active_id, e);
                }
            }
            sync_slint_tuning_state();
    sync_slint_display_state();
        }
        "tuning_import_preset" => {
            tauri::async_runtime::spawn(async move {
                if let Some(file_path) = open_file_dialog("JSON 调教方案", "json") {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let fname = file_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
                        let _ = crate::commands::tuning_cmd::import_profile_preset(content, fname);
                        sync_slint_tuning_state();
                        sync_slint_display_state();
                    }
                }
            });
        }
        "tuning_export_preset" => {
            let active_id = {
                let cfg = crate::system::config::load_config();
                if !cfg.active_profile_id.is_empty() {
                    cfg.active_profile_id
                } else {
                    "preset_balance".to_string()
                }
            };
            let _ = crate::commands::tuning_cmd::export_profile_preset(active_id);
        }
        "tuning_delete_preset" => {
            let active_id = {
                let cfg = crate::system::config::load_config();
                if !cfg.active_profile_id.is_empty() {
                    cfg.active_profile_id
                } else {
                    "preset_balance".to_string()
                }
            };
            let _ = crate::commands::tuning_cmd::delete_profile_preset(active_id);
            sync_slint_tuning_state();
    sync_slint_display_state();
        }
        "tuning_open_preset_dir" => {
            let _ = crate::commands::tuning_cmd::open_profiles_folder();
        }
        _ => {}
    }
}

pub fn send_slint_ipc_message(payload: serde_json::Value) {
    let mut lock = match SLINT_STDIN.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };

    if let Some(ref mut stdin) = *lock {
        let line = format!("{}\n", payload);
        let _ = stdin.write_all(line.as_bytes());
        let _ = stdin.flush();
    }
}

pub fn show_slint_window() {
    send_slint_ipc_message(serde_json::json!({ "action": "show" }));
}

pub fn hide_slint_window() {
    send_slint_ipc_message(serde_json::json!({ "action": "hide" }));
}

pub fn sync_slint_telemetry(state: &crate::commands::system_cmd::HardwareState) {
    let vram_used_gb = (state.gpu_vram_used_mb as f64) / 1024.0;
    let vram_total_gb = (state.gpu_vram_mb as f64) / 1024.0;
    let vram_text = if state.gpu_vram_mb > 0 {
        format!("{:.1} / {:.1} GB", vram_used_gb, vram_total_gb)
    } else {
        "- / -".to_string()
    };

    let cpu_watts_text = format!("{:.1} W", state.cpu_power_w);
    let gpu_watts_text = format!("{:.1} W", state.gpu_power_w);

    let sel_idx = SELECTED_MONITOR_INDEX.load(std::sync::atomic::Ordering::Relaxed);
    let (monitor_name, curr_hz, supported_rates) = if !state.monitors.is_empty() {
        let m = if sel_idx < state.monitors.len() { &state.monitors[sel_idx] } else { &state.monitors[0] };
        (m.friendly_name.clone(), m.current_hz, m.supported_rates.clone())
    } else {
        ("京东方 (主屏)".to_string(), state.refresh_rate, vec![60, 144, 240])
    };

    let (rates_row1, rates_row2) = if supported_rates.len() > 5 {
        let mid = (supported_rates.len() + 1) / 2;
        (supported_rates[..mid].to_vec(), supported_rates[mid..].to_vec())
    } else {
        (supported_rates.clone(), Vec::new())
    };

    send_slint_ipc_message(serde_json::json!({
        "action": "telemetry",
        "cpu_temp": state.cpu_temp,
        "cpu_mhz": state.cpu_freq,
        "cpu_watts_text": cpu_watts_text,
        "cpu_usage": state.cpu_usage,
        "cpu_name": state.cpu_name,
        "gpu_temp": state.gpu_temp,
        "gpu_mhz": state.gpu_core_freq,
        "gpu_watts_text": gpu_watts_text,
        "gpu_usage": state.gpu_usage,
        "gpu_name": state.gpu_name,
        "cpu_rpm": state.cpu_fan_rpm,
        "gpu_rpm": state.gpu_fan_rpm,
        "vram_text": vram_text,
        "vram_mhz": state.gpu_mem_freq,
        "vram_usage": if state.gpu_vram_mb > 0 { ((state.gpu_vram_used_mb as f64 / state.gpu_vram_mb as f64) * 100.0) as u32 } else { 0 },
        "refresh_rate": curr_hz,
        "monitor_name": monitor_name,
        "rates_row1": rates_row1,
        "rates_row2": rates_row2,
        "power_mode": state.power_mode,
        "fan_boost": state.fan_boost,
    }));
}

pub fn sync_slint_tuning_state() {
    let cfg = crate::system::config::load_config();
    let manifest = crate::hal::declarative::load_matched_manifest();
    let is_bld = manifest.as_ref().map(|m| m.id.contains("bld")).unwrap_or(false);
    let (_cpu_name, is_amd) = crate::driver::uefi_nvram::get_cpu_info();
    let cpu_vendor = if is_amd { "amd" } else { "intel" };
    let cpu_tuning_type = if manifest.as_ref().and_then(|m| m.capabilities.cpu_tuning_type.as_deref()) == Some("none") {
        "none"
    } else if is_amd {
        "amd_smu"
    } else {
        "intel_pl"
    };
    let cpu_supports_fppt = manifest.as_ref().and_then(|m| m.capabilities.cpu_supports_fppt).unwrap_or(!is_bld);

    let active_id = {
        let cfg = crate::system::config::load_config();
        if !cfg.active_profile_id.is_empty() {
            cfg.active_profile_id
        } else {
            "preset_balance".to_string()
        }
    };
    let presets = crate::commands::tuning_cmd::get_profiles_list().unwrap_or_default();
    let current_preset = presets.iter().find(|p| p.id == active_id).cloned().or_else(|| presets.first().cloned());
    let profile = current_preset.as_ref().map(|p| &p.profile).unwrap_or(&cfg.custom_profile);
    let preset_name = current_preset.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "均衡电竞".to_string());

    let cpu_curve: Vec<i32> = profile.cpu_fan_curve.iter().map(|p| p.duty as i32).collect();
    let gpu_curve: Vec<i32> = profile.gpu_fan_curve.iter().map(|p| p.duty as i32).collect();

    let base_tgp = crate::system::power_mode::get_cached_factory_power_presets().map(|fp| fp.base_tgp).unwrap_or(150);
    let is_clamped = crate::driver::bld_driver_mgr::is_bld_fan_clamped();

    let preset_ids: Vec<String> = presets.iter().map(|p| p.id.clone()).collect();
    let preset_names: Vec<String> = presets.iter().map(|p| p.name.clone()).collect();

    let payload = serde_json::json!({
        "action": "sync_tuning_state",
        "cpu_vendor": cpu_vendor,
        "cpu_tuning_type": cpu_tuning_type,
        "cpu_supports_fppt": cpu_supports_fppt,
        "is_bld": is_bld,
        "active_preset_name": preset_name,
        "preset_ids": preset_ids,
        "preset_names": preset_names,
        "temp_wall": profile.cpu_temp_target,
        "temp_wall_enabled": profile.cpu_temp_target_enabled,
        "temp_wall_max": 95,
        "pl1": profile.cpu_pl1_watts,
        "pl1_max": 230,
        "pl2": profile.cpu_pl2_watts,
        "pl2_max": 230,
        "pl4": profile.cpu_pl4_watts,
        "pl4_max": 275,
        "bios_oc_enabled": profile.bios_oc_enabled,
        "super_perf": profile.gpu_super_perf,
        "core_offset": profile.gpu_core_offset_mhz,
        "mem_offset": profile.gpu_mem_offset_mhz,
        "db_enabled": profile.gpu_db_enabled,
        "db_watts": profile.gpu_db_watts,
        "base_tgp": base_tgp,
        "smooth_transition": profile.fan_ramp_rate,
        "smooth_time": format!("{}ms", profile.fan_ramp_speed_ms),
        "fan_isolated_output": profile.fan_isolated_output,
        "fan_linked": profile.fan_curve_linked,
        "cpu_curve": cpu_curve,
        "gpu_curve": gpu_curve,
        "bld_rpm_cap": 5800,
        "bld_boost": is_clamped,
    });

    send_slint_ipc_message(payload);
}


pub fn sync_slint_display_state() {
    let tuning_state = crate::commands::display_cmd::get_display_tuning_state();
    let calib_state = crate::system::color_calibration::get_color_calibration_state(None);

    let monitors = crate::system::display::get_all_monitors();
    let sel_idx = SELECTED_MONITOR_INDEX.load(std::sync::atomic::Ordering::Relaxed);
    let (monitor_name, curr_hz, supported_rates) = if !monitors.is_empty() {
        let m = if sel_idx < monitors.len() { &monitors[sel_idx] } else { &monitors[0] };
        (m.friendly_name.clone(), m.current_hz, m.supported_rates.clone())
    } else {
        ("京东方 (主屏)".to_string(), 240, vec![60, 144, 240])
    };

    let (rates_row1, rates_row2) = if supported_rates.len() > 4 {
        let mid = (supported_rates.len() + 1) / 2;
        (supported_rates[..mid].to_vec(), supported_rates[mid..].to_vec())
    } else {
        (supported_rates.clone(), Vec::new())
    };

    let mut profiles_list: Vec<String> = Vec::new();
    let mut profiles_ids: Vec<String> = Vec::new();
    for p in &calib_state.profiles {
        profiles_list.push(p.name.clone());
        profiles_ids.push(p.id.clone());
    }

    send_slint_ipc_message(serde_json::json!({
        "action": "sync_display_state",
        "tuning_enabled": tuning_state.enabled,
        "active_preset_id": tuning_state.active_preset.id,
        "active_preset_name": tuning_state.active_preset.name,
        "tuning_temp_k": tuning_state.active_preset.color_temp_k,
        "tuning_saturation": (tuning_state.active_preset.saturation * 100.0).round() as i64,
        "tuning_contrast": (tuning_state.active_preset.contrast * 100.0).round() as i64,
        "tuning_gamma": (tuning_state.active_preset.gamma * 100.0).round() as i64,
        "tuning_gain_r": (tuning_state.active_preset.gain_r * 100.0).round() as i64,
        "tuning_gain_g": (tuning_state.active_preset.gain_g * 100.0).round() as i64,
        "tuning_gain_b": (tuning_state.active_preset.gain_b * 100.0).round() as i64,
        "tuning_is_modified": false,
        "active_color_profile": calib_state.active_profile_name,
        "active_color_profile_id": calib_state.active_profile_id,
        "color_profiles_list": profiles_list,
        "color_profiles_ids": profiles_ids,
        "monitor_name": monitor_name,
        "refresh_rate": curr_hz,
        "rates_row1": rates_row1,
        "rates_row2": rates_row2,
    }));
}
