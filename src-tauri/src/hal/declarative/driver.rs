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

#[cfg(test)]
mod contract_tests {
    use super::*;

    fn ptl_manifest() -> ModelManifest {
        serde_json::from_str(include_str!("../../../resources/drivers/xingyao_14_p916f_ptl.json")).unwrap()
    }

    fn driver(manifest: ModelManifest) -> DeclarativeHardwareDriver {
        DeclarativeHardwareDriver { manifest: Arc::new(manifest), last_power_mode: AtomicU8::new(2) }
    }

    #[test]
    fn ptl_unsupported_writes_fail_before_transport_access() {
        let mut manifest = ptl_manifest();
        manifest.channels.remove("pm2");
        manifest.capabilities.battery_charge_limit = false;
        manifest.capabilities.power_modes.retain(|mode| mode != "custom");
        let driver = driver(manifest);
        assert!(matches!(driver.set_battery_health(BatteryHealthMode::Balance), Err(HalError::UnsupportedModel(_))));
        assert!(matches!(driver.set_fan_boost(true), Err(HalError::UnsupportedModel(_))));
        assert!(matches!(driver.set_performance_mode(HardwarePowerMode::Custom), Err(HalError::UnsupportedModel(_))));
    }

    #[test]
    fn missing_battery_action_never_reports_success() {
        let mut manifest = ptl_manifest();
        manifest.channels.remove("pm2");
        manifest.capabilities.battery_charge_limit = true;
        assert!(matches!(driver(manifest).set_battery_health(BatteryHealthMode::Balance), Err(HalError::UnsupportedModel(_))));
    }

    #[test]
    fn fan_query_honors_its_declared_channel() {
        let mut manifest = ptl_manifest();
        if let ActionDef::BufferInvoke { channel, .. } = manifest.actions.get_mut("diagnostic_cpu_fan_rpm").unwrap() {
            *channel = Some("missing_channel".into());
        }
        assert!(matches!(driver(manifest).read_fan_channel("diagnostic_cpu_fan_rpm"), Err(HalError::DriverUnavailable(_))));
    }

    #[test]
    fn invalid_fan_query_byte_is_rejected_before_transport_access() {
        let mut manifest = ptl_manifest();
        if let ActionDef::BufferInvoke { input, .. } = manifest.actions.get_mut("diagnostic_cpu_fan_rpm").unwrap() {
            input[2] = serde_json::json!(256);
        }
        assert!(matches!(driver(manifest).read_fan_channel("diagnostic_cpu_fan_rpm"), Err(HalError::InvalidParameter(_))));
    }

    #[test]
    fn ptl_tachometers_never_fall_back_to_stale_wmi() {
        let mut manifest = ptl_manifest();
        if let ActionDef::BufferInvoke { channel, .. } = manifest.actions.get_mut("read_fan_rpm").unwrap() {
            *channel = Some("wmi_mailbox".into());
        }
        assert!(matches!(driver(manifest).read_fan_rpm(), Err(HalError::DriverUnavailable(_))));
    }

    #[test]
    fn mode_action_never_falls_back_from_its_explicit_channel() {
        let mut manifest = ptl_manifest();
        if let ActionDef::ModeInvoke { channel, .. } = manifest.actions.get_mut("set_power_mode").unwrap() {
            *channel = "missing_channel".into();
        }
        let driver = driver(manifest);
        assert!(matches!(driver.set_performance_mode(HardwarePowerMode::Office), Err(HalError::DriverUnavailable(_))));
        assert_eq!(driver.last_power_mode.load(Ordering::SeqCst),2);
    }

