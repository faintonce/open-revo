use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_AWAYMODE_REQUIRED, ES_CONTINUOUS, ES_DISPLAY_REQUIRED,
    ES_SYSTEM_REQUIRED,
};

/// Start dedicated background watcher for Bluetooth, Wi-Fi, Camera, WinLock, and FnLock states
/// Polls every 600ms (direct EC & WinRT in-process COM call, ~0.5ms execution, 0.0% CPU)
/// and emits events to Tauri whenever external keyboard hotkeys or control center changes occur.
pub fn start_hardware_state_watcher(
    app: tauri::AppHandle,
    ec_bridge: std::sync::Arc<parking_lot::Mutex<crate::driver::ec_bridge::EcBridge>>,
) {
    std::thread::spawn(move || {
        crate::log_info!("DEVICE_SWITCH", "Starting hardware state watcher (Bluetooth, Wi-Fi, Camera, WinLock, FnLock)...");
        let is_ultrabook = crate::hal::platform_probe::is_ultrabook();
        let supports_legacy_ec_switches = crate::hal::platform_probe::is_tongfang_family() && !is_ultrabook;
        let cfg = crate::system::config::load_config();
        if cfg.keep_awake_enabled {
            super::set_keep_awake(true);
        }
        if !cfg.wifi_enabled {
            let _ = super::radio::set_wifi_radio_state_native(false);
        }
        if !cfg.bluetooth_enabled {
            let _ = super::radio::set_native_bluetooth_radio_state(false);
        }
        if supports_legacy_ec_switches {
            crate::driver::wmi_ec::wmi_modify_ec_mask(
                crate::driver::wmi_ec::ADDR_FN_LOCK,
                0x10,
                cfg.fn_lock_enabled,
            );
            crate::driver::wmi_ec::wmi_modify_ec_mask(
                crate::driver::wmi_ec::ADDR_STATUS_BYTE,
                crate::driver::wmi_ec::STATUS_WINLOCK,
                cfg.win_key_locked,
            );
        }
        crate::system::hooks::set_win_key_locked(cfg.win_key_locked);

        if supports_legacy_ec_switches && !cfg.touchpad_enabled {
            crate::driver::wmi_ec::wmi_modify_ec_mask(
                crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE,
                crate::driver::wmi_ec::TOUCHPAD_BITMASK,
                false,
            );
            super::touchpad::write_ptp_registry_enabled(false);
        }

        if !cfg.mic_enabled {
            let _ = super::audio::set_microphone_state(false);
        }

        let mut last_bt = Some(cfg.bluetooth_enabled);
        let mut last_wifi = Some(cfg.wifi_enabled);
        let mut last_fn_lock = Some(cfg.fn_lock_enabled);
        let has_ec_switches = super::has_ec_device_switches();
        let mut last_cam = Some(super::camera::get_realtime_camera_state(has_ec_switches, cfg.camera_enabled));
        let mut last_mic = super::audio::get_microphone_state();
        let mut last_ac_line: Option<bool> = None;
        let mut last_dc_line: Option<bool> = None;
        let mut last_win_lock = Some(cfg.win_key_locked);
        let mut last_win_toggle_time = std::time::Instant::now() - std::time::Duration::from_secs(10);
        let mut last_fn_toggle_time = std::time::Instant::now() - std::time::Duration::from_secs(10);
        let mut custom_trace_ticks: u32 = 0;
        let mut last_was_throttled = false;
        let mut tick_count: u32 = 0;
        let mut last_color_profiles: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();

        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        enum ActivePowerLock {
            Normal,
            KeepAwake,
            ScreenOffAway,
        }
        let mut current_power_lock = ActivePowerLock::Normal;

        loop {
            // Guard: completely freeze watcher if system is rebooting
            if crate::driver::uefi_nvram::is_reboot_in_progress() {
                std::thread::sleep(std::time::Duration::from_millis(2000));
                continue;
            }

            let loop_delay = if is_ultrabook {
                std::time::Duration::from_millis(500)
            } else {
                std::time::Duration::from_millis(50)
            };
            std::thread::sleep(loop_delay);

            // Fast Track: Watch Win Key Lock & Fn Lock for instantaneous, responsive keyboard hotkey reaction
            // 1. Win Key Lock (Instant OSD feedback & tracking with 180ms debounce to filter contact chatter)
            let current_win = if supports_legacy_ec_switches {
                crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_STATUS_BYTE)
                    .map(|val| (val & crate::driver::wmi_ec::STATUS_WINLOCK) != 0)
                    .unwrap_or_else(crate::system::hooks::is_win_key_locked)
            } else {
                crate::system::hooks::is_win_key_locked()
            };
            if last_win_lock != Some(current_win) {
                if last_win_toggle_time.elapsed() >= std::time::Duration::from_millis(180) {
                    crate::log_info!("DEVICE_SWITCH", "External Win Key lock state change: {:?} -> {}", last_win_lock, current_win);
                    last_win_lock = Some(current_win);
                    last_win_toggle_time = std::time::Instant::now();
                    crate::system::hooks::set_win_key_locked(current_win);
                    let mut cfg = crate::system::config::load_config();
                    cfg.win_key_locked = current_win;
                    crate::system::config::save_config(&cfg);
                    use tauri::Emitter;
                    crate::system::osd_window::show_key_lock_toast("win", current_win);
                    crate::system::osd_window::wake_osd_if_enabled(&app);
                    let _ = app.emit("win-lock-state-changed", current_win);
                    let _ = app.emit_to("main", "win-lock-state-changed", current_win);
                    let _ = app.emit_to("osd", "win-lock-state-changed", current_win);
                } else {
                    crate::log_debug!("DEVICE_SWITCH", "Win Key lock rapid transition suppressed by debounce (<180ms)");
                }
            }

            // 2. Fn Lock (Bit 4: 0x10 with 180ms debounce)
            if supports_legacy_ec_switches {
                if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FN_LOCK) {
                    let current_fn = (val & 0x10) != 0;
                    if last_fn_lock != Some(current_fn) {
                        if last_fn_toggle_time.elapsed() >= std::time::Duration::from_millis(180) {
                            crate::log_info!("DEVICE_SWITCH", "External Fn Lock state change: {:?} -> {}", last_fn_lock, current_fn);
                            last_fn_lock = Some(current_fn);
                            last_fn_toggle_time = std::time::Instant::now();
                            let mut cfg = crate::system::config::load_config();
                            cfg.fn_lock_enabled = current_fn;
                            crate::system::config::save_config(&cfg);
                            use tauri::Emitter;
                            crate::system::osd_window::show_key_lock_toast("fn", current_fn);
                            crate::system::osd_window::wake_osd_if_enabled(&app);
                            let _ = app.emit("fn-lock-state-changed", current_fn);
                            let _ = app.emit_to("main", "fn-lock-state-changed", current_fn);
                            let _ = app.emit_to("osd", "fn-lock-state-changed", current_fn);
                        } else {
                            crate::log_debug!("DEVICE_SWITCH", "Fn Lock rapid transition suppressed by debounce (<180ms)");
                        }
                    }
                }
            }

            // 3. Fast Track: Screen-off peripheral interaction wakeup
            if super::is_screen_off() {
                let unblock_tick = super::get_screen_off_unblock_tick();
                if unblock_tick > 0 {
                    let mut lii = super::LASTINPUTINFO {
                        cb_size: std::mem::size_of::<super::LASTINPUTINFO>() as u32,
                        dw_time: 0,
                    };
                    if unsafe { super::GetLastInputInfo(&mut lii) } != 0 {
                        if lii.dw_time > unblock_tick.saturating_add(50) {
                            super::set_screen_off(false);
                            crate::log_info!("SCREEN", "User peripheral interaction detected after screen off; display active.");
                        }
                    }
                }
            }

            // 4. Fast Track: Centralized Power Execution State Machine
            // All SetThreadExecutionState calls are strictly bound to this dedicated watcher thread
            // to eliminate cross-thread lock leaks and ensure complete two-way state release.
            let keep_awake = super::is_keep_awake();
            let screen_off = super::is_screen_off();
            let desired_lock = if screen_off {
                ActivePowerLock::ScreenOffAway
            } else if keep_awake {
                ActivePowerLock::KeepAwake
            } else {
                ActivePowerLock::Normal
            };

            if desired_lock != current_power_lock {
                unsafe {
                    match desired_lock {
                        ActivePowerLock::Normal => {
                            SetThreadExecutionState(ES_CONTINUOUS);
                            crate::log_info!("POWER_STATE", "Power lock released: Normal system sleep and display idle timers restored.");
                        }
                        ActivePowerLock::KeepAwake => {
                            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
                            crate::log_info!("POWER_STATE", "Power lock acquired: Keep-awake active (display and system kept awake).");
                        }
                        ActivePowerLock::ScreenOffAway => {
                            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_AWAYMODE_REQUIRED);
                            crate::log_info!("POWER_STATE", "Power lock acquired: Screen-off active (system awake in away-mode).");
                        }
                    }
                }
                current_power_lock = desired_lock;
            }

            // Slow Track: Heavy peripherals (Bluetooth, Wi-Fi, Camera, Mic, AC/DC)
            tick_count = tick_count.wrapping_add(1);
            let slow_mod = if is_ultrabook { 2 } else { 12 };
            if tick_count % slow_mod != 0 {
                continue;
            }

            // 1. Watch Bluetooth
            if let Some(current_bt) = super::radio::get_native_bluetooth_radio_state() {
                if last_bt != Some(current_bt) {
                    crate::log_info!("DEVICE_SWITCH", "External Bluetooth state change: {:?} -> {}", last_bt, current_bt);
                    last_bt = Some(current_bt);
                    let mut cfg = crate::system::config::load_config();
                    cfg.bluetooth_enabled = current_bt;
                    crate::system::config::save_config(&cfg);
                    use tauri::Emitter;
                    crate::system::osd_window::show_device_switch("bluetooth", current_bt);
                    crate::system::osd_window::wake_osd_if_enabled(&app);
                    let _ = app.emit("bluetooth-state-changed", current_bt);
                    let _ = app.emit_to("main", "bluetooth-state-changed", current_bt);
                    let _ = app.emit_to("osd", "bluetooth-state-changed", current_bt);
                }
            }

            // 2. Watch Wi-Fi
            if let Some(current_wifi) = super::radio::get_realtime_wifi_state() {
                if last_wifi != Some(current_wifi) {
                    crate::log_info!("DEVICE_SWITCH", "External Wi-Fi state change: {:?} -> {}", last_wifi, current_wifi);
                    last_wifi = Some(current_wifi);
                    let mut cfg = crate::system::config::load_config();
                    cfg.wifi_enabled = current_wifi;
                    crate::system::config::save_config(&cfg);
                    use tauri::Emitter;
                    crate::system::osd_window::show_device_switch("wifi", current_wifi);
                    crate::system::osd_window::wake_osd_if_enabled(&app);
                    let _ = app.emit("wifi-state-changed", current_wifi);
                    let _ = app.emit_to("main", "wifi-state-changed", current_wifi);
                    let _ = app.emit_to("osd", "wifi-state-changed", current_wifi);
                }
            }

            // 3. Watch Camera
            let current_cam = super::camera::get_realtime_camera_state(has_ec_switches, true);
            if last_cam != Some(current_cam) {
                crate::log_info!("DEVICE_SWITCH", "External Camera state change: {:?} -> {}", last_cam, current_cam);
                last_cam = Some(current_cam);
                let mut cfg = crate::system::config::load_config();
                cfg.camera_enabled = current_cam;
                crate::system::config::save_config(&cfg);
                use tauri::Emitter;
                crate::system::osd_window::show_device_switch("camera", current_cam);
                crate::system::osd_window::wake_osd_if_enabled(&app);
                let _ = app.emit("camera-state-changed", current_cam);
                let _ = app.emit_to("main", "camera-state-changed", current_cam);
                let _ = app.emit_to("osd", "camera-state-changed", current_cam);
            }

            // 4. Watch Microphone Mute State
            if let Some(current_mic) = super::audio::get_microphone_state() {
                if last_mic != Some(current_mic) {
                    crate::log_info!("DEVICE_SWITCH", "External Microphone state change: {:?} -> {}", last_mic, current_mic);
                    last_mic = Some(current_mic);
                    let mut cfg = crate::system::config::load_config();
                    cfg.mic_enabled = current_mic;
                    crate::system::config::save_config(&cfg);
                    use tauri::Emitter;
                    crate::system::osd_window::show_device_switch("mic", current_mic);
                    crate::system::osd_window::wake_osd_if_enabled(&app);
                    let _ = app.emit("mic-state-changed", current_mic);
                    let _ = app.emit_to("main", "mic-state-changed", current_mic);
                    let _ = app.emit_to("osd", "mic-state-changed", current_mic);
                }
            }

            // 7. Watch AC Power Transition for Auto Min Refresh Rate on Battery
            let (_, is_ac, _) = crate::commands::hardware_cmd::get_live_power_status();
            let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();

            let is_thin_light = crate::commands::power_cmd::is_thin_and_light_or_pure_igpu();

            if is_thin_light {
                // Pure iGPU / Thin & Light Laptops:
                // Completely respect user's manual power mode choice!
                // Never force switch to Beast on AC plug, and never force Balance on battery.
                if let Some(last_dc) = last_dc_line {
                    if last_dc != is_dc {
                        use tauri::Emitter;
                        let _ = app.emit("dc-status-changed", is_dc);
                    }
                }
            } else {
                // Dual-parameter AC/Battery safety & persistence guard for Gaming Laptops:
                // Gaming laptops must not run in Beast (3) or Custom (4) on battery/Type-C to prevent battery overcurrent.
                // Thin & light laptops rely on hardware EC power envelopes and must NOT be demoted or forced into balance.
                let is_thin_light = crate::commands::power_cmd::is_thin_and_light_or_pure_igpu();
                if let Some(last_dc) = last_dc_line {
                    if last_dc && !is_dc {
                        // DC round adapter unplugged (Transition: DC Online -> On Battery/Type-C)
                        let mut updated_cfg = crate::system::config::load_config();
                        let cur_mode = updated_cfg.power_mode;
                        if cur_mode >= 1 && cur_mode <= 4 {
                            updated_cfg.power_mode_ac = cur_mode;
                        }
                        if is_thin_light {
                            crate::log_info!("POWER_WATCHER", "Power line unplugged on thin-and-light laptop: preserving user power mode {}", cur_mode);
                            crate::system::config::save_config(&updated_cfg);
                        } else if !updated_cfg.unlock_battery_power_limit {
                            let target_mode = if updated_cfg.power_mode_battery == 1 { 1 } else { 2 };
                            crate::log_info!("POWER_WATCHER", "DC round adapter unplugged: safely switching power mode {} -> {} (Battery mode) on Battery/Type-C", cur_mode, target_mode);
                            updated_cfg.power_mode = target_mode;
                            crate::system::config::save_config(&updated_cfg);
                            ec_bridge.lock().set_power_mode(target_mode);
                            crate::update_tray_power_mode(target_mode);
                            use tauri::Emitter;
                            crate::system::osd_window::wake_osd_if_enabled(&app);
                            let _ = app.emit("physical-mode-switched", target_mode);
                            let _ = app.emit_to("main", "physical-mode-switched", target_mode);
                            let _ = app.emit_to("osd", "physical-mode-switched", target_mode);
                        } else {
                            crate::log_info!("POWER_WATCHER", "DC round adapter unplugged: unlock_battery_power_limit enabled, keeping current power mode {}", cur_mode);
                            crate::system::config::save_config(&updated_cfg);
                        }
                        use tauri::Emitter;
                        let _ = app.emit("dc-status-changed", false);
                        crate::system::watcher_hub::notify_power_event("ac_disconnected");
                    } else if !last_dc && is_dc {
                        // DC round adapter plugged back in (Transition: On Battery/Type-C -> DC Online)
                        let mut cfg = crate::system::config::load_config();
                        let target_mode = match cfg.power_mode_ac {
                            1..=4 => cfg.power_mode_ac,
                            _ => 3, // Default Beast on AC
                        };
                        crate::log_info!(
                            "POWER_WATCHER",
                            "DC round adapter plugged in: restoring AC power mode -> {} ({})",
                            target_mode,
                            match target_mode {
                                1 => "Office",
                                2 => "Balance",
                                3 => "Beast",
                                4 => "Custom",
                                5 => "Unknown",
                                6 => "Unknown",
                                _ => "Unknown",
                            }
                        );
                        cfg.power_mode = target_mode;
                        crate::system::config::save_config(&cfg);
                        ec_bridge.lock().set_power_mode(target_mode);
                        crate::update_tray_power_mode(target_mode);
                        use tauri::Emitter;
                        crate::system::osd_window::wake_osd_if_enabled(&app);
                        let _ = app.emit("physical-mode-switched", target_mode);
                        let _ = app.emit_to("main", "physical-mode-switched", target_mode);
                        let _ = app.emit_to("osd", "physical-mode-switched", target_mode);
                        let _ = app.emit("dc-status-changed", true);
                        crate::system::watcher_hub::notify_power_event("ac_connected");
                    }
                } else if is_dc {
                    // Initial first loop (last_dc_line is None): DC is online at boot
                    let mut cfg = crate::system::config::load_config();
                    let target_mode = match cfg.power_mode_ac {
                        1..=4 => cfg.power_mode_ac,
                        _ => 3,
                    };
                    if cfg.power_mode != target_mode {
                        crate::log_info!("POWER_WATCHER", "Initial loop: DC round adapter online, aligning power mode {} -> {}", cfg.power_mode, target_mode);
                        cfg.power_mode = target_mode;
                        crate::system::config::save_config(&cfg);
                        ec_bridge.lock().set_power_mode(target_mode);
                        crate::update_tray_power_mode(target_mode);
                        use tauri::Emitter;
                        let _ = app.emit("physical-mode-switched", target_mode);
                        let _ = app.emit_to("main", "physical-mode-switched", target_mode);
                        let _ = app.emit_to("osd", "physical-mode-switched", target_mode);
                    }
                } else if !is_dc && !is_thin_light {
                    // Initial first loop (last_dc_line is None): Laptop running on Battery/Type-C (gaming laptops only)
                    let mut cfg = crate::system::config::load_config();
                    if !cfg.unlock_battery_power_limit && (cfg.power_mode == 3 || cfg.power_mode == 4) {
                        let target_mode = if cfg.power_mode_battery == 1 { 1 } else { 2 };
                        crate::log_info!("POWER_WATCHER", "Initial loop: on Battery/Type-C, safely adjusting mode {} -> {}", cfg.power_mode, target_mode);
                        cfg.power_mode = target_mode;
                        crate::system::config::save_config(&cfg);
                        ec_bridge.lock().set_power_mode(target_mode);
                        crate::update_tray_power_mode(target_mode);
                        use tauri::Emitter;
                        let _ = app.emit("physical-mode-switched", target_mode);
                        let _ = app.emit_to("main", "physical-mode-switched", target_mode);
                        let _ = app.emit_to("osd", "physical-mode-switched", target_mode);
                    }
                }
            }
            last_dc_line = Some(is_dc);


            if let Some(last_ac) = last_ac_line {
                if last_ac && !is_ac {
                    // Transition: Plugged -> Unplugged
                    let cfg = crate::system::config::load_config();
                    if cfg.auto_min_refresh_on_battery {
                        let monitors = crate::system::display::get_all_monitors();
                        if let Some(target_mon) = monitors.iter().find(|m| m.is_internal).or_else(|| monitors.first()) {
                            if let Some(&min_rate) = target_mon.supported_rates.first() {
                                if target_mon.current_hz != min_rate {
                                    crate::log_info!("POWER_WATCHER", "AC unplugged: auto-switching internal screen ({}) refresh rate {}Hz -> {}Hz", target_mon.friendly_name, target_mon.current_hz, min_rate);
                                    let _ = crate::system::display::set_monitor_refresh_rate(&target_mon.device_name, min_rate);
                                    let mut updated_cfg = crate::system::config::load_config();
                                    updated_cfg.refresh_rate = min_rate;
                                    crate::system::config::save_config(&updated_cfg);
                                    use tauri::Emitter;
                                    let _ = app.emit("refresh-rate-changed", min_rate);
                                }
                            }
                        }
                    }

                    // Auto-fallback keyboard engine to factory hardware mode on battery (Gaming laptops only)
                    if !is_ultrabook && cfg.auto_fallback_kb_on_battery {
                        let lighting = crate::driver::ite_lighting::get_global_lighting();
                        if lighting.has_keyboard_device() {
                            let current_engine = {
                                let st = lighting.state.lock();
                                st.kb_engine.clone()
                            };
                            if current_engine == "streamer" {
                                crate::log_info!("POWER_WATCHER", "AC unplugged: auto-fallback keyboard lighting to factory hardware mode (0% CPU)");
                                let streamer = crate::driver::ite_streamer::get_global_streamer();
                                streamer.stop();
                                let st_snapshot = {
                                    let mut st = lighting.state.lock();
                                    st.kb_engine = "hardware".to_string();
                                    st.clone()
                                };
                                crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
                                lighting.apply_keyboard(
                                    st_snapshot.kb_enabled,
                                    st_snapshot.kb_effect,
                                    st_snapshot.kb_brightness,
                                    st_snapshot.kb_speed,
                                    st_snapshot.kb_direction,
                                    &st_snapshot.kb_color,
                                );
                                use tauri::Emitter;
                                let _ = app.emit("kb-engine-changed", "hardware");
                            }
                        }
                    }
                } else if !last_ac && is_ac {
                    // Transition: Unplugged -> Plugged back in
                    let cfg = crate::system::config::load_config();
                    if cfg.auto_min_refresh_on_battery {
                        let monitors = crate::system::display::get_all_monitors();
                        if let Some(target_mon) = monitors.iter().find(|m| m.is_internal).or_else(|| monitors.first()) {
                            if let Some(&max_rate) = target_mon.supported_rates.last() {
                                if target_mon.current_hz != max_rate {
                                    crate::log_info!("POWER_WATCHER", "AC plugged in: restoring high refresh rate ({}) {}Hz -> {}Hz", target_mon.friendly_name, target_mon.current_hz, max_rate);
                                    let _ = crate::system::display::set_monitor_refresh_rate(&target_mon.device_name, max_rate);
                                    let mut updated_cfg = crate::system::config::load_config();
                                    updated_cfg.refresh_rate = max_rate;
                                    crate::system::config::save_config(&updated_cfg);
                                    use tauri::Emitter;
                                    let _ = app.emit("refresh-rate-changed", max_rate);
                                }
                            }
                        }
                    }

                    if !is_ultrabook {
                        // Auto-restore BetterRGB streamer on AC plug-in
                        let lighting = crate::driver::ite_lighting::get_global_lighting();
                        if cfg.auto_fallback_kb_on_battery && lighting.has_keyboard_device() {
                            let st_snapshot = {
                                let mut st = lighting.state.lock();
                                st.kb_engine = "streamer".to_string();
                                st.clone()
                            };
                            crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
                            crate::log_info!("POWER_WATCHER", "AC plugged in: auto-restoring BetterRGB 60FPS streamer engine");
                            let streamer = crate::driver::ite_streamer::get_global_streamer();
                            let brightness_pct = match st_snapshot.kb_brightness {
                                0 => 0, 1 => 20, 2 => 45, 3 => 75, 4 => 100, _ => 75,
                            };
                            let sp = crate::driver::ite_streamer::StreamerParams {
                                enabled: st_snapshot.kb_enabled,
                                effect: st_snapshot.streamer_effect,
                                fps: st_snapshot.kb_fps,
                                brightness_pct,
                                speed: st_snapshot.kb_speed,
                                direction: st_snapshot.kb_direction,
                                color_hex: st_snapshot.kb_color.clone(),
                            };
                            streamer.update_params(sp);
                            streamer.start();
                            use tauri::Emitter;
                            let _ = app.emit("kb-engine-changed", "streamer");
                        }

                        // Auto-restore lighting on AC plug-in (Wake from EC power cut / sleep lock)
                        std::thread::spawn(|| {
                            // Sleep 300ms to allow power line voltage & USB rail to settle
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            crate::log_info!("POWER_WATCHER", "AC power reconnected! Re-arming keyboard and lighting...");
                            crate::driver::ite_lighting::get_global_lighting().resume_after_wake();
                        });
                    }
                }
            }
            last_ac_line = Some(is_ac);

            // 8. Custom Mode (Mode 4) Live Telemetry & Throttle Diagnostics (Gaming Laptops Only)
            let cur_mode = crate::system::config::load_config().power_mode;
            if !is_ultrabook && cur_mode == 4 && supports_legacy_ec_switches {
                custom_trace_ticks = custom_trace_ticks.wrapping_add(1);
                if let Some((curr_mhz, max_mhz, limit_mhz)) = crate::system::power_mode::get_cpu_frequency_info() {
                    let is_throttled = curr_mhz < 1000 && is_dc;
                    if is_throttled {
                        if !last_was_throttled || custom_trace_ticks % 5 == 0 {
                            let raw_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL1).unwrap_or(0);
                            let raw_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL2).unwrap_or(0);
                            let raw_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL4).unwrap_or(0);
                            let raw_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_TCC).unwrap_or(0);
                            let raw_fan_clamp = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FAN_BOOST_CLAMP).unwrap_or(0);
                            let raw_power_status = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_POWER_STATUS).unwrap_or(0);
                            let raw_turbo_flag = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_TRIGGER_1115).unwrap_or(0);
                            crate::log_warn!(
                                "CUSTOM_TRACE",
                                "[THROTTLE ANOMALY DETECTED] CPU Frequency collapsed to {}MHz (Max: {}MHz, Limit: {}MHz)! EC Diagnostic Dump: PL1={}W, PL2={}W, PL4={}W, Tcc=0x{:02X}, Boost=0x{:02X}, Power=0x{:02X}, Silent={}",
                                curr_mhz, max_mhz, limit_mhz,
                                raw_pl1, raw_pl2, raw_pl4, raw_tcc, raw_fan_clamp, raw_power_status, raw_turbo_flag
                            );

                            // SELF-HEALING GUARD: If Tcc in EC is 0x00 while in Custom Mode,
                            // EC triggers BD PROCHOT (0C wall). Instantly self-heal to 95C!
                            if raw_tcc == 0 {
                                crate::log_warn!("CUSTOM_TRACE", "[SELF-HEALING ACTIVATED] EC TCC was 0x00 causing BD PROCHOT 0.8GHz freeze! Re-asserting safe 95C thermal wall...");
                                crate::driver::wmi_ec::set_cpu_temp_target(true, 95);
                                let cur_trig = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
                                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, (cur_trig & 0x10) | 0x20);
                            }
                        }
                        last_was_throttled = true;
                    } else {
                        last_was_throttled = false;
                        if custom_trace_ticks % 8 == 0 {
                            // Periodic live status every ~5 seconds
                            let raw_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL1).unwrap_or(0);
                            let raw_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL2).unwrap_or(0);
                            let raw_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL4).unwrap_or(0);
                            let raw_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_TCC).unwrap_or(0);
                            let raw_db_watts = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_GPU_DB_MAX).unwrap_or(0);
                            let raw_power_status = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_POWER_STATUS).unwrap_or(0);
                            crate::log_info!(
                                "CUSTOM_TRACE",
                                "Custom Mode Runtime: CPU Freq={}MHz (Max={}MHz), PL1={}W, PL2={}W, PL4={}W, Tcc=0x{:02X}, DB={}W, Power=0x{:02X} (DC_Online={})",
                                curr_mhz, max_mhz,
                                raw_pl1, raw_pl2, raw_pl4, raw_tcc, raw_db_watts, raw_power_status, is_dc
                            );
                        }
                    }
                }
            } else {
                last_was_throttled = false;
            }

            // 9. Watch Windows Color Management / Active ICM Profile Changes (every ~1.2s)
            if tick_count % 24 == 0 {
                let monitors = crate::system::display::get_all_monitors();
                let cfg = crate::system::color_calibration::load_calibration_config();
                let mut changed = false;

                for mon in &monitors {
                    // Only query registry snapshot (real user-assigned profile, avoiding GDI fallback to OEM default)
                    let cur_icm = crate::system::color_calibration::get_registry_active_icm_for_device(&mon.device_name);

                    let mon_setting = cfg.monitors.get(&mon.device_name);
                    let is_configured_uncalibrated = mon_setting.map(|s| !s.enabled || s.selected_profile_id == "default_native").unwrap_or(false);

                    // If user in OpenRevo explicitly chose default_native, do NOT auto-restore!
                    if is_configured_uncalibrated && cur_icm.is_none() {
                        last_color_profiles.insert(mon.device_name.clone(), None);
                        continue;
                    }

                    let prev_icm = last_color_profiles.get(&mon.device_name).cloned();
                    if let Some(prev) = prev_icm {
                        if prev != cur_icm {
                            crate::log_info!(
                                "COLOR_WATCHER",
                                "External Windows color profile changed on {}: {:?} -> {:?}",
                                mon.device_name,
                                prev,
                                cur_icm
                            );
                            changed = true;
                        }
                    }
                    last_color_profiles.insert(mon.device_name.clone(), cur_icm);
                }
                if changed {
                    use tauri::Emitter;
                    let _ = app.emit("color-calibration-changed", ());
                    let _ = app.emit_to("main", "color-calibration-changed", ());
                }
            }
        }
    });
}
