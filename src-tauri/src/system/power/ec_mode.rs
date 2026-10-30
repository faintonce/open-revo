use super::fan_table::{apply_fan_table_to_ec, apply_mode_fan_table, build_custom_fan_table};

/// Set Physical Motherboard EC Hardware Registers
pub fn apply_ec_hardware_mode(ui_mode: u8) {
    // 0. If current driver is Declarative and DOES NOT use Tongfang ACPI kernel driver channel,
    // route performance mode application via HAL and avoid poking legacy registers.
    // If Tongfang ACPI hardware channel is available (both gaming laptops and Tongfang ultrabooks like Wujie 14X/15X),
    // we MUST proceed with writing native Tongfang EC registers (APExist, MODE_INDEX, TRIGGER_BYTE, etc.)!
    let is_tongfang_hardware = crate::driver::wmi_ec::is_acpi_driver_available()
        || crate::hal::declarative::load_matched_manifest()
            .map(|m| m.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. })))
            .unwrap_or(false);

    if !is_tongfang_hardware {
        let hw_mode = match ui_mode {
            1 => crate::hal::HardwarePowerMode::Office,
            2 => crate::hal::HardwarePowerMode::Balance,
            3 => crate::hal::HardwarePowerMode::Turbo,
            4 => crate::hal::HardwarePowerMode::Custom,
            _ => crate::hal::HardwarePowerMode::Balance,
        };
        let hal = crate::hal::get_hardware_driver();
        if let Err(e) = hal.set_performance_mode(hw_mode) {
            crate::log_warn!("POWER_MODE", "HAL set_performance_mode failed: {}", e);
        } else {
            crate::log_info!("POWER_MODE", "Successfully applied power mode via HAL ({:?})", hal.kind());
        }
        return;
    }

    use crate::driver::wmi_ec;

    crate::log_info!("POWER_MODE", "Writing EC hardware registers for UI mode {}...", ui_mode);

    // A. Assert APExist so motherboard grants control to userland
    if let Some(val) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_AP_OEM_BYTE) {
        if (val & 0x01) == 0 {
            let ok = wmi_ec::wmi_write_ec(wmi_ec::ADDR_AP_OEM_BYTE, val | 0x01);
            crate::log_info!("POWER_MODE", "Assert APExist result: {}", ok);
        }
    }

    let _is_amd = crate::system::cpu_monitor::is_amd_cpu();
    let _factory_tcc = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TURBO_TCC).unwrap_or(0);

    let cfg = crate::system::config::load_config();
    let fan_boost_active = cfg.fan_boost;

    // Sanitize and align Fan Boost hardware state:
    // If user has NOT explicitly activated Fan Boost, ensure ADDR_FAN_BOOST_CLAMP is cleared
    // so the fan returns to automatic EC curve control.
    if !fan_boost_active {
        if wmi_ec::is_legacy_dual_mode_ec() {
            let cur_clamp = wmi_ec::wmi_read_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP).unwrap_or(0);
            if (cur_clamp & wmi_ec::FAN_BOOST_CLAMP_BIT) != 0 {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, 0x10);
            }
        } else {
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP) {
                if (v & wmi_ec::FAN_BOOST_CLAMP_BIT) != 0 {
                    wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, v & !wmi_ec::FAN_BOOST_CLAMP_BIT);
                }
            }
        }
    }

    // Helper to read hardware factory presets with graceful fallback
    let _read_preset = |addr: u16, fallback: u8| -> u8 {
        match wmi_ec::wmi_read_ec(addr) {
            Some(val) if val > 0 && val < 255 => val,
            _ => fallback,
        }
    };

    // Helper to safely write EC ADDR_MODE_INDEX without corrupting Logo brightness (bits [7:5]) or Logo hardware support (bit 0)
    let set_ec_mode_index = |target_mode_code: u8| {
        let cur_mode = wmi_ec::wmi_read_ec(wmi_ec::ADDR_MODE_INDEX).unwrap_or(0);
        let br_bits = cur_mode & 0xE0;
        let is_ec_logo = crate::driver::ite_lighting::IteLightingManager::is_ec_logo_supported();
        let support_bit = if is_ec_logo { 0x01 } else { cur_mode & 0x01 };
        let mode_bits = target_mode_code & 0x1E;
        let new_mode = br_bits | mode_bits | support_bit;
        wmi_ec::wmi_write_ec(wmi_ec::ADDR_MODE_INDEX, new_mode);
    };

    let active_profile = crate::system::model_engine::get_active_profile();
    let user_presets = active_profile.as_ref().and_then(|p| p.presets.as_ref());

    match ui_mode {
        1 => {
            // --- Office / Silent Mode (Green LED) ---
            // Clear Custom White LED override (preserve Bit 3 AC Recovery and Camera)
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_AC_RECOVERY) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_AC_RECOVERY, v & !wmi_ec::MASK_QKEY_WHITE_1830);
            }
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CAMERA_CUTOFF) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_CAMERA_CUTOFF, v & !wmi_ec::MASK_QKEY_WHITE_1831);
            }

            // Physical Mode & Chassis Q-Key LED: Green (0xA0 on 40/50-series, 0x10/0x50 on legacy)
            if wmi_ec::is_legacy_dual_mode_ec() {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, if fan_boost_active { 0x50 } else { 0x10 });
            } else {
                let base_clamp = 0xA0;
                let val_clamp = if fan_boost_active { base_clamp | wmi_ec::FAN_BOOST_CLAMP_BIT } else { base_clamp };
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, val_clamp);
            }

            // 1. EC Mode Index: 0x00 (Office Mode, preserves Logo brightness & support bit 0)
            set_ec_mode_index(0x00);
            // 2. Trigger Silent Mode via ADDR_TRIGGER_BYTE (preserve Bit 4 USB Charging)
            let cur_trig = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TRIGGER_BYTE, (cur_trig & 0x10) | 0x28);
            // 3. Mark silent mode status
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TURBO_TRIGGER_1115, 1);

            // 4. Dynamic/Configured CPU Power Limits:
            // GROUND TRUTH RULE: If user has NOT set custom user_presets, DO NOT write power limit registers!
            // Yield directly to EC firmware to read factory table for zero distortion.
            if let Some(p) = user_presets {
                let pl1 = p.office.pl1;
                let pl2 = p.office.pl2;
                let pl4 = p.office.pl4;
                let tcc = p.office.temp_offset;
                wmi_ec::set_custom_pl_limits(pl1, pl2, pl4);
                wmi_ec::apply_preset_tcc_adaptive(tcc);
                crate::log_info!("POWER_MODE", "Office Mode: User presets applied -> PL1={}W, PL2={}W, PL4={}W, TccOffset={}°C", pl1, pl2, pl4, tcc);
            } else {
                wmi_ec::set_cpu_temp_target(false, 95);
                crate::log_info!("POWER_MODE", "Office Mode: Yielding PL1/PL2/PL4 directly to EC native factory table. Zero manual override.");
            }

            // 5. Dynamic GPU DB & TGP (Eco / Quiet Baseline)
            // Neutralize any GPU overclocking offsets ONLY if OpenRevo custom mode had super_perf enabled
            if crate::driver::gpu_bridge::is_nvidia_dgpu_available() {
                let cfg = crate::system::config::load_config();
                if cfg.custom_profile.gpu_super_perf {
                    let _ = crate::driver::nvapi::reset_gpu_clock_offsets();
                    let gpu = crate::driver::gpu_bridge::get_global_gpu();
                    gpu.lock().set_clock_offsets(0, 0);
                }

                let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
                if gpu_caps.supports_gpu_tuning {
                    let actuator = crate::system::actuator::UnifiedPowerActuator::global();
                    let floor = gpu_caps.ctgp_floor_watts as u8;
                    let max_base = gpu_caps.base_tgp_max_watts as u8;
                    let max_offset = max_base.saturating_sub(floor);
                    let _ = actuator.apply_gpu_state(
                        Some(false),
                        Some(false),
                        Some(0),
                        Some(max_offset),
                        "Office Mode Eco Baseline",
                    );
                }
            }

            // 6. Fan Transition Speed (0x83 = enable smoothing filter at 300ms/0.5%)
            wmi_ec::set_custom_mode_ec(false);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_RAMP_RATE, 0x83);
            wmi_ec::set_fan_isolated_output(false);
            // 7. Apply Silent Fan Table
            apply_mode_fan_table(1);
        }
        2 => {
            // --- Balance / Gaming Mode (Blue LED) ---
            // Clear Custom White LED override (preserve Bit 3 AC Recovery and Camera)
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_AC_RECOVERY) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_AC_RECOVERY, v & !wmi_ec::MASK_QKEY_WHITE_1830);
            }
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CAMERA_CUTOFF) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_CAMERA_CUTOFF, v & !wmi_ec::MASK_QKEY_WHITE_1831);
            }

            // Physical Mode & Chassis Q-Key LED: Blue (0x00 on 40/50-series, 0x10/0x50 on legacy)
            if wmi_ec::is_legacy_dual_mode_ec() {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, if fan_boost_active { 0x50 } else { 0x10 });
            } else {
                let base_clamp = 0x00;
                let val_clamp = if fan_boost_active { base_clamp | wmi_ec::FAN_BOOST_CLAMP_BIT } else { base_clamp };
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, val_clamp);
            }

            // 1. EC Mode Index: 0x00 (Balance Mode, preserves Logo brightness & support bit 0)
            set_ec_mode_index(0x00);
            // 2. Clear Trigger Byte (preserve Bit 4 USB Charging)
            let cur_trig = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TRIGGER_BYTE, (cur_trig & 0x10) | 0x20);
            // 3. Clear silent mode status
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TURBO_TRIGGER_1115, 0);

            // 4. Dynamic/Configured CPU Power Limits:
            // GROUND TRUTH RULE: If user has NOT set custom user_presets, DO NOT write power limit registers!
            // Yield directly to EC firmware to read factory table for zero distortion.
            if let Some(p) = user_presets {
                let pl1 = p.balanced.pl1;
                let pl2 = p.balanced.pl2;
                let pl4 = p.balanced.pl4;
                let tcc = p.balanced.temp_offset;
                wmi_ec::set_custom_pl_limits(pl1, pl2, pl4);
                wmi_ec::apply_preset_tcc_adaptive(tcc);
                crate::log_info!("POWER_MODE", "Balanced Mode: User presets applied -> PL1={}W, PL2={}W, PL4={}W, TccOffset={}°C", pl1, pl2, pl4, tcc);
            } else {
                wmi_ec::set_cpu_temp_target(false, 95);
                crate::log_info!("POWER_MODE", "Balanced Mode: Yielding PL1/PL2/PL4 directly to EC native factory table. Zero manual override.");
            }

            // 5. Dynamic GPU DB & TGP (Full factory Base TGP and DB capacity, discrete GPU only)
            // Neutralize any GPU overclocking offsets ONLY if OpenRevo custom mode had super_perf enabled
            if crate::driver::gpu_bridge::is_nvidia_dgpu_available() {
                let cfg = crate::system::config::load_config();
                if cfg.custom_profile.gpu_super_perf {
                    let _ = crate::driver::nvapi::reset_gpu_clock_offsets();
                    let gpu = crate::driver::gpu_bridge::get_global_gpu();
                    gpu.lock().set_clock_offsets(0, 0);
                }

                let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
                if gpu_caps.supports_gpu_tuning {
                    let actuator = crate::system::actuator::UnifiedPowerActuator::global();
                    let db = gpu_caps.max_db_watts.clamp(5, 35) as u8;
                    let floor = gpu_caps.ctgp_floor_watts as u8;
                    let max_base = gpu_caps.base_tgp_max_watts as u8;
                    let max_offset = max_base.saturating_sub(floor);
                    let _ = actuator.apply_gpu_state(
                        Some(false),
                        Some(true),
                        Some(db),
                        Some(max_offset),
                        "Balanced Mode Factory Baseline",
                    );
                }
            }

            // 6. Fan Transition Speed (0x83 = enable smoothing filter at 300ms/0.5%)
            wmi_ec::set_custom_mode_ec(false);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_RAMP_RATE, 0x83);
            wmi_ec::set_fan_isolated_output(false);
            // 7. Apply Balanced Gaming Fan Table
            apply_mode_fan_table(2);
        }
        3 => {
            // --- Beast / Turbo Mode (Red LED) ---
            // Clear Custom White LED override (preserve Bit 3 AC Recovery and Camera)
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_AC_RECOVERY) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_AC_RECOVERY, v & !wmi_ec::MASK_QKEY_WHITE_1830);
            }
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CAMERA_CUTOFF) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_CAMERA_CUTOFF, v & !wmi_ec::MASK_QKEY_WHITE_1831);
            }

            // Physical Mode & Chassis Q-Key LED: Red (0x10 on 40/50-series, 0x10/0x50 on legacy)
            if wmi_ec::is_legacy_dual_mode_ec() {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, if fan_boost_active { 0x50 } else { 0x10 });
            } else {
                let val_clamp = if fan_boost_active { 0x10 | wmi_ec::FAN_BOOST_CLAMP_BIT } else { 0x10 };
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, val_clamp);
            }

            // 1. EC Mode Index: 0x10 (Turbo Mode, preserves Logo brightness & support bit 0)
            set_ec_mode_index(0x10);
            // 2. Clear Trigger Byte (preserve Bit 4 USB Charging)
            let cur_trig = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TRIGGER_BYTE, (cur_trig & 0x10) | 0x20);
            // 3. Clear silent mode status
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TURBO_TRIGGER_1115, 0);

            // 4. Dynamic/Configured CPU Power Limits:
            // GROUND TRUTH RULE: If user has NOT set custom user_presets, DO NOT write power limit registers!
            // Yield directly to EC firmware to read factory table for zero distortion.
            if let Some(p) = user_presets {
                let pl1 = p.turbo.pl1;
                let pl2 = p.turbo.pl2;
                let pl4 = p.turbo.pl4;
                let tcc = p.turbo.temp_offset;
                wmi_ec::set_custom_pl_limits(pl1, pl2, pl4);
                wmi_ec::apply_preset_tcc_adaptive(tcc);
                crate::log_info!("POWER_MODE", "Turbo Mode: User presets applied -> PL1={}W, PL2={}W, PL4={}W, TccOffset={}°C", pl1, pl2, pl4, tcc);
            } else {
                wmi_ec::set_cpu_temp_target(false, 95);
                crate::log_info!("POWER_MODE", "Turbo Mode: Yielding PL1/PL2/PL4 directly to EC native factory table. Zero manual override.");
            }

            // 5. Dynamic GPU DB & TGP (Full factory Base TGP and DB capacity, discrete GPU only)
            // Neutralize any GPU overclocking offsets ONLY if OpenRevo custom mode had super_perf enabled
            if crate::driver::gpu_bridge::is_nvidia_dgpu_available() {
                let cfg = crate::system::config::load_config();
                if cfg.custom_profile.gpu_super_perf {
                    let _ = crate::driver::nvapi::reset_gpu_clock_offsets();
                    let gpu = crate::driver::gpu_bridge::get_global_gpu();
                    gpu.lock().set_clock_offsets(0, 0);
                }

                let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
                if gpu_caps.supports_gpu_tuning {
                    let actuator = crate::system::actuator::UnifiedPowerActuator::global();
                    let db = gpu_caps.max_db_watts.clamp(5, 35) as u8;
                    let floor = gpu_caps.ctgp_floor_watts as u8;
                    let max_base = gpu_caps.base_tgp_max_watts as u8;
                    let max_offset = max_base.saturating_sub(floor);
                    let _ = actuator.apply_gpu_state(
                        Some(true),
                        Some(true),
                        Some(db),
                        Some(max_offset),
                        "Turbo Mode Full Assertion",
                    );
                }
            }

            // 6. Fan Transition Speed: 0x00 (Zero latency instantaneous turbo fan response)
            wmi_ec::set_custom_mode_ec(false);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_RAMP_RATE, 0x00);
            wmi_ec::set_fan_isolated_output(false);
            // 7. Apply Turbo Fan Table
            apply_mode_fan_table(3);
        }
        4 => {
            // --- Custom Mode (White LED & User Custom Profile) ---
            // A. Pre-apply Diagnostic EC Dump
            let pre_pwr_mode = wmi_ec::wmi_read_ec(wmi_ec::ADDR_MODE_INDEX).unwrap_or(0);
            let pre_fan_clamp = wmi_ec::wmi_read_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP).unwrap_or(0);
            let pre_trigger_byte = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
            let pre_turbo_flag = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TURBO_TRIGGER_1115).unwrap_or(0);
            let pre_pl1 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_PL1).unwrap_or(0);
            let pre_pl2 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_PL2).unwrap_or(0);
            let pre_pl4 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_PL4).unwrap_or(0);
            let pre_tcc = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_TCC).unwrap_or(0);
            let pre_db_ctrl = wmi_ec::wmi_read_ec(wmi_ec::ADDR_DB_CTRL1).unwrap_or(0);
            let pre_tgp_offset = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TGP_OFFSET).unwrap_or(0);
            let pre_db_watts = wmi_ec::wmi_read_ec(wmi_ec::ADDR_GPU_DB_MAX).unwrap_or(0);
            let pre_power_status = wmi_ec::wmi_read_ec(wmi_ec::ADDR_POWER_STATUS).unwrap_or(0);

            crate::log_info!(
                "CUSTOM_TRACE",
                "=== [CUSTOM MODE APPLY] Pre-Apply EC Dump ===\n\
                 Mode=0x{:02X}, Trigger=0x{:02X}, Boost=0x{:02X}, Silent={}\n\
                 PL1={}W, PL2={}W, PL4={}W, Tcc=0x{:02X}\n\
                 DB_Ctrl=0x{:02X}, cTGP={}, DB_Watts={}W, PowerStatus=0x{:02X}",
                pre_pwr_mode, pre_trigger_byte, pre_fan_clamp, pre_turbo_flag,
                pre_pl1, pre_pl2, pre_pl4, pre_tcc,
                pre_db_ctrl, pre_tgp_offset, pre_db_watts, pre_power_status
            );

            // B. Power Gating Check: DC Round Socket or valid AC
            let is_dc_online = crate::commands::hardware_cmd::is_dc_round_plugged_in();
            if !is_dc_online {
                crate::log_warn!(
                    "CUSTOM_TRACE",
                    "[WARNING: DC BARREL UNPLUGGED] PowerStatus=0x{:02X} (Bit0=0). Machine is running on Battery or Type-C PD ({}W). High-performance Custom Mode may trigger instant throttling!",
                    pre_power_status, (pre_power_status >> 1) & 0x1F
                );
            }

            // Physical Q-Key LED: Custom Mode (preserve Bit 3 AC Recovery) - Gaming laptops only
            if !crate::hal::platform_probe::is_ultrabook() {
                if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_AC_RECOVERY) {
                    wmi_ec::wmi_write_ec(wmi_ec::ADDR_AC_RECOVERY, v | wmi_ec::MASK_QKEY_WHITE_1830);
                } else {
                    wmi_ec::wmi_write_ec(wmi_ec::ADDR_AC_RECOVERY, wmi_ec::MASK_QKEY_WHITE_1830);
                }
                if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CAMERA_CUTOFF) {
                    wmi_ec::wmi_write_ec(wmi_ec::ADDR_CAMERA_CUTOFF, v | wmi_ec::MASK_QKEY_WHITE_1831);
                } else {
                    wmi_ec::wmi_write_ec(wmi_ec::ADDR_CAMERA_CUTOFF, wmi_ec::MASK_QKEY_WHITE_1831);
                }
            }
            
            // Read saved user custom profile
            let cfg = crate::system::config::load_config();
            let profile = cfg.custom_profile;

            crate::log_info!(
                "CUSTOM_TRACE",
                "Applying Profile Config: PL1={}W, PL2={}W, PL4={}W, TempTargetEn={}, TempTarget={}°C, SuperPerf={}, DB_En={}, DB_Watts={}W, FanRamp={}, FanIso={}",
                profile.cpu_pl1_watts, profile.cpu_pl2_watts, profile.cpu_pl4_watts,
                profile.cpu_temp_target_enabled, profile.cpu_temp_target,
                profile.gpu_super_perf, profile.gpu_db_enabled, profile.gpu_db_watts,
                profile.fan_ramp_rate, profile.fan_isolated_output
            );

            let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();

            // Physical Mode & Super Performance:
            // - Super Performance (HWOC): Follows user preference (profile.gpu_super_perf).
            //   When enabled, unlocks GPU hardware overclocking via NVAPI.
            //   When disabled, resets GPU clock offsets to 0 and allows strict Dynamic Boost power budgeting.
            //   In 10/20-series (Pascal/Turing), Super Performance is unsupported -> bypass bit 4.
            let cur_clamp = wmi_ec::wmi_read_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP).unwrap_or(0);
            let super_bit = if gpu_caps.supports_gpu_tuning && profile.gpu_super_perf {
                wmi_ec::SUPER_PERF_BIT
            } else {
                0
            };
            let boost_bit = if fan_boost_active { wmi_ec::FAN_BOOST_CLAMP_BIT } else { 0 };
            let new_clamp = (cur_clamp & !(wmi_ec::FAN_BOOST_CLAMP_BIT | wmi_ec::SUPER_PERF_BIT)) | super_bit | boost_bit;
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP, new_clamp);
            set_ec_mode_index(0x10);

            // Clear silent mode status
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TURBO_TRIGGER_1115, 0);

            // Apply or reset GPU Clock Offsets via native NVAPI
            if gpu_caps.supports_gpu_tuning {
                if profile.gpu_super_perf {
                    if let Err(e) = crate::driver::nvapi::set_gpu_clock_offsets(
                        profile.gpu_core_offset_mhz,
                        profile.gpu_mem_offset_mhz,
                    ) {
                        crate::log_warn!("POWER_MODE", "Failed to apply GPU clock offsets: {}", e);
                    } else {
                        let gpu = crate::driver::gpu_bridge::get_global_gpu();
                        gpu.lock().set_clock_offsets(profile.gpu_core_offset_mhz, profile.gpu_mem_offset_mhz);
                    }
                } else {
                    // Critical Guard: When gpu_super_perf is disabled, do NOT touch or reset NVAPI clock offsets.
                    // This prevents wiping out external tuning tools (e.g. MSI Afterburner, GPU Curve Tuner plugin).
                    crate::log_info!("POWER_MODE", "Custom Mode: gpu_super_perf is OFF, bypassing GPU clock offset overrides.");
                }
            }

            // 1. CPU Power limits & Thermals
            let is_cpu_tuning_disabled = crate::hal::declarative::load_matched_manifest()
                .map(|m| m.capabilities.cpu_tuning_type.as_deref() == Some("none"))
                .unwrap_or(false);

            if is_cpu_tuning_disabled {
                wmi_ec::set_cpu_temp_target(false, 95);
                crate::log_info!("POWER_MODE", "Custom Mode: Ultrabook CPU tuning disabled in manifest. Yielding PL limits to EC native factory table (e.g. 65W).");
            } else {
                wmi_ec::set_custom_pl_limits(profile.cpu_pl1_watts, profile.cpu_pl2_watts, profile.cpu_pl4_watts);
                wmi_ec::set_cpu_temp_target(profile.cpu_temp_target_enabled, profile.cpu_temp_target);
            }

            // 2. GPU Power State: Atomically assert Base TGP, Dynamic Boost and cTGP via Unified Power Actuator
            // Strictly respects user's custom profile from config.json (DB enabled/disabled, Super Performance, etc.)
            // Never blindly writes 0x07 or overwrites user's disabled DB state!
            if gpu_caps.supports_gpu_tuning {
                let actuator = crate::system::actuator::UnifiedPowerActuator::global();
                let max_db_cap = gpu_caps.max_db_watts.clamp(5, 35) as u8;
                let target_db_watts = if profile.gpu_super_perf {
                    max_db_cap
                } else {
                    profile.gpu_db_watts.clamp(5, max_db_cap)
                };

                let target_db_en = profile.gpu_db_enabled || profile.gpu_super_perf;

                let floor = gpu_caps.ctgp_floor_watts as u8;
                let max_base = gpu_caps.base_tgp_max_watts as u8;
                let target_offset = if profile.gpu_super_perf {
                    max_base.saturating_sub(floor)
                } else if profile.gpu_ctgp_enabled && profile.gpu_ctgp_watts > 0 {
                    profile.gpu_ctgp_watts.clamp(floor, max_base).saturating_sub(floor)
                } else {
                    max_base.saturating_sub(floor)
                };

                let _ = actuator.apply_gpu_state(
                    Some(profile.gpu_super_perf),
                    Some(target_db_en),
                    Some(target_db_watts),
                    Some(target_offset),
                    "Custom Mode Profile Truth",
                );
            }

            // 3. Hardware Trigger Latch: Assert Trigger Byte (preserve Bit 4 USB Charging)
            // Latch all CPU & GPU power/thermal configurations into EC/SMM state machine
            let cur_trig = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
            wmi_ec::wmi_write_ec(wmi_ec::ADDR_TRIGGER_BYTE, (cur_trig & 0x10) | 0x20);

            // 4. Custom Mode Hardware Switch & Fan strategies / Sensitivity
            wmi_ec::set_custom_mode_ec(true);
            wmi_ec::set_fan_ramp_rate(profile.fan_ramp_rate, profile.fan_ramp_speed_ms);
            
            // If custom fan isolated output / curve is enabled in profile, build and apply user curve; otherwise default to turbo fan table
            // If Fan Boost is active, do not latch curve into EC RAM to preserve full speed!
            if !fan_boost_active {
                if profile.fan_isolated_output || profile.fan_curve_enabled {
                    if wmi_ec::is_legacy_dual_mode_ec() {
                        crate::log_warn!("POWER_MODE", "Legacy Dual-Mode chassis does not support RamFan1p5 RAM curve injection, bypassing to protect fan FSM");
                    } else {
                        // Assert ADDR_FAN_ISOLATED Bit 7 (Fan Independent Output / Custom Fan Table Active)
                        wmi_ec::set_fan_isolated_output(true);

                        let custom_table = build_custom_fan_table(&profile.cpu_fan_curve, &profile.gpu_fan_curve);
                        apply_fan_table_to_ec(&custom_table);

                        crate::log_info!(
                            "POWER_MODE",
                            "Custom Fan Curve Activated: CustomMode asserted, FanIsolated asserted, ramp rate set, duties written to RAM"
                        );
                    }
                } else {
                    if !wmi_ec::is_legacy_dual_mode_ec() {
                        wmi_ec::set_fan_isolated_output(false);
                        apply_mode_fan_table(3);
                    }
                }
            } else {
                crate::log_info!("POWER_MODE", "Custom Mode: Fan Boost is active, skipping fan curve latch to preserve full fan boost speed");
            }

            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_BACKLIGHT_WHITE) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_BACKLIGHT_WHITE, v & !0x10);
            }

            // Re-confirm White Q-Key LED post-latch to guarantee hardware latch permanence
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_AC_RECOVERY) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_AC_RECOVERY, v | wmi_ec::MASK_QKEY_WHITE_1830);
            }
            if let Some(v) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CAMERA_CUTOFF) {
                wmi_ec::wmi_write_ec(wmi_ec::ADDR_CAMERA_CUTOFF, v | wmi_ec::MASK_QKEY_WHITE_1831);
            }

            crate::log_info!("POWER_MODE", "Custom Mode (Mode 4) successfully asserted with White Q-Key LED, 0x10 Turbo base & Trigger");
        }
        _ => {}
    }

    // Self-healing: if on legacy Pascal/Turing machine, verify and restore fan table
    wmi_ec::heal_legacy_fan_table_if_needed();

    // Hardware readback verification & Deep Telemetry
    let raw_mode_index = wmi_ec::wmi_read_ec(wmi_ec::ADDR_MODE_INDEX).unwrap_or(0);
    let raw_trigger_byte = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
    let raw_fan_clamp = wmi_ec::wmi_read_ec(wmi_ec::ADDR_FAN_BOOST_CLAMP).unwrap_or(0);
    let raw_turbo_flag = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TURBO_TRIGGER_1115).unwrap_or(0);
    let raw_pl1 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_PL1).unwrap_or(0);
    let raw_pl2 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_PL2).unwrap_or(0);
    let raw_pl4 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_PL4).unwrap_or(0);
    let raw_tcc = wmi_ec::wmi_read_ec(wmi_ec::ADDR_CUSTOM_TCC).unwrap_or(0);
    let raw_db_ctrl1 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_DB_CTRL1).unwrap_or(0);
    let raw_tgp_offset = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TGP_OFFSET).unwrap_or(0);
    let raw_db_watts1 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_GPU_DB_MAX).unwrap_or(0);
    let raw_db_ctrl2 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_DB_CTRL2).unwrap_or(0);
    let raw_db_watts2 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_DB_WATTS2).unwrap_or(0);
    let raw_power_status = wmi_ec::wmi_read_ec(wmi_ec::ADDR_POWER_STATUS).unwrap_or(0);

    crate::log_info!(
        "POWER_MODE",
        "Hardware readback: ModeIndex=0x{:02X}, Trigger=0x{:02X}, PL1={}, TGP={}",
        raw_mode_index, raw_trigger_byte, raw_pl1, raw_tgp_offset
    );

    if ui_mode == 4 {
        crate::log_info!(
            "CUSTOM_TRACE",
            "=== [CUSTOM MODE READBACK VERIFICATION] ===\n\
             Mode=0x{:02X}, Trigger=0x{:02X}, SuperPerf=0x{:02X}, Silent={}\n\
             PL1={}W, PL2={}W, PL4={}W, Tcc=0x{:02X}\n\
             DB_Ctrl1=0x{:02X}, cTGP={}, DB_Watts1={}W, DB_Ctrl2=0x{:02X}, DB_Watts2={}W\n\
             PowerStatus=0x{:02X} (DC_Online={})",
            raw_mode_index, raw_trigger_byte, raw_fan_clamp, raw_turbo_flag,
            raw_pl1, raw_pl2, raw_pl4, raw_tcc,
            raw_db_ctrl1, raw_tgp_offset, raw_db_watts1, raw_db_ctrl2, raw_db_watts2,
            raw_power_status, (raw_power_status & wmi_ec::POWER_STATUS_DC_JACK) != 0
        );

        // Sanity check checks
        if raw_pl4 > 0 && raw_pl2 > 0 && raw_pl4 < (raw_pl2 + 20) {
            crate::log_warn!(
                "CUSTOM_TRACE",
                "[CRITICAL ANOMALY] PL4 ({}W) has less than 20W buffer above PL2 ({}W)! CPU VRM will trip emergency clamp (locking CPU to 0.6GHz) under transient burst!",
                raw_pl4, raw_pl2
            );
        }
        let cfg = crate::system::config::load_config();
        if cfg.custom_profile.gpu_super_perf && (raw_fan_clamp & wmi_ec::SUPER_PERF_BIT) == 0 {
            crate::log_warn!(
                "CUSTOM_TRACE",
                "[CRITICAL ANOMALY] Super Performance requested but hardware clamp bit is 0!"
            );
        }
        if raw_turbo_flag != 0 {
            crate::log_warn!(
                "CUSTOM_TRACE",
                "[ANOMALY] SilentMode status is active ({})! EC may override CPU fan curves and power targets.",
                raw_turbo_flag
            );
        }
    }
}
