// OpenRevo Declarative Hardware Driver Implementation
// Binds high-level HAL commands to declarative ModelManifest actions & channels.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use super::model_manifest::*;
use super::wmi_mailbox::{extract_value_from_buffer, invoke_wmi_mailbox};
use crate::hal::traits::*;

pub struct DeclarativeHardwareDriver {
    manifest: Arc<ModelManifest>,
    last_power_mode: AtomicU8,
}

impl DeclarativeHardwareDriver {
    pub fn new(manifest: ModelManifest) -> Self {
        let saved_mode = crate::system::config::load_config().power_mode;
        let initial_mode = if (1..=4).contains(&saved_mode) {
            saved_mode
        } else if manifest.channels.values().any(|c| matches!(c, ChannelDef::WmiIp3)) {
            if let Some(raw) = crate::driver::wmi_ip3::ip3_get_power_mode() {
                match raw {
                    2 => 1, // Office / Silence
                    0 => 2, // Balance
                    1 => 3, // Turbo / Rage
                    _ => 2,
                }
            } else {
                2
            }
        } else {
            2
        };

        Self {
            manifest: Arc::new(manifest),
            last_power_mode: AtomicU8::new(initial_mode),
        }
    }

    pub fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }

    fn execute_buffer_action(&self, in_bytes: &[u8]) -> Result<Vec<u8>, HalError> {
        // Find default wmi_mailbox channel
        let channel = self
            .manifest
            .channels
            .values()
            .find_map(|c| match c {
                ChannelDef::WmiMailbox {
                    namespace,
                    class_name,
                    method_name,
                    buffer_sizes,
                } => Some((namespace, class_name, method_name, buffer_sizes)),
                _ => None,
            })
            .ok_or_else(|| {
                HalError::DriverUnavailable("No WmiMailbox channel configured in manifest".to_string())
            })?;

        invoke_wmi_mailbox(
            channel.0,
            channel.1,
            channel.2,
            in_bytes,
            channel.3.in_size,
            channel.3.out_size,
        )
        .map_err(|e| HalError::RegisterAccessFailed(e))
    }
}

