use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Power::{
    RegisterPowerSettingNotification, POWERBROADCAST_SETTING,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassExW, TranslateMessage, MSG, WINDOW_EX_STYLE, WNDCLASSEXW,
    WS_OVERLAPPED,
};

const WM_POWERBROADCAST: u32 = 0x0218;
const WM_QUERYENDSESSION: u32 = 0x0011;
const WM_ENDSESSION: u32 = 0x0016;
const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_DESTROY: u32 = 0x0002;

const PBT_APMSUSPEND: u32 = 0x0004;
const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;
const PBT_APMRESUMESUSPEND: u32 = 0x0007;
const PBT_POWERSETTINGCHANGE: u32 = 0x8013;
const DEVICE_NOTIFY_WINDOW_HANDLE: windows::Win32::UI::WindowsAndMessaging::REGISTER_NOTIFICATION_FLAGS =
    windows::Win32::UI::WindowsAndMessaging::REGISTER_NOTIFICATION_FLAGS(0);

// GUID for Lid switch state: {BA3E0F4D-B817-4094-A2D1-D56379E6A0F3}
const GUID_LIDSWITCH_STATE_CHANGE: windows::core::GUID =
    windows::core::GUID::from_u128(0xba3e0f4d_b817_4094_a2d1_d56379e6a0f3);

static IS_LISTENING: AtomicBool = AtomicBool::new(false);
static POWER_WATCHER_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
static GLOBAL_EC: parking_lot::RwLock<Option<crate::driver::ec_bridge::SharedEcBridge>> = parking_lot::RwLock::new(None);

pub fn init_power_event_watcher(ec: crate::driver::ec_bridge::SharedEcBridge) {
    *GLOBAL_EC.write() = Some(ec);
    start_power_event_watcher();
}

pub fn get_power_watcher_hwnd() -> Option<HWND> {
    let val = POWER_WATCHER_HWND.load(Ordering::SeqCst);
    if val != 0 {
        Some(HWND(val as _))
    } else {
        None
    }
}

static PREV_POWER_MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

fn handle_sleep_auto_office() {
    let cfg = crate::system::config::load_config();
    if cfg.sleep_auto_office {
        let cur_mode = cfg.power_mode;
        if cur_mode != 1 {
            println!("[Sleep Guard] Auto-switching from mode {} to Office Mode (1) before sleep / lid close!", cur_mode);
            PREV_POWER_MODE.store(cur_mode, Ordering::SeqCst);
            crate::system::power_mode::apply_full_power_mode(1);
        }
    }
}

fn handle_wake_auto_office() {
    let cfg = crate::system::config::load_config();
    if cfg.sleep_auto_office {
        let prev = PREV_POWER_MODE.swap(0, Ordering::SeqCst);
        if prev > 1 && prev <= 4 {
            println!("[Sleep Guard] Waking up! Restoring previous power mode: {}", prev);
            crate::system::power_mode::apply_full_power_mode(prev);
        }
    }
}

