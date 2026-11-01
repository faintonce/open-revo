// OpenRevo - Tuning, Custom Profiles, GPU Mode & Hardware Overclocking Commands
// Handles CPU frequency limits, GPU MUX modes, custom power limits, fan curves, presets, and BIOS NVRAM unlocks.

use tauri::{AppHandle, Emitter, State};
use crate::driver::ec_bridge::SharedEcBridge;
use crate::driver::gpu_bridge::SharedGpuBridge;
use super::power_cmd::{check_dgpu_present, is_dc_round_plugged_in, set_fan_boost};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GpuModeInfo {
    pub current_mode: String,        // "dgpu" | "hybrid" | "igpu" | "unknown"
    pub has_igpu_present: bool,      // true if Intel/AMD iGPU is currently visible in OS
    pub has_dgpu: bool,              // true if machine has a physical discrete GPU (NVIDIA / Radeon RX / Arc)
    pub gpu_name: String,            // "NVIDIA GeForce RTX 4090 Laptop GPU"
    pub cpu_guard_active: bool,      // true if 14900HX safe frequency limit is active
    pub platform: String,            // "intel" | "amd"
    pub cpu_name: String,            // Full CPU Brand Name
    pub hardware_read: bool,         // true if read directly from SPI Flash NVRAM
    pub error: Option<String>,
    pub capabilities: crate::driver::uefi_nvram::GpuCapabilities,
}

static IGPU_CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub fn check_igpu_present() -> bool {
    *IGPU_CACHE.get_or_init(|| {
        let snap = crate::system::hardware_probe::get_hardware_snapshot_cached();
        for gpu in &snap.gpu_environment.all_gpu_devices {
            let g = gpu.to_lowercase();
            if g.contains("intel") || g.contains("radeon") || g.contains("uhd") || g.contains("iris") {
                return true;
            }
        }
        // CPU-level hardware assertion: All Intel 12/13/14th Gen mobile HX/H CPUs (e.g. 14900HX)
        // and AMD mobile APUs physically possess integrated graphics, even if disconnected during dGPU Direct mode.
        let (cpu, is_amd) = crate::driver::uefi_nvram::get_cpu_info();
        let u = cpu.to_uppercase();
        if !is_amd && (u.contains("HX") || u.contains("H") || u.contains("CORE")) {
            return true;
        }
        if is_amd && (u.contains("HS") || u.contains("HX") || u.contains("H") || u.contains("RYZEN")) {
            return true;
        }
        true
    })
}

#[tauri::command]
pub fn get_gpu_mode_info(
    gpu_state: State<'_, SharedGpuBridge>,
) -> GpuModeInfo {
    let mut cfg = crate::system::config::load_config();
    let hw = crate::driver::uefi_nvram::read_gpu_mode();
    let capabilities = crate::driver::uefi_nvram::probe_gpu_capabilities();
    let has_igpu = check_igpu_present();
    let has_dgpu = if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        manifest.capabilities.discrete_gpu && check_dgpu_present()
    } else {
        check_dgpu_present()
    };
    let gpu_name = gpu_state.lock().get_name(0);

    let effective_mode = if !has_dgpu {
        "igpu".to_string()
    } else if hw.hardware_read && hw.current_mode != "unknown" {
        if cfg.gpu_mode != hw.current_mode {
            cfg.gpu_mode = hw.current_mode.clone();
            crate::system::config::save_config(&cfg);
        }
        hw.current_mode.clone()
    } else {
        cfg.gpu_mode.clone()
    };

    GpuModeInfo {
        current_mode: effective_mode,
        has_igpu_present: has_igpu,
        has_dgpu,
        gpu_name,
        cpu_guard_active: cfg.cpu_safety_guard,
        platform: hw.platform,
        cpu_name: hw.cpu_name,
        hardware_read: hw.hardware_read,
        error: hw.error,
        capabilities,
    }
}

#[tauri::command]
pub fn set_gpu_mode(
    app: AppHandle,
    mode: String,
    restart: Option<bool>,
    gpu_state: State<'_, SharedGpuBridge>,
) -> Result<GpuModeInfo, String> {
    if !check_dgpu_present() {
        return Err("当前设备为纯核显架构轻薄本（无物理独立显卡与 MUX 切换芯片），无需且不支持显卡模式切换。".to_string());
    }

    let caps = crate::driver::uefi_nvram::probe_gpu_capabilities();
    if mode.eq_ignore_ascii_case("igpu") && !caps.supports_igpu_only {
        let reason = caps.reason_igpu_unsupported.unwrap_or_else(|| {
            "当前模具硬件架构不支持纯核显模式 (iGPU Only)。已拦截写入以彻底防止主板黑屏变砖！".to_string()
        });
        return Err(reason);
    }

    let should_restart = restart.unwrap_or(false);

    let write_res = crate::driver::uefi_nvram::write_gpu_mode(&mode);
    if let Err(ref e) = write_res {
        eprintln!("[OpenRevo GPU Mode] Hardware write failed: {}", e);
        return Err(format!("写入主板显卡模式寄存器/NVRAM 失败: {}\n请确认是否以管理员身份运行 OpenRevo！", e));
    }

    if should_restart {
        crate::log_info!("GPU_MODE", "Triggering reinforced graceful system reboot for MUX switch to take effect...");
        crate::driver::uefi_nvram::trigger_graceful_reboot(1800)?;
    }

    let mut cfg = crate::system::config::load_config();
    cfg.gpu_mode = mode;
    crate::system::config::save_config(&cfg);

    let info = get_gpu_mode_info(gpu_state);
    let _ = app.emit("gpu-mode-changed", info.clone());

    Ok(info)
}

#[tauri::command]
pub fn hot_switch_gpu_mode(
    app: AppHandle,
    target_mode: String,
    gpu_state: State<'_, SharedGpuBridge>,
) -> Result<GpuModeInfo, String> {
    crate::log_warn!("GPU_MODE", "hot_switch_gpu_mode is neutralized in favor of reinforced hardware NVRAM switch.");
    set_gpu_mode(app, target_mode, Some(false), gpu_state)
}

