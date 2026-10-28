use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use windows::core::{w, BSTR, VARIANT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Wmi::{
    IEnumWbemClassObject, IWbemClassObject, IWbemLocator, WbemLocator, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE,
};

use crate::driver::ec_bridge::SharedEcBridge;

static IS_RUNNING: AtomicBool = AtomicBool::new(false);

const RPC_C_AUTHN_WINNT: u32 = 10;
const RPC_C_AUTHZ_NONE: u32 = 0;
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Safely kill any stale legacy companion processes left by previous versions
fn cleanup_legacy_listener() {
    #[cfg(windows)]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/F", "/IM", "openrevo_acpi_listener.exe"])
            .output();

        // Also clean up legacy dropped exe file if present in %APPDATA%/OpenRevo
        if let Ok(appdata) = std::env::var("APPDATA") {
            let legacy_exe = std::path::PathBuf::from(appdata)
                .join("OpenRevo")
                .join("openrevo_acpi_listener.exe");
            if legacy_exe.exists() {
                let _ = std::fs::remove_file(legacy_exe);
            }
        }
    }
}

/// Helper function to safely extract u32 from VARIANT (supports integers and string representation)
fn extract_variant_u32(val: &VARIANT) -> Option<u32> {
    let s = val.to_string();
    if let Ok(num) = s.trim().parse::<u32>() {
        return Some(num);
    }
    // COM VARIANT memory inspection fallback
    unsafe {
        let raw_ptr = val as *const VARIANT as *const u16;
        let vt = *raw_ptr;
        let data_ptr = (val as *const VARIANT as *const u8).add(8);
        match vt {
            16 => Some(*data_ptr as u32),                        // VT_I1
            17 => Some(*data_ptr as u32),                        // VT_UI1
            2 => Some(*(data_ptr as *const i16) as u32),         // VT_I2
            18 => Some(*(data_ptr as *const u16) as u32),        // VT_UI2
            3 => Some(*(data_ptr as *const i32) as u32),         // VT_I4
            19 => Some(*(data_ptr as *const u32)),               // VT_UI4
            _ => None,
        }
    }
}

