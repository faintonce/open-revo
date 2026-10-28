// OpenRevo - Lighting Idle Watcher & Keyboard Brightness Hotkey Handlers
// Manages power saving sleep timeouts, instant wake-up, and keyboard brightness steps.

use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use crate::driver::hid_ffi::get_system_idle_ms;
use crate::driver::wmi_ec::ADDR_BACKLIGHT_WHITE;
use super::{get_global_lighting, save_lighting_state, IteLightingManager};

pub fn start_idle_watcher(mgr_ref: &Arc<IteLightingManager>) {
    let mgr = Arc::clone(mgr_ref);
    std::thread::Builder::new()
        .name("ite_idle_watcher".to_string())
        .spawn(move || {
            let mut watchdog_retries: u32 = 0;
            // Give startup initialization (power::init_from_saved_config and USB enumeration)
            // settling time before runtime health monitoring begins.
            std::thread::sleep(Duration::from_millis(2500));

            loop {
                std::thread::sleep(Duration::from_millis(500));

                let sleep_minutes = {
                    let st = mgr.state.lock();
                    st.sleep_minutes
                };

                // If sleep timer is disabled (0), wake up immediately if currently sleeping
                if sleep_minutes == 0 {
                    if mgr.is_sleeping.swap(false, std::sync::atomic::Ordering::SeqCst) {
                        crate::driver::ite_streamer::get_global_streamer().set_unsupported(false);
                        watchdog_retries = 0;
                        mgr.restore_all();
                    } else {
                        check_streamer_watchdog(&mgr, &mut watchdog_retries);
                    }
                    continue;
                }

                let idle_ms = get_system_idle_ms();
                let timeout_ms = (sleep_minutes as u64) * 60 * 1000;

                if idle_ms >= timeout_ms {
                    // User idle timeout reached -> turn off all lights if not already sleeping
                    if !mgr.is_sleeping.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        mgr.turn_off_all();
                    }
                } else {
                    // User has recent input activity (pressed any key or moved mouse)
                    // If currently sleeping, instantly wake up & restore preset effects!
                    if mgr.is_sleeping.swap(false, std::sync::atomic::Ordering::SeqCst) {
                        crate::driver::ite_streamer::get_global_streamer().set_unsupported(false);
                        watchdog_retries = 0;
                        mgr.restore_all();
                    } else {
                        check_streamer_watchdog(&mgr, &mut watchdog_retries);
                    }
                }
            }
        })
        .expect("Failed to spawn ite_idle_watcher thread");
}

/// Watchdog: If awake and in streamer mode, ensure streamer didn't abruptly exit from USB glitch or thread fault.
fn check_streamer_watchdog(mgr: &IteLightingManager, watchdog_retries: &mut u32) {
    if !mgr.has_keyboard_device() {
        return;
    }
    let cfg = crate::system::config::load_config();
    if cfg.lighting.kb_engine != "streamer" {
        return;
    }
    let streamer = crate::driver::ite_streamer::get_global_streamer();
    let params = streamer.get_params();
    if params.enabled && params.brightness_pct > 0 && streamer.was_abruptly_stopped() && !streamer.is_unsupported() {
        if *watchdog_retries < 3 {
            *watchdog_retries += 1;
            crate::log_warn!(
                "STREAMER_WATCHDOG",
                "Streamer thread stopped unexpectedly while active (attempt {}/3). Auto-reviving...",
                *watchdog_retries
            );
            streamer.start();
        } else if *watchdog_retries == 3 {
            *watchdog_retries = 4;
            crate::log_info!(
                "STREAMER_WATCHDOG",
                "Streamer thread inactive after 3 attempts. Safely falling back to Hardware MCU."
            );
            streamer.set_unsupported(true);
            streamer.apply_hardware(
                cfg.lighting.kb_enabled,
                cfg.lighting.kb_effect,
                cfg.lighting.kb_brightness,
                cfg.lighting.kb_speed,
                cfg.lighting.kb_direction,
                &cfg.lighting.kb_color,
            );
        }
    } else if streamer.is_running() {
        *watchdog_retries = 0;
    }
}

/// Adjust keyboard brightness by +/- 1 step, synchronized across hardware & streamer engines.
pub fn adjust_keyboard_brightness_step(step_up: bool, app_handle: Option<&AppHandle>) -> u8 {
    let current_level = {
        let mgr = get_global_lighting();
        let st = mgr.state.lock();
        if !st.kb_enabled {
            0
        } else {
            st.kb_brightness
        }
    };
    let contract = get_global_lighting().active_contract();
    let is_monochrome = contract.keyboard_type == "MonochromeWhite";
    let max_level = if is_monochrome { 2 } else { 4 };

    let new_level = if step_up {
        (current_level + 1).min(max_level)
    } else {
        current_level.saturating_sub(1)
    };
    set_keyboard_brightness_level(new_level, app_handle)
}

/// Cycle keyboard brightness (0 -> 1 -> 2 -> 0 for monochrome, 0..=4 for RGB).
pub fn cycle_keyboard_brightness(app_handle: Option<&AppHandle>) -> u8 {
    let current_level = {
        let mgr = get_global_lighting();
        let st = mgr.state.lock();
        if !st.kb_enabled {
            0
        } else {
            st.kb_brightness
        }
    };
    let contract = get_global_lighting().active_contract();
    let is_monochrome = contract.keyboard_type == "MonochromeWhite";
    let cycle_mod = if is_monochrome { 3 } else { 5 };

    let new_level = (current_level + 1) % cycle_mod;
    set_keyboard_brightness_level(new_level, app_handle)
}