#[tauri::command]
pub fn set_experimental_igpu_unlock(
    app: AppHandle,
    unlock: bool,
    gpu_state: State<'_, SharedGpuBridge>,
) -> Result<GpuModeInfo, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.allow_experimental_igpu_only = unlock;
    crate::system::config::save_config(&cfg);

    let info = get_gpu_mode_info(gpu_state);
    let _ = app.emit("gpu-mode-changed", info.clone());
    Ok(info)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CpuFreqInfo {
    pub enabled: bool,
    pub current_limit_mhz: u32,
    pub min_mhz: u32,
    pub max_mhz: u32,
    pub recommended_mhz: u32,
    pub is_intel_13_14: bool,
    pub cpu_brand: String,
    pub cpu_tjmax: u8,
}

#[tauri::command]
pub fn get_cpu_freq_info() -> CpuFreqInfo {
    let cfg = crate::system::config::load_config();
    let cpu_brand = crate::system::cpu_monitor::get_cached_cpu_brand();
    let is_intel_13_14 = crate::system::cpu_monitor::is_intel_13_or_14_gen();
    let max_mhz = crate::system::cpu_monitor::get_cpu_max_mhz();
    let cpu_tjmax = crate::system::cpu_monitor::get_cpu_native_tjmax();
    let min_mhz = 3600;
    let recommended_mhz = if is_intel_13_14 && max_mhz > 5400 { 5400 } else { max_mhz };

    let current_limit_mhz = if cfg.cpu_freq_limit > 0 {
        cfg.cpu_freq_limit.clamp(min_mhz, max_mhz)
    } else {
        recommended_mhz
    };

    CpuFreqInfo {
        enabled: cfg.cpu_safety_guard,
        current_limit_mhz,
        min_mhz,
        max_mhz,
        recommended_mhz,
        is_intel_13_14,
        cpu_brand,
        cpu_tjmax,
    }
}

/// Comprehensive deep unlock:
/// 1. Enumerates all power schemes on the machine (via powercfg /list, registry, and canonical GUIDs).
/// 2. Deep-scrubs all frequency limit registry keys (75b0ae3f-bce0-45a7-8c89-c9611c25e100, 101, 102) from every scheme.
/// 3. Resets PROCTHROTTLEMAX to 100%, PROCTHROTTLEMIN to 5%, and PERFBOOSTMODE to 2 (Aggressive).
/// 4. Flushes SCHEME_CURRENT via powercfg.
/// 5. Re-applies current EC power mode to ensure EC didn't stay stuck with a 25W PL1 clamp.
pub fn deep_unlock_cpu_frequency() -> Result<(), String> {
    crate::system::power::ppm::restore_official_power_plan().map(|_| ())
}

#[tauri::command]
pub fn set_cpu_freq_limit(_enable: bool, _mhz: u32) -> Result<u32, String> {
    crate::log_info!("POWER_MODE", "PPM CPU frequency limiting has been permanently deprecated. Operation neutralized.");
    Ok(0)
}

#[tauri::command]
pub fn reset_cpu_freq_limit() -> Result<bool, String> {
    crate::system::power::ppm::restore_official_power_plan()?;
    Ok(true)
}

#[tauri::command]
pub fn set_cpu_safety_guard(_enable: bool) -> Result<bool, String> {
    crate::log_info!("POWER_MODE", "CPU safety guard PPM has been permanently deprecated. Operation neutralized.");
    Ok(false)
}

#[tauri::command]
pub fn get_custom_profile() -> crate::system::config::CustomProfileConfig {
    let mut cfg = crate::system::config::load_config();
    let model_res = crate::system::model_engine::detect_model_profile();
    let model = &model_res.matched_profile;

    let is_generic_default = cfg.custom_profile.cpu_pl1_watts == 75
        && cfg.custom_profile.cpu_pl2_watts == 75
        && cfg.custom_profile.cpu_pl4_watts == 125
        && !cfg.custom_profile.fan_curve_enabled;

    if is_generic_default {
        let model_default = crate::system::model_engine::build_model_default_custom_profile(model);
        cfg.custom_profile = model_default;
    } else {
        let effective_max_db = (model.max_db as u8).max(35);
        if cfg.custom_profile.gpu_db_watts > effective_max_db {
            cfg.custom_profile.gpu_db_watts = effective_max_db;
        }
    }
    cfg.custom_profile
}

#[tauri::command]
pub fn reset_custom_profile_to_defaults() -> crate::system::config::CustomProfileConfig {
    let mut cfg = crate::system::config::load_config();
    let model_default = crate::system::model_engine::get_model_default_custom_profile();
    cfg.custom_profile = model_default.clone();
    crate::system::config::save_config(&cfg);
    model_default
}

#[tauri::command]
pub fn apply_custom_profile(
    profile: crate::system::config::CustomProfileConfig,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<(), String> {
    crate::log_info!(
        "CUSTOM_TRACE",
        "IPC 'apply_custom_profile' received: PL1={}W, PL2={}W, PL4={}W, TempEn={}, Temp={}°C, cTGP_En={}, cTGP_Watts={}W, DB_En={}, DB_Watts={}W, SuperPerf={}",
        profile.cpu_pl1_watts, profile.cpu_pl2_watts, profile.cpu_pl4_watts,
        profile.cpu_temp_target_enabled, profile.cpu_temp_target,
        profile.gpu_ctgp_enabled, profile.gpu_ctgp_watts,
        profile.gpu_db_enabled, profile.gpu_db_watts, profile.gpu_super_perf
    );

    let mut cfg = crate::system::config::load_config();
    if !is_dc_round_plugged_in() && !cfg.unlock_battery_power_limit {
        crate::log_warn!(
            "CUSTOM_TRACE",
            "apply_custom_profile rejected: Machine is NOT on full-power AC/DC adapter!"
        );
        let msg = if check_dgpu_present() {
            "在电池或 Type-C 供电下，无法应用自定义模式超频。请连接 DC 圆口电源适配器，或在【系统】设置中开启“解除离电限制”。"
        } else {
            "在电池供电下，无法应用自定义模式超频。请连接电源适配器，或在【系统】设置中开启“解除离电限制”。"
        };
        return Err(msg.to_string());
    }

    let is_dc = is_dc_round_plugged_in();
    cfg.custom_profile = profile.clone();
    cfg.power_mode = 4;
    if is_dc {
        cfg.power_mode_ac = 4;
    } else {
        cfg.power_mode_battery = 4;
    }
    crate::system::config::save_config(&cfg);

    // Declarative hardware dispatch on custom profile apply
    sync_custom_profile_to_declarative_hardware(&profile);

    ec_state.lock().set_power_mode(4);

    Ok(())
}

/// Dispatches custom profile SPL/sPPT/Temp parameters to declarative hardware (e.g. Baolongda MRID6/BM5235)
pub fn sync_custom_profile_to_declarative_hardware(profile: &crate::system::config::CustomProfileConfig) {
    if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        if manifest.id.contains("bld") {
            let p1 = (profile.cpu_pl1_watts as u8).clamp(25, 120);
            let p2 = (profile.cpu_pl2_watts as u8).clamp(25, 140);
            let temp = (profile.cpu_temp_target as u8).clamp(70, 100);
            let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 23, 2, p1]);
            let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 23, 3, p2]);
            if profile.cpu_temp_target_enabled {
                let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 23, 4, temp]);
            }
            if profile.fan_isolated_output || profile.fan_curve_enabled {
                crate::driver::bld_driver_mgr::set_bld_fan_curve(
                    profile.cpu_fan_curve.clone(),
                    profile.gpu_fan_curve.clone(),
                    profile.fan_ramp_rate,
                    profile.fan_ramp_speed_ms,
                );
                crate::log_info!("EC_TUNING", "BLD synced custom fan curve to 1Hz heartbeat arbiter");
            }
            crate::log_info!("EC_TUNING", "BLD synced custom profile: SPL={}W, sPPT={}W, Temp={}°C", p1, p2, temp);
        }
    }
}

