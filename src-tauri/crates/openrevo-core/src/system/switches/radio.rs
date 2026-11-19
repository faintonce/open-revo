use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use windows::Devices::Radios::{Radio, RadioKind, RadioState};

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub static LAST_BT_CHECK: AtomicU64 = AtomicU64::new(0);
pub static LAST_BT_STATE: AtomicBool = AtomicBool::new(true);

pub fn record_bluetooth_state(enable: bool) {
    LAST_BT_STATE.store(enable, Ordering::Relaxed);
    LAST_BT_CHECK.store(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        Ordering::Relaxed,
    );
}

/// Query real-time Bluetooth radio status using native WinRT Radio API (0ms, zero PowerShell)
pub fn get_native_bluetooth_radio_state() -> Option<bool> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
    }
    let _ = Radio::RequestAccessAsync().ok().and_then(|op| op.get().ok());
    let radios_op = Radio::GetRadiosAsync().ok()?;
    let radios = radios_op.get().ok()?;
    for radio in radios {
        if let Ok(kind) = radio.Kind() {
            if kind == RadioKind::Bluetooth {
                if let Ok(state) = radio.State() {
                    return Some(state == RadioState::On);
                }
            }
        }
    }
    None
}

/// Set Bluetooth radio state using native WinRT Radio API (zero PowerShell)
pub fn set_native_bluetooth_radio_state(enable: bool) -> bool {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
    }
    let _ = Radio::RequestAccessAsync().ok().and_then(|op| op.get().ok());
    let target = if enable { RadioState::On } else { RadioState::Off };
    let mut found = false;
    if let Ok(radios_op) = Radio::GetRadiosAsync() {
        if let Ok(radios) = radios_op.get() {
            for radio in radios {
                if let Ok(kind) = radio.Kind() {
                    if kind == RadioKind::Bluetooth {
                        if let Ok(op) = radio.SetStateAsync(target) {
                            let _ = op.get();
                            found = true;
                        }
                    }
                }
            }
        }
    }
    found
}

/// Read real-time Bluetooth adapter / radio status (cached for 3s, 0ms, zero PowerShell)
pub fn get_realtime_bluetooth_state() -> Option<bool> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let last = LAST_BT_CHECK.load(Ordering::Relaxed);
    if now.saturating_sub(last) < 3 {
        return Some(LAST_BT_STATE.load(Ordering::Relaxed));
    }

    if let Some(state) = get_native_bluetooth_radio_state() {
        LAST_BT_STATE.store(state, Ordering::Relaxed);
        LAST_BT_CHECK.store(now, Ordering::Relaxed);
        return Some(state);
    }

    Some(LAST_BT_STATE.load(Ordering::Relaxed))
}

/// Query real-time Wi-Fi radio status using native WinRT Radio API (0ms, zero PowerShell)
pub fn get_wifi_radio_state_native() -> Option<bool> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
    }
    let _ = Radio::RequestAccessAsync().ok().and_then(|op| op.get().ok());
    let radios_op = Radio::GetRadiosAsync().ok()?;
    let radios = radios_op.get().ok()?;
    for radio in radios {
        if let Ok(kind) = radio.Kind() {
            if kind == RadioKind::WiFi {
                if let Ok(state) = radio.State() {
                    return Some(state == RadioState::On);
                }
            }
        }
    }
    None
}

/// Set Wi-Fi radio state using native WinRT Radio API (fast RF toggle, ~10ms)
pub fn set_wifi_radio_state_native(enable: bool) -> bool {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
    }
    let _ = Radio::RequestAccessAsync().ok().and_then(|op| op.get().ok());
    let target = if enable { RadioState::On } else { RadioState::Off };
    let mut found = false;
    if let Ok(radios_op) = Radio::GetRadiosAsync() {
        if let Ok(radios) = radios_op.get() {
            for radio in radios {
                if let Ok(kind) = radio.Kind() {
                    if kind == RadioKind::WiFi {
                        if let Ok(op) = radio.SetStateAsync(target) {
                            let _ = op.get();
                            found = true;
                        }
                    }
                }
            }
        }
    }
    found
}

/// Read real-time Wi-Fi status (WinRT Radio first, fallback to netsh)
pub fn get_realtime_wifi_state() -> Option<bool> {
    // 1. Try WinRT Radio API first (instantaneous, 0ms)
    if let Some(state) = get_wifi_radio_state_native() {
        return Some(state);
    }

    // 2. Fallback: netsh interface show interface (~10ms)
    let output = Command::new("netsh")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["interface", "show", "interface"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let l = line.to_lowercase();
        if l.contains("wlan") || l.contains("wi-fi") || l.contains("wireless") || line.contains("无线") {
            if l.contains("enabled") || line.contains("已启用") {
                return Some(true);
            } else if l.contains("disabled") || line.contains("已禁用") {
                return Some(false);
            }
        }
    }
    None
}

/// Auto-detect Wi-Fi/WLAN adapter name dynamically via netsh (5ms, zero PowerShell)
/// 把配置里的无线电状态对齐到系统真实状态（只读 + 写配置 + 发事件，**不调用任何 set_***）。
///
/// 用途：厂商 EC 在某些机型上会自己处理无线电开关（固件已经把 Wi-Fi 关了/开了），
/// 我们只需要跟随记录，绝不能再切一次，否则会和固件打架。
/// 返回是否有状态发生变化。
pub fn sync_radio_state_from_system() -> bool {
    let mut changed = false;

    if let Some(wifi) = get_realtime_wifi_state() {
        let mut cfg = crate::system::config::load_config();
        if cfg.wifi_enabled != wifi {
            cfg.wifi_enabled = wifi;
            crate::system::config::save_config(&cfg);
            crate::event::emit_global("wifi-state-changed", serde_json::json!(wifi));
            crate::log_info!("DEVICE_SWITCH", "Wi-Fi state synced from system: {}", wifi);
            changed = true;
        }
    }

    if let Some(bt) = get_realtime_bluetooth_state() {
        let mut cfg = crate::system::config::load_config();
        if cfg.bluetooth_enabled != bt {
            cfg.bluetooth_enabled = bt;
            crate::system::config::save_config(&cfg);
            crate::event::emit_global("bluetooth-state-changed", serde_json::json!(bt));
            crate::log_info!("DEVICE_SWITCH", "Bluetooth state synced from system: {}", bt);
            changed = true;
        }
    }

    changed
}

pub fn get_or_detect_wifi_adapter(cfg: &mut crate::system::config::AppConfig) -> String {
    if !cfg.wifi_adapter_name.is_empty() {
        return cfg.wifi_adapter_name.clone();
    }

    if let Ok(output) = Command::new("netsh")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["wlan", "show", "interfaces"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Name") || trimmed.starts_with("名称") {
                    if let Some((_, name)) = trimmed.split_once(':') {
                        let adapter_name = name.trim().to_string();
                        if !adapter_name.is_empty() {
                            crate::log_info!("DEVICE_SWITCH", "Auto-discovered Wi-Fi adapter name via netsh: {}", adapter_name);
                            cfg.wifi_adapter_name = adapter_name.clone();
                            crate::system::config::save_config(cfg);
                            return adapter_name;
                        }
                    }
                }
            }
        }
    }

    "WLAN".to_string()
}