    #[test]
    fn ptl_mode_query_errors_never_return_cached_turbo() {
        let mut manifest = ptl_manifest();
        if let ActionDef::BufferInvoke { input, .. } = manifest.actions.get_mut("get_power_mode").unwrap() {
            input[0] = serde_json::json!(256);
        }
        let driver = driver(manifest);
        driver.last_power_mode.store(3, Ordering::SeqCst);
        assert!(matches!(driver.get_performance_mode(), Err(HalError::InvalidParameter(_))));
    }
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
        self.execute_buffer_action_on_channel(in_bytes, None)
    }

    fn execute_buffer_action_on_channel(&self, in_bytes: &[u8], channel_name: Option<&str>) -> Result<Vec<u8>, HalError> {
        self.execute_buffer_action_with_status(in_bytes, channel_name, None)
    }

    fn execute_buffer_action_with_status(&self, in_bytes: &[u8], channel_name: Option<&str>, response_status: Option<&ResponseStatus>) -> Result<Vec<u8>, HalError> {
        // An explicit action channel must never fall back to another transport.
        let channel = self
            .manifest
            .channels
            .iter()
            .filter(|(name, _)| channel_name.map_or(true, |requested| name.as_str() == requested))
            .find_map(|(_, c)| match c {
                ChannelDef::WmiMailbox {
                    namespace,
                    class_name,
                    method_name,
                    buffer_sizes,
                    response_status,
                } => Some((namespace, class_name, method_name, buffer_sizes, response_status)),
                _ => None,
            })
            .ok_or_else(|| {
                HalError::DriverUnavailable("No WmiMailbox channel configured in manifest".to_string())
            })?;

        let response = invoke_wmi_mailbox(
            channel.0,
            channel.1,
            channel.2,
            in_bytes,
            channel.3.in_size,
            channel.3.out_size,
        )
        .map_err(HalError::RegisterAccessFailed)?;
        if let Some(status) = response_status.or(channel.4.as_ref()) {
            status.validate(&response).map_err(HalError::RegisterAccessFailed)?;
        }
        Ok(response)
    }

    fn read_fan_channel(&self, action_name: &str) -> Result<u16, HalError> {
        let Some(ActionDef::BufferInvoke { channel, input, parse, .. }) = self.manifest.actions.get(action_name) else {
            return Err(HalError::UnsupportedModel(format!("{} is not configured", action_name)));
        };
        let input: Vec<u8> = input.iter().map(|value| {
            value.as_u64().and_then(|number| u8::try_from(number).ok())
                .ok_or_else(|| HalError::InvalidParameter("Invalid fan query byte".into()))
        }).collect::<Result<_, _>>()?;
        let response = self.execute_buffer_action_on_channel(&input, channel.as_deref())?;
        let rpm = parse.as_ref().and_then(|rules| rules.get("rpm"))
            .and_then(|rule| extract_value_from_buffer(&response, rule.offset, &rule.value_type))
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| HalError::RegisterAccessFailed("Missing fan reading in firmware response".into()))?;
        Ok(rpm)
    }

    fn read_declared_power_mode(&self) -> Result<HardwarePowerMode, HalError> {
        let Some(ActionDef::BufferInvoke { channel, input, parse, mapping, .. }) = self.manifest.actions.get("get_power_mode") else {
            return Err(HalError::UnsupportedModel("No power mode readback action declared".into()));
        };
        let input = input.iter().map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok())
            .ok_or_else(|| HalError::InvalidParameter("Invalid power mode query byte".into())))
            .collect::<Result<Vec<_>, _>>()?;
        let response = self.execute_buffer_action_on_channel(&input, channel.as_deref())?;
        let raw = parse.as_ref().and_then(|p| p.get("mode_code"))
            .and_then(|rule| extract_value_from_buffer(&response, rule.offset, &rule.value_type))
            .ok_or_else(|| HalError::RegisterAccessFailed("Missing power mode in firmware response".into()))?;
        let label = mapping.as_ref().and_then(|m| m.get(&raw.to_string())).map(String::as_str);
        let mode = match (label, raw) {
            (Some("office" | "silence"), _) | (None, 1) => HardwarePowerMode::Office,
            (Some("balance"), _) | (None, 2) => HardwarePowerMode::Balance,
            (Some("turbo" | "rage"), _) | (None, 3) => HardwarePowerMode::Turbo,
            _ => return Err(HalError::RegisterAccessFailed(format!("Invalid firmware power mode {raw}"))),
        };
        Ok(mode)
    }

    fn confirm_power_mode(&self, expected: HardwarePowerMode) -> Result<(), HalError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(400);
        loop {
            let actual = self.read_declared_power_mode()?;
            if actual == expected { return Ok(()); }
            if std::time::Instant::now() >= deadline {
                return Err(HalError::RegisterAccessFailed(format!("Power mode readback mismatch: requested {expected:?}, observed {actual:?}")));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

impl DeclarativeHardwareDriver {
    fn set_declared_mode(&self, mode: HardwarePowerMode) -> Result<(), HalError> {
        let requested_mode = match mode {
            HardwarePowerMode::Office => "office",
            HardwarePowerMode::Balance => "balance",
            HardwarePowerMode::Turbo => "turbo",
            HardwarePowerMode::Custom => "custom",
        };
        if !self.manifest.capabilities.power_modes.iter().any(|supported| supported == requested_mode) {
            return Err(HalError::UnsupportedModel(format!("{} mode is not supported by this contract", requested_mode)));
        }
        let mode_key = match mode {
            HardwarePowerMode::Office => "office",
            HardwarePowerMode::Balance => "balance",
            HardwarePowerMode::Turbo => "turbo",
            HardwarePowerMode::Custom => "turbo",
        };

        if let Some(action) = self.manifest.actions.get("set_power_mode") {
            if let Some(invocation) = action.for_mode(mode_key) {
                if invocation.channel.is_some_and(|name| !self.manifest.channels.contains_key(name)) {
                    return Err(HalError::DriverUnavailable("Power mode channel is not declared".into()));
                }
                let u8_bytes: Vec<u8> = invocation.input
                    .iter()
                    .map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok())
                        .ok_or_else(|| HalError::InvalidParameter("Invalid power mode command byte".into())))
                    .collect::<Result<_, _>>()?;

                if !u8_bytes.is_empty() {
                    for (_, channel) in self.manifest.channels.iter()
                        .filter(|(name, _)| invocation.channel.map_or(true, |requested| name.as_str() == requested)) {
                        match channel {
                            ChannelDef::PmcEc { .. } => continue,
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
                                self.execute_buffer_action_with_status(&u8_bytes, invocation.channel, invocation.response_status)?;
                                if self.manifest.uses_ptl_pmc() { self.confirm_power_mode(mode)?; }
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

}

impl MechrevoHardwareHAL for DeclarativeHardwareDriver {
    fn set_performance_mode(&self, mode: HardwarePowerMode) -> Result<(), HalError> {
        if self.manifest.capabilities.cpu_power_backend.as_deref() != Some("intel_rapl") {
            return self.set_declared_mode(mode);
        }
        if mode == HardwarePowerMode::Custom {
            if !self.manifest.capabilities.power_modes.iter().any(|m| m == "custom") {
                return Err(HalError::UnsupportedModel("CPU custom mode is not declared".into()));
            }
            let p = crate::system::config::load_config().custom_profile;
            return self.apply_cpu_power_profile(p.cpu_pl1_watts, p.cpu_pl2_watts);
        }
        let mut firmware_error = None;
        let result = crate::hal::cpu_power::transition(None, || match self.set_declared_mode(mode) {
            Ok(()) => Ok(()),
            Err(error) => {
                let message = error.to_string();
                firmware_error = Some(error);
                Err(message)
            }
        });
        if let Some(error) = firmware_error { return Err(error); }
        result.map_err(HalError::RegisterAccessFailed)
    }

    fn set_battery_health(&self, mode: BatteryHealthMode) -> Result<(), HalError> {
        if !self.manifest.capabilities.battery_charge_limit {
            return Err(HalError::UnsupportedModel(
                "Battery charge limit not supported on this model".to_string(),
            ));
        }

        if self.manifest.uses_ptl_pmc() {
            let limit = match mode { BatteryHealthMode::LongLife => 100, BatteryHealthMode::Balance => 80, BatteryHealthMode::Station => 60 };
            return crate::hal::ptl_pmc::set_battery_limit(limit).map_err(HalError::RegisterAccessFailed);
        }

        // Native Tongfang ACPI channel support: bridge to MechrevoHALImpl
        if self.manifest.channels.values().any(|c| matches!(c, ChannelDef::AcpiIoctl { .. }))
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

        Err(HalError::UnsupportedModel("set_battery_limit is not defined in driver manifest".into()))
    }

    fn set_fan_boost(&self, active: bool) -> Result<(), HalError> {
        if self.manifest.uses_ptl_pmc() {
            return if active {
                crate::hal::ptl_pmc::set_manual_rpm([7500, 7500])
            } else { crate::hal::ptl_pmc::restore_auto() }.map_err(HalError::RegisterAccessFailed);
        }
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
            self.execute_buffer_action(&u8_bytes)?;
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
        if self.manifest.uses_ptl_pmc() {
            let Some(ActionDef::BufferInvoke { channel: Some(channel), .. }) = self.manifest.actions.get("read_fan_rpm") else {
                return Err(HalError::UnsupportedModel("No PTL fan tachometer action declared".into()));
            };
            if !matches!(self.manifest.channels.get(channel), Some(ChannelDef::PmcEc { protocol }) if protocol == "xingyao_ptl_v1") {
                return Err(HalError::DriverUnavailable("PTL tachometer action must use its PM2 channel".into()));
            }
            let rpm = crate::hal::ptl_pmc::read_rpm().map_err(HalError::RegisterAccessFailed)?;
            return Ok((rpm[0], rpm[1]));
        }
        if self.manifest.actions.contains_key("read_cpu_fan_rpm")
            || self.manifest.actions.contains_key("read_gpu_fan_rpm")
        {
            return Ok((self.read_fan_channel("read_cpu_fan_rpm")?, self.read_fan_channel("read_gpu_fan_rpm")?));
        }
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
    fn update_cpu_power_limits(&self, pl1: u16, pl2: u16) -> Result<(), HalError> {
        if self.manifest.capabilities.cpu_power_backend.as_deref() != Some("intel_rapl") {
            return Err(HalError::UnsupportedModel("CPU package power backend is not declared".into()));
        }
        crate::hal::cpu_power::update([pl1, pl2]).map_err(HalError::RegisterAccessFailed)
    }

    fn apply_cpu_power_profile(&self, pl1: u16, pl2: u16) -> Result<(), HalError> {
        if self.manifest.capabilities.cpu_power_backend.as_deref() != Some("intel_rapl")
            || !self.manifest.capabilities.power_modes.iter().any(|m| m == "custom") {
            return Err(HalError::UnsupportedModel("CPU custom mode is not declared".into()));
        }
        let active = crate::hal::cpu_power::is_active();
        let previous = self.read_declared_power_mode()?;
        let result = crate::hal::cpu_power::transition(Some([pl1, pl2]), ||
            self.set_declared_mode(HardwarePowerMode::Turbo).map_err(|e| e.to_string()));
        if result.is_err() && !active {
            // The CPU policy handles MSR rollback; also restore the chassis preset.
            self.set_declared_mode(previous)?;
        }
        result.map_err(HalError::RegisterAccessFailed)?;
        self.last_power_mode.store(HardwarePowerMode::Custom as u8, Ordering::SeqCst);
        Ok(())
    }


    fn commit_cpu_power_profile(&self, pl1: u16, pl2: u16, active_only: bool,
        persist: &mut dyn FnMut() -> Result<(), String>) -> Result<(), HalError> {
        if self.manifest.capabilities.cpu_power_backend.as_deref() != Some("intel_rapl")
            || !self.manifest.capabilities.power_modes.iter().any(|mode| mode == "custom") {
            return Err(HalError::UnsupportedModel("CPU custom mode is not declared".into()));
        }
        let previous = std::cell::Cell::new(None);
        crate::hal::cpu_power::commit_profile([pl1, pl2], active_only,
            || {
                previous.set(Some(self.read_declared_power_mode().map_err(|error| error.to_string())?));
                self.set_declared_mode(HardwarePowerMode::Turbo).map_err(|error| error.to_string())
            },
            || match previous.take() {
                Some(mode) => self.set_declared_mode(mode).map_err(|error| error.to_string()),
                None => Ok(()),
            },
            persist,
        ).map_err(HalError::RegisterAccessFailed)?;
        self.last_power_mode.store(HardwarePowerMode::Custom as u8, Ordering::SeqCst);
        Ok(())
    }

    fn set_rpm_fan_curves(&self, curves: crate::hal::fan_policy::RpmFanCurves) -> Result<(), HalError> {
        if !self.manifest.uses_ptl_pmc() { return Err(HalError::UnsupportedModel("No PM2 RPM curve channel declared".into())); }
        crate::hal::ptl_pmc::set_rpm_curves(curves).map_err(HalError::RegisterAccessFailed)
    }
    fn set_rpm_fan_smoothing(&self, smoothing: crate::hal::fan_policy::RpmSmoothing) -> Result<(), HalError> {
        if !self.manifest.uses_ptl_pmc() { return Err(HalError::UnsupportedModel("No PM2 software smoothing declared".into())); }
        crate::hal::ptl_pmc::set_smoothing(smoothing).map_err(HalError::RegisterAccessFailed)
    }
    fn set_manual_fan_rpm(&self, targets: [u16; 2]) -> Result<(), HalError> {
        if !self.manifest.uses_ptl_pmc() { return Err(HalError::UnsupportedModel("No PM2 fan channel declared".into())); }
        crate::hal::ptl_pmc::set_manual_rpm(targets).map_err(HalError::RegisterAccessFailed)
    }
    fn set_software_fan_curve(&self, curve: Vec<(u8, u8)>) -> Result<(), HalError> {
        if !self.manifest.uses_ptl_pmc() { return Err(HalError::UnsupportedModel("No PM2 curve channel declared".into())); }
        crate::hal::ptl_pmc::set_curve(curve).map_err(HalError::RegisterAccessFailed)
    }
    fn restore_fan_auto(&self) -> Result<(), HalError> {
        if !self.manifest.uses_ptl_pmc() { return Err(HalError::UnsupportedModel("No PM2 fan channel declared".into())); }
        crate::hal::ptl_pmc::restore_auto().map_err(HalError::RegisterAccessFailed)
    }
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
        if self.manifest.uses_ptl_pmc() {
            let physical = self.read_declared_power_mode()?;
            return Ok(if physical == HardwarePowerMode::Turbo && crate::hal::cpu_power::is_active() {
                HardwarePowerMode::Custom
            } else { physical });
        }
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
        if let Ok(temp) = self.read_cpu_temp_sensor() {
            return Ok(temp);
        }
        let temp = crate::commands::system_cmd::get_cpu_monitor().get_temp();
        if temp > 0 && temp < 125 {
            Ok(temp as u8)
        } else {
            Ok(48)
        }
    }

    fn read_cpu_temp_sensor(&self) -> Result<u8, HalError> {
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

        Err(HalError::DriverUnavailable("CPU temperature sensor unavailable".into()))
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
