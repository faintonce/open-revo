use std::os::windows::process::CommandExt;
use std::process::Command;
use windows::core::w;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    RegNotifyChangeKeyValue, HKEY_CURRENT_USER,
    KEY_READ, KEY_SET_VALUE, KEY_NOTIFY, REG_DWORD,
    REG_NOTIFY_CHANGE_LAST_SET, HKEY,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_F24, VK_LCONTROL, VK_LWIN,
};

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 合成按键的标记，写进 `dwExtraInfo`。键盘钩子据此区分「我们自己注入的触摸板热键」
/// 与用户真实按下的按键，避免注入的键再被当成无线电热键处理。
pub const SYNTHETIC_KEY_TAG: usize = 0x4F52_5654;

/// 发一次 Windows 官方触摸板快捷键（Win + Ctrl + F24）。
fn send_touchpad_hotkey() {
    unsafe {
        keybd_event(VK_LWIN.0 as u8, 0, KEYBD_EVENT_FLAGS(0), SYNTHETIC_KEY_TAG);
        keybd_event(VK_LCONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), SYNTHETIC_KEY_TAG);
        keybd_event(VK_F24.0 as u8, 0, KEYBD_EVENT_FLAGS(0), SYNTHETIC_KEY_TAG);
        keybd_event(VK_F24.0 as u8, 0, KEYEVENTF_KEYUP, SYNTHETIC_KEY_TAG);
        keybd_event(VK_LCONTROL.0 as u8, 0, KEYEVENTF_KEYUP, SYNTHETIC_KEY_TAG);
        keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, SYNTHETIC_KEY_TAG);
    }
}

/// 切换触摸板：按「最快且验证得到」的通道依次尝试。
///
/// 返回 `(channel, success)`，channel 为 `already` / `hotkey` / `adminflows` / `failed`。
/// 之所以不用厂商通道打头：它要拉起 SystemSettingsAdminFlows.exe（会闪设置窗口、需要提权、
/// 单次几百毫秒到数秒），而 Win+Ctrl+F24 是 Windows 自己的 PTP 快捷键，几乎瞬时。
pub fn apply_touchpad_state(enable: bool) -> (&'static str, bool) {
    // 1. 已经是目标状态：什么都不做，别去打扰 EC 和设置应用
    if read_ptp_registry_enabled() == Some(enable) {
        return ("already", true);
    }

    // 2. 合成 Win+Ctrl+F24 + 回读确认（无需提权，不闪窗口）
    send_touchpad_hotkey();
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(60));
        if read_ptp_registry_enabled() == Some(enable) {
            return ("hotkey", true);
        }
    }

    // 3. 兜底：厂商通道（设备管理器恢复 + 官方设置流 + PTP 注册表）
    ensure_touchpad_device_enabled();
    let admin_flows_val = if enable { "1" } else { "0" };
    let _ = Command::new("C:\\Windows\\System32\\SystemSettingsAdminFlows.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["EnableTouchpad", admin_flows_val])
        .output();
    write_ptp_registry_enabled(enable);
    if read_ptp_registry_enabled() == Some(enable) {
        ("adminflows", true)
    } else {
        ("failed", false)
    }
}

/// Ensure touchpad is NOT left in PnP-disabled state in Windows Device Manager
pub fn ensure_touchpad_device_enabled() {
    let _ = Command::new("pnputil")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/enable-device", "/deviceid", "HID\\UNIW0001&Col02"])
        .output();
    let _ = Command::new("pnputil")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/enable-device", "/deviceid", "HID\\UNIW0001&Col01"])
        .output();
}

pub fn read_ptp_registry_enabled() -> Option<bool> {
    unsafe {
        let mut hkey = HKEY::default();
        let sub_key = w!(r"Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad\Status");
        let value_name = w!("Enabled");
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub_key, 0, KEY_READ, &mut hkey).is_ok() {
            let mut data_type = REG_DWORD;
            let mut data = 0u32;
            let mut data_size = std::mem::size_of::<u32>() as u32;
            let status = RegQueryValueExW(
                hkey,
                value_name,
                None,
                Some(&mut data_type),
                Some(&mut data as *mut u32 as *mut u8),
                Some(&mut data_size),
            );
            let _ = RegCloseKey(hkey);
            if status.is_ok() {
                return Some(data != 0);
            }
        }
    }
    None
}

