pub mod arbiter;
pub mod ec_mode;
pub mod fan_table;
pub mod oem_sync;
pub mod ppm;
pub mod presets;
pub mod status;

pub use arbiter::{PowerArbiter, PowerIntentSource};
pub use status::*;
pub use ec_mode::apply_ec_hardware_mode;
pub use fan_table::{
    apply_fan_table_to_ec, apply_mode_fan_table, build_custom_fan_table, get_mode_fan_table,
    FanTableData, ModelFanTables, FAN_TABLE_GAMING, FAN_TABLE_OFFICE, FAN_TABLE_TURBO,
};
pub use oem_sync::{sync_oem_main_option, sync_oem_mode4_profile};
pub use ppm::{
    apply_all_sleep_guard, apply_sleep_guard_option, deep_unlock_cpu_frequency,
    get_windows_power_schemes, restore_official_power_plan, set_active_windows_power_scheme,
    WindowsPowerScheme,
};
pub use presets::{get_factory_power_presets, get_cached_factory_power_presets, refresh_factory_power_presets, FactoryPowerPresets};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

// Windows 10/11 Official Power Overlay Scheme GUIDs
// Best Power Efficiency: 961cc777-2547-4f9d-8174-7d86181b8a7a
pub const GUID_OFFICE_SAVER: GUID = GUID {
    data1: 0x961cc777,
    data2: 0x2547,
    data3: 0x4f9d,
    data4: [0x81, 0x74, 0x7d, 0x86, 0x18, 0x1b, 0x8a, 0x7a],
};

pub const GUID_BALANCED: GUID = GUID {
    data1: 0x00000000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0, 0, 0, 0, 0, 0, 0, 0],
};

pub const GUID_TURBO_PERF: GUID = GUID {
    data1: 0xded574b5,
    data2: 0x45a0,
    data3: 0x4f42,
    data4: [0x87, 0x37, 0x46, 0x34, 0x5c, 0x09, 0xc2, 0x38],
};

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryA(lp_lib_filename: *const u8) -> *mut std::ffi::c_void;
    fn GetProcAddress(h_module: *mut std::ffi::c_void, lp_proc_name: *const u8) -> *mut std::ffi::c_void;
}

type FnPowerSetActiveOverlayScheme = unsafe extern "system" fn(*const GUID) -> u32;