/// Set keyboard brightness level directly (0..=4 for gaming RGB, 0..=2 for monochrome white).
pub fn set_keyboard_brightness_level(level: u8, app_handle: Option<&AppHandle>) -> u8 {
    let mgr = get_global_lighting();
    let contract = mgr.active_contract();
    let is_monochrome = contract.keyboard_type == "MonochromeWhite";
    let max_level = if is_monochrome { 2 } else { 4 };
    let target_level = level.min(max_level);

    let st_snapshot = {
        let mut st = mgr.state.lock();
        st.kb_brightness = target_level;
        st.kb_enabled = target_level > 0;
        st.clone()
    };
    save_lighting_state(&st_snapshot);

    // 0. Bridge to monochrome white backlight for Thin & Light models (IP3 / WMI)
    if is_monochrome {
        mgr.apply_monochrome_white_ec(target_level > 0, target_level);
        if let Some(cur_trigger_byte) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, cur_trigger_byte | 0x20);
        }
    }

    let raw_kb_support = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_SUPPORT_BYTE2).unwrap_or(0);
    let is_sz = (raw_kb_support & 0x04) == 0x04 || contract.keyboard_type == "SingleZone";

    // 1. EC ADDR_BACKLIGHT_WHITE Hardware Power Guard & direct brightness bits
    if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE) {
        let mut new_bl = val;
        if target_level > 0 {
            new_bl &= !0x02; // Bit 1 cleared = Enabled
            new_bl &= 0x0D; // Clear high 4 bits & bit 1
            let high_nibble = if is_sz {
                match target_level {
                    1 => 0x30,
                    2 => 0x50,
                    3 => 0x70,
                    4 | _ => 0x90,
                }
            } else if is_monochrome {
                match target_level {
                    1 => 0x20, // 50%
                    2 | _ => 0x40, // 100%
                }
            } else {
                match target_level {
                    1 => 0x30,
                    2 => 0x50,
                    3 | _ => 0x90,
                }
            };
            new_bl |= high_nibble;
        } else {
            new_bl = (new_bl & 0x0D) | 0x12; // Bit 1 set = Disabled / Off, Bit 4 = 1
        }
        let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, new_bl);
    }

    // 2. Dual-Engine synchronization (BetterRGB Streamer vs Factory Hardware MCU vs Four-Zone vs SingleZone)
    if mgr.has_perkey_keyboard_device() {
        let streamer = crate::driver::ite_streamer::get_global_streamer();
        let is_streamer = st_snapshot.kb_engine == "streamer";
        if is_streamer {
            let brightness_pct = match target_level {
                0 => 0,
                1 => 20,
                2 => 45,
                3 => 75,
                4 => 100,
                _ => 75,
            };
            let mut sp = streamer.get_params();
            sp.enabled = target_level > 0;
            sp.brightness_pct = brightness_pct;
            streamer.update_params(sp);
            crate::log_info!("WMI_KEYBOARD", "Streamer brightness updated to: level {} ({}%)", target_level, brightness_pct);
        } else {
            streamer.apply_hardware(
                target_level > 0,
                st_snapshot.kb_effect,
                target_level,
                st_snapshot.kb_speed,
                st_snapshot.kb_direction,
                &st_snapshot.kb_color,
            );
            crate::log_info!("WMI_KEYBOARD", "Hardware MCU brightness updated to: level {}", target_level);
        }
    } else if mgr.has_four_zone_keyboard_device() {
        mgr.apply_four_zone_keyboard(
            target_level > 0,
            st_snapshot.kb_effect,
            target_level,
            st_snapshot.kb_speed,
            st_snapshot.kb_direction,
            &st_snapshot.kb_color,
            Some(&st_snapshot.kb_zones),
        );
        crate::log_info!("WMI_KEYBOARD", "Four-zone keyboard brightness updated to: level {}", target_level);
    } else if is_sz {
        if let Some(cur_fan_isolated) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FAN_ISOLATED) {
            if (cur_fan_isolated & 0x07) != 0 {
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_FAN_ISOLATED, cur_fan_isolated & 0xF8);
            }
        }
        let cur_trigger_byte = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
        if target_level == 0 {
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_RED, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_GREEN, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_BLUE, 0);
            let next_trigger = (cur_trigger_byte & !0x80) | 0x20;
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, next_trigger);
        } else if st_snapshot.kb_effect == 2 {
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_RED, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_GREEN, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_BLUE, 0);
            let next_trigger = cur_trigger_byte | 0x80;
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, next_trigger);
        } else {
            let (r, g, b) = crate::driver::lighting::types::parse_hex_color(&st_snapshot.kb_color);
            let (r_val, g_val, b_val) = if r == 255 && g == 255 && b == 255 {
                (50u8, 40u8, 40u8)
            } else {
                let r_l = if r == 0 { 0 } else { ((r as u32 * 50) / 255).clamp(1, 50) as u8 };
                let g_l = if g == 0 { 0 } else { ((g as u32 * 50) / 255).clamp(1, 50) as u8 };
                let b_l = if b == 0 { 0 } else { ((b as u32 * 50) / 255).clamp(1, 50) as u8 };
                (r_l, g_l, b_l)
            };
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_RED, r_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_GREEN, g_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_RGB_BLUE, b_val);
            let next_trigger = (cur_trigger_byte & !0x80) | 0x20;
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, next_trigger);
        }
    }

    // 3. Emit Tauri event to frontend
    if let Some(handle) = app_handle {
        crate::system::osd_window::show_keyboard_brightness(target_level);
        crate::system::osd_window::wake_osd_if_enabled(handle);
        let _ = handle.emit("keyboard-brightness-changed", target_level);
        let _ = handle.emit_to("main", "keyboard-brightness-changed", target_level);
        let _ = handle.emit_to("osd", "keyboard-brightness-changed", target_level);
    }

    target_level
}
