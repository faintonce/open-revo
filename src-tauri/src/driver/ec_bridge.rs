use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use crate::driver::wmi_ec;
use crate::system::process;

#[derive(Clone, Debug)]
pub struct HardwareStateCache {
    pub power_mode: u8,
    pub fan_boost: bool,
    pub battery_limit: u8,
    pub cpu_fan_rpm: u16,
    pub gpu_fan_rpm: u16,
    pub cpu_temp: i32,
    // Cooldown protection to prevent background poller from oscillating user state
    pub fan_boost_cooldown_until: Instant,
    pub power_mode_cooldown_until: Instant,
    pub battery_limit_cooldown_until: Instant,
}

impl Default for HardwareStateCache {
    fn default() -> Self {
        Self {
            power_mode: 2, // Default UI mode: 2 (Gaming / Balance)
            fan_boost: false,
            battery_limit: 80,
            cpu_fan_rpm: 0,
            gpu_fan_rpm: 0,
            cpu_temp: 45,
            fan_boost_cooldown_until: Instant::now(),
            power_mode_cooldown_until: Instant::now(),
            battery_limit_cooldown_until: Instant::now(),
        }
    }
}

pub struct EcBridge {
    cache: Arc<Mutex<HardwareStateCache>>,
    pub is_mock: bool,
    pub oem_conflict_detected: bool,
}