fn set_active_overlay_scheme(guid_ptr: *const GUID) -> u32 {
    unsafe {
        let dll_name = b"powrprof.dll\0";
        let h_mod = LoadLibraryA(dll_name.as_ptr());
        if h_mod.is_null() {
            return 1;
        }
        let fn_name = b"PowerSetActiveOverlayScheme\0";
        let p_func = GetProcAddress(h_mod, fn_name.as_ptr());
        if p_func.is_null() {
            return 2;
        }
        let func: FnPowerSetActiveOverlayScheme = std::mem::transmute(p_func);
        func(guid_ptr)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct ProcessorPowerInformation {
    pub number: u32,
    pub max_mhz: u32,
    pub current_mhz: u32,
    pub mhz_limit: u32,
    pub max_idle_state: u32,
    pub current_idle_state: u32,
}

type FnCallNtPowerInformation = unsafe extern "system" fn(
    information_level: u32,
    input_buffer: *const std::ffi::c_void,
    input_buffer_length: u32,
    output_buffer: *mut std::ffi::c_void,
    output_buffer_length: u32,
) -> i32;

/// Query live CPU frequency and limits across cores (zero overhead, in-memory kernel query).
/// Returns (current_mhz, max_mhz, mhz_limit) for core 0.
pub fn get_cpu_frequency_info() -> Option<(u32, u32, u32)> {
    unsafe {
        let dll_name = b"powrprof.dll\0";
        let h_mod = LoadLibraryA(dll_name.as_ptr());
        if h_mod.is_null() {
            return None;
        }
        let fn_name = b"CallNtPowerInformation\0";
        let p_func = GetProcAddress(h_mod, fn_name.as_ptr());
        if p_func.is_null() {
            return None;
        }
        let func: FnCallNtPowerInformation = std::mem::transmute(p_func);
        let mut info: [ProcessorPowerInformation; 64] = [ProcessorPowerInformation::default(); 64];
        let buf_size = std::mem::size_of_val(&info) as u32;
        // 11 = ProcessorInformation
        let status = func(11, std::ptr::null(), 0, info.as_mut_ptr() as *mut std::ffi::c_void, buf_size);
        if status == 0 && info[0].max_mhz > 0 {
            Some((info[0].current_mhz, info[0].max_mhz, info[0].mhz_limit))
        } else {
            None
        }
    }
}

static POWER_MODE_LOCK: std::sync::Mutex<Option<(u8, std::time::Instant)>> = std::sync::Mutex::new(None);

/// Apply full-stack performance release mode across Windows Power Overlay, Power Plan, and Motherboard EC
pub fn apply_full_power_mode(ui_mode: u8) {
    let mut lock = match POWER_MODE_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    if let Some((last_mode, last_time)) = *lock {
        if last_mode == ui_mode && last_time.elapsed() < std::time::Duration::from_millis(600) {
            crate::log_info!("POWER_MODE", "apply_full_power_mode for mode {} debounced (applied {:?} ago)", ui_mode, last_time.elapsed());
            return;
        }
    }
    *lock = Some((ui_mode, std::time::Instant::now()));

    crate::log_info!("POWER_MODE", "=== Apply Full Power Mode: UI Mode {} ===", ui_mode);

    // 1. Windows OS Energy Scheduler (Power Overlay Scheme)
    apply_windows_power_overlay(ui_mode);

    // 2. Hardware EC RAM Registers (PL1 / PL2 / TGP / Fan Mode / APExist)
    apply_ec_hardware_mode(ui_mode);

    // 3. OEM Official Configuration Sync (MainOption.json & Mode4_Profile.json)
    sync_oem_main_option(ui_mode);
    if ui_mode == 4 {
        let cfg = crate::system::config::load_config();
        sync_oem_mode4_profile(&cfg.custom_profile);
    }
}

/// Set Windows Power Overlay Scheme slider (Best Efficiency / Balanced / Best Performance)
pub fn apply_windows_power_overlay(ui_mode: u8) {
    let cfg = crate::system::config::load_config();
    if !cfg.high_perf_scheme {
        crate::log_info!("POWER_MODE", "apply_windows_power_overlay skipped: high_perf_scheme link is disabled");
        return;
    }

    let is_thin_light = is_thin_and_light_or_pure_igpu();
    let (guid_ptr, name) = match ui_mode {
        1 => {
            if is_thin_light {
                (&GUID_BALANCED as *const GUID, "GUID_BALANCED (平衡-轻薄本保留硬件EC功耗)")
            } else {
                (&GUID_OFFICE_SAVER as *const GUID, "GUID_OFFICE_SAVER (最佳能效)")
            }
        }
        2 => (&GUID_BALANCED as *const GUID, "GUID_BALANCED (平衡)"),
        3 => (&GUID_TURBO_PERF as *const GUID, "GUID_TURBO_PERF (最佳性能)"),
        4 => (&GUID_BALANCED as *const GUID, "GUID_BALANCED (平衡)"),
        _ => (&GUID_BALANCED as *const GUID, "GUID_BALANCED (平衡)"),
    };

    let ret = set_active_overlay_scheme(guid_ptr);
    if ret == 0 {
        crate::log_info!("POWER_MODE", "PowerSetActiveOverlayScheme succeeded for mode {} ({})", ui_mode, name);
        crate::log_info!("CUSTOM_TRACE", "PowerSetActiveOverlayScheme succeeded for mode {} ({})", ui_mode, name);
    } else {
        crate::log_warn!("POWER_MODE", "PowerSetActiveOverlayScheme failed with code {} for mode {} ({})", ret, ui_mode, name);
        crate::log_warn!("CUSTOM_TRACE", "PowerSetActiveOverlayScheme failed with code {} for mode {} ({})", ret, ui_mode, name);
    }
}

/// DEPRECATED: Retained as a safe no-op to ensure backwards compatibility.
/// OpenRevo strictly respects user's active Windows power scheme and NEVER forcibly overrides it.
pub fn ensure_standard_balanced_plan() {
    // Intentionally no-op: user power schemes are preserved and never overridden.
}

/// Spawns the startup hardware state restoration thread (power mode, limits, fan boost, AC recovery, display, lighting).
pub fn spawn_hardware_state_restore(ec_for_restore: crate::driver::ec_bridge::SharedEcBridge) {
    std::thread::spawn(move || {
        crate::system::boot_trace::boot_log("RESTORE", "Background hardware state restore thread started");
        let cfg = crate::system::config::load_config();
        let is_oem_running = crate::system::process::is_oem_service_running();

        // Enforce OEM takeover if previously activated by user
        if cfg.takeover_oem {
            let _ = crate::system::service_mgr::pause_oem_services();
        } else if is_oem_running {
            // PASSIVE SAFEGUARD ACTIVATION:
            // If OEM service (GCUService/GCUBridge/ControlCenter) is active and user hasn't taken over,
            // DO NOT perform any hardware writes, refresh rate switches, or lighting streams!
            // This strictly prevents hardware register collisions and display blinking loops.
            crate::log_warn!(
                "MAIN",
                "OEM Service is active and takeover_oem is false! PASSIVE SAFEGUARD ACTIVATED: suppressing startup hardware writes, display refresh rate, and lighting streamer."
            );
            // Safe maintenance only: task scheduler sync & self-heal
            crate::system::autostart::sync_autostart_target_path();
            return;
        }

        // If OEM service is not running or takeover is active, restore all hardware state:
        let mut is_dc_online = is_dc_round_plugged_in();
        // 启动初期电源状态防抖：若首次读取为 false，等待 250ms 重试一次，过滤开机登录阶段驱动瞬态未决
        if !is_dc_online {
            std::thread::sleep(std::time::Duration::from_millis(250));
            is_dc_online = is_dc_round_plugged_in();
        }

        let is_thin_light = is_thin_and_light_or_pure_igpu();

        let effective_power_mode = if is_thin_light {
            // 纯核显轻薄本：完全尊重用户记忆的 power_mode (1..=3)，不执行开机强制切狂暴/强制切平衡
            let mode = match cfg.power_mode {
                1..=3 => cfg.power_mode,
                _ => 2, // 默认均衡
            };
            crate::log_info!("MAIN", "Thin & light laptop: restoring user configured power mode {}", mode);
            mode
        } else if is_dc_online {
            // 插电开机：根据 power_mode_ac 恢复 (1..=4，默认 3: 狂暴)
            let target_mode = match cfg.power_mode_ac {
                1..=4 => cfg.power_mode_ac,
                _ => 3,
            };
            if cfg.power_mode != target_mode {
                crate::log_info!("MAIN", "Machine booted on DC power: aligning power mode to AC preference {} (was {})", target_mode, cfg.power_mode);
                let mut updated_cfg = cfg.clone();
                updated_cfg.power_mode = target_mode;
                crate::system::config::save_config(&updated_cfg);
            }
            target_mode
        } else {
            // 纯电池开机：为了电池硬件安全，采用 power_mode_battery (1: 办公, 2: 均衡，默认 2)
            // 绝对不破坏 power_mode_ac 中的高性能设置！
            if cfg.unlock_battery_power_limit {
                crate::log_info!("MAIN", "Machine booted on battery with unlock_battery_power_limit active: keeping power mode {}", cfg.power_mode);
                cfg.power_mode
            } else {
                let target_mode = if cfg.power_mode_battery == 1 { 1 } else { 2 };
                crate::log_info!("MAIN", "Machine booted on battery: setting power mode to battery preference {} (AC preference is {})", target_mode, cfg.power_mode_ac);
                if cfg.power_mode != target_mode {
                    let mut updated_cfg = cfg.clone();
                    if updated_cfg.power_mode == 3 || updated_cfg.power_mode == 4 {
                        updated_cfg.power_mode_ac = updated_cfg.power_mode;
                    }
                    updated_cfg.power_mode = target_mode;
                    crate::system::config::save_config(&updated_cfg);
                }
                target_mode
            }
        };

        crate::event::power_mode_changed_global(effective_power_mode);
        if is_thin_light {
            let hal = crate::hal::get_hardware_driver();
            let hw_mode = match effective_power_mode {
                1 => crate::hal::HardwarePowerMode::Office,
                2 => crate::hal::HardwarePowerMode::Balance,
                3 => crate::hal::HardwarePowerMode::Turbo,
                4 => crate::hal::HardwarePowerMode::Custom,
                _ => crate::hal::HardwarePowerMode::Balance,
            };
            let _ = hal.set_performance_mode(hw_mode);
            ec_for_restore.lock().set_power_mode_from_hardware(effective_power_mode);
            crate::log_info!("POWER_RESTORE", "Thin & light startup power mode applied via HAL: {:?}", hw_mode);

            // Restore battery health limit via HAL if applicable
            let batt_mode = match cfg.battery_limit {
                100 => Some(crate::hal::BatteryHealthMode::LongLife),
                80 => Some(crate::hal::BatteryHealthMode::Balance),
                60 => Some(crate::hal::BatteryHealthMode::Station),
                _ => None,
            };
            if let Some(bm) = batt_mode {
                let _ = hal.set_battery_health(bm);
            }

            crate::system::boot_trace::boot_log("RESTORE", "Thin & light startup hardware restoration completed");
            return;
        }

        {
            let ec = ec_for_restore.lock();
            ec.set_power_mode(effective_power_mode);

            // Legacy residual cleanup: earlier builds wrote 0x0770=4 or 0x0770=5 (ADDR_BATTERY_PLATFORM_OVERRIDE)
            // which breaks Fn keys on gaming laptops. The new Platform4-Gate method no longer uses this register.
            // Auto-detect and neutralize stale values from old installs before applying the battery limit.
            if let Some(override_val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BATTERY_PLATFORM_OVERRIDE) {
                if override_val == 4 || override_val == 5 {
                    crate::driver::wmi_ec::wmi_write_ec(
                        crate::driver::wmi_ec::ADDR_BATTERY_PLATFORM_OVERRIDE,
                        crate::driver::wmi_ec::BATTERY_OVERRIDE_RESET_VALUE,
                    );
                    crate::log_info!(
                        "POWER_RESTORE",
                        "Legacy residual detected: ADDR_BATTERY_PLATFORM_OVERRIDE was {} (old build artifact). Neutralized to 0xFF.",
                        override_val
                    );
                }
            }

            ec.set_battery_limit(cfg.battery_limit);
            ec.set_fan_boost(cfg.fan_boost);
        }

        // Secondary debounce: re-assert battery limit 2.5s after boot to eliminate ACPI driver race
        {
            let ec_delayed = ec_for_restore.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2500));
                let cfg_delayed = crate::system::config::load_config();
                if cfg_delayed.battery_hardware_limit_enabled && cfg_delayed.battery_limit < 95 {
                    ec_delayed.lock().apply_battery_limit_with_override(cfg_delayed.battery_limit, true);
                }
            });
        }
        if effective_power_mode == 3 || effective_power_mode == 4 {
            let _ = crate::system::actuator::UnifiedPowerActuator::global().unlock_factory_full_release();
        }
        // Heal MyFan2 fan speed table if running on legacy chassis and previously corrupted
        crate::driver::wmi_ec::heal_legacy_fan_table_if_needed();
        // Restore AC Recovery and protect Custom Mode Q-Key White LED
        if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY) {
            let mut new_v = if cfg.ac_recovery_enabled { v | crate::driver::wmi_ec::AC_RECOVERY_BITMASK } else { v & !crate::driver::wmi_ec::AC_RECOVERY_BITMASK };
            if effective_power_mode == 4 {
                new_v |= crate::driver::wmi_ec::MASK_QKEY_WHITE_1830;
            } else {
                new_v &= !crate::driver::wmi_ec::MASK_QKEY_WHITE_1830;
            }
            crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY, new_v);
        } else if effective_power_mode == 4 {
            let new_v = if cfg.ac_recovery_enabled { crate::driver::wmi_ec::MASK_QKEY_WHITE_1830 | crate::driver::wmi_ec::AC_RECOVERY_BITMASK } else { crate::driver::wmi_ec::MASK_QKEY_WHITE_1830 };
            crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY, new_v);
        }

        if effective_power_mode == 4 {
            if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF) {
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF, v | crate::driver::wmi_ec::MASK_QKEY_WHITE_1831);
            } else {
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF, crate::driver::wmi_ec::MASK_QKEY_WHITE_1831);
            }
        }
        // Restore USB Charge while powered off (Trigger Bit 4: 0x10)
        crate::driver::wmi_ec::wmi_modify_ec_mask(
            crate::driver::wmi_ec::ADDR_TRIGGER_BYTE,
            crate::driver::wmi_ec::TRIGGER_USB_CHARGE,
            cfg.usb_charge_enabled,
        );

        // Restore Win Key Lock state (Software Hook + EC TRIGGER_WINLOCK)
        let is_ultrabook = crate::hal::platform_probe::is_ultrabook();
        let supports_legacy_ec = crate::hal::platform_probe::is_tongfang_family() && !is_ultrabook;
        if supports_legacy_ec {
            let hw_locked = crate::driver::wmi_ec::is_winkey_locked_hw();
            if hw_locked != cfg.win_key_locked {
                crate::driver::wmi_ec::send_winkey_lock_trigger();
            }
        }
        crate::system::switches::set_win_key_locked(cfg.win_key_locked);
        crate::log_info!("POWER_RESTORE", "Startup restored Win Key lock state: locked={}", cfg.win_key_locked);

        // Restore Touchpad state if disabled by user
        if !cfg.touchpad_enabled {
            crate::driver::wmi_ec::wmi_modify_ec_mask(
                crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE,
                crate::driver::wmi_ec::TOUCHPAD_BITMASK,
                false,
            );
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new("C:\\Windows\\System32\\SystemSettingsAdminFlows.exe")
                    .creation_flags(0x08000000)
                    .args(["EnableTouchpad", "0"])
                    .output();
            }
            crate::system::switches::touchpad::write_ptp_registry_enabled(false);
            crate::log_info!("POWER_RESTORE", "Startup restored touchpad state: disabled");
        }

        // Restore Microphone state if muted by user
        if !cfg.mic_enabled {
            let _ = crate::system::switches::audio::set_microphone_state(false);
            crate::log_info!("POWER_RESTORE", "Startup restored microphone state: muted");
        }

        // Restore Camera EC cutoff state if disabled by user
        if !cfg.camera_enabled {
            if let Some(mut val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF) {
                val |= crate::driver::wmi_ec::CAMERA_CUTOFF_BITMASK;
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF, val);
            }
            crate::log_info!("POWER_RESTORE", "Startup restored camera state: disabled/cutoff");
        }

        // Restore Fn Lock state from persistent config (Bit 4: 0x11 = Locked, 0x01 = Unlocked)
        if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FN_LOCK) {
            let target_v = if cfg.fn_lock_enabled { v | 0x10 } else { v & !0x10 };
            if v != target_v {
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_FN_LOCK, target_v);
                crate::log_info!("POWER_RESTORE", "Startup restored persistent Fn Lock state: enabled={}", cfg.fn_lock_enabled);
            }
        }
        // Restore Keep Awake state from persistent config
        if cfg.keep_awake_enabled {
            crate::system::switches::set_keep_awake(true);
            crate::log_info!("POWER_RESTORE", "Startup restored keep awake state: enabled=true");
        }
        // Restore Wi-Fi radio state if explicitly disabled by user
        if !cfg.wifi_enabled {
            let _ = crate::system::switches::radio::set_wifi_radio_state_native(false);
            crate::log_info!("POWER_RESTORE", "Startup restored Wi-Fi state: disabled");
        }
        // Restore Bluetooth radio state if explicitly disabled by user
        if !cfg.bluetooth_enabled {
            let _ = crate::system::switches::radio::set_native_bluetooth_radio_state(false);
            crate::log_info!("POWER_RESTORE", "Startup restored Bluetooth state: disabled");
        }
        // Auto-heal Task Scheduler startup task to guarantee current executable path alignment & 2-second delay buffer
        crate::system::autostart::sync_autostart_target_path();
        // Restore user preferred Windows Power Scheme if explicitly configured
        if let Some(ref target_guid) = cfg.preferred_power_scheme {
            if let Err(e) = ppm::set_active_windows_power_scheme(target_guid) {
                crate::log_warn!("POWER_RESTORE", "Failed to restore preferred Windows power scheme {}: {}", target_guid, e);
            }
        }
        if cfg.high_perf_scheme {
            crate::system::power_mode::apply_windows_power_overlay(cfg.power_mode);
        }
        // Water cooler module check: user manual override or physical EC detection
        let is_water_cooler = crate::system::config::is_water_cooler_enabled();
        if is_water_cooler {
            let cooler = crate::driver::ble_cooler::get_global_cooler();
            if !cooler.get_telemetry().connected {
                if let Some(paired) = crate::driver::ble_cooler::find_paired_water_cooler_mac() {
                    if cooler.get_mac() != paired {
                        crate::log_info!("POWER_RESTORE", "Boot restored water cooler target MAC to Windows PnP paired device: {:012X}", paired);
                        cooler.set_mac(paired);
                    }
                }
            }
            cooler.resume_after_wake();
            cooler.start_active_cooling();
        }

        // Restore lighting presets seamlessly on startup
        std::thread::sleep(std::time::Duration::from_millis(600));
        let lighting = crate::driver::ite_lighting::get_global_lighting();
        lighting.refresh_devices();
        if !crate::driver::ite_streamer::get_global_streamer().is_running() {
            lighting.restore_all();
            crate::driver::ite_streamer::IteStreamerController::init_from_saved_config();
        }

        // 26H2 delayed USB HID enumeration fallback: if streamer should be active but was skipped, retry after settling
        if cfg.lighting.kb_engine == "streamer" && !crate::driver::ite_streamer::get_global_streamer().is_running() {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            crate::driver::ite_streamer::IteStreamerController::invalidate_probe_cache();
            lighting.refresh_devices();
            crate::driver::ite_streamer::IteStreamerController::init_from_saved_config();
        }

        crate::log_info!("LIGHTING", "Startup lighting presets successfully restored to hardware & BetterRGB Streamer");
        crate::system::boot_trace::boot_log("RESTORE", "Startup hardware & lighting restoration completed successfully");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::config::FanCurvePoint;

    #[test]
    fn test_custom_fan_table_gpu_fix() {
        let cpu_curve = vec![
            FanCurvePoint { temp: 45, duty: 25 },
            FanCurvePoint { temp: 60, duty: 45 },
            FanCurvePoint { temp: 70, duty: 60 },
            FanCurvePoint { temp: 80, duty: 80 },
            FanCurvePoint { temp: 90, duty: 100 },
        ];
        let gpu_curve = vec![
            FanCurvePoint { temp: 45, duty: 20 },
            FanCurvePoint { temp: 60, duty: 35 },
            FanCurvePoint { temp: 70, duty: 50 },
            FanCurvePoint { temp: 80, duty: 70 },
            FanCurvePoint { temp: 90, duty: 90 },
        ];

        let table = build_custom_fan_table(&cpu_curve, &gpu_curve);

        // 1. Step 0 must always be Zero-RPM standby
        assert_eq!(table.cpu_duty[0], 0);
        assert_eq!(table.gpu_duty[0], 0);

        // 2. GPU UpT must not be stuck at 65°C across steps 1..5
        assert!(table.gpu_upt[1] <= 55, "GPU UpT step 1 should be <= 55°C, got {}", table.gpu_upt[1]);
        assert_ne!(table.gpu_upt[1], table.gpu_upt[5], "GPU UpT steps 1..5 should have progression");

        // 3. Monotonic non-decreasing progression
        for i in 1..16 {
            assert!(table.cpu_duty[i] >= table.cpu_duty[i - 1], "CPU duty monotonic check failed at step {}", i);
            assert!(table.gpu_duty[i] >= table.gpu_duty[i - 1], "GPU duty monotonic check failed at step {}", i);
        }

        // 4. Test all-zero profile safety: must not panic, step 0 is 0
        let zero_curve = vec![
            FanCurvePoint { temp: 45, duty: 0 },
            FanCurvePoint { temp: 90, duty: 0 },
        ];
        let zero_table = build_custom_fan_table(&zero_curve, &zero_curve);
        assert_eq!(zero_table.cpu_duty[0], 0);
        assert_eq!(zero_table.gpu_duty[0], 0);
    }
}