/// Dispatch received ACPI Event code to OpenRevo hardware controllers
fn dispatch_acpi_event(
    code: u32,
    app_handle: &AppHandle,
    ec_bridge: &SharedEcBridge,
    last_switch_time: &mut Instant,
    last_brightness_time: &mut Instant,
    last_fan_boost_time: &mut Instant,
) {
    match code {
        176 | 192 | 201 => {
            // QKEY_MODESWITCH / OSD_FanModeSwitch (176) / SC_SwitchOC (192) / SC_CustomMode (201)
            if last_switch_time.elapsed() >= Duration::from_millis(500) {
                *last_switch_time = Instant::now();
                let current_mode = ec_bridge.lock().get_power_mode();
                let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
                let mut cfg = crate::system::config::load_config();
                let next_mode = if is_dc || cfg.unlock_battery_power_limit {
                    // Explicit 4-stage mode cycle: 1 (Office) -> 2 (Balance) -> 3 (Beast) -> 4 (Custom) -> 1 (Office)
                    match current_mode {
                        1 => 2,
                        2 => 3,
                        3 => 4,
                        4 => 1,
                        _ => 2,
                    }
                } else {
                    // Strictly clamp to 1 (Office) <-> 2 (Balance) when running on battery
                    match current_mode {
                        1 => 2,
                        _ => 1,
                    }
                };

                crate::log_info!(
                    "WMI_LISTENER",
                    "Q-Key pressed (scancode: {})! Cycling mode: {} -> {} (is_dc: {}, unlock_battery: {})",
                    code,
                    current_mode,
                    next_mode,
                    is_dc,
                    cfg.unlock_battery_power_limit
                );
                ec_bridge.lock().set_power_mode(next_mode);

                // Update persistent config
                cfg.power_mode = next_mode;
                if is_dc {
                    cfg.power_mode_ac = next_mode;
                } else {
                    cfg.power_mode_battery = next_mode;
                }
                crate::system::config::save_config(&cfg);

                crate::update_tray_power_mode(next_mode);
                crate::system::osd_window::show_power_mode(next_mode);

                // Broadcast to UI so frontend badges, charts, and colors update immediately
                crate::system::osd_window::wake_osd_if_enabled(app_handle);
                let _ = app_handle.emit("physical-mode-switched", next_mode);
                let _ = app_handle.emit_to("main", "physical-mode-switched", next_mode);
                let _ = app_handle.emit_to("osd", "physical-mode-switched", next_mode);
            } else {
                crate::log_debug!("WMI_LISTENER", "Q-Key debounced (elapsed < 500ms)");
            }
        }
        177 => {
            // Fn+F6: EC_RGBKBBKL_LEVEL_DOWN (Keyboard brightness down)
            if last_brightness_time.elapsed() >= Duration::from_millis(150) {
                *last_brightness_time = Instant::now();
                let new_lvl =
                    crate::driver::ite_lighting::adjust_keyboard_brightness_step(false, Some(app_handle));
                crate::log_info!(
                    "WMI_LISTENER",
                    "Fn+F6 hotkey received! Decreased keyboard brightness to level {}",
                    new_lvl
                );
            }
        }
        178 => {
            // Fn+F7: EC_RGBKBBKL_LEVEL_UP (Keyboard brightness up)
            if last_brightness_time.elapsed() >= Duration::from_millis(150) {
                *last_brightness_time = Instant::now();
                let new_lvl =
                    crate::driver::ite_lighting::adjust_keyboard_brightness_step(true, Some(app_handle));
                crate::log_info!(
                    "WMI_LISTENER",
                    "Fn+F7 hotkey received! Increased keyboard brightness to level {}",
                    new_lvl
                );
            }
        }
        185 => {
            // EC_RGBKBBKL_LEVEL_CYCLE (Keyboard brightness cycle)
            if last_brightness_time.elapsed() >= Duration::from_millis(150) {
                *last_brightness_time = Instant::now();
                let new_lvl =
                    crate::driver::ite_lighting::cycle_keyboard_brightness(Some(app_handle));
                crate::log_info!(
                    "WMI_LISTENER",
                    "Keyboard brightness cycle hotkey received! New level {}",
                    new_lvl
                );
            }
        }
        59..=63 => {
            // OSD_KB_LED_LEVEL0..4 (59=0%, 60=25%, 61=50%, 62=75%, 63=100%)
            if last_brightness_time.elapsed() >= Duration::from_millis(150) {
                *last_brightness_time = Instant::now();
                let lvl = (code - 59) as u8;
                crate::driver::ite_lighting::set_keyboard_brightness_level(lvl, Some(app_handle));
                crate::log_info!(
                    "WMI_LISTENER",
                    "Direct ACPI keyboard brightness event received: level {}",
                    lvl
                );
            }
        }
        168 => {
            crate::log_info!("WMI_LISTENER", "ACPI Mode State Echo received: 168 (0xA8)");
        }
        167 | 67 => {
            // Fn+1 Hardware Hotkey / Fan Boost ACPI Notification (Events 167 [0xA7] and 67 [0x43])
            if last_fan_boost_time.elapsed() >= Duration::from_millis(500) {
                *last_fan_boost_time = Instant::now();
                let is_in_cooldown = ec_bridge.lock().is_in_fan_boost_cooldown();
                if is_in_cooldown {
                    crate::log_info!(
                        "WMI_LISTENER",
                        "Fn+1 / Fan Boost event {} received during cooldown - ignoring echo",
                        code
                    );
                } else {
                    let cur_active = ec_bridge.lock().is_fan_boost();
                    let next_state = !cur_active;
                    crate::log_info!(
                        "WMI_LISTENER",
                        "Fn+1 hotkey pressed! Toggling Fan Boost: {} -> {}",
                        cur_active,
                        next_state
                    );
                    ec_bridge.lock().set_fan_boost(next_state);
                    crate::system::osd_window::show_fan_boost(next_state);

                    crate::system::osd_window::wake_osd_if_enabled(app_handle);
                    let _ = app_handle.emit("fan-boost-changed", next_state);
                    let _ = app_handle.emit_to("main", "fan-boost-changed", next_state);
                    let _ = app_handle.emit_to("osd", "fan-boost-changed", next_state);
                    let _ = app_handle.emit("physical-fan-boost-switched", next_state);
                    let _ = app_handle.emit_to("main", "physical-fan-boost-switched", next_state);
                    let _ = app_handle.emit_to("osd", "physical-fan-boost-switched", next_state);
                }
            } else {
                crate::log_debug!("WMI_LISTENER", "Fn+1 hotkey event {} debounced (elapsed < 500ms)", code);
            }
        }
        240 => {
            // EC_RGBKBBKL_LEVEL_UPDATE echo notification
            crate::log_info!("WMI_LISTENER", "ACPI Keyboard brightness status update echo: 240");
        }
        _ => {
            crate::log_debug!("WMI_LISTENER", "Received other ACPI Event code: {} (0x{:X})", code, code);
        }
    }
}

