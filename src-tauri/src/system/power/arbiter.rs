//! Centralized Power & PPM Arbiter State Machine
//!
//! Enforces Level 1.2 Specification from Sovereign Architecture Blueprint:
//! - Unifies all power state transition intents under a single arbiter
//! - Implements Boot Grace Window (10s): Ignores hardware poll noise during startup
//! - Thin & light protection: Prevents forced beast mode on pure iGPU models upon AC plug-in
//! - Debounce and single execution pipeline

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static BOOT_TIME: OnceLock<Instant> = OnceLock::new();
static ARBITER_LOCK: OnceLock<Mutex<Option<(u8, Instant)>>> = OnceLock::new();

fn get_boot_time() -> Instant {
    *BOOT_TIME.get_or_init(Instant::now)
}

fn get_arbiter_mutex() -> &'static Mutex<Option<(u8, Instant)>> {
    ARBITER_LOCK.get_or_init(|| Mutex::new(None))
}

/// All power mode change intents must declare their source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerIntentSource {
    UserExplicitUi(u8),        // Frontend UI click (Highest priority)
    BootRestore(u8),           // Startup persistent restore
    AcPluggedIn,               // DC round/adapter plugged in
    BatteryDischarged,         // DC round/adapter unplugged (running on battery)
    SystemResumeFromSleep,     // OS resumed from sleep/hibernate
    HardwarePhysicalQKey(u8),  // Chassis physical toggle key press
    HardwarePollerSync(u8),    // Hardware background poller (Observation only)
}

/// Pure evaluation function for startup grace window suppression
pub fn should_suppress_by_boot_grace(intent: PowerIntentSource, uptime: Duration) -> bool {
    if uptime < Duration::from_secs(10) {
        matches!(intent, PowerIntentSource::HardwarePollerSync(_))
    } else {
        false
    }
}

/// Pure resolution function determining the target power mode according to machine profile and intent
pub fn resolve_target_mode(
    intent: PowerIntentSource,
    is_thin_light: bool,
    cfg_power_mode: u8,
    cfg_power_mode_ac: u8,
    cfg_power_mode_battery: u8,
    unlock_battery_power_limit: bool,
    is_dc: bool,
) -> u8 {
    match intent {
        PowerIntentSource::UserExplicitUi(mode) => mode,
        PowerIntentSource::BootRestore(mode) => mode,
        PowerIntentSource::HardwarePhysicalQKey(mode) => mode,
        PowerIntentSource::HardwarePollerSync(mode) => mode,
        PowerIntentSource::AcPluggedIn => {
            if is_thin_light {
                cfg_power_mode
            } else {
                match cfg_power_mode_ac {
                    1..=4 => cfg_power_mode_ac,
                    _ => 3,
                }
            }
        }
        PowerIntentSource::BatteryDischarged => {
            if is_thin_light || unlock_battery_power_limit {
                cfg_power_mode
            } else {
                if cfg_power_mode_battery == 1 { 1 } else { 2 }
            }
        }
        PowerIntentSource::SystemResumeFromSleep => {
            if is_thin_light {
                cfg_power_mode
            } else if is_dc {
                match cfg_power_mode_ac {
                    1..=4 => cfg_power_mode_ac,
                    _ => 3,
                }
            } else {
                if cfg_power_mode_battery == 1 { 1 } else { 2 }
            }
        }
    }
}

/// Central arbiter handling all power transition requests safely
pub struct PowerArbiter;

impl PowerArbiter {
    /// Dispatches a power intent through the sovereign decision tree.
    /// Returns the effective mode applied, or None if suppressed by guards.
    pub fn dispatch(intent: PowerIntentSource) -> Option<u8> {
        let uptime = get_boot_time().elapsed();

        // 1. Boot Grace Window (10-second noise suppression)
        if should_suppress_by_boot_grace(intent, uptime) {
            if let PowerIntentSource::HardwarePollerSync(raw_ec_mode) = intent {
                crate::log_info!(
                    "POWER_ARBITER",
                    "Boot Grace Window active ({:.1}s < 10s): Suppressing uncalibrated EC poll noise (mode {})",
                    uptime.as_secs_f32(),
                    raw_ec_mode
                );
                return None;
            }
        }

        let cfg = crate::system::config::load_config();
        let is_thin_light = crate::commands::power_cmd::is_thin_and_light_or_pure_igpu();
        let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();

        // 2. Resolve target power mode based on intent source and machine topology
        let target_mode = resolve_target_mode(
            intent,
            is_thin_light,
            cfg.power_mode,
            cfg.power_mode_ac,
            cfg.power_mode_battery,
            cfg.unlock_battery_power_limit,
            is_dc,
        );

        // 3. Debounce & Cooldown barrier (600ms)
        {
            let mut lock = match get_arbiter_mutex().lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };

            if let Some((last_mode, last_time)) = *lock {
                if last_mode == target_mode && last_time.elapsed() < Duration::from_millis(600) {
                    return Some(target_mode);
                }
            }
            *lock = Some((target_mode, Instant::now()));
        }

        crate::log_info!(
            "POWER_ARBITER",
            "Sovereign power transition committed: intent={:?} -> target_mode={}",
            intent,
            target_mode
        );

