use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::process::Command;

pub mod audio;
pub mod camera;
pub mod radio;
pub mod touchpad;
pub mod watcher;

pub use audio::{get_microphone_state, set_microphone_state};
pub use camera::{
    get_camera_pnp_status, get_realtime_camera_state, read_camera_consent_registry,
    read_camera_policy_registry, write_camera_consent_registry, write_camera_policy_registry,
};
pub use radio::{
    get_native_bluetooth_radio_state, get_or_detect_wifi_adapter, get_realtime_bluetooth_state,
    get_realtime_wifi_state, get_wifi_radio_state_native, set_native_bluetooth_radio_state,
    set_wifi_radio_state_native,
};
pub use touchpad::{
    ensure_touchpad_device_enabled, get_realtime_touchpad_state, read_ptp_registry_enabled,
    start_touchpad_listener, write_ptp_registry_enabled,
};
pub use watcher::start_hardware_state_watcher;

const CREATE_NO_WINDOW: u32 = 0x08000000;

static KEEP_AWAKE_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SCREEN_OFF_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SCREEN_OFF_UNBLOCK_TICK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[repr(C)]
pub struct LASTINPUTINFO {
    pub cb_size: u32,
    pub dw_time: u32,
}

#[link(name = "user32")]
extern "system" {
    pub fn BlockInput(f_block_it: i32) -> i32;
    pub fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetTickCount() -> u32;
}

struct BlockInputGuard;

impl Drop for BlockInputGuard {
    fn drop(&mut self) {
        unsafe {
            BlockInput(0);
        }
    }
}