/// Start the background WMI ACPI Event Listener for physical chassis keys (Q-Key mode switch & Fan Boost)
/// Pure Rust in-process implementation via Windows COM (IWbemServices::ExecNotificationQuery)
pub fn start_wmi_event_listener(app_handle: AppHandle, ec_bridge: SharedEcBridge) {
    if crate::hal::platform_probe::is_ultrabook() || !crate::hal::platform_probe::has_acpi_physical_key_events() {
        crate::log_info!(
            "WMI_LISTENER",
            "Ultrabook platform: AcpiTest_EventULong listener bypassed gracefully."
        );
        return;
    }

    if IS_RUNNING.swap(true, Ordering::SeqCst) {
        crate::log_info!("WMI_LISTENER", "Event listener is already running.");
        return;
    }

    std::thread::Builder::new()
        .name("wmi-acpi-event-watcher".to_string())
        .spawn(move || {
            crate::log_info!("WMI_LISTENER", "Initializing native in-process WMI ACPI event bridge...");
            cleanup_legacy_listener();

            let mut last_switch_time = Instant::now() - Duration::from_secs(10);
            let mut last_brightness_time = Instant::now() - Duration::from_secs(10);
            let mut last_fan_boost_time = Instant::now() - Duration::from_secs(10);
            let mut consecutive_failures = 0u32;

            loop {
                if consecutive_failures >= 3 {
                    crate::log_warn!(
                        "WMI_LISTENER",
                        "AcpiTest_EventULong native listener failed 3 times consecutively. Disabling listener permanently."
                    );
                    break;
                }

                unsafe {
                    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

                    let res = (|| -> windows::core::Result<()> {
                        let locator: IWbemLocator =
                            CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;
                        let namespace = BSTR::from(r"root\WMI");
                        let services = locator.ConnectServer(
                            &namespace,
                            &BSTR::new(),
                            &BSTR::new(),
                            &BSTR::new(),
                            0,
                            &BSTR::new(),
                            None,
                        )?;

                        let _ = CoSetProxyBlanket(
                            &services,
                            RPC_C_AUTHN_WINNT,
                            RPC_C_AUTHZ_NONE,
                            None,
                            RPC_C_AUTHN_LEVEL_CALL,
                            RPC_C_IMP_LEVEL_IMPERSONATE,
                            None,
                            EOAC_NONE,
                        );

                        let query_lang = BSTR::from("WQL");
                        let query = BSTR::from("SELECT * FROM AcpiTest_EventULong");
                        let enumerator: IEnumWbemClassObject = services.ExecNotificationQuery(
                            &query_lang,
                            &query,
                            WBEM_GENERIC_FLAG_TYPE(
                                (WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0) as i32,
                            ),
                            None,
                        )?;

                        consecutive_failures = 0;
                        crate::log_info!(
                            "WMI_LISTENER",
                            "Native AcpiTest_EventULong subscription active and listening (in-process)."
                        );

                        let mut objects: [Option<IWbemClassObject>; 1] = [None];
                        let mut returned = 0u32;

                        loop {
                            // Semi-synchronous block with 1000ms timeout for clean cancellation & liveness
                            let hr = enumerator.Next(
                                1000,
                                &mut objects,
                                &mut returned,
                            );

                            if hr.is_err() {
                                crate::log_warn!("WMI_LISTENER", "WMI event enumerator Next failed: {:?}", hr);
                                return Err(hr.into());
                            }

                            if returned == 0 {
                                continue;
                            }

                            if let Some(event_obj) = objects[0].take() {
                                let mut val = VARIANT::default();
                                if event_obj.Get(w!("ULong"), 0, &mut val, None, None).is_ok() {
                                    if let Some(code) = extract_variant_u32(&val) {
                                        crate::log_debug!("WMI_LISTENER", "Raw ACPI Event received: {}", code);
                                        dispatch_acpi_event(
                                            code,
                                            &app_handle,
                                            &ec_bridge,
                                            &mut last_switch_time,
                                            &mut last_brightness_time,
                                            &mut last_fan_boost_time,
                                        );
                                    }
                                }
                            }
                        }
                    })();

                    if let Err(e) = res {
                        crate::log_warn!("WMI_LISTENER", "Native WMI ACPI event stream error: {:?}", e);
                        consecutive_failures += 1;
                        let err_str = format!("{:?}", e);
                        if err_str.contains("0x80041010") || err_str.to_lowercase().contains("invalid class") {
                            crate::log_warn!(
                                "WMI_LISTENER",
                                "AcpiTest_EventULong class not supported on this platform. Terminating listener gracefully."
                            );
                            CoUninitialize();
                            return;
                        }
                    }

                    CoUninitialize();
                }

                // Sleep briefly before self-healing / restarting subscription
                std::thread::sleep(Duration::from_secs(2));
                crate::log_warn!("WMI_LISTENER", "Re-arming native WMI event subscription pipeline...");
            }
        })
        .expect("Failed to spawn WMI event watcher thread");
}