impl MechrevoHardwareHAL for DeclarativeHardwareDriver {
    fn set_performance_mode(&self, mode: HardwarePowerMode) -> Result<(), HalError> {
        crate::system::power::ensure_power_mode_allowed(mode as u8).map_err(HalError::InvalidParameter)?;
        let mode_key = match mode {
            HardwarePowerMode::Office => "office",
            HardwarePowerMode::Balance => "balance",
            HardwarePowerMode::Turbo => "turbo",
            HardwarePowerMode::Custom => "turbo",
        };

        if let Some(action) = self.manifest.actions.get("set_power_mode") {
            let bytes_opt = match action {
                ActionDef::ModeMap(map) => map.get(mode_key).cloned(),
                ActionDef::BufferInvoke { input, .. } => Some(input.clone()),
                ActionDef::RawBytes(bytes) => Some(bytes.clone()),
            };

            if let Some(val_bytes) = bytes_opt {
                let u8_bytes: Vec<u8> = val_bytes
                    .iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();

                if !u8_bytes.is_empty() {
                    for channel in self.manifest.channels.values() {
                        match channel {
                            ChannelDef::WmiIp3 => {
                                if let Some(&raw_mode) = u8_bytes.first() {
                                    if crate::driver::wmi_ip3::ip3_set_power_mode(raw_mode) {
                                        self.last_power_mode.store(mode as u8, Ordering::SeqCst);
                                        crate::log_info!(
                                            "DECLARATIVE_HAL",
                                            "Applied performance mode {:?} via WmiIp3 channel (raw_mode={})",
                                            mode,
                                            raw_mode
                                        );
                                        return Ok(());
                                    } else {
                                        return Err(HalError::RegisterAccessFailed("WmiIp3 SetPowerMode failed".into()));
                                    }
                                }
                            }
                            ChannelDef::WmiMailbox { .. } => {
                                self.execute_buffer_action(&u8_bytes)?;
                                self.last_power_mode.store(mode as u8, Ordering::SeqCst);
                                crate::log_info!(
                                    "DECLARATIVE_HAL",
                                    "Applied performance mode {:?} via WmiMailbox channel ({:?})",
                                    mode,
                                    u8_bytes
                                );
                                return Ok(());
                            }
                            ChannelDef::AcpiIoctl { .. } => {
                                let real = crate::hal::driver_impl::MechrevoHALImpl::new();
                                real.set_performance_mode(mode)?;
                                self.last_power_mode.store(mode as u8, Ordering::SeqCst);
                                crate::log_info!(
                                    "DECLARATIVE_HAL",
                                    "Applied performance mode {:?} via Native Tongfang ACPI IOCTL channel",
                                    mode
                                );
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        Err(HalError::UnsupportedModel(format!(
            "set_power_mode for {:?} not defined in driver manifest",
            mode
        )))
    }

    fn set_battery_health(&self, mode: BatteryHealthMode) -> Result<(), HalError> {
        if !self.manifest.capabilities.battery_charge_limit {
            return Err(HalError::UnsupportedModel(
                "Battery charge limit not supported on this model".to_string(),
            ));
        }

        // Native Tongfang ACPI channel support: bridge to MechrevoHALImpl
        if self.manifest.channels.values().any(|c| matches!(c, ChannelDef::AcpiIoctl { .. }))
            || crate::driver::wmi_ec::is_acpi_driver_available()
        {
            let real = crate::hal::driver_impl::MechrevoHALImpl::new();
            real.set_battery_health(mode)?;
            crate::log_info!("DECLARATIVE_HAL", "Applied battery health {:?} via Native Tongfang ACPI channel", mode);
            return Ok(());
        }

        let limit_val = match mode {
            BatteryHealthMode::LongLife => 100u8,
            BatteryHealthMode::Balance => 80u8,
            BatteryHealthMode::Station => 60u8,
        };

        // Declarative battery protection command if configured in actions
        if let Some(ActionDef::BufferInvoke { input, .. }) =
            self.manifest.actions.get("set_battery_limit")
        {
            let u8_bytes: Vec<u8> = input
                .iter()
                .map(|v| {
                    if v.as_str() == Some("{val}") {
                        limit_val
                    } else {
                        v.as_u64().unwrap_or(0) as u8
                    }
                })
                .collect();

            let _ = self.execute_buffer_action(&u8_bytes)?;
            return Ok(());
        }

        Ok(())
    }

    fn set_fan_boost(&self, active: bool) -> Result<(), HalError> {
        let is_direct_clamp = self.manifest.capabilities.fan_boost_mode.as_deref() == Some("direct_rpm_clamp")
            || self.manifest.capabilities.fan_control_mode.as_deref() == Some("direct_rpm_clamp");

        if is_direct_clamp {
            let max_rpm = self.manifest.capabilities.max_fan_rpm.unwrap_or(5800);
            if active {
                let rpm_u16 = max_rpm as u16;
                crate::driver::bld_driver_mgr::set_bld_fan_manual_clamp(rpm_u16, rpm_u16);
                crate::log_info!("BLD_FAN", "Physical Fan Boost locked via 1Hz arbiter: {} RPM", max_rpm);
            } else {
                crate::driver::bld_driver_mgr::restore_bld_fan_auto();
                crate::log_info!("BLD_FAN", "Physical Fan Boost released to BIOS thermal curve via 1Hz arbiter");
            }
            return Ok(());
        }

        // Hardware toggle / Native Tongfang ACPI channel support
        if self.manifest.capabilities.fan_boost_mode.as_deref() == Some("hardware_toggle")
            || self.manifest.channels.values().any(|c| matches!(c, ChannelDef::AcpiIoctl { .. }))
        {
            let real = crate::hal::driver_impl::MechrevoHALImpl::new();
            real.set_fan_boost(active)?;
            crate::log_info!("DECLARATIVE_HAL", "Set physical Fan Boost to {} via Native Tongfang ACPI channel", active);
            return Ok(());
        }

        let action_name = if active {
            "set_fan_boost_on"
        } else {
            "set_fan_boost_off"
        };
        if let Some(ActionDef::BufferInvoke { input, .. }) = self.manifest.actions.get(action_name) {
            let u8_bytes: Vec<u8> = input
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            let _ = self.execute_buffer_action(&u8_bytes);
            return Ok(());
        }

        if is_direct_clamp {
            return Ok(());
        }

        Err(HalError::UnsupportedModel("Fan boost not supported in manifest".into()))
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
        let action_key = format!("switch_{}_{}", switch_name, if enable { "on" } else { "off" });
        if let Some(ActionDef::BufferInvoke { input, .. }) = self.manifest.actions.get(&action_key) {
            let u8_bytes: Vec<u8> = input
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            let _ = self.execute_buffer_action(&u8_bytes)?;
            return Ok(());
        }
        // Fallback to Win32 software switch
        crate::system::device_switches::set_switch(switch_name, enable)
            .map(|_| ())
            .map_err(|e| HalError::RegisterAccessFailed(e))
    }

    fn read_fan_rpm(&self) -> Result<(u16, u16), HalError> {
        if let Some(ActionDef::BufferInvoke { input, parse, .. }) =
            self.manifest.actions.get("read_fan_rpm")
        {
            let u8_bytes: Vec<u8> = input
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            let out_buf = self.execute_buffer_action(&u8_bytes)?;

            if let Some(parse_rules) = parse {
                let cpu_rpm = parse_rules
                    .get("cpu_rpm")
                    .and_then(|rule| extract_value_from_buffer(&out_buf, rule.offset, &rule.value_type))
                    .unwrap_or(0) as u16;

                let gpu_rpm = parse_rules
                    .get("gpu_rpm")
                    .and_then(|rule| extract_value_from_buffer(&out_buf, rule.offset, &rule.value_type))
                    .unwrap_or(0) as u16;

                return Ok((cpu_rpm, gpu_rpm));
            }
        }

        // Native Tongfang ACPI channel fallback: directly query hardware EC RPM registers
        if self.manifest.channels.values().any(|c| matches!(c, ChannelDef::AcpiIoctl { .. }))
            || crate::driver::wmi_ec::is_acpi_driver_available()
        {
            let real = crate::hal::driver_impl::MechrevoHALImpl::new();
            return real.read_fan_rpm();
        }

        Err(HalError::UnsupportedModel("read_fan_rpm not defined in manifest".into()))
    }

    fn set_monochrome_backlight(&self, level: u8) -> Result<(), HalError> {
        for channel in self.manifest.channels.values() {
            match channel {
                ChannelDef::WmiIp3 => {
                    let ok = crate::driver::wmi_ip3::ip3_set_keyboard_backlight_level(level);
                    return if ok {
                        Ok(())
                    } else {
                        Err(HalError::RegisterAccessFailed("WmiIp3 backlight write failed".into()))
                    };
                }
                ChannelDef::WmiMailbox { .. } => {
                    if let Some(ActionDef::BufferInvoke { input, .. }) =
                        self.manifest.actions.get("set_keyboard_backlight")
                    {
                        let u8_bytes: Vec<u8> = input
                            .iter()
                            .map(|v| {
                                if v.as_str() == Some("{val}") {
                                    level
                                } else {
                                    v.as_u64().unwrap_or(0) as u8
                                }
                            })
                            .collect();
                        let _ = self.execute_buffer_action(&u8_bytes)?;
                        crate::log_info!("DECLARATIVE_HAL", "Set monochrome backlight to level {} via manifest action", level);
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        Err(HalError::UnsupportedModel(
            "set_keyboard_backlight not supported by configured channels".into(),
        ))
    }

    fn get_monochrome_backlight(&self) -> Option<u8> {
        for channel in self.manifest.channels.values() {
            if matches!(channel, ChannelDef::WmiIp3) {
                return crate::driver::wmi_ip3::ip3_get_keyboard_backlight_level();
            }
        }
        let cfg = crate::system::config::load_config();
        if cfg.lighting.kb_enabled {
            Some(cfg.lighting.kb_brightness.min(3).max(1))
        } else {
            Some(0)
        }
    }

    fn is_hardware_supported(&self) -> bool {
        true
    }
}

impl HardwareDriver for DeclarativeHardwareDriver {
    fn name(&self) -> &'static str {
        "Declarative-WMI"
    }

    fn kind(&self) -> DriverKind {
        DriverKind::GenericWmi
    }

    fn probe(&self) -> bool {
        true
    }

    fn get_performance_mode(&self) -> Result<HardwarePowerMode, HalError> {
        for channel in self.manifest.channels.values() {
            if matches!(channel, ChannelDef::WmiIp3) {
                if let Some(raw) = crate::driver::wmi_ip3::ip3_get_power_mode() {
                    let hw_mode = match raw {
                        2 => HardwarePowerMode::Office,
                        0 => HardwarePowerMode::Balance,
                        1 => HardwarePowerMode::Turbo,
                        _ => HardwarePowerMode::Balance,
                    };
                    self.last_power_mode.store(hw_mode as u8, Ordering::SeqCst);
                    return Ok(hw_mode);
                }
            }
        }

        // Generic get_power_mode buffer action (e.g. Baolongda MIFS, Xingyao)
        if let Some(ActionDef::BufferInvoke { input, parse, mapping, .. }) = self.manifest.actions.get("get_power_mode") {
            let u8_bytes: Vec<u8> = input
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            if let Ok(out_buf) = self.execute_buffer_action(&u8_bytes) {
                if let Some(parse_rules) = parse {
                    if let Some(rule) = parse_rules.get("mode_code") {
                        if let Some(raw_val) = extract_value_from_buffer(&out_buf, rule.offset, &rule.value_type) {
                            let hw_mode = if let Some(map) = mapping {
                                let key = raw_val.to_string();
                                match map.get(&key).map(|s| s.as_str()) {
                                    Some("office") | Some("silence") => HardwarePowerMode::Office,
                                    Some("balance") => HardwarePowerMode::Balance,
                                    Some("turbo") | Some("rage") => HardwarePowerMode::Turbo,
                                    _ => HardwarePowerMode::Balance,
                                }
                            } else {
                                match raw_val {
                                    1 => HardwarePowerMode::Office,
                                    2 => HardwarePowerMode::Balance,
                                    3 => HardwarePowerMode::Turbo,
                                    _ => HardwarePowerMode::Balance,
                                }
                            };
                            self.last_power_mode.store(hw_mode as u8, Ordering::SeqCst);
                            return Ok(hw_mode);
                        }
                    }
                }
            }
        }

        let val = self.last_power_mode.load(Ordering::SeqCst);
        match val {
            1 => Ok(HardwarePowerMode::Office),
            2 => Ok(HardwarePowerMode::Balance),
            3 => Ok(HardwarePowerMode::Turbo),
            _ => Ok(HardwarePowerMode::Balance),
        }
    }

    fn read_cpu_temp(&self) -> Result<u8, HalError> {
        if let Some(ActionDef::BufferInvoke { input, parse, .. }) = self.manifest.actions.get("read_cpu_temp") {
            let u8_bytes: Vec<u8> = input
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            if let Ok(out_buf) = self.execute_buffer_action(&u8_bytes) {
                if let Some(parse_rules) = parse {
                    if let Some(rule) = parse_rules.get("cpu_temp") {
                        if let Some(val) = extract_value_from_buffer(&out_buf, rule.offset, &rule.value_type) {
                            if val > 0 && val < 125 {
                                return Ok(val as u8);
                            }
                        }
                    }
                }
            }
        }

        let temp = crate::commands::system_cmd::get_cpu_monitor().get_temp();
        if temp > 0 && temp < 125 {
            Ok(temp as u8)
        } else {
            Ok(48)
        }
    }

    fn get_gpu_mux(&self) -> Result<String, HalError> {
        if let Some(ActionDef::BufferInvoke { input, parse, mapping, .. }) = self.manifest.actions.get("get_gpu_mux") {
            let u8_bytes: Vec<u8> = input
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            let out_buf = self.execute_buffer_action(&u8_bytes)?;
            if let Some(parse_rules) = parse {
                if let Some(rule) = parse_rules.get("mode_code") {
                    if let Some(raw_val) = extract_value_from_buffer(&out_buf, rule.offset, &rule.value_type) {
                        if let Some(map) = mapping {
                            let key = raw_val.to_string();
                            if let Some(mapped) = map.get(&key) {
                                return Ok(mapped.clone());
                            }
                        }
                        return match raw_val {
                            0 => Ok("hybrid".to_string()),
                            1 => Ok("dgpu".to_string()),
                            _ => Ok("unknown".to_string()),
                        };
                    }
                }
            }
        }
        Err(HalError::UnsupportedModel("get_gpu_mux not defined on this manifest".into()))
    }

    fn set_gpu_mux(&self, discrete_only: bool) -> Result<(), HalError> {
        let action_name = if discrete_only { "set_gpu_mux_discrete" } else { "set_gpu_mux_hybrid" };
        if let Some(ActionDef::BufferInvoke { input, .. }) = self.manifest.actions.get(action_name) {
            let u8_bytes: Vec<u8> = input
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            let _ = self.execute_buffer_action(&u8_bytes)?;
            crate::log_info!("DECLARATIVE_HAL", "Executed GPU MUX switch (discrete_only={}) via manifest", discrete_only);
            return Ok(());
        }
        Err(HalError::UnsupportedModel("GPU MUX not implemented on this manifest".into()))
    }

    fn set_keyboard_rgb(&self, r: u8, g: u8, b: u8) -> Result<(), HalError> {
        if let Some(ActionDef::BufferInvoke { input, .. }) = self.manifest.actions.get("set_keyboard_rgb") {
            let u8_bytes: Vec<u8> = input
                .iter()
                .map(|v| {
                    match v.as_str() {
                        Some("{r}") => r,
                        Some("{g}") => g,
                        Some("{b}") => b,
                        _ => v.as_u64().unwrap_or(0) as u8,
                    }
                })
                .collect();
            let _ = self.execute_buffer_action(&u8_bytes)?;
            crate::log_info!("DECLARATIVE_HAL", "Applied keyboard RGB ({}, {}, {}) via manifest", r, g, b);
            return Ok(());
        }

        Err(HalError::UnsupportedModel(
            "RGB lighting not supported on this model manifest".into(),
        ))
    }

    fn set_keyboard_effect(&self, effect: u8, r: u8, g: u8, b: u8) -> Result<(), HalError> {
        let is_bld = self.manifest.id.contains("bld");
        match effect {
            0 => {
                // Turn off backlight
                if is_bld {
                    crate::driver::bld_driver_mgr::stop_bld_rainbow();
                }
                if let Some(ActionDef::BufferInvoke { input, .. }) = self.manifest.actions.get("set_keyboard_mode_off") {
                    let u8_bytes: Vec<u8> = input.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect();
                    let _ = self.execute_buffer_action(&u8_bytes)?;
                }
                let _ = self.set_keyboard_rgb(0, 0, 0);
                Ok(())
            }
            2 => {
                // Auto cyclic / rainbow mode
                if is_bld {
                    crate::driver::bld_driver_mgr::start_bld_rainbow(Some((r, g, b)));
                    crate::log_info!("DECLARATIVE_HAL", "Activated Baolongda 10 FPS Hue Cycle Rainbow Gradient");
                    return Ok(());
                }
                if let Some(ActionDef::BufferInvoke { input, .. }) = self.manifest.actions.get("set_keyboard_mode_cyclic") {
                    let u8_bytes: Vec<u8> = input.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect();
                    let _ = self.execute_buffer_action(&u8_bytes)?;
                    crate::log_info!("DECLARATIVE_HAL", "Activated Rainbow AutoCyclic keyboard mode via manifest action");
                    return Ok(());
                }
                // Fallback to RGB if cyclic not defined
                self.set_keyboard_rgb(r, g, b)
            }
            _ => {
                // Fixed static color mode (Mode 2 in Baolongda WMI)
                if is_bld {
                    crate::driver::bld_driver_mgr::stop_bld_rainbow();
                }
                if let Some(ActionDef::BufferInvoke { input, .. }) = self.manifest.actions.get("set_keyboard_mode_fixed") {
                    let u8_bytes: Vec<u8> = input.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect();
                    let _ = self.execute_buffer_action(&u8_bytes)?;
                }
                self.set_keyboard_rgb(r, g, b)
            }
        }
    }
}