#[tauri::command]
pub fn apply_fan_curve_live(
    app: AppHandle,
    ec_state: State<'_, SharedEcBridge>,
    cpu_curve: Vec<crate::system::config::FanCurvePoint>,
    gpu_curve: Vec<crate::system::config::FanCurvePoint>,
) -> Result<(), String> {
    let is_bld_manifest = crate::hal::declarative::load_matched_manifest()
        .map(|m| m.channels.values().any(|ch| matches!(ch, crate::hal::declarative::model_manifest::ChannelDef::WmiMailbox { .. })))
        .unwrap_or(false);

    if is_bld_manifest {
        let cfg = crate::system::config::load_config();
        crate::driver::bld_driver_mgr::set_bld_fan_curve(
            cpu_curve,
            gpu_curve,
            cfg.custom_profile.fan_ramp_rate,
            cfg.custom_profile.fan_ramp_speed_ms,
        );
        crate::log_info!("BLD_FAN", "BLD live fan curve applied to 1Hz heartbeat arbiter");
        return Ok(());
    }

    let cfg = crate::system::config::load_config();
    if (cfg.custom_profile.fan_isolated_output || cfg.custom_profile.fan_curve_enabled) && ec_state.lock().is_fan_boost() {
        crate::log_info!("POWER_MODE", "apply_fan_curve_live: Custom fan isolated output is enabled and Fan Boost is ON. Immediately terminating Fan Boost to yield control to live curve!");
        let _ = set_fan_boost(app, false, ec_state);
    }

    crate::driver::wmi_ec::set_custom_mode_ec(true);
    crate::driver::wmi_ec::set_fan_isolated_output(true);

    let table = crate::system::power_mode::build_custom_fan_table(&cpu_curve, &gpu_curve);
    crate::system::power_mode::apply_fan_table_to_ec(&table);

    Ok(())
}

#[tauri::command]
pub fn toggle_fan_isolated_output(
    app: AppHandle,
    ec_state: State<'_, SharedEcBridge>,
    enable: bool,
    cpu_curve: Option<Vec<crate::system::config::FanCurvePoint>>,
    gpu_curve: Option<Vec<crate::system::config::FanCurvePoint>>,
) -> Result<bool, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.custom_profile.fan_isolated_output = enable;
    cfg.custom_profile.fan_curve_enabled = enable;
    crate::system::config::save_config(&cfg);

    let is_bld_manifest = crate::hal::declarative::load_matched_manifest()
        .map(|m| m.channels.values().any(|ch| matches!(ch, crate::hal::declarative::model_manifest::ChannelDef::WmiMailbox { .. })))
        .unwrap_or(false);

    if is_bld_manifest {
        if enable {
            let c_curve = cpu_curve.unwrap_or_else(|| cfg.custom_profile.cpu_fan_curve.clone());
            let g_curve = gpu_curve.unwrap_or_else(|| cfg.custom_profile.gpu_fan_curve.clone());
            crate::driver::bld_driver_mgr::set_bld_fan_curve(
                c_curve,
                g_curve,
                cfg.custom_profile.fan_ramp_rate,
                cfg.custom_profile.fan_ramp_speed_ms,
            );
            crate::log_info!("BLD_FAN", "BLD fan isolated output enabled, running 1Hz curve heartbeat");
        } else {
            crate::driver::bld_driver_mgr::restore_bld_fan_auto();
            crate::log_info!("BLD_FAN", "BLD fan isolated output disabled, restored to factory auto thermal policy");
        }
        return Ok(enable);
    }

    if enable && ec_state.lock().is_fan_boost() {
        crate::log_info!("EC_TUNING", "toggle_fan_isolated_output: Enabling fan isolated output while Fan Boost is ON. Terminating Fan Boost to yield to custom curve!");
        let _ = set_fan_boost(app, false, ec_state);
    }

    if enable {
        crate::driver::wmi_ec::set_custom_mode_ec(true);
    }
    let ok = crate::driver::wmi_ec::set_fan_isolated_output(enable);

    if enable {
        let c_curve = cpu_curve.unwrap_or_else(|| cfg.custom_profile.cpu_fan_curve.clone());
        let g_curve = gpu_curve.unwrap_or_else(|| cfg.custom_profile.gpu_fan_curve.clone());
        let table = crate::system::power_mode::build_custom_fan_table(&c_curve, &g_curve);
        crate::system::power_mode::apply_fan_table_to_ec(&table);
    } else {
        crate::system::power_mode::apply_mode_fan_table(3);
    }
    crate::log_info!("EC_TUNING", "toggle_fan_isolated_output: enable={} -> ok={}", enable, ok);
    Ok(ok)
}