        // 4. Update configuration persistence according to source
        let mut updated_cfg = cfg.clone();
        match intent {
            PowerIntentSource::UserExplicitUi(m) => {
                updated_cfg.power_mode = m;
                if is_dc {
                    updated_cfg.power_mode_ac = m;
                } else {
                    updated_cfg.power_mode_battery = m;
                }
                crate::system::config::save_config(&updated_cfg);
            }
            PowerIntentSource::AcPluggedIn => {
                if updated_cfg.power_mode != target_mode {
                    updated_cfg.power_mode = target_mode;
                    crate::system::config::save_config(&updated_cfg);
                }
            }
            PowerIntentSource::BatteryDischarged => {
                if updated_cfg.power_mode != target_mode {
                    if updated_cfg.power_mode == 3 || updated_cfg.power_mode == 4 {
                        updated_cfg.power_mode_ac = updated_cfg.power_mode;
                    }
                    updated_cfg.power_mode = target_mode;
                    crate::system::config::save_config(&updated_cfg);
                }
            }
            PowerIntentSource::HardwarePhysicalQKey(m) => {
                updated_cfg.power_mode = m;
                if is_dc {
                    updated_cfg.power_mode_ac = m;
                } else {
                    updated_cfg.power_mode_battery = m;
                }
                crate::system::config::save_config(&updated_cfg);
            }
            _ => {}
        }

        // 5. Apply full hardware and OS power states (EC hardware mode & overlay)
        super::apply_full_power_mode(target_mode);

        // 6. Synchronize Tray status
        crate::system::tray::update_tray_power_mode(target_mode);

        // 7. Trigger OSD overlay
        crate::system::osd_window::show_power_mode(target_mode);

        Some(target_mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_grace_window_suppresses_poll_noise_under_10s() {
        assert!(should_suppress_by_boot_grace(
            PowerIntentSource::HardwarePollerSync(1),
            Duration::from_secs(3)
        ));
        assert!(should_suppress_by_boot_grace(
            PowerIntentSource::HardwarePollerSync(2),
            Duration::from_millis(9999)
        ));

        // At or after 10s, poll sync is accepted
        assert!(!should_suppress_by_boot_grace(
            PowerIntentSource::HardwarePollerSync(1),
            Duration::from_secs(10)
        ));
        assert!(!should_suppress_by_boot_grace(
            PowerIntentSource::HardwarePollerSync(3),
            Duration::from_secs(15)
        ));

        // Other intents (User UI, BootRestore, AC Plug) are NEVER suppressed by grace window
        assert!(!should_suppress_by_boot_grace(
            PowerIntentSource::BootRestore(3),
            Duration::from_secs(1)
        ));
        assert!(!should_suppress_by_boot_grace(
            PowerIntentSource::UserExplicitUi(1),
            Duration::from_secs(1)
        ));
        assert!(!should_suppress_by_boot_grace(
            PowerIntentSource::AcPluggedIn,
            Duration::from_secs(2)
        ));
    }

    #[test]
    fn test_thin_and_light_protects_user_choice_on_ac_plugin() {
        // On thin & light (is_thin_light = true), plugging in AC must NOT force beast mode (3)
        let mode = resolve_target_mode(
            PowerIntentSource::AcPluggedIn,
            true,  // is_thin_light
            1,     // user is currently on Office (1)
            3,     // cfg_power_mode_ac (gaming default)
            1,     // cfg_power_mode_battery
            false,
            true,
        );
        assert_eq!(mode, 1, "Thin and light must preserve user mode (1) on AC plug-in");

        // On gaming laptop (is_thin_light = false), plugging in AC restores power_mode_ac (3)
        let gaming_mode = resolve_target_mode(
            PowerIntentSource::AcPluggedIn,
            false, // is_thin_light
            1,     // currently on Office (1)
            3,     // cfg_power_mode_ac
            1,
            false,
            true,
        );
        assert_eq!(gaming_mode, 3, "Gaming laptop must restore power_mode_ac (3) on AC plug-in");
    }

    #[test]
    fn test_battery_discharged_guards_high_power_modes() {
        // Unplugging AC without unlock_battery_power_limit drops mode to battery preference (default 2)
        let mode = resolve_target_mode(
            PowerIntentSource::BatteryDischarged,
            false,
            3,     // currently on Beast (3)
            3,
            2,     // battery preference
            false, // unlock_battery_power_limit = false
            false,
        );
        assert_eq!(mode, 2, "Must throttle to battery preference 2 when unplugged");

        // When unlock_battery_power_limit is active, preserves current mode 3
        let unlocked_mode = resolve_target_mode(
            PowerIntentSource::BatteryDischarged,
            false,
            3,     // currently on Beast (3)
            3,
            2,     // battery preference
            true,  // unlock_battery_power_limit = true
            false,
        );
        assert_eq!(unlocked_mode, 3, "Must preserve mode 3 when unlock_battery_power_limit is true");

        // Thin & light laptop (is_thin_light = true) ALWAYS preserves mode 3 on battery discharge
        let thin_mode = resolve_target_mode(
            PowerIntentSource::BatteryDischarged,
            true,  // is_thin_light = true
            3,     // Beast (3)
            3,
            2,
            false, // even without unlock_battery_power_limit
            false,
        );
        assert_eq!(thin_mode, 3, "Thin & light laptop must ALWAYS preserve Beast mode on battery");
    }

    #[test]
    fn test_user_explicit_ui_has_absolute_priority() {
        let mode = resolve_target_mode(
            PowerIntentSource::UserExplicitUi(1),
            false,
            3,
            3,
            2,
            false,
            true,
        );
        assert_eq!(mode, 1, "UserExplicitUi must always take precedence");
    }
}
