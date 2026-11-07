// OpenRevo Concrete Hardware HAL Implementation
// Bridges HAL trait with Mechrevo / Uniwill EC RAM primitives and Hardware Vault.

use super::traits::*;
use super::vault::{ec, mask};
use crate::driver::wmi_ec;

pub struct MechrevoHALImpl;

impl MechrevoHALImpl {
    pub fn new() -> Self {
        Self
    }
}

impl MechrevoHardwareHAL for MechrevoHALImpl {
    fn set_performance_mode(&self, mode: HardwarePowerMode) -> Result<(), HalError> {
        crate::system::power::ensure_power_mode_allowed(mode as u8)
            .map_err(HalError::InvalidParameter)?;
        let mode_u8 = mode as u8;
        crate::system::power_mode::apply_ec_hardware_mode(mode_u8).map_err(HalError::RegisterAccessFailed)
    }

    fn set_battery_health(&self, mode: BatteryHealthMode) -> Result<(), HalError> {
        let (limit_up, mode_byte) = match mode {
            BatteryHealthMode::LongLife => (100u8, mask::BATTERY_MODE_LONGLIFE),
            BatteryHealthMode::Balance => (80u8, mask::BATTERY_MODE_BALANCE),
            BatteryHealthMode::Station => (60u8, mask::BATTERY_MODE_STATION),
        };

        let down_limit = if limit_up > 5 { limit_up - 5 } else { limit_up };

        // 1. Write cutoff limits (Upper, Lower)
        let ok1 = wmi_ec::wmi_write_ec(ec::ADDR_BATTERY_LIMIT_UP, limit_up);
        let ok2 = wmi_ec::wmi_write_ec(ec::ADDR_BATTERY_LIMIT_DOWN, down_limit);

        // 2. Atomically update battery protection mode preserving touchpad bit
        let ok3 = wmi_ec::wmi_modify_ec(ec::ADDR_BATTERY_PROTECTION_MODE, |cur_bat_mode| {
            let preserved_touchpad = cur_bat_mode & mask::TOUCHPAD_BITMASK;
            (mode_byte & !mask::TOUCHPAD_BITMASK) | preserved_touchpad
        }).is_some();

        if ok1 && ok2 && ok3 {
            Ok(())
        } else {
            Err(HalError::RegisterAccessFailed("Failed to update battery health registers".to_string()))
        }
    }

    fn set_fan_boost(&self, active: bool) -> Result<(), HalError> {
        let success = wmi_ec::set_fan_boost_hardware(active);
        if success {
            Ok(())
        } else {
            Err(HalError::RegisterAccessFailed("Failed to toggle Fan Boost".to_string()))
        }
    }

    fn set_device_switch(&self, switch: DeviceSwitchType, enable: bool) -> Result<(), HalError> {
        let switch_name = match switch {
            DeviceSwitchType::WinKeyLock => "win_lock",
            DeviceSwitchType::Touchpad => "touchpad",
            DeviceSwitchType::CameraPower => "camera",
            DeviceSwitchType::FnLock => "fn_lock",
            DeviceSwitchType::AcPowerRecovery => "ac_recovery",
            DeviceSwitchType::PowerOffUsbCharge => "usb_charge",
        };

        crate::system::device_switches::set_switch(switch_name, enable)
            .map(|_| ())
            .map_err(|e| HalError::RegisterAccessFailed(e))
    }

    fn read_fan_rpm(&self) -> Result<(u16, u16), HalError> {
        let registers = [
            ec::ADDR_MAIN_FAN_RPM_B1,
            ec::ADDR_MAIN_FAN_RPM_B2,
            ec::ADDR_SECOND_FAN_RPM_B1,
            ec::ADDR_SECOND_FAN_RPM_B2,
        ];
        if let Some(bytes) = wmi_ec::wmi_batch_read_ec(&registers) {
            if bytes.len() == 4 {
                let cpu_rpm = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                let gpu_rpm = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                return Ok((cpu_rpm, gpu_rpm));
            }
        }
        Err(HalError::RegisterAccessFailed("Failed to batch read fan RPM".to_string()))
    }

    fn is_hardware_supported(&self) -> bool {
        self.probe()
    }
}

impl HardwareDriver for MechrevoHALImpl {
    fn name(&self) -> &'static str {
        "Tongfang-ACPI"
    }

    fn kind(&self) -> DriverKind {
        DriverKind::TongfangAcpi
    }

    fn probe(&self) -> bool {
        crate::hal::platform_probe::is_tongfang_family()
            && wmi_ec::wmi_read_ec(ec::ADDR_PROJECT_ID).is_some()
    }

    fn get_performance_mode(&self) -> Result<HardwarePowerMode, HalError> {
        let cfg = crate::system::config::load_config();
        match cfg.power_mode {
            1 => Ok(HardwarePowerMode::Office),
            2 => Ok(HardwarePowerMode::Balance),
            3 => Ok(HardwarePowerMode::Turbo),
            4 => Ok(HardwarePowerMode::Custom),
            _ => Ok(HardwarePowerMode::Balance),
        }
    }

    fn read_cpu_temp(&self) -> Result<u8, HalError> {
        let temp = crate::commands::system_cmd::get_cpu_monitor().get_temp();
        if temp > 0 && temp < 125 {
            Ok(temp as u8)
        } else {
            Ok(50)
        }
    }
}