#[tauri::command]
pub fn toggle_fan_ramp_rate(enable: bool, speed_ms: Option<u16>) -> Result<bool, String> {
    let mut cfg = crate::system::config::load_config();
    let ms = speed_ms.unwrap_or(cfg.custom_profile.fan_ramp_speed_ms).clamp(100, 12700);
    let ok = crate::driver::wmi_ec::set_fan_ramp_rate(enable, ms);
    cfg.custom_profile.fan_ramp_rate = enable;
    cfg.custom_profile.fan_ramp_speed_ms = ms;
    crate::system::config::save_config(&cfg);
    crate::log_info!("EC_TUNING", "toggle_fan_ramp_rate: enable={}, speed={}ms -> ok={}", enable, ms, ok);
    Ok(ok)
}

#[tauri::command]
pub fn toggle_fan_curve_control(
    app: AppHandle,
    ec_state: State<'_, SharedEcBridge>,
    enable: bool,
    cpu_curve: Vec<crate::system::config::FanCurvePoint>,
    gpu_curve: Vec<crate::system::config::FanCurvePoint>,
) -> Result<(), String> {
    let _ = toggle_fan_isolated_output(app, ec_state, enable, Some(cpu_curve), Some(gpu_curve))?;
    Ok(())
}

#[tauri::command]
pub fn get_profiles_list() -> Result<Vec<crate::system::config::CustomProfilePreset>, String> {
    Ok(crate::system::config::list_profiles())
}

#[tauri::command]
pub fn get_default_builtin_presets() -> Result<Vec<crate::system::config::CustomProfilePreset>, String> {
    Ok(crate::system::config::get_default_builtin_presets())
}

#[tauri::command]
pub fn get_active_profile_id() -> Result<String, String> {
    Ok(crate::system::config::load_config().active_profile_id)
}

