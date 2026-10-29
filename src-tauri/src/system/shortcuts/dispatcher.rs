// OpenRevo - Universal Shortcut Bus Action Dispatcher
// Executes hardware control, system automation, and cross-plugin events triggered by global shortcuts.

use tauri::{AppHandle, Emitter};
use crate::hal::traits::HardwarePowerMode;
use super::types::{ShortcutAction, ShortcutBinding};

#[cfg(target_os = "windows")]
extern "system" {
    fn LockWorkStation() -> windows::Win32::Foundation::BOOL;
}

static LAST_REFRESH_SWITCH: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

pub fn dispatch_action(app: AppHandle, binding: ShortcutBinding) {
    std::thread::spawn(move || {
        let osd = binding.osd;
        match binding.action {
            // 1. Toggle Fan Boost
            ShortcutAction::ToggleFanBoost => {
                let hal = crate::hal::get_hardware_driver();
                let mut cfg = crate::system::config::load_config();
                let current_boost = cfg.fan_boost;
                let next_boost = !current_boost;
                if hal.set_fan_boost(next_boost).is_ok() {
                    cfg.fan_boost = next_boost;
                    crate::system::config::save_config(&cfg);
                    crate::log_info!("SHORTCUTS", "Toggled Fan Boost: {} -> {}", current_boost, next_boost);
                    if osd {
                        crate::system::osd_window::wake_osd_if_enabled(&app);
                    }
                    let _ = app.emit("fan-boost-changed", next_boost);
                    let _ = app.emit_to("osd", "fan-boost-changed", next_boost);
                    let _ = app.emit_to("main", "fan-boost-changed", next_boost);
                }
            }

            // 2. Cycle Power Mode (1-Office -> 2-Balance -> 3-Turbo -> 1-Office)
            ShortcutAction::CyclePowerMode => {
                let hal = crate::hal::get_hardware_driver();
                let mut cfg = crate::system::config::load_config();
                let current_mode = cfg.power_mode;
                let next_mode = match current_mode {
                    1 => 2,
                    2 => 3,
                    3 => 1,
                    _ => 1,
                };
                let hw_mode = match next_mode {
                    1 => HardwarePowerMode::Office,
                    2 => HardwarePowerMode::Balance,
                    3 => HardwarePowerMode::Turbo,
                    _ => HardwarePowerMode::Balance,
                };
                if hal.set_performance_mode(hw_mode).is_ok() {
                    cfg.power_mode = next_mode;
                    crate::system::config::save_config(&cfg);
                    crate::update_tray_power_mode(next_mode);
                    crate::log_info!("SHORTCUTS", "Cycled Power Mode: {} -> {}", current_mode, next_mode);
                    if osd {
                        crate::system::osd_window::wake_osd_if_enabled(&app);
                    }
                    let _ = app.emit("physical-mode-switched", next_mode);
                    let _ = app.emit_to("osd", "physical-mode-switched", next_mode);
                    let _ = app.emit_to("main", "physical-mode-switched", next_mode);
                }
            }

            // 2.1 Set Power Mode directly
            ShortcutAction::SetPowerMode(target_mode) => {
                let hal = crate::hal::get_hardware_driver();
                let hw_mode = match target_mode {
                    1 => HardwarePowerMode::Office,
                    2 => HardwarePowerMode::Balance,
                    3 => HardwarePowerMode::Turbo,
                    4 => HardwarePowerMode::Custom,
                    _ => HardwarePowerMode::Balance,
                };
                if hal.set_performance_mode(hw_mode).is_ok() {
                    let mut cfg = crate::system::config::load_config();
                    cfg.power_mode = target_mode;
                    crate::system::config::save_config(&cfg);
                    crate::update_tray_power_mode(target_mode);
                    crate::log_info!("SHORTCUTS", "Set Power Mode: {}", target_mode);
                    if osd {
                        crate::system::osd_window::wake_osd_if_enabled(&app);
                    }
                    let _ = app.emit("physical-mode-switched", target_mode);
                    let _ = app.emit_to("osd", "physical-mode-switched", target_mode);
                    let _ = app.emit_to("main", "physical-mode-switched", target_mode);
                }
            }

            // 3. Toggle Refresh Rate (e.g. 60Hz <-> High Hz)
            ShortcutAction::ToggleRefreshRate => {
                {
                    let mut last = LAST_REFRESH_SWITCH.lock().unwrap();
                    if let Some(t) = *last {
                        if t.elapsed() < std::time::Duration::from_millis(1200) {
                            crate::log_warn!("SHORTCUTS", "Suppressed rapid ToggleRefreshRate call (< 1200ms cooldown)");
                            return;
                        }
                    }
                    *last = Some(std::time::Instant::now());
                }

                let current_hz = crate::system::display::get_refresh_rate();
                let available = crate::system::display::get_available_refresh_rates();
                if available.len() >= 2 {
                    let target_hz = if current_hz == available[0] {
                        *available.last().unwrap_or(&available[1])
                    } else {
                        available[0]
                    };
                    let ok = crate::system::display::set_refresh_rate(target_hz);
                    if ok.is_ok() {
                        crate::log_info!("SHORTCUTS", "Toggled Refresh Rate: {}Hz -> {}Hz", current_hz, target_hz);
                        if osd {
                            crate::system::osd_window::wake_osd_if_enabled(&app);
                        }
                        let _ = app.emit("refresh-rate-changed", target_hz);
                        let _ = app.emit_to("osd", "refresh-rate-changed", target_hz);
                        let _ = app.emit_to("main", "refresh-rate-changed", target_hz);
                    }
                }
            }

            // 4. Toggle Touchpad
            ShortcutAction::ToggleTouchpad => {
                let current = crate::system::config::load_config().touchpad_enabled;
                let next = !current;
                let _ = crate::system::device_switches::set_switch("touchpad", next);
                if osd {
                    crate::system::osd_window::wake_osd_if_enabled(&app);
                }
                let _ = app.emit("touchpad-state-changed", next);
                let _ = app.emit_to("osd", "touchpad-state-changed", next);
            }

            // 5. Toggle Win Key Lock
            ShortcutAction::ToggleWinLock => {
                let current = crate::system::hooks::is_win_key_locked();
                let next = !current;
                crate::system::hooks::set_win_key_locked(next);
                if osd {
                    crate::system::osd_window::wake_osd_if_enabled(&app);
                }
                let _ = app.emit("win-lock-state-changed", next);
                let _ = app.emit_to("osd", "win-lock-state-changed", next);
            }

            // 6. Toggle Fn Key Lock
            ShortcutAction::ToggleFnLock => {
                let current = crate::system::config::load_config().fn_lock_enabled;
                let next = !current;
                let _ = crate::system::device_switches::set_switch("fnlock", next);
                if osd {
                    crate::system::osd_window::wake_osd_if_enabled(&app);
                }
                let _ = app.emit("fn-lock-state-changed", next);
                let _ = app.emit_to("osd", "fn-lock-state-changed", next);
            }

            // 7. Cycle Keyboard Brightness
            ShortcutAction::CycleKeyboardBrightness => {
                crate::driver::ite_lighting::cycle_keyboard_brightness(Some(&app));
            }

            // 8. Trigger OSD Preview
            ShortcutAction::TriggerOsdPreview => {
                crate::system::osd_window::present_osd_window(&app);
                let _ = app.emit("trigger-osd-preview", ());
                let _ = app.emit_to("osd", "trigger-osd-preview", ());
            }

            // 9. Volume Controls (Simulated Win32 Media Keys)
            ShortcutAction::VolumeMute => {
                simulate_win32_key(0xAD); // VK_VOLUME_MUTE
            }
            ShortcutAction::VolumeDown => {
                simulate_win32_key(0xAE); // VK_VOLUME_DOWN
            }
            ShortcutAction::VolumeUp => {
                simulate_win32_key(0xAF); // VK_VOLUME_UP
            }

            // 10. Turn Monitor Off (Low Power Sleep)
            ShortcutAction::MonitorOff => {
                #[cfg(target_os = "windows")]
                unsafe {
                    use windows::Win32::Foundation::{LPARAM, WPARAM};
                    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, HWND_BROADCAST, WM_SYSCOMMAND};
                    const SC_MONITORPOWER: usize = 0xF170;
                    let _ = PostMessageW(HWND_BROADCAST, WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER), LPARAM(2));
                }
            }

            // 11. Lock Workstation
            ShortcutAction::LockScreen => {
                #[cfg(target_os = "windows")]
                unsafe {
                    let _ = LockWorkStation();
                }
            }

            // 12. Launch External Application or Script
            ShortcutAction::LaunchApp { path, args } => {
                if !path.trim().is_empty() {
                    let mut cmd = std::process::Command::new(&path);
                    cmd.args(&args);
                    if let Err(e) = cmd.spawn() {
                        crate::log_error!("SHORTCUTS", "Failed to launch application '{}': {}", path, e);
                    }
                }
            }

            // 13. Custom Cross-Plugin Event Bus
            ShortcutAction::PluginEvent { event_name, payload } => {
                let _ = app.emit(&event_name, &payload);
                if let Some(ref p_id) = binding.plugin_id {
                    let plugin_topic = format!("plugin:{}:shortcut", p_id);
                    let _ = app.emit(&plugin_topic, serde_json::json!({
                        "shortcut_id": binding.id,
                        "name": binding.name,
                        "combo": binding.combo,
                        "payload": payload,
                    }));
                }
            }
        }
    });
}

fn simulate_win32_key(vk: u8) {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP};
        keybd_event(vk, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
}