pub fn write_ptp_registry_enabled(enabled: bool) {
    unsafe {
        let mut hkey = HKEY::default();
        let sub_key = w!(r"Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad\Status");
        let value_name = w!("Enabled");
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub_key, 0, KEY_SET_VALUE, &mut hkey).is_ok() {
            let val = if enabled { 1u32 } else { 0u32 };
            let _ = RegSetValueExW(
                hkey,
                value_name,
                0,
                REG_DWORD,
                Some(std::slice::from_raw_parts(&val as *const u32 as *const u8, 4)),
            );
            let _ = RegCloseKey(hkey);
        }
    }
}

/// Read real-time Touchpad hardware / system status (0ms native Win32 API)
pub fn get_realtime_touchpad_state(has_ec_switches: bool, fallback: bool) -> bool {
    // 1. Primary: Windows Precision Touchpad (PTP) Registry (0ms native Win32 API)
    // Tracks whether Windows input subsystem is actually accepting touch input (e.g. toggled by Fn+F5)
    if let Some(enabled) = read_ptp_registry_enabled() {
        return enabled;
    }

    // 2. Hardware fallback: ADDR_BATTERY_PROTECTION_MODE TOUCHPAD_BITMASK
    if has_ec_switches {
        if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE) {
            return (val & crate::driver::wmi_ec::TOUCHPAD_BITMASK) != 0;
        }
    }

    fallback
}

/// Start kernel-level registry notification listener on PrecisionTouchPad status
/// Automatically wakes on Fn+F5 or Windows settings toggle and broadcasts to Tauri UI
pub fn start_touchpad_listener() {
    std::thread::spawn(move || {
        crate::log_info!("DEVICE_SWITCH", "Starting Touchpad PrecisionTouchPad registry kernel listener thread...");
        let sub_key = w!(r"Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad\Status");

        loop {
            let mut hkey = HKEY::default();
            let open_status = unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    sub_key,
                    0,
                    KEY_NOTIFY | KEY_READ,
                    &mut hkey,
                )
            };

            if open_status.is_err() {
                std::thread::sleep(std::time::Duration::from_secs(3));
                continue;
            }

            let event = unsafe { CreateEventW(None, false, false, None) };
            let Ok(event_handle) = event else {
                unsafe { let _ = RegCloseKey(hkey); }
                std::thread::sleep(std::time::Duration::from_secs(3));
                continue;
            };

            let mut last_known_state = read_ptp_registry_enabled();

            loop {
                let notify_res = unsafe {
                    RegNotifyChangeKeyValue(
                        hkey,
                        false,
                        REG_NOTIFY_CHANGE_LAST_SET,
                        event_handle,
                        true,
                    )
                };

                if notify_res.is_err() {
                    break;
                }

                // Block thread until registry changes (0% CPU, wakes within <1ms of Fn+F5 toggle)
                let _ = unsafe { WaitForSingleObject(event_handle, 2000) };

                let current_state = read_ptp_registry_enabled();
                if let Some(new_state) = current_state {
                    if last_known_state != Some(new_state) {
                        crate::log_info!("DEVICE_SWITCH", "Touchpad state change detected by kernel: {:?} -> {}", last_known_state, new_state);
                        last_known_state = Some(new_state);

                        // 1. Sync config
                        let mut cfg = crate::system::config::load_config();
                        cfg.touchpad_enabled = new_state;
                        crate::system::config::save_config(&cfg);

                        // 2. Sync Touchpad register on supported hardware
                        if super::has_ec_device_switches() {
                            crate::driver::wmi_ec::wmi_modify_ec_mask(
                                crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE,
                                crate::driver::wmi_ec::TOUCHPAD_BITMASK,
                                new_state,
                            );
                        }

                        // 3. Emit event to UI / subscribers
                        crate::event::osd_notify_global("touchpad", &new_state.to_string());
                        crate::event::emit_global("touchpad-state-changed", serde_json::json!(new_state));
                    }
                }
            }

            unsafe {
                let _ = CloseHandle(event_handle);
                let _ = RegCloseKey(hkey);
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}