#[tauri::command]
pub fn set_active_profile_id(
    id: String,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<crate::system::config::CustomProfilePreset, String> {
    let profiles = crate::system::config::list_profiles();
    let target = profiles.into_iter().find(|p| p.id == id)
        .ok_or_else(|| format!("未找到指定的方案 ID: {}", id))?;

    let mut cfg = crate::system::config::load_config();
    cfg.active_profile_id = id.clone();
    cfg.custom_profile = target.profile.clone();
    let is_dc = is_dc_round_plugged_in();
    if is_dc {
        cfg.power_mode_ac = 4;
    } else {
        cfg.power_mode_battery = 4;
    }
    if is_dc || cfg.unlock_battery_power_limit {
        cfg.power_mode = 4;
        let is_declarative = crate::hal::declarative::load_matched_manifest().is_some();
        if is_declarative {
            let hal = crate::hal::get_hardware_driver();
            let _ = hal.set_performance_mode(crate::hal::HardwarePowerMode::Custom);
            sync_custom_profile_to_declarative_hardware(&target.profile);
            ec_state.lock().set_power_mode_from_hardware(4);
        } else {
            ec_state.lock().set_power_mode(4);
        }
    }
    crate::system::config::save_config(&cfg);
    Ok(target)
}

#[tauri::command]
pub fn save_profile_preset(
    preset: crate::system::config::CustomProfilePreset,
    _ec_state: State<'_, SharedEcBridge>,
) -> Result<(), String> {
    crate::system::config::save_profile_file(&preset)?;

    let mut cfg = crate::system::config::load_config();
    if cfg.active_profile_id == preset.id {
        cfg.custom_profile = preset.profile.clone();
        cfg.power_mode = 4;
        crate::system::config::save_config(&cfg);
        sync_custom_profile_to_declarative_hardware(&preset.profile);
    }
    crate::system::power_mode::sync_oem_mode4_profile(&preset.profile);
    Ok(())
}

#[tauri::command]
pub fn create_profile_preset(
    name: String,
    base_id: Option<String>,
    initial_profile: Option<crate::system::config::CustomProfileConfig>,
) -> Result<crate::system::config::CustomProfilePreset, String> {
    let base_profile = if let Some(init) = initial_profile {
        init
    } else if let Some(bid) = base_id {
        let profiles = crate::system::config::list_profiles();
        profiles.iter().find(|p| p.id == bid).map(|p| p.profile.clone())
            .unwrap_or_else(|| crate::system::config::load_config().custom_profile)
    } else {
        crate::system::config::load_config().custom_profile
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let id = format!("custom_{}", timestamp);
    let file_name = format!("profile_{}.json", timestamp);

    let preset = crate::system::config::CustomProfilePreset {
        id,
        name: if name.trim().is_empty() { "自定义方案".to_string() } else { name.trim().to_string() },
        file_name,
        icon_key: "star".to_string(),
        is_builtin: false,
        profile: base_profile,
    };

    crate::system::config::save_profile_file(&preset)?;
    Ok(preset)
}

#[tauri::command]
pub fn reset_profile_preset(id: String) -> Result<crate::system::config::CustomProfilePreset, String> {
    let target = if id.starts_with("preset_") {
        let defaults = crate::system::config::get_default_builtin_presets();
        let factory = defaults.into_iter().find(|p| p.id == id)
            .ok_or_else(|| format!("未找到原厂预设: {}", id))?;
        crate::system::config::save_profile_file(&factory)?;
        factory
    } else {
        let profiles = crate::system::config::list_profiles();
        let mut target = profiles.into_iter().find(|p| p.id == id)
            .ok_or_else(|| format!("未找到方案: {}", id))?;
        let default_custom = crate::system::model_engine::get_model_default_custom_profile();
        target.profile = default_custom;
        crate::system::config::save_profile_file(&target)?;
        target
    };

    let is_declarative = crate::hal::declarative::load_matched_manifest().is_some();
    if is_declarative {
        sync_custom_profile_to_declarative_hardware(&target.profile);
    } else {
        // Heal GPU hardware contract baseline immediately upon reset
        crate::driver::wmi_ec::heal_gpu_power_contract(
            target.profile.gpu_super_perf,
            Some(target.profile.gpu_db_watts),
        );

        // Heal CPU hardware contract baseline immediately upon reset (clear 105C/0x00 wall & break 55W Limp Mode)
        crate::driver::wmi_ec::heal_cpu_power_contract(
            target.profile.cpu_pl1_watts as u16,
            target.profile.cpu_pl2_watts as u16,
            target.profile.cpu_pl4_watts as u16,
            target.profile.cpu_temp_target_enabled,
            target.profile.cpu_temp_target,
        );
    }

    let mut cfg = crate::system::config::load_config();
    if cfg.active_profile_id == id {
        cfg.custom_profile = target.profile.clone();
        cfg.cpu_temp_target_max = target.profile.cpu_temp_target.max(95);
        let fp_opt = crate::system::power_mode::get_cached_factory_power_presets();
        let tur_pl1 = fp_opt.as_ref().map(|f| f.turbo_pl1).filter(|&v| v > 0).unwrap_or(90);
        let tur_pl2 = fp_opt.as_ref().map(|f| f.turbo_pl2).filter(|&v| v > 0).unwrap_or(105);
        let pl4_margin = crate::system::config::get_cpu_pl4_margin();
        let tur_pl4 = fp_opt.as_ref().map(|f| f.turbo_pl4).filter(|&v| v > 0).unwrap_or(tur_pl2.saturating_add(pl4_margin));
        let p1_max = (tur_pl1 + 25) as i32 + cfg.cpu_pl1_offset as i32;
        let p2_max = (tur_pl2 + 25) as i32 + cfg.cpu_pl2_offset as i32;
        let p4_max = (tur_pl4 + 50) as i32 + cfg.cpu_pl4_offset as i32;
        cfg.cpu_pl1_max = Some(p1_max.clamp(15, 255) as u16);
        cfg.cpu_pl2_max = Some(p2_max.clamp(15, 255) as u16);
        cfg.cpu_pl4_max = Some(p4_max.clamp(15, 255) as u16);
        crate::system::config::save_config(&cfg);
        crate::system::power_mode::sync_oem_mode4_profile(&target.profile);
    }

    Ok(target)
}

#[tauri::command]
pub fn heal_gpu_power_contract(
    super_perf_enable: Option<bool>,
    target_db_watts: Option<u8>,
) -> Result<bool, String> {
    if crate::hal::declarative::load_matched_manifest().is_some() {
        return Ok(true);
    }
    let cfg = crate::system::config::load_config();
    let super_perf = super_perf_enable.unwrap_or(cfg.custom_profile.gpu_super_perf);
    let db_watts = target_db_watts.or(Some(cfg.custom_profile.gpu_db_watts));
    let ok = crate::driver::wmi_ec::heal_gpu_power_contract(super_perf, db_watts);
    Ok(ok)
}

#[tauri::command]
pub fn heal_cpu_power_contract(
    pl1: Option<u16>,
    pl2: Option<u16>,
    pl4: Option<u16>,
    temp_target_enabled: Option<bool>,
    temp_target: Option<u8>,
) -> Result<bool, String> {
    if crate::hal::declarative::load_matched_manifest().is_some() {
        return Ok(true);
    }
    let cfg = crate::system::config::load_config();
    let p1 = pl1.unwrap_or(cfg.custom_profile.cpu_pl1_watts as u16);
    let p2 = pl2.unwrap_or(cfg.custom_profile.cpu_pl2_watts as u16);
    let p4 = pl4.unwrap_or(cfg.custom_profile.cpu_pl4_watts as u16);
    let temp_en = temp_target_enabled.unwrap_or(cfg.custom_profile.cpu_temp_target_enabled);
    let temp_c = temp_target.unwrap_or(cfg.custom_profile.cpu_temp_target);
    let ok = crate::driver::wmi_ec::heal_cpu_power_contract(p1, p2, p4, temp_en, temp_c);
    Ok(ok)
}

#[tauri::command]
pub fn rename_profile_preset(
    id: String,
    new_name: String,
) -> Result<crate::system::config::CustomProfilePreset, String> {
    if id.starts_with("preset_") {
        return Err("原厂内置方案不可重命名！".to_string());
    }
    let profiles = crate::system::config::list_profiles();
    let mut target = profiles.into_iter().find(|p| p.id == id)
        .ok_or_else(|| format!("未找到方案 ID: {}", id))?;
    if target.is_builtin {
        return Err("原厂内置方案不可重命名！".to_string());
    }

    target.name = new_name.trim().to_string();
    crate::system::config::save_profile_file(&target)?;
    Ok(target)
}

#[tauri::command]
pub fn delete_profile_preset(id: String) -> Result<(), String> {
    crate::system::config::delete_profile_file(&id)?;
    let mut cfg = crate::system::config::load_config();
    if cfg.active_profile_id == id {
        cfg.active_profile_id = "preset_silent_beast".to_string();
        crate::system::config::save_config(&cfg);
    }
    Ok(())
}

#[tauri::command]
pub fn open_profiles_folder() -> Result<(), String> {
    let dir = crate::system::config::get_profiles_dir();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(dir.to_string_lossy().to_string())
            .spawn();
    }
    Ok(())
}

#[tauri::command]
pub fn import_profile_preset(
    json_content: String,
    file_name: Option<String>,
) -> Result<crate::system::config::CustomProfilePreset, String> {
    crate::system::config::import_profile_json(&json_content, file_name)
}

#[cfg(target_os = "windows")]
pub fn save_file_dialog(default_name: &str, filter_name: &str, filter_ext: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwndOwner: isize,
        hInstance: isize,
        lpstrFilter: *const u16,
        lpstrCustomFilter: *mut u16,
        nMaxCustFilter: u32,
        nFilterIndex: u32,
        lpstrFile: *mut u16,
        nMaxFile: u32,
        lpstrFileTitle: *mut u16,
        nMaxFileTitle: u32,
        lpstrInitialDir: *const u16,
        lpstrTitle: *const u16,
        Flags: u32,
        nFileOffset: u16,
        nFileExtension: u16,
        lpstrDefExt: *const u16,
        lCustData: isize,
        lpfnHook: usize,
        lpTemplateName: *const u16,
        pvReserved: *mut std::ffi::c_void,
        dwReserved: u32,
        FlagsEx: u32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    }

    let mut file_buf = [0u16; 1024];
    let default_name_w: Vec<u16> = OsStr::new(default_name).encode_wide().collect();
    let copy_len = default_name_w.len().min(1023);
    file_buf[..copy_len].copy_from_slice(&default_name_w[..copy_len]);

    let filter_str = format!("{} (*.{})\0*.{}\0All Files (*.*)\0*.*\0\0", filter_name, filter_ext, filter_ext);
    let filter_w: Vec<u16> = filter_str.encode_utf16().collect();

    let def_ext_w: Vec<u16> = OsStr::new(filter_ext).encode_wide().chain(std::iter::once(0)).collect();
    let title_w: Vec<u16> = OsStr::new("导出配置文件").encode_wide().chain(std::iter::once(0)).collect();

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: 0,
        hInstance: 0,
        lpstrFilter: filter_w.as_ptr(),
        lpstrCustomFilter: std::ptr::null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: 1,
        lpstrFile: file_buf.as_mut_ptr(),
        nMaxFile: file_buf.len() as u32,
        lpstrFileTitle: std::ptr::null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: std::ptr::null(),
        lpstrTitle: title_w.as_ptr(),
        Flags: 0x00000002 | 0x00000800 | 0x00000004,
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: def_ext_w.as_ptr(),
        lCustData: 0,
        lpfnHook: 0,
        lpTemplateName: std::ptr::null(),
        pvReserved: std::ptr::null_mut(),
        dwReserved: 0,
        FlagsEx: 0,
    };

    let res = unsafe { GetSaveFileNameW(&mut ofn) };
    if res != 0 {
        let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
        use std::os::windows::ffi::OsStringExt;
        let path_str = std::ffi::OsString::from_wide(&file_buf[..len]);
        Some(std::path::PathBuf::from(path_str))
    } else {
        None
    }
}

