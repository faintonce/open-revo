use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::Mutex;
use tauri::AppHandle;

pub mod actions;
pub mod dialog;
pub mod sync;
pub mod webview_dock;

pub use actions::handle_slint_incoming_action;
pub use dialog::*;
pub use sync::*;
pub use webview_dock::*;

pub(crate) static SLINT_STDIN: Mutex<Option<ChildStdin>> = Mutex::new(None);
pub(crate) static LAST_CLOUD_ICC_IDENTITY: parking_lot::Mutex<Option<crate::system::cloud_icc::CloudIccIdentity>> = parking_lot::Mutex::new(None);
pub(crate) static LAST_CLOUD_ICC_HIT: parking_lot::Mutex<Option<crate::system::cloud_icc::CloudIccFetchHit>> = parking_lot::Mutex::new(None);
pub(crate) static SELECTED_MONITOR_INDEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub(crate) static SLINT_APP_HANDLE: std::sync::Mutex<Option<AppHandle>> = std::sync::Mutex::new(None);
static LAST_SLINT_TOGGLE: Mutex<Option<std::time::Instant>> = Mutex::new(None);

/// Ensure the native Slint main window companion process is spawned
pub fn ensure_slint_main_process(app: Option<&AppHandle>) {
    if let Some(a) = app {
        if let Ok(mut g) = SLINT_APP_HANDLE.lock() {
            *g = Some(a.clone());
        }
    }

    let cfg = crate::system::config::load_config();
    if cfg.use_classic_webview {
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
        let should_expand = match cfg.wake_window_mode.as_str() {
            "full" => true,
            "mini" => false,
            _ => cfg.last_window_expanded,
        };
        if should_expand {
            cmd.arg("--expanded");
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    // Send initial state sync
                    let switches = crate::system::device_switches::get_switches_state();
                    let (live_percent, live_ac, _) = crate::commands::power_cmd::get_live_power_status();
                    let should_expand = match cfg.wake_window_mode.as_str() {
                        "full" => true,
                        "mini" => false,
                        _ => cfg.last_window_expanded,
                    };
                    let (init_pl1, init_tgp, init_power_text) = sync::get_current_power_targets(cfg.power_mode);
                    let line = format!("{}\n", serde_json::json!({
                        "action": "sync_state",
                        "app_version": format!("v{}", env!("CARGO_PKG_VERSION")),
                        "power_mode": cfg.power_mode,
                        "pl1_target": init_pl1,
                        "tgp_target": init_tgp,
                        "power_target_text": init_power_text,
                        "gpu_mode": cfg.gpu_mode,
                        "refresh_rate": cfg.refresh_rate,
                        "auto_min_hz": cfg.auto_min_refresh_on_battery,
                        "brightness": 76,
                        "battery_limit": cfg.battery_limit,
                        "battery_percent": live_percent,
                        "is_ac_online": live_ac,
                        "is_dc_online": crate::commands::power_cmd::is_dc_round_plugged_in(),
                        "is_thin_and_light": crate::commands::power_cmd::is_thin_and_light_or_pure_igpu(),
                        "battery_mode": if cfg.battery_limit >= 95 { "long_life" } else if cfg.battery_limit <= 65 { "station" } else { "balance" },
                        "usb_charge_enabled": cfg.usb_charge_enabled,
                        "ac_recovery_enabled": cfg.ac_recovery_enabled,
                        "high_perf_scheme": cfg.high_perf_scheme,
                        "sleep_guard_enabled": cfg.sleep_guard_enabled,
                        "sleep_disconnect_net": cfg.sleep_disconnect_net,
                        "sleep_block_away": cfg.sleep_block_away,
                        "sleep_ignore_sysreq": cfg.sleep_ignore_sysreq,
                        "sleep_disable_wake_timers": cfg.sleep_disable_wake_timers,
                        "sleep_deep_sleep": cfg.sleep_deep_sleep,
                        "sleep_hibernate_30m": cfg.sleep_hibernate_30m,
                        "sleep_auto_office": cfg.sleep_auto_office,
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
                        "balanced_db_enabled": cfg.balanced_gpu_dynamic_boost,
                        "oc_core_mhz": cfg.gpu_oc_core_mhz,
                        "oc_mem_mhz": cfg.gpu_oc_mem_mhz,
                        "unlock_battery_limit": cfg.unlock_battery_power_limit,
                        "wake_window_mode": crate::commands::window_cmd::get_wake_window_mode(),
                        "is_expanded": should_expand,
                        "theme_mode": cfg.theme_mode,
                        "mini_auto_hide": crate::commands::window_cmd::get_mini_auto_hide(),
                        "use_classic_webview": cfg.use_classic_webview,
                        "update_channel": cfg.update_channel,
                        "full_window_x": cfg.full_window_x,
                        "full_window_y": cfg.full_window_y,
                        "osd_config": {
                            "enabled": switches.osd,
                            "theme": cfg.osd_theme,
                            "scale": cfg.osd_scale,
                            "position": cfg.osd_position,
                        },
                        "data_dir_info": crate::commands::system_cmd::get_user_data_dir_info().unwrap_or(crate::commands::system_cmd::UserDataDirInfo {
                            path: "C:\\Users\\AppData\\Roaming\\OpenRevo".into(),
                            mode: "appdata".into(),
                        }),
                    }));
                    let _ = stdin.write_all(line.as_bytes());
                    let _ = stdin.flush();
                    *lock = Some(stdin);
                }
                // Release lock before calling sync_slint_tuning_state to avoid self-deadlock
                drop(lock);

                // Immediately sync states to Slint companion
                sync_slint_tuning_state();
                sync_slint_display_state();
                sync_slint_power_state();
                sync_slint_switches_state();
                sync_slint_lighting_state();
                sync_slint_water_state(None, None);
                sync_slint_power_schemes(None);
                sync_slint_plugins_state();

                // Listen for stdout from companion
                if let Some(stdout) = child.stdout.take() {
                    std::thread::Builder::new()
                        .name("slint-companion-stdout-reader".into())
                        .spawn(move || {
                            let reader = BufReader::new(stdout);
                            for line_res in reader.lines() {
                                if let Ok(line) = line_res {
                                    let app_fallback = SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone());
                                    handle_slint_incoming_action(&line, app_fallback.as_ref());
                                }
                            }
                            crate::log_info!("SLINT_MAIN", "Slint companion process terminated, clearing STDIN handle.");
                            if let Ok(mut g) = SLINT_STDIN.lock() {
                                *g = None;
                            }
                            crate::system::process::trim_process_working_set();
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

pub(crate) fn mutate_active_preset_profile<F>(f: F)
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

pub fn send_slint_ipc_message(payload: serde_json::Value) {
    let mut lock = match SLINT_STDIN.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };

    if let Some(ref mut stdin) = *lock {
        let line = format!("{}\n", payload);
        if let Err(e) = stdin.write_all(line.as_bytes()) {
            crate::log_warn!("SLINT_MAIN", "Failed writing to companion stdin: {}, resetting", e);
            *lock = None;
        } else if let Err(e) = stdin.flush() {
            crate::log_warn!("SLINT_MAIN", "Failed flushing companion stdin: {}, resetting", e);
            *lock = None;
        }
    }
}

pub fn show_slint_window() {
    let app_opt = SLINT_APP_HANDLE.lock().ok().and_then(|g| g.clone());
    ensure_slint_main_process(app_opt.as_ref());
    let cfg = crate::system::config::load_config();
    let should_expand = match cfg.wake_window_mode.as_str() {
        "full" => true,
        "mini" => false,
        _ => cfg.last_window_expanded,
    };
    send_slint_ipc_message(serde_json::json!({
        "action": "show",
        "is_expanded": should_expand,
    }));
}

pub fn hide_slint_window() {
    send_slint_ipc_message(serde_json::json!({ "action": "hide" }));
}

/// Explicitly shut down and terminate the Slint main companion process
pub fn shutdown_slint_process() {
    let mut lock = match SLINT_STDIN.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(mut stdin) = lock.take() {
        let _ = stdin.write_all(b"{\"action\":\"quit\"}\n");
        let _ = stdin.flush();
        drop(stdin);
        crate::log_info!("SLINT_MAIN", "Slint companion process cleanly shut down.");
    }
}

pub fn toggle_slint_window() {
    let mut last = match LAST_SLINT_TOGGLE.lock() {
        Ok(l) => l,
        Err(p) => p.into_inner(),
    };
    if let Some(t) = *last {
        if t.elapsed() < std::time::Duration::from_millis(300) {
            return;
        }
    }
    *last = Some(std::time::Instant::now());

    let is_running = {
        let lock = match SLINT_STDIN.lock() {
            Ok(l) => l,
            Err(p) => p.into_inner(),
        };
        lock.is_some()
    };

    if !is_running {
        show_slint_window();
    } else {
        send_slint_ipc_message(serde_json::json!({ "action": "toggle" }));
    }
}