impl EcBridge {
    pub fn new(conflict: bool) -> Self {
        let initial_cfg = crate::system::config::load_config();
        let initial_mode = if initial_cfg.power_mode >= 1 && initial_cfg.power_mode <= 4 {
            initial_cfg.power_mode
        } else {
            2
        };
        let mut initial_cache = HardwareStateCache::default();
        initial_cache.power_mode = initial_mode;
        initial_cache.fan_boost = if initial_cfg.fan_boost && wmi_ec::is_acpi_driver_available() {
            wmi_ec::is_fan_boost_active()
        } else {
            false
        };
        initial_cache.battery_limit = initial_cfg.battery_limit;
        // 5 second cooldown on boot so poller never overrides initial configuration during bootup
        initial_cache.power_mode_cooldown_until = Instant::now() + Duration::from_secs(5);
        initial_cache.fan_boost_cooldown_until = Instant::now() + Duration::from_secs(5);
        initial_cache.battery_limit_cooldown_until = Instant::now() + Duration::from_secs(5);

        let cache = Arc::new(Mutex::new(initial_cache));
        let cache_for_poller = Arc::clone(&cache);

        // Dedicated background poller thread
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));

            // On non-Tongfang Ultrabooks, background polling is completely disabled!
            // Fan RPM is only fetched on-demand by Telemetry when the window is in the foreground.
            if crate::hal::platform_probe::is_ultrabook() && !crate::hal::platform_probe::is_tongfang_family() {
                crate::log_info!("EC_BRIDGE", "Non-Tongfang Ultrabook detected: background EC/WMI fan poller permanently bypassed.");
                return;
            }

            let register_list = [
                wmi_ec::ADDR_SUPER_PERFORMANCE,       // Fan Boost Hardware Clamp (Bit 6: 0x40)
                wmi_ec::ADDR_MODE_INDEX,              // Operating Mode (1: Office, 0: Balance, 2: Turbo)
                wmi_ec::ADDR_BATTERY_PROTECTION_MODE, // Battery Mode | Touchpad mask
                wmi_ec::ADDR_BATTERY_LIMIT_UP,        // Battery Limit Upper
                wmi_ec::ADDR_MAIN_FAN_RPM_B1,         // CPU Fan RPM High
                wmi_ec::ADDR_MAIN_FAN_RPM_B2,         // CPU Fan RPM Low
                wmi_ec::ADDR_SECOND_FAN_RPM_B1,       // GPU Fan RPM High
                wmi_ec::ADDR_SECOND_FAN_RPM_B2,       // GPU Fan RPM Low
            ];

            loop {
                if !wmi_ec::is_acpi_driver_available() {
                    let hal = crate::hal::get_hardware_driver();
                    if hal.kind() != crate::hal::DriverKind::TongfangAcpi && hal.is_hardware_supported() {
                        if let Ok((cpu_rpm, gpu_rpm)) = hal.read_fan_rpm() {
                            let mut c = cache_for_poller.lock();
                            if cpu_rpm > 0 {
                                c.cpu_fan_rpm = cpu_rpm;
                            }
                            if gpu_rpm > 0 {
                                c.gpu_fan_rpm = gpu_rpm;
                            }
                        }
                        // Ultrabooks do not possess autonomous physical rocker switches for power mode.
                        // Performance mode on ultrabooks is managed authoritatively by OpenRevo software.
                        // We must NEVER let periodic background polling blindly overwrite c.power_mode,
                        // which caused the catastrophic 5-second mode revert back to Balance!
                    }
                    thread::sleep(Duration::from_millis(1500));
                    continue;
                }

                if let Some(bytes) = wmi_ec::wmi_batch_read_ec(&register_list) {
                    if bytes.len() == 8 {
                        let raw_fan_clamp = bytes[0];
                        let raw_mode = bytes[1];
                        let bat_mode_byte = bytes[2];
                        let battery_up = bytes[3];
                        let cpu_rpm_high = bytes[4] as u16;
                        let cpu_rpm_low = bytes[5] as u16;
                        let gpu_rpm_high = bytes[6] as u16;
                        let gpu_rpm_low = bytes[7] as u16;

                        let now = Instant::now();
                        let mut c = cache_for_poller.lock();

                        // 1. Fan Boost Hardware Clamp - Direct zero-latency hardware clamp state
                        if now >= c.fan_boost_cooldown_until {
                            let is_ultrabook = crate::hal::platform_probe::is_ultrabook();
                            if !is_ultrabook {
                                let is_bld_manifest = crate::hal::declarative::load_matched_manifest()
                                    .map(|m| m.channels.values().any(|ch| matches!(ch, crate::hal::declarative::model_manifest::ChannelDef::WmiMailbox { .. })))
                                    .unwrap_or(false);
                                if !is_bld_manifest {
                                    let hw_fb = (raw_fan_clamp & wmi_ec::FAN_BOOST_CLAMP_BIT) != 0;
                                    c.fan_boost = hw_fb;
                                }
                            } else {
                                // Ultrabooks do not possess autonomous physical Fn+1 hotkeys or hardware toggle rockers.
                                // Fan Boost on ultrabooks is managed authoritatively by OpenRevo software.
                                // We must NEVER let periodic background polling blindly overwrite c.fan_boost!
                                let cfg_boost = crate::system::config::load_config().fan_boost;
                                c.fan_boost = cfg_boost;
                                // Self-healing: if software Fan Boost is ON but EC clamp bit dropped, re-assert clamp!
                                if cfg_boost && (raw_fan_clamp & wmi_ec::FAN_BOOST_CLAMP_BIT) == 0 {
                                    let _ = wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, raw_fan_clamp | wmi_ec::FAN_BOOST_CLAMP_BIT);
                                }
                            }
                        }

                        // 2. Power Mode - Respect user action cooldown window!
                        // In Custom Mode (4), raw_mode is purposefully set to base performance (0x00 or 0x10)
                        // by OpenRevo custom profile, NOT an autonomous EC mode switch.
                        // We must NEVER let the poller misinterpret 0 as Balance and revert Custom Mode!
                        if now >= c.power_mode_cooldown_until && c.power_mode != 4 {
                            let mode_bits = raw_mode & 0x1E;
                            if (mode_bits & 0x10) != 0 {
                                c.power_mode = 3; // Turbo
                            }
                        }

                        // 3. Battery Limit - Respect cooldown window!
                        if now >= c.battery_limit_cooldown_until {
                            let clean_mode = bat_mode_byte & !0x08; // Mask out touchpad enable bit (0x08)
                            if clean_mode == 0x21 || (clean_mode & 0xF0) == 0x20 || (battery_up <= 65 && battery_up > 0) {
                                c.battery_limit = 60;
                            } else if clean_mode == 0x11 || (clean_mode & 0xF0) == 0x10 || (battery_up <= 85 && battery_up > 65) {
                                c.battery_limit = 80;
                            } else if clean_mode == 0x01 || battery_up >= 95 || (clean_mode == 0 && battery_up == 0) {
                                c.battery_limit = 100;
                            } else if battery_up > 0 && battery_up <= 100 {
                                c.battery_limit = battery_up;
                            }
                        }

                        // 4. Fan RPMs (Always update with real RPM readings)
                        let cpu_rpm = (cpu_rpm_high << 8) | cpu_rpm_low;
                        if cpu_rpm > 0 && cpu_rpm < 8000 {
                            c.cpu_fan_rpm = cpu_rpm;
                        }

                        let gpu_rpm = (gpu_rpm_high << 8) | gpu_rpm_low;
                        if gpu_rpm > 0 && gpu_rpm < 8000 {
                            c.gpu_fan_rpm = gpu_rpm;
                        }

                        // 5. Battery Gate Watchdog: Deprecated legacy spoofing loop.
                        // Advanced charge gating is now fully decoupled into the official demonstration plugin
                        // using non-invasive pulse-trigger storage (zero persistent platform tampering, zero keyboard matrix conflict).
                    }
                }

                // Heartbeat: ensure APExist is maintained continuously
                if let Some(val) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_AP_OEM_BYTE) {
                    if (val & 0x01) == 0 {
                        let _ = wmi_ec::wmi_write_ec(wmi_ec::ADDR_AP_OEM_BYTE, val | 0x01);
                    }
                }

                thread::sleep(Duration::from_millis(2000));
            }
        });

        let hal_supported = crate::hal::get_hardware_driver().is_hardware_supported();
        let is_mock = !wmi_ec::is_acpi_driver_available()
            && !crate::driver::wmi_mulong::is_mulong_available()
            && !hal_supported;
        Self {
            cache,
            is_mock,
            oem_conflict_detected: conflict,
        }
    }

    pub fn dummy() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HardwareStateCache::default())),
            is_mock: true,
            oem_conflict_detected: false,
        }
    }

    pub fn auto_load() -> Self {
        #[cfg(windows)]
        {
            let hal = crate::hal::get_hardware_driver();
            if !crate::hal::platform_probe::is_tongfang_family() && !hal.is_hardware_supported() {
                crate::log_info!(
                    "EC_BRIDGE",
                    "Non-Tongfang platform without active HAL detected. Falling back to dummy."
                );
                return Self::dummy();
            }
        }

        let oem_running = process::is_oem_service_running();
        Self::new(oem_running)
    }

    pub fn reload(&mut self) {
        *self = Self::auto_load();
    }

    pub fn get_cpu_temp(&self) -> i32 {
        self.cache.lock().cpu_temp
    }

    pub fn get_gpu_temp(&self) -> i32 {
        42
    }

    pub fn get_power_mode(&self) -> u8 {
        self.cache.lock().power_mode
    }

    pub fn set_power_mode(&self, ui_mode: u8) {
        crate::log_info!("EC_BRIDGE", "User triggered set_power_mode: {}", ui_mode);
        {
            let mut c = self.cache.lock();
            c.power_mode = ui_mode;
            c.power_mode_cooldown_until = Instant::now() + Duration::from_secs(5);
        }

        thread::spawn(move || {
            crate::system::power_mode::apply_full_power_mode(ui_mode);
        });
    }

    pub fn set_power_mode_from_hardware(&self, detected_mode: u8) {
        crate::log_info!("EC_BRIDGE", "Hardware aligned set_power_mode: {}", detected_mode);
        {
            let mut c = self.cache.lock();
            c.power_mode = detected_mode;
            c.power_mode_cooldown_until = Instant::now() + Duration::from_secs(3);
        }

        thread::spawn(move || {
            crate::system::power_mode::apply_windows_power_overlay(detected_mode);
        });
    }

    pub fn is_fan_boost(&self) -> bool {
        self.cache.lock().fan_boost
    }

    pub fn is_in_fan_boost_cooldown(&self) -> bool {
        Instant::now() < self.cache.lock().fan_boost_cooldown_until
    }

    pub fn set_fan_boost(&self, enable: bool) {
        let support_flags = crate::driver::wmi_ec::get_hardware_support_flags();
        if !support_flags.has_fan_boost {
            crate::log_info!("EC_BRIDGE", "Platform does not support fan boost: ignoring request.");
            return;
        }
        {
            let mut c = self.cache.lock();
            c.fan_boost = enable;
            // Cooldown prevents background poller and event echoes from oscillating status
            c.fan_boost_cooldown_until = Instant::now() + Duration::from_secs(3);
        }

        // Proactively align user config and tray icon
        let mut cfg = crate::system::config::load_config();
        cfg.fan_boost = enable;
        crate::system::config::save_config(&cfg);
        crate::update_tray_fan_boost(enable);

        // Apply fan boost to hardware driver (Tongfang WMI EC or Declarative HAL)
        thread::spawn(move || {
            wmi_ec::set_fan_boost_hardware(enable);
        });
    }

    pub fn set_fan_boost_from_hardware(&self, enable: bool) {
        crate::log_info!("EC_BRIDGE", "Hardware aligned set_fan_boost: {}", enable);
        {
            let mut c = self.cache.lock();
            c.fan_boost = enable;
            c.fan_boost_cooldown_until = Instant::now() + Duration::from_secs(3);
        }

        // Proactively align user config and tray icon without re-pulsing hardware EC
        let mut cfg = crate::system::config::load_config();
        cfg.fan_boost = enable;
        crate::system::config::save_config(&cfg);
        crate::update_tray_fan_boost(enable);
    }

    pub fn get_battery_limit(&self) -> u8 {
        self.cache.lock().battery_limit
    }

    pub fn set_battery_limit(&self, limit: u8) {
        let cfg = crate::system::config::load_config();
        self.apply_battery_limit_with_override(limit, cfg.battery_hardware_limit_enabled);
    }

    pub fn set_battery_hardware_limit_override(&self, hardware_override: bool) {
        let cfg = crate::system::config::load_config();
        self.apply_battery_limit_with_override(cfg.battery_limit, hardware_override);
    }

    pub fn apply_battery_limit_with_override(&self, limit: u8, hardware_override: bool) {
        {
            let mut c = self.cache.lock();
            c.battery_limit = limit;
            c.battery_limit_cooldown_until = Instant::now() + Duration::from_secs(5);
        }

        thread::spawn(move || {
            let is_tongfang_hardware = crate::driver::wmi_ec::is_acpi_driver_available()
                || crate::hal::declarative::load_matched_manifest()
                    .map(|m| m.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. })))
                    .unwrap_or(false);

            if !is_tongfang_hardware {
                let health_mode = if limit >= 95 {
                    crate::hal::BatteryHealthMode::LongLife
                } else if limit <= 65 {
                    crate::hal::BatteryHealthMode::Station
                } else {
                    crate::hal::BatteryHealthMode::Balance
                };
                let hal = crate::hal::get_hardware_driver();
                let _ = hal.set_battery_health(health_mode);
                return;
            }

            let mode_code = if limit >= 95 {
                wmi_ec::BATTERY_MODE_LONGLIFE
            } else if limit <= 65 {
                wmi_ec::BATTERY_MODE_STATION
            } else {
                wmi_ec::BATTERY_MODE_BALANCE
            };

            let down_limit = if limit > 5 { limit - 5 } else { limit };

            // 0. Auto-restore platform ID: ensure platform identity is clean and restored (prevents keyboard mapping issues)
            let is_declarative = crate::hal::declarative::load_matched_manifest().is_some();
            if !is_declarative {
                let cur_id = wmi_ec::wmi_read_ec(wmi_ec::ADDR_PLATFORM_REAL_ID).unwrap_or(0x07);
                let restore_id = if cur_id == 0x04 || cur_id == 0x05 { 0x07 } else { cur_id };
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_PLATFORM_REAL_ID, restore_id);
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_BATTERY_PLATFORM_OVERRIDE, wmi_ec::BATTERY_OVERRIDE_RESET_VALUE);
            }

            // 1. First write cut-off thresholds (Upper, Lower)
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_BATTERY_LIMIT_UP, limit);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_BATTERY_LIMIT_DOWN, down_limit);

            // 2. Atomically update battery protection mode while strictly preserving Touchpad state bit
            let final_mode_byte = wmi_ec::wmi_modify_ec(wmi_ec::ADDR_BATTERY_PROTECTION_MODE, |cur_bat_mode| {
                let tp_bit = cur_bat_mode & wmi_ec::TOUCHPAD_BITMASK;
                mode_code | tp_bit
            }).unwrap_or(mode_code);

            // 3. Immediately align Q-Key LED clamp for active Office or Balance mode under platform override (Gaming laptops only)
            let current_power_mode = crate::system::config::load_config().power_mode;
            if !crate::hal::platform_probe::is_ultrabook() && (current_power_mode == 1 || current_power_mode == 2) {
                let fan_boost = crate::system::config::load_config().fan_boost;
                let boost_bit = if fan_boost { wmi_ec::FAN_BOOST_CLAMP_BIT } else { 0 };
                let target_clamp = (if current_power_mode == 1 { 0xA0 } else { 0x00 }) | boost_bit;
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, target_clamp);
            }

            crate::log_info!(
                "EC_BATTERY",
                "Hardware battery limit applied [Platform4-REAL_ID]: limit={}%, down={}, hw_override={}, mode_byte=0x{:02X}",
                limit, down_limit, hardware_override, final_mode_byte
            );
        });
    }

    pub fn get_cpu_fan_rpm(&self) -> u16 {
        self.cache.lock().cpu_fan_rpm
    }

    pub fn get_gpu_fan_rpm(&self) -> u16 {
        self.cache.lock().gpu_fan_rpm
    }

    pub fn update_fan_rpm(&self, cpu: u16, gpu: u16) {
        let mut c = self.cache.lock();
        if cpu > 0 { c.cpu_fan_rpm = cpu; }
        if gpu > 0 { c.gpu_fan_rpm = gpu; }
    }
}

pub type SharedEcBridge = Arc<Mutex<EcBridge>>;