#[tauri::command]
pub fn export_profile_preset(id: String) -> Result<Option<String>, String> {
    let profiles = crate::system::config::list_profiles();
    let target = profiles.into_iter().find(|p| p.id == id)
        .ok_or_else(|| format!("未找到方案 ID: {}", id))?;
    let content = serde_json::to_string_pretty(&target).map_err(|e| format!("导出序列化失败: {}", e))?;
    let default_name = target.file_name;

    #[cfg(target_os = "windows")]
    {
        if let Some(dest) = save_file_dialog(&default_name, "JSON 调教方案", "json") {
            std::fs::write(&dest, content).map_err(|e| format!("保存方案文件失败: {}", e))?;
            return Ok(Some(dest.to_string_lossy().to_string()));
        }
        Ok(None)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = default_name;
        Ok(Some(content))
    }
}

#[tauri::command]
pub fn apply_live_custom_tweak(
    app: AppHandle,
    pl1: Option<u16>,
    pl2: Option<u16>,
    pl4: Option<u16>,
    temp_target: Option<u8>,
    _ctgp_enabled: Option<bool>,
    ctgp_watts: Option<u8>,
    db_enabled: Option<bool>,
    db_watts: Option<u8>,
    super_perf: Option<bool>,
    gpu_core_offset: Option<i32>,
    gpu_mem_offset: Option<i32>,
    fan_speed_cap: Option<u16>,
    #[allow(non_snake_case)]
    fanSpeedCap: Option<u16>,
) -> Result<(), String> {
    use crate::driver::wmi_ec;
    let cfg = crate::system::config::load_config();
    if !is_dc_round_plugged_in() && !cfg.unlock_battery_power_limit {
        return Ok(());
    }

    // 0. Declarative Hardware Dispatch (e.g. Baolongda MRID6)
    if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        if manifest.id.contains("bld") {
            if let Some(p1) = pl1 {
                let clamped = (p1 as u8).clamp(25, 120);
                let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 23, 2, clamped]);
                crate::log_info!("EC_TUNING", "BLD applied CPU SPL (PL1) = {}W", clamped);
            }
            if let Some(p2) = pl2 {
                let clamped = (p2 as u8).clamp(25, 140);
                let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 23, 3, clamped]);
                crate::log_info!("EC_TUNING", "BLD applied CPU sPPT (PL2) = {}W", clamped);
            }
            if let Some(val) = temp_target {
                let clamped = val.clamp(70, 100);
                let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 23, 4, clamped]);
                crate::log_info!("EC_TUNING", "BLD applied CPU Temp Wall = {}°C", clamped);
            }
            let fan_cap = fanSpeedCap.or(fan_speed_cap);
            if let Some(cap) = fan_cap {
                if cap == 0 {
                    let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 20, 0]);
                    let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 21, 0]);
                    crate::log_info!("EC_TUNING", "BLD restored factory auto thermal policy");
                } else {
                    let cap_val = if cap > 100 { (cap / 100).clamp(28, 58) as u8 } else { (cap as u8).clamp(28, 58) };
                    let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 20, 1]);
                    let _ = crate::hal::declarative::invoke_raw_mailbox(&[0, 251, 0, 21, cap_val]);
                    crate::log_info!("EC_TUNING", "BLD set fan speed cap = {} ({}00 RPM)", cap_val, cap_val);
                }
            }
        }

        // Apply GPU clock offsets only if super_perf is active (or explicitly being turned off)
        let is_super_perf = super_perf.unwrap_or(cfg.custom_profile.gpu_super_perf);
        if is_super_perf || super_perf == Some(false) {
            if let (Some(c), Some(m)) = (gpu_core_offset, gpu_mem_offset) {
                let gpu = crate::driver::gpu_bridge::get_global_gpu();
                gpu.lock().set_clock_offsets(c, m);
            } else if let Some(c) = gpu_core_offset {
                let gpu = crate::driver::gpu_bridge::get_global_gpu();
                let m = gpu.lock().get_mem_offset();
                gpu.lock().set_clock_offsets(c, m);
            } else if let Some(m) = gpu_mem_offset {
                let gpu = crate::driver::gpu_bridge::get_global_gpu();
                let c = gpu.lock().get_core_offset();
                gpu.lock().set_clock_offsets(c, m);
            }
        }

        return Ok(());
    }

    wmi_ec::wmi_write_ec(wmi_ec::ADDR_TURBO_TRIGGER_1115, 0);

    let mut pl_or_temp_changed = false;
    if let (Some(p1), Some(p2), Some(p4)) = (pl1, pl2, pl4) {
        let (p1_safe, p2_safe, p4_safe) = wmi_ec::compute_safe_pl_limits(p1, p2, p4);
        wmi_ec::wmi_write_ec(wmi_ec::ADDR_CUSTOM_PL1, p1_safe);
        wmi_ec::wmi_write_ec(wmi_ec::ADDR_CUSTOM_PL2, p2_safe);
        wmi_ec::wmi_write_ec(wmi_ec::ADDR_CUSTOM_PL4, p4_safe);
        crate::log_info!(
            "EC_TUNING",
            "apply_live_custom_tweak atomic triplet: PL1={}W, PL2={}W, PL4={}W",
            p1_safe, p2_safe, p4_safe
        );
        pl_or_temp_changed = true;
    } else {
        if let Some(p1) = pl1 {
            wmi_ec::set_custom_pl1_ec(p1);
            pl_or_temp_changed = true;
        }
        if let Some(p2) = pl2 {
            wmi_ec::set_custom_pl2_ec(p2);
            pl_or_temp_changed = true;
        }
        if let Some(p4) = pl4 {
            wmi_ec::set_custom_pl4_ec(p4);
            pl_or_temp_changed = true;
        }
    }

    if let Some(val) = temp_target {
        wmi_ec::set_cpu_temp_target(true, val);
        pl_or_temp_changed = true;
    }

    if pl_or_temp_changed {
        wmi_ec::set_custom_mode_ec(true);
        let cur_trig = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
        wmi_ec::wmi_write_ec(wmi_ec::ADDR_TRIGGER_BYTE, (cur_trig & 0x10) | 0x20);
    }

    // Single Atomic Power Actuator replaces all historical ad-hoc GPU writes
    let actuator = crate::system::actuator::UnifiedPowerActuator::global();
    let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    let floor = gpu_caps.ctgp_floor_watts as u8;
    let target_offset = ctgp_watts.map(|w| w.saturating_sub(floor));

    let _ = actuator.apply_gpu_state(
        super_perf,
        db_enabled,
        db_watts,
        target_offset,
        "Custom Profile Live Tweak",
    );

    // Apply GPU clock offsets only if super_perf is active (or explicitly being turned off)
    let is_super_perf = super_perf.unwrap_or(cfg.custom_profile.gpu_super_perf);
    if is_super_perf || super_perf == Some(false) {
        if let (Some(c), Some(m)) = (gpu_core_offset, gpu_mem_offset) {
            let gpu = crate::driver::gpu_bridge::get_global_gpu();
            gpu.lock().set_clock_offsets(c, m);
            let _ = app.emit("gpu-clock-offsets-changed", serde_json::json!({
                "core_offset_mhz": c,
                "mem_offset_mhz": m
            }));
        } else if let Some(c) = gpu_core_offset {
            let gpu = crate::driver::gpu_bridge::get_global_gpu();
            let m = gpu.lock().get_mem_offset();
            gpu.lock().set_clock_offsets(c, m);
            let _ = app.emit("gpu-clock-offsets-changed", serde_json::json!({
                "core_offset_mhz": c,
                "mem_offset_mhz": m
            }));
        } else if let Some(m) = gpu_mem_offset {
            let gpu = crate::driver::gpu_bridge::get_global_gpu();
            let c = gpu.lock().get_core_offset();
            gpu.lock().set_clock_offsets(c, m);
            let _ = app.emit("gpu-clock-offsets-changed", serde_json::json!({
                "core_offset_mhz": c,
                "mem_offset_mhz": m
            }));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn apply_gpu_tuning_state(
    super_perf: Option<bool>,
    db_enabled: Option<bool>,
    db_watts: Option<u8>,
    offset_watts: Option<u8>,
    label: Option<String>,
) -> Result<crate::system::actuator::UnifiedGpuStateTruth, String> {
    let lbl = label.unwrap_or_else(|| "UI Tuning State Dispatch".to_string());
    crate::system::actuator::UnifiedPowerActuator::global().apply_gpu_state(
        super_perf,
        db_enabled,
        db_watts,
        offset_watts,
        &lbl,
    )
}

#[tauri::command]
pub fn get_gpu_power_state_truth() -> crate::system::actuator::UnifiedGpuStateTruth {
    crate::system::actuator::UnifiedPowerActuator::global().get_live_truth()
}

#[tauri::command]
pub fn emergency_hard_unlock_gpu() -> Result<crate::system::actuator::UnifiedGpuStateTruth, String> {
    crate::system::actuator::UnifiedPowerActuator::global().emergency_hard_unlock()
}

#[tauri::command]
pub fn get_gpu_tuning_capabilities() -> crate::driver::gpu_bridge::GpuTuningCapabilities {
    crate::driver::gpu_bridge::probe_gpu_tuning_capabilities()
}

#[tauri::command]
pub fn get_bios_advanced_menu_status() -> crate::driver::uefi_nvram::BiosMenuStatus {
    crate::driver::uefi_nvram::read_bios_advanced_menu_status()
}

#[tauri::command]
pub fn toggle_bios_advanced_menu(enable: bool) -> Result<crate::driver::uefi_nvram::BiosMenuStatus, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.custom_profile.bios_oc_enabled = enable;
    crate::system::config::save_config(&cfg);

    match crate::driver::uefi_nvram::write_bios_advanced_menu_status(enable) {
        Ok(status) => Ok(status),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn get_model_profile_info() -> crate::system::model_engine::ModelMatchResult {
    crate::system::model_engine::detect_model_profile()
}

#[tauri::command]
pub fn get_models_file_path() -> String {
    crate::system::model_engine::get_models_file_path().to_string_lossy().to_string()
}

#[tauri::command]
pub fn reload_model_profiles() -> Result<crate::system::power_mode::FactoryPowerPresets, String> {
    // models.json may have been edited by hand while the app was running; drop the
    // in-memory copy so the reload actually re-reads the file.
    crate::system::model_engine::invalidate_model_profiles_cache();
    let presets = crate::system::power_mode::refresh_factory_power_presets(true);
    Ok(presets)
}

#[tauri::command]
pub fn open_models_in_notepad() -> Result<(), String> {
    let path = crate::system::model_engine::get_models_file_path();
    if !path.exists() {
        let _ = crate::system::model_engine::load_model_profiles();
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("notepad.exe")
            .arg(path.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("无法启动记事本: {}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

#[tauri::command]
pub fn export_models_json() -> Result<Option<String>, String> {
    let path = crate::system::model_engine::get_models_file_path();
    if !path.exists() {
        let _ = crate::system::model_engine::load_model_profiles();
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取 models.json 失败: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        if let Some(dest) = save_file_dialog("models.json", "JSON 配置文件", "json") {
            std::fs::write(&dest, content).map_err(|e| format!("保存 models.json 失败: {}", e))?;
            return Ok(Some(dest.to_string_lossy().to_string()));
        }
        Ok(None)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = content;
        Ok(None)
    }
}

#[tauri::command]
pub fn import_models_json(json_content: String) -> Result<String, String> {
    let mut profiles = serde_json::from_str::<Vec<crate::system::model_engine::ModelProfile>>(&json_content)
        .map_err(|e| format!("JSON 解析失败，请检查配置文件格式: {}", e))?;

    if profiles.is_empty() {
        return Err("导入的配置列表中未包含有效的机型数据".to_string());
    }

    crate::system::model_engine::sync_active_machine_profile(&mut profiles);
    crate::system::model_engine::save_model_profiles(&profiles)?;
    let _ = crate::system::power_mode::refresh_factory_power_presets(true);

    let count = profiles.len();
    let my_alias = crate::system::model_engine::get_active_alias();
    Ok(format!("成功导入 {} 项机型配置！当前本机识别为：{}", count, my_alias))
}

#[tauri::command]
pub fn set_bld_fan_speed_cap(
    app: AppHandle,
    ec_state: State<'_, SharedEcBridge>,
    rpm: u16,
) -> Result<(), String> {
    let mut cfg = crate::system::config::load_config();
    if rpm == 0 {
        crate::driver::bld_driver_mgr::restore_bld_fan_auto();
        crate::log_info!("BLD_FAN", "BLD restored factory auto thermal policy via restore_bld_fan_auto");

        cfg.fan_boost = false;
        crate::system::config::save_config(&cfg);
        ec_state.lock().set_fan_boost_from_hardware(false);

        crate::update_tray_fan_boost(false);
        let _ = app.emit("fan-boost-changed", false);
        let _ = app.emit("physical-fan-boost-switched", false);
    } else {
        let raw_cap = if rpm > 100 { (rpm / 100) as u8 } else { rpm as u8 };
        if raw_cap == 0 {
            crate::driver::bld_driver_mgr::restore_bld_fan_auto();
            crate::log_info!("BLD_FAN", "BLD zero rpm requested: released to firmware thermal curve");

            cfg.fan_boost = false;
            crate::system::config::save_config(&cfg);
            ec_state.lock().set_fan_boost_from_hardware(false);

            crate::update_tray_fan_boost(false);
            let _ = app.emit("fan-boost-changed", false);
            let _ = app.emit("physical-fan-boost-switched", false);
        } else {
            let cap_val = raw_cap.clamp(22, 58);
            let target_rpm = (cap_val as u16) * 100;
            crate::driver::bld_driver_mgr::set_bld_fan_manual_clamp(target_rpm, target_rpm);
            crate::log_info!("BLD_FAN", "BLD set fan speed clamp to {} RPM via 1Hz heartbeat arbiter", target_rpm);

            cfg.fan_boost = true;
            crate::system::config::save_config(&cfg);
            ec_state.lock().set_fan_boost_from_hardware(true);

            crate::update_tray_fan_boost(true);
            let _ = app.emit("fan-boost-changed", true);
            let _ = app.emit("physical-fan-boost-switched", true);
        }
    }
    Ok(())
}



#[tauri::command]
pub fn get_gpu_tuning_info() -> crate::driver::nvapi::GpuTuningInfo {
    crate::driver::nvapi::get_gpu_tuning_info()
}

#[tauri::command]
pub fn set_gpu_clock_offsets(
    app: AppHandle,
    core_offset_mhz: i32,
    mem_offset_mhz: i32,
) -> Result<(), String> {
    // 1. Direct hardware application via NVAPI driver
    crate::driver::nvapi::set_gpu_clock_offsets(core_offset_mhz, mem_offset_mhz)?;

    // 2. Sync internal GpuBridge atomic state
    let gpu = crate::driver::gpu_bridge::get_global_gpu();
    let _ = gpu.lock().set_clock_offsets(core_offset_mhz, mem_offset_mhz);

    // 3. Auto-sync to config.json when in Custom Mode (power_mode == 4)
    let mut cfg = crate::system::config::load_config();
    if cfg.power_mode == 4 {
        cfg.custom_profile.gpu_core_offset_mhz = core_offset_mhz;
        cfg.custom_profile.gpu_mem_offset_mhz = mem_offset_mhz;
        let _ = crate::system::config::save_config(&cfg);
        crate::log_info!("CONFIG", "Synchronized GPU clock offsets to custom_profile in config.json: Core={}MHz, Mem={}MHz", core_offset_mhz, mem_offset_mhz);
    }

    // 4. Emit event so both host CustomTuningPanel and all plugins receive the update
    let _ = app.emit("gpu-clock-offsets-changed", serde_json::json!({
        "core_offset_mhz": core_offset_mhz,
        "mem_offset_mhz": mem_offset_mhz
    }));

    Ok(())
}

#[tauri::command]
pub fn set_gpu_vf_curve_deltas(deltas_mhz: Vec<i32>) -> Result<(), String> {
    crate::driver::nvapi::set_gpu_vf_curve_deltas(&deltas_mhz)
}

#[tauri::command]
pub fn set_gpu_undervolt_curve(target_volt_mv: i32, max_boost_mhz: i32) -> Result<(), String> {
    crate::driver::nvapi::set_gpu_undervolt_curve(target_volt_mv, max_boost_mhz)
}

#[tauri::command]
pub fn reset_gpu_vf_curve() -> Result<(), String> {
    crate::driver::nvapi::reset_gpu_vf_curve()
}

#[tauri::command]
pub fn reset_gpu_tuning(app: AppHandle) -> Result<(), String> {
    crate::driver::nvapi::reset_gpu_tuning()?;

    // Sync internal GpuBridge atomic state
    let gpu = crate::driver::gpu_bridge::get_global_gpu();
    let _ = gpu.lock().set_clock_offsets(0, 0);

    // Reset in config.json if in Custom mode
    let mut cfg = crate::system::config::load_config();
    if cfg.power_mode == 4 {
        cfg.custom_profile.gpu_core_offset_mhz = 0;
        cfg.custom_profile.gpu_mem_offset_mhz = 0;
        let _ = crate::system::config::save_config(&cfg);
    }

    let _ = app.emit("gpu-clock-offsets-changed", serde_json::json!({
        "core_offset_mhz": 0,
        "mem_offset_mhz": 0
    }));

    Ok(())
}