fn handle_system_resume_pipeline(event_name: &str) {
    crate::system::telemetry::set_screen_suspended(false);
    if crate::hal::platform_probe::is_ultrabook() {
        println!("[Power Guard] Ultrabook {}! Restoring power mode only.", event_name);
        handle_wake_auto_office();
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            crate::system::display_tuning::init_display_tuning();
        });
        return;
    }

    println!("[Power Guard] Laptop {}! Restoring power mode, cooler & lighting...", event_name);
    handle_wake_auto_office();

    std::thread::spawn(|| {
        let cooler = crate::driver::ble_cooler::get_global_cooler();
        cooler.resume_after_wake();
        cooler.start_active_cooling();
    });

    std::thread::spawn(|| {
        crate::driver::ite_lighting::get_global_lighting().resume_after_wake();
        std::thread::sleep(std::time::Duration::from_millis(500));
        crate::system::display_tuning::init_display_tuning();
    });

    let cfg = crate::system::config::load_config();
    if let Some(ec) = GLOBAL_EC.read().clone() {
        let cfg_limit = cfg.battery_limit;
        let cfg_override = cfg.battery_hardware_limit_enabled;
        println!(
            "[Power Guard] System resumed: Re-asserting battery limit ({}%) with override={}",
            cfg_limit, cfg_override
        );
        ec.lock().apply_battery_limit_with_override(cfg_limit, cfg_override);

        // Delayed re-assertion to defend against late ACPI battery driver initialization
        let ec_delayed = ec.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(2000));
            let cfg_delayed = crate::system::config::load_config();
            if cfg_delayed.battery_hardware_limit_enabled && cfg_delayed.battery_limit < 95 {
                ec_delayed.lock().apply_battery_limit_with_override(
                    cfg_delayed.battery_limit,
                    true,
                );
            }
        });
    }

    crate::driver::wmi_ec::wmi_modify_ec_mask(
        crate::driver::wmi_ec::ADDR_FN_LOCK,
        0x10,
        cfg.fn_lock_enabled,
    );
    let is_ultrabook = crate::hal::platform_probe::is_ultrabook();
    let supports_legacy_ec = crate::hal::platform_probe::is_tongfang_family() && !is_ultrabook;
    if supports_legacy_ec {
        let hw_locked = crate::driver::wmi_ec::is_winkey_locked_hw();
        if hw_locked != cfg.win_key_locked {
            crate::driver::wmi_ec::send_winkey_lock_trigger();
        }
    }
    crate::system::hooks::set_win_key_locked(cfg.win_key_locked);
    if !cfg.touchpad_enabled {
        crate::driver::wmi_ec::wmi_modify_ec_mask(
            crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE,
            crate::driver::wmi_ec::TOUCHPAD_BITMASK,
            false,
        );
        crate::system::switches::touchpad::write_ptp_registry_enabled(false);
    }
    if !cfg.mic_enabled {
        let _ = crate::system::switches::audio::set_microphone_state(false);
    }
    if cfg.keep_awake_enabled {
        crate::system::switches::set_keep_awake(true);
    }
    if !cfg.wifi_enabled {
        let _ = crate::system::switches::radio::set_wifi_radio_state_native(false);
    }
    if !cfg.bluetooth_enabled {
        let _ = crate::system::switches::radio::set_native_bluetooth_radio_state(false);
    }
}

