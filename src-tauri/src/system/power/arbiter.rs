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
static ARBITER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn get_boot_time() -> Instant {
    *BOOT_TIME.get_or_init(Instant::now)
}

fn get_arbiter_mutex() -> &'static Mutex<()> {
    ARBITER_LOCK.get_or_init(|| Mutex::new(()))
}

/// All power mode change intents must declare their source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerIntentSource {
    UserExplicitUi(u8),       // Frontend UI click (Highest priority)
    BootRestore(u8),          // Startup persistent restore
    AcPluggedIn,              // DC round/adapter plugged in
    BatteryDischarged,        // DC round/adapter unplugged (running on battery)
    SystemResumeFromSleep,    // OS resumed from sleep/hibernate
    HardwarePhysicalQKey(u8), // Chassis physical toggle key press
    HardwarePollerSync(u8),   // Hardware background poller (Observation only)
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
                if cfg_power_mode_battery == 1 {
                    1
                } else {
                    2
                }
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
                if cfg_power_mode_battery == 1 {
                    1
                } else {
                    2
                }
            }
        }
    }
}

/// Central arbiter handling all power transition requests safely
pub struct PowerArbiter;

impl PowerArbiter {
    pub fn dispatch(intent: PowerIntentSource) -> Result<Option<u8>, String> {
        if should_suppress_by_boot_grace(intent, get_boot_time().elapsed()) {
            return Ok(None);
        }
        if let PowerIntentSource::HardwarePollerSync(mode) = intent {
            if !(1..=4).contains(&mode) {
                return Err("Invalid observed power mode".into());
            }
            return Ok(Some(mode));
        }
        let _operation = crate::hal::power_control::MODE_OPERATION.lock();
        let _transition = get_arbiter_mutex()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let cfg = crate::system::config::load_config();
        let is_thin_light = crate::commands::power_cmd::is_thin_and_light_or_pure_igpu();
        let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
        let target_mode = resolve_target_mode(
            intent,
            is_thin_light,
            cfg.power_mode,
            cfg.power_mode_ac,
            cfg.power_mode_battery,
            cfg.unlock_battery_power_limit,
            is_dc,
        );
        // The preference is user intent: record it even when the hardware apply fails below.
        // `power_mode` itself is written by `apply_full_power_mode` only after the EC
        // acknowledged the switch, so a rejected or half-applied switch cannot make the
        // stored mode (and therefore the UI) claim something the hardware never did.
        let mut updated_cfg = cfg.clone();
        match intent {
            PowerIntentSource::UserExplicitUi(_) | PowerIntentSource::HardwarePhysicalQKey(_) => {
                if is_dc {
                    updated_cfg.power_mode_ac = target_mode;
                } else {
                    updated_cfg.power_mode_battery = target_mode;
                }
            }
            PowerIntentSource::BatteryDischarged if cfg.power_mode >= 3 => {
                updated_cfg.power_mode_ac = cfg.power_mode;
            }
            _ => {}
        }
        crate::system::config::try_save_config(&updated_cfg)?;
        super::apply_full_power_mode(target_mode)?;
        // 6. Synchronize Tray status
        crate::system::tray::update_tray_power_mode(target_mode);

        // 7. Trigger OSD overlay
        crate::system::osd_window::show_power_mode(target_mode);

        Ok(Some(target_mode))
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
            true, // is_thin_light
            1,    // user is currently on Office (1)
            3,    // cfg_power_mode_ac (gaming default)
            1,    // cfg_power_mode_battery
            false,
            true,
        );
        assert_eq!(
            mode, 1,
            "Thin and light must preserve user mode (1) on AC plug-in"
        );

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
        assert_eq!(
            gaming_mode, 3,
            "Gaming laptop must restore power_mode_ac (3) on AC plug-in"
        );
    }

    #[test]
    fn test_battery_discharged_guards_high_power_modes() {
        // Unplugging AC without unlock_battery_power_limit drops mode to battery preference (default 2)
        let mode = resolve_target_mode(
            PowerIntentSource::BatteryDischarged,
            false,
            3, // currently on Beast (3)
            3,
            2,     // battery preference
            false, // unlock_battery_power_limit = false
            false,
        );
        assert_eq!(
            mode, 2,
            "Must throttle to battery preference 2 when unplugged"
        );

        // When unlock_battery_power_limit is active, preserves current mode 3
        let unlocked_mode = resolve_target_mode(
            PowerIntentSource::BatteryDischarged,
            false,
            3, // currently on Beast (3)
            3,
            2,    // battery preference
            true, // unlock_battery_power_limit = true
            false,
        );
        assert_eq!(
            unlocked_mode, 3,
            "Must preserve mode 3 when unlock_battery_power_limit is true"
        );

        // Thin & light laptop (is_thin_light = true) ALWAYS preserves mode 3 on battery discharge
        let thin_mode = resolve_target_mode(
            PowerIntentSource::BatteryDischarged,
            true, // is_thin_light = true
            3,    // Beast (3)
            3,
            2,
            false, // even without unlock_battery_power_limit
            false,
        );
        assert_eq!(
            thin_mode, 3,
            "Thin & light laptop must ALWAYS preserve Beast mode on battery"
        );
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