pub fn set_keep_awake(enable: bool) {
    KEEP_AWAKE_ACTIVE.store(enable, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_keep_awake() -> bool {
    KEEP_AWAKE_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn is_screen_off() -> bool {
    SCREEN_OFF_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_screen_off(active: bool) {
    SCREEN_OFF_ACTIVE.store(active, std::sync::atomic::Ordering::Relaxed);
    if !active {
        SCREEN_OFF_UNBLOCK_TICK.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn get_screen_off_unblock_tick() -> u32 {
    SCREEN_OFF_UNBLOCK_TICK.load(std::sync::atomic::Ordering::Relaxed)
}

/// Turn off screen immediately with a 3-second peripheral input grace period.
/// Suppresses mouse/keyboard wakeup triggers for 3s to let the user move their hand away;
/// guarantees system execution (CPU & Agent tasks) stays awake overnight without forcing display on
/// and NEVER broadcasts SC_MONITORPOWER to HWND_BROADCAST.
pub fn turn_off_screen() -> Result<(), String> {
    crate::log_info!("SCREEN", "Turn off screen requested. Starting 3-second peripheral grace period...");
    set_screen_off(true);

    std::thread::spawn(|| {
        let _guard = BlockInputGuard;

        // 1. Brief delay to allow mouse-up / click release events to clear
        std::thread::sleep(std::time::Duration::from_millis(150));

        // 2. Block peripheral inputs (mouse & keyboard) so hand movement doesn't wake display immediately
        unsafe {
            BlockInput(1);
        }

        // 4. Send SC_MONITORPOWER message to power off monitors via our own dedicated window
        // NEVER broadcast to HWND_BROADCAST (0xFFFF), which triggers Modern Standby & OS sleep!
        unsafe {
            const WM_SYSCOMMAND: u32 = 0x0112;
            const SC_MONITORPOWER: usize = 0xF170;
            if let Some(hwnd) = crate::system::power_listener::get_power_watcher_hwnd() {
                let _ = windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(
                    hwnd,
                    WM_SYSCOMMAND,
                    windows::Win32::Foundation::WPARAM(SC_MONITORPOWER),
                    windows::Win32::Foundation::LPARAM(2),
                );
            } else {
                let desktop = windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow();
                let _ = windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(
                    desktop,
                    WM_SYSCOMMAND,
                    windows::Win32::Foundation::WPARAM(SC_MONITORPOWER),
                    windows::Win32::Foundation::LPARAM(2),
                );
            }
        }

        // 5. 3-second grace period for the user to step away
        std::thread::sleep(std::time::Duration::from_millis(3000));

        // 6. Record the unblock tick before restoring input
        let tick = unsafe { GetTickCount() };
        SCREEN_OFF_UNBLOCK_TICK.store(tick, std::sync::atomic::Ordering::Relaxed);

        // 7. Unblock peripheral inputs safely
        drop(_guard);
        crate::log_info!("SCREEN", "3-second grace period ended. Peripheral input restored (subsequent interaction will wake display).");
    });

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickSwitchesState {
    pub wifi: bool,
    pub bluetooth: bool,
    pub touchpad: bool,
    pub camera: bool,
    pub mic: bool,
    pub osd: bool,
    pub win_key: bool,
    pub fn_lock: bool,
    pub keep_awake: bool,
}

/// Detect whether the current machine supports EC physical hardware device switches
/// (Direct hardware register communication for Camera Cutoff, Touchpad state, etc.)
pub fn has_ec_device_switches() -> bool {
    crate::driver::wmi_ec::is_acpi_driver_available()
        && crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_PROJECT_ID).is_some()
}

pub fn get_switches_state() -> QuickSwitchesState {
    let mut cfg = crate::system::config::load_config();
    static ONCE_RESTORE: std::sync::Once = std::sync::Once::new();
    ONCE_RESTORE.call_once(|| {
        // 1. Touchpad restore
        if !cfg.touchpad_enabled {
            crate::driver::wmi_ec::wmi_modify_ec_mask(
                crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE,
                crate::driver::wmi_ec::TOUCHPAD_BITMASK,
                false,
            );
            let _ = Command::new("C:\\Windows\\System32\\SystemSettingsAdminFlows.exe")
                .creation_flags(CREATE_NO_WINDOW)
                .args(["EnableTouchpad", "0"])
                .output();
            touchpad::write_ptp_registry_enabled(false);
            crate::log_info!("DEVICE_SWITCH", "First-run restored Touchpad: disabled");
        } else {
            touchpad::ensure_touchpad_device_enabled();
            crate::driver::wmi_ec::wmi_modify_ec_mask(
                crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE,
                crate::driver::wmi_ec::TOUCHPAD_BITMASK,
                true,
            );
            touchpad::write_ptp_registry_enabled(true);
        }

        // 2. Win Key Lock restore
        let is_ultrabook = crate::hal::platform_probe::is_ultrabook();
        let supports_legacy_ec = crate::hal::platform_probe::is_tongfang_family() && !is_ultrabook;
        if supports_legacy_ec {
            let hw_locked = crate::driver::wmi_ec::is_winkey_locked_hw();
            if hw_locked != cfg.win_key_locked {
                crate::driver::wmi_ec::send_winkey_lock_trigger();
            }
        }
        crate::system::hooks::set_win_key_locked(cfg.win_key_locked);
        crate::log_info!("DEVICE_SWITCH", "First-run restored Win Key lock: locked={}", cfg.win_key_locked);

        // 3. Fn Lock restore
        if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FN_LOCK) {
            let target_val = if cfg.fn_lock_enabled {
                val | 0x10
            } else {
                val & !0x10
            };
            if val != target_val {
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_FN_LOCK, target_val);
                crate::log_info!("DEVICE_SWITCH", "First-run restored Fn Lock to EC: enabled={}", cfg.fn_lock_enabled);
            }
        }

        // 4. Keep Awake restore
        if cfg.keep_awake_enabled {
            set_keep_awake(true);
            crate::log_info!("DEVICE_SWITCH", "First-run restored Keep Awake: enabled=true");
        }

        // 5. Wi-Fi restore
        if !cfg.wifi_enabled {
            let _ = radio::set_wifi_radio_state_native(false);
            crate::log_info!("DEVICE_SWITCH", "First-run restored Wi-Fi radio: disabled");
        }

        // 6. Bluetooth restore
        if !cfg.bluetooth_enabled {
            let _ = radio::set_native_bluetooth_radio_state(false);
            crate::log_info!("DEVICE_SWITCH", "First-run restored Bluetooth radio: disabled");
        }

        // 7. Microphone restore
        if !cfg.mic_enabled {
            let _ = audio::set_microphone_state(false);
            crate::log_info!("DEVICE_SWITCH", "First-run restored Microphone: muted");
        }

        // 8. Camera restore (EC physical cutoff on supported hardware)
        if !cfg.camera_enabled {
            if has_ec_device_switches() {
                if let Some(mut val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF) {
                    val |= crate::driver::wmi_ec::CAMERA_CUTOFF_BITMASK;
                    crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF, val);
                }
            }
            camera::write_camera_policy_registry(false);
            camera::write_camera_consent_registry(false);
            crate::log_info!("DEVICE_SWITCH", "First-run restored Camera: disabled");
        }
    });

    let has_ec_switches = has_ec_device_switches();

    // 1. Read real-time Fn lock state directly from EC Bit 4 (0x11 = Locked/ON, 0x01 = Unlocked/OFF)
    let fn_lock_active = if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FN_LOCK) {
        (val & 0x10) != 0
    } else {
        cfg.fn_lock_enabled
    };

    // 2. Read real-time Win key lock state
    let win_key_active = if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_STATUS_BYTE) {
        (val & crate::driver::wmi_ec::STATUS_WINLOCK) != 0
    } else {
        crate::system::hooks::is_win_key_locked()
    };

    // 3. Read real-time Camera state (hardware power cutoff on supported hardware)
    let camera_active = camera::get_realtime_camera_state(has_ec_switches, cfg.camera_enabled);

    // 4. Read real-time Touchpad state (hardware status on supported hardware)
    let touchpad_active = touchpad::get_realtime_touchpad_state(has_ec_switches, cfg.touchpad_enabled);

    // 5. Read real-time Wi-Fi state
    let wifi_active = radio::get_realtime_wifi_state().unwrap_or(cfg.wifi_enabled);

    // 6. Read real-time Bluetooth state
    let bt_active = radio::get_realtime_bluetooth_state().unwrap_or(cfg.bluetooth_enabled);

    let mic_active = audio::get_microphone_state().unwrap_or(cfg.mic_enabled);
    let keep_awake_active = is_keep_awake();

    // Synchronize current live hardware state to persistent config.json
    let mut changed = false;
    if cfg.fn_lock_enabled != fn_lock_active {
        cfg.fn_lock_enabled = fn_lock_active;
        changed = true;
    }
    if cfg.win_key_locked != win_key_active {
        cfg.win_key_locked = win_key_active;
        changed = true;
    }
    if cfg.camera_enabled != camera_active {
        cfg.camera_enabled = camera_active;
        changed = true;
    }
    if cfg.mic_enabled != mic_active {
        cfg.mic_enabled = mic_active;
        changed = true;
    }
    if cfg.keep_awake_enabled != keep_awake_active {
        cfg.keep_awake_enabled = keep_awake_active;
        changed = true;
    }
    if cfg.touchpad_enabled != touchpad_active {
        cfg.touchpad_enabled = touchpad_active;
        changed = true;
    }
    if cfg.wifi_enabled != wifi_active {
        cfg.wifi_enabled = wifi_active;
        changed = true;
    }
    if cfg.bluetooth_enabled != bt_active {
        cfg.bluetooth_enabled = bt_active;
        changed = true;
    }
    if changed {
        crate::system::config::save_config(&cfg);
    }

    QuickSwitchesState {
        wifi: wifi_active,
        bluetooth: bt_active,
        touchpad: touchpad_active,
        camera: camera_active,
        mic: mic_active,
        osd: cfg.osd_enabled,
        win_key: win_key_active,
        fn_lock: fn_lock_active,
        keep_awake: keep_awake_active,
    }
}

pub fn set_switch(switch_name: &str, enable: bool) -> Result<bool, String> {
    let mut cfg = crate::system::config::load_config();
    let has_ec_switches = has_ec_device_switches();

    match switch_name {
        "wifi" => {
            // 1. Try native WinRT Radio API first (fast RF transmitter toggle, ~10ms)
            let radio_ok = radio::set_wifi_radio_state_native(enable);
            if radio_ok {
                crate::log_info!("DEVICE_SWITCH", "Wi-Fi radio toggled to {} via WinRT Radio API", enable);
            } else {
                let adapter_name = radio::get_or_detect_wifi_adapter(&mut cfg);
                let admin_state = if enable { "ENABLED" } else { "DISABLED" };
                let _ = Command::new("netsh")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args(["interface", "set", "interface", &format!("name=\"{}\"", adapter_name), &format!("admin={}", admin_state)])
                    .output();
                crate::log_info!("DEVICE_SWITCH", "Wi-Fi ({}) set to {} via netsh", adapter_name, admin_state);
            }
            cfg.wifi_enabled = enable;
        }
        "bluetooth" => {
            let success = radio::set_native_bluetooth_radio_state(enable);
            if success {
                crate::log_info!("DEVICE_SWITCH", "Bluetooth radio toggled to {} via WinRT", enable);
            } else {
                crate::log_warn!("DEVICE_SWITCH", "WinRT Radio could not find Bluetooth adapter");
            }
            radio::record_bluetooth_state(enable);
            cfg.bluetooth_enabled = enable;
        }
        "touchpad" => {
            // Always ensure PnP device in Device Manager is restored/enabled so Windows & Fn+F5 operate normally
            touchpad::ensure_touchpad_device_enabled();

            // 1. Hardware EC: Synchronize Touchpad state
            // (Controls physical corner indicator LED on models with LED, and keeps EC state aligned)
            crate::driver::wmi_ec::wmi_modify_ec_mask(
                crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE,
                crate::driver::wmi_ec::TOUCHPAD_BITMASK,
                enable,
            );

            // 2. Windows System-Level Precision Touchpad Toggle:
            // Invoke SystemSettingsAdminFlows.exe (Windows native Settings utility, runs elevated)
            let admin_flows_val = if enable { "1" } else { "0" };
            let _ = Command::new("C:\\Windows\\System32\\SystemSettingsAdminFlows.exe")
                .creation_flags(CREATE_NO_WINDOW)
                .args(["EnableTouchpad", admin_flows_val])
                .output();

            // 3. Synchronize Windows Precision Touchpad (PTP) Registry (1 = On, 0 = Off) via 0ms native Win32 API
            touchpad::write_ptp_registry_enabled(enable);

            crate::log_info!("DEVICE_SWITCH", "Touchpad set to enable={} (Hardware, AdminFlows, Win32 PTP Registry synced)", enable);
            cfg.touchpad_enabled = enable;
            crate::system::osd_window::show_key_lock_toast("touchpad", !enable);
        }
        "camera" => {
            // 1. Hardware device disable/enable via Windows PnP utility (Universal across all Windows laptops)
            let action = if enable { "/enable-device" } else { "/disable-device" };
            let _ = Command::new("pnputil")
                .creation_flags(CREATE_NO_WINDOW)
                .args([action, "/class", "Camera"])
                .output();
            let _ = Command::new("pnputil")
                .creation_flags(CREATE_NO_WINDOW)
                .args([action, "/class", "Image"])
                .output();

            // 2. Windows Camera Group Policy (AllowCamera)
            camera::write_camera_policy_registry(enable);

            // 3. Windows Privacy ConsentStore (Packaged + NonPackaged desktop apps)
            camera::write_camera_consent_registry(enable);

            // 4. Hardware EC: on supported hardware with physical LDO line
            if has_ec_switches {
                // Clear bit 3 -> Camera Power ON (enable), Set bit 3 -> Camera Physical Cutoff (!enable)
                crate::driver::wmi_ec::wmi_modify_ec_mask(
                    crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF,
                    crate::driver::wmi_ec::CAMERA_CUTOFF_BITMASK,
                    !enable,
                );
            }
            crate::log_info!("DEVICE_SWITCH", "Camera toggled to {} (PnP, Policy, ConsentStore + EC synced)", enable);
            cfg.camera_enabled = enable;
            crate::system::osd_window::show_device_switch("camera", enable);
        }
        "win_key" => {
            // 1. Software Low-Level Keyboard Hook
            crate::system::hooks::set_win_key_locked(enable);

            // 2. Hardware EC Keyboard Controller WinLock Trigger (pulsed on state difference)
            let is_ultrabook = crate::hal::platform_probe::is_ultrabook();
            let supports_legacy_ec = crate::hal::platform_probe::is_tongfang_family() && !is_ultrabook;
            if supports_legacy_ec {
                let hw_locked = crate::driver::wmi_ec::is_winkey_locked_hw();
                if hw_locked != enable {
                    crate::driver::wmi_ec::send_winkey_lock_trigger();
                }
            }

            cfg.win_key_locked = enable;
            crate::system::osd_window::show_key_lock_toast("win", enable);
            crate::log_info!("DEVICE_SWITCH", "WinKey lock set to {} (legacy_ec: {})", enable, supports_legacy_ec);
        }
        "mic" => {
            let res = audio::set_microphone_state(enable)?;
            cfg.mic_enabled = res;
            crate::system::osd_window::show_device_switch("mic", res);
            crate::log_info!("DEVICE_SWITCH", "Microphone state set to {}", res);
        }
        "keep_awake" => {
            set_keep_awake(enable);
            cfg.keep_awake_enabled = enable;
            crate::system::osd_window::show_device_switch("keep_awake", enable);
            crate::log_info!("DEVICE_SWITCH", "Keep awake state set to {}", enable);
        }
        "fn_lock" => {
            // Verified EC FnLock Bit 4: 0x11 (17) = FnLock Locked/ON, 0x01 (1) = FnLock Unlocked/OFF
            if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FN_LOCK) {
                let new_val = if enable {
                    val | 0x10  // Set bit 4 -> Fn Lock Locked (0x11)
                } else {
                    val & !0x10 // Clear bit 4 -> Fn Lock Unlocked (0x01)
                };
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_FN_LOCK, new_val);
                crate::log_info!("DEVICE_SWITCH", "FnLock toggled to {}: mode=0x{:02X}", enable, new_val);
            }
            cfg.fn_lock_enabled = enable;
            crate::system::osd_window::show_key_lock_toast("fn", enable);
        }
        "osd" => {
            cfg.osd_enabled = enable;
            crate::log_info!("DEVICE_SWITCH", "OSD overlay switch toggled to {}", enable);
        }
        _ => return Err(format!("Unknown switch: {}", switch_name)),
    }

    crate::system::config::save_config(&cfg);
    Ok(enable)
}