unsafe extern "system" fn power_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_POWERBROADCAST => {
            let pbt = wparam.0 as u32;
            if pbt == PBT_APMSUSPEND {
                // If Keep-Awake is active or screen was turned off for overnight tasks,
                // NEVER degrade power mode to Office (1) and NEVER shut down water cooling!
                let is_running_tasks = crate::system::switches::is_keep_awake() || crate::system::switches::is_screen_off();
                if is_running_tasks {
                    println!("[Power Guard] Standby event received but Keep-Awake or Screen-Off is ACTIVE. Preserving active power mode and water cooling for background tasks.");
                } else if crate::hal::platform_probe::is_ultrabook() {
                    println!("[Power Guard] Ultrabook entering Standby: skipping water cooler & lighting suspension.");
                    crate::system::telemetry::set_screen_suspended(true);
                    handle_sleep_auto_office();
                } else {
                    println!("[Power Guard] System is entering Sleep / Hibernate / Standby! Stopping water cooler & suspending lighting...");
                    crate::system::telemetry::set_screen_suspended(true);
                    handle_sleep_auto_office();
                    crate::driver::ble_cooler::get_global_cooler().stop_and_shutdown();
                    let streamer = crate::driver::ite_streamer::get_global_streamer();
                    streamer.suspend_for_sleep();
                    let lighting = crate::driver::ite_lighting::get_global_lighting();
                    lighting.turn_off_all();
                    lighting.invalidate_devices();
                    lighting.is_sleeping.store(true, Ordering::SeqCst);
                }
            } else if pbt == PBT_POWERSETTINGCHANGE {
                let setting_ptr = lparam.0 as *const POWERBROADCAST_SETTING;
                if !setting_ptr.is_null() {
                    let setting = &*setting_ptr;
                    if setting.PowerSetting == GUID_LIDSWITCH_STATE_CHANGE && setting.DataLength >= 4 {
                        static LAST_LID_STATE: std::sync::atomic::AtomicI8 = std::sync::atomic::AtomicI8::new(-1);
                        let lid_state = *(setting.Data.as_ptr() as *const u32);
                        let prev = LAST_LID_STATE.swap(lid_state as i8, Ordering::SeqCst);
                        if prev == -1 {
                            // Windows automatically posts initial state upon RegisterPowerSettingNotification.
                            // Record baseline only; NEVER trigger a false wake/resume pipeline on app startup!
                            println!("[Power Guard] Initial lid state registered: {} (startup baseline set)", if lid_state == 0 { "Closed" } else { "Open" });
                        } else if prev != lid_state as i8 {
                            // 0 = Lid Closed, 1 = Lid Open
                            if lid_state == 0 {
                                crate::system::telemetry::set_screen_suspended(true);
                                if crate::hal::platform_probe::is_ultrabook() {
                                    println!("[Power Guard] Ultrabook lid closed! Adjusting power mode only.");
                                    handle_sleep_auto_office();
                                } else {
                                    println!("[Power Guard] Laptop lid closed! Switching to office & stopping water cooler...");
                                    handle_sleep_auto_office();
                                    crate::driver::ble_cooler::get_global_cooler().stop_and_shutdown();
                                    let streamer = crate::driver::ite_streamer::get_global_streamer();
                                    streamer.suspend_for_sleep();
                                    let lighting = crate::driver::ite_lighting::get_global_lighting();
                                    lighting.turn_off_all();
                                    lighting.invalidate_devices();
                                    lighting.is_sleeping.store(true, Ordering::SeqCst);
                                }
                            } else {
                                handle_system_resume_pipeline("lid opened");
                            }
                        }
                    }
                }
            } else if pbt == PBT_APMRESUMEAUTOMATIC || pbt == PBT_APMRESUMESUSPEND {
                handle_system_resume_pipeline("resumed from Sleep / Hibernate");
            }
            LRESULT(1)
        }
        WM_QUERYENDSESSION | WM_ENDSESSION => {
            println!("[Power Guard] System is shutting down or restarting! Stopping water cooler & lighting...");
            crate::system::telemetry::set_screen_suspended(true);
            crate::system::display_tuning::reset_display_tuning_to_default();
            if !crate::hal::platform_probe::is_ultrabook() {
                crate::driver::ble_cooler::get_global_cooler().stop_and_shutdown();
                crate::driver::ite_streamer::get_global_streamer().suspend_for_sleep();
                let lighting = crate::driver::ite_lighting::get_global_lighting();
                lighting.turn_off_all();
                lighting.invalidate_devices();
            }
            LRESULT(1)
        }
        // Display topology or mode changed (resolution/refresh switch, monitor hot-plug,
        // projection mode). The cached mode table is stale from this moment on.
        WM_DISPLAYCHANGE => {
            crate::system::display::invalidate_display_mode_cache();
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Start background Windows power broadcast watcher thread.
/// Guaranteed to catch:
/// 1. Tray quit / App exit
/// 2. Laptop Lid Close (半闭/合盖)
/// 3. Sleep / Standby (待机睡眠)
/// 4. Hibernate (休眠)
/// 5. System Shutdown / Restart (系统关机)
pub fn start_power_event_watcher() {
    if IS_LISTENING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| unsafe {
        let class_name = w!("OpenRevoPowerWatcherClass");
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(power_wnd_proc),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassExW(&wnd_class);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("OpenRevoPowerWatcher"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND::default(),
            None,
            None,
            None,
        );

        if hwnd.is_err() {
            eprintln!("[Power Guard] Failed to create power watcher window");
            return;
        }

        let hwnd = hwnd.unwrap();
        POWER_WATCHER_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

        // Register explicitly for Lid Switch state change notifications
        let _ = RegisterPowerSettingNotification(
            HANDLE(hwnd.0 as _),
            &GUID_LIDSWITCH_STATE_CHANGE,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        );

        println!("[Power Guard] Dedicated power event listener successfully registered and running.");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}
