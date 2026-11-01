// OpenRevo - System, Status, Configuration & Diagnostics Commands
// Handles hardware state polling, app configuration, OEM service takeover, logging, reboot, and diagnostic reports.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use crate::driver::ec_bridge::SharedEcBridge;
use crate::driver::gpu_bridge::SharedGpuBridge;
use crate::system::autostart;
use crate::system::display;
use crate::system::hooks;
use crate::system::process;
use crate::system::service_mgr;
use super::power_cmd::is_dc_round_plugged_in;

pub use crate::system::power_mode::{FactoryPowerPresets, get_factory_power_presets, refresh_factory_power_presets};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareState {
    pub power_mode: u8,
    pub fan_boost: bool,
    pub battery_limit: u8,
    pub cpu_temp: i32,
    pub cpu_usage: u32,
    pub cpu_freq: u32,
    pub gpu_temp: u32,
    pub cpu_fan_rpm: u16,
    pub gpu_fan_rpm: u16,
    pub gpu_usage: u32,
    pub gpu_core_freq: u32,
    pub gpu_mem_freq: u32,
    pub gpu_vram_mb: u64,
    pub gpu_vram_used_mb: u64,
    pub refresh_rate: u32,
    pub available_refresh_rates: Vec<u32>,
    pub monitors: Vec<crate::system::display::DisplayMonitor>,
    pub auto_min_refresh_on_battery: bool,
    pub unlock_battery_power_limit: bool,
    pub win_key_locked: bool,
    pub is_mock: bool,
    pub oem_running: bool,
    pub oem_installed: bool,
    pub autostart: bool,
    pub is_intel_13_14_gen: bool,
    pub water_cooler_enabled: bool,
    pub support_flags: crate::driver::wmi_ec::HardwareSupportFlags,
    pub cpu_name: String,
    pub gpu_name: String,
    pub model_name: String,
    pub power_presets: FactoryPowerPresets,
    pub is_dc_online: bool,
    pub cpu_power_w: f32,
    pub gpu_power_w: f32,
}

pub fn get_machine_model() -> String {
    crate::system::model_engine::get_active_alias()
}

static CPU_MONITOR: std::sync::OnceLock<std::sync::Arc<crate::system::cpu_monitor::CpuMonitor>> = std::sync::OnceLock::new();

pub fn get_cpu_monitor() -> std::sync::Arc<crate::system::cpu_monitor::CpuMonitor> {
    CPU_MONITOR.get_or_init(|| crate::system::cpu_monitor::CpuMonitor::new()).clone()
}

pub fn collect_hardware_state(
    ec_state: &SharedEcBridge,
    gpu_state: &SharedGpuBridge,
    query_gpu: bool,
) -> HardwareState {
    let is_bld_manifest = crate::hal::declarative::load_matched_manifest()
        .map(|m| m.channels.values().any(|ch| matches!(ch, crate::hal::declarative::model_manifest::ChannelDef::WmiMailbox { .. })))
        .unwrap_or(false);
    // 1. Nanosecond EC state copy: Immediately drop lock to prevent cross-thread starvation
    let (power_mode, fan_boost, battery_limit, mut cpu_fan_rpm, mut gpu_fan_rpm, is_mock) = {
        let ec = ec_state.lock();
        (
            ec.get_power_mode(),
            if is_bld_manifest {
                crate::driver::bld_driver_mgr::is_bld_fan_clamped()
            } else {
                ec.is_fan_boost()
            },
            ec.get_battery_limit(),
            ec.get_cpu_fan_rpm(),
            ec.get_gpu_fan_rpm(),
            ec.is_mock && !crate::hal::get_hardware_driver().is_hardware_supported(),
        )
    };

    // On-demand foreground fan telemetry for ultrabooks (zero background polling)
    if crate::hal::platform_probe::is_ultrabook() {
        let hal = crate::hal::get_hardware_driver();
        if let Ok((c_rpm, g_rpm)) = hal.read_fan_rpm() {
            if c_rpm > 0 {
                cpu_fan_rpm = c_rpm;
            }
            if g_rpm > 0 {
                gpu_fan_rpm = g_rpm;
            }
            ec_state.lock().update_fan_rpm(c_rpm, g_rpm);
        }
    }

    // 2. Nanosecond GPU telemetry copy: Query only if requested to prevent waking up NVIDIA dGPU from D3Cold
    let is_discrete_gpu_supported = if let Some(m) = crate::hal::declarative::load_matched_manifest() {
        m.capabilities.discrete_gpu
    } else {
        crate::commands::power_cmd::check_dgpu_present()
    };

    let (gpu_temp, gpu_usage, gpu_core_freq, gpu_mem_freq, gpu_vram_mb, gpu_vram_used_mb, gpu_power_w, gpu_name, has_gpu) = {
        if !is_discrete_gpu_supported {
            // Declarative Decision Center Authority: strictly pure iGPU / thin & light laptop, no dGPU
            (0, 0, 0, 0, 0, 0, 0.0, "".to_string(), false)
        } else if query_gpu {
            let gpu = gpu_state.lock();
            let count = gpu.get_gpu_count();
            let name = gpu.get_name(0);
            if count > 0 {
                (
                    gpu.get_temp(0),
                    gpu.get_usage(0),
                    gpu.get_core_freq(0),
                    gpu.get_mem_freq(0),
                    gpu.get_vram_mb(0),
                    gpu.get_vram_used_mb(0),
                    gpu.get_power_w(0),
                    name,
                    true,
                )
            } else {
                (0, 0, 0, 0, 0, 0, 0.0, name, false)
            }
        } else {
            (0, 0, 0, 0, 0, 0, 0.0, "NVIDIA GeForce (Standby/Sleep)".to_string(), false)
        }
    };

    let cpu = get_cpu_monitor();
    let cpu_temp = cpu.get_temp();
    let oem_running = process::is_oem_service_running();
    let oem_installed = process::is_oem_installed();
    let is_intel_13_14_gen = cpu.is_intel_13_14_gen();
    let autostart_enabled = autostart::is_autostart_enabled_cached();

    let cfg = crate::system::config::load_config();

    HardwareState {
        power_mode,
        fan_boost,
        battery_limit,
        cpu_temp,
        cpu_usage: cpu.get_usage(),
        cpu_freq: cpu.get_current_mhz(),
        gpu_temp,
        cpu_fan_rpm,
        gpu_fan_rpm,
        gpu_usage,
        gpu_core_freq,
        gpu_mem_freq,
        gpu_vram_mb,
        gpu_vram_used_mb,
        refresh_rate: display::get_refresh_rate(),
        available_refresh_rates: display::get_available_refresh_rates(),
        monitors: display::get_all_monitors(),
        auto_min_refresh_on_battery: cfg.auto_min_refresh_on_battery,
        unlock_battery_power_limit: cfg.unlock_battery_power_limit,
        win_key_locked: if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_STATUS_BYTE) {
            (val & crate::driver::wmi_ec::STATUS_WINLOCK) != 0
        } else {
            hooks::is_win_key_locked()
        },
        is_mock,
        oem_running,
        oem_installed,
        autostart: autostart_enabled,
        is_intel_13_14_gen,
        water_cooler_enabled: crate::system::config::is_water_cooler_enabled(),
        support_flags: crate::driver::wmi_ec::get_hardware_support_flags(),
        cpu_name: cpu.get_brand(),
        gpu_name,
        model_name: get_machine_model(),
        power_presets: get_factory_power_presets(has_gpu),
        is_dc_online: is_dc_round_plugged_in(),
        cpu_power_w: cpu.get_power_w(),
        gpu_power_w,
    }
}

#[tauri::command]
pub fn get_hardware_status(
    ec_state: State<'_, SharedEcBridge>,
    gpu_state: State<'_, SharedGpuBridge>,
) -> HardwareState {
    collect_hardware_state(ec_state.inner(), gpu_state.inner(), true)
}

#[tauri::command]
pub fn get_app_config() -> crate::system::config::AppConfig {
    crate::system::config::load_config()
}

#[tauri::command]
pub fn save_app_config(cfg: crate::system::config::AppConfig) -> Result<(), String> {
    crate::system::config::save_config(&cfg);
    Ok(())
}

#[tauri::command]
pub fn set_unlock_battery_power_limit(
    enable: bool,
    app: AppHandle,
    ec_state: State<'_, SharedEcBridge>,
) -> Result<bool, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.unlock_battery_power_limit = enable;
    crate::system::config::save_config(&cfg);
    crate::log_info!("POWER", "Battery power limit unlock state changed to: {}", enable);

    // If disabled while running on battery/Type-C and currently in Beast (3) or Custom (4),
    // safely downgrade to Balance to protect hardware immediately.
    if !enable && !is_dc_round_plugged_in() && (cfg.power_mode == 3 || cfg.power_mode == 4) {
        let target_mode = if cfg.power_mode_battery == 1 { 1 } else { 2 };
        crate::log_warn!(
            "POWER",
            "Battery limit re-enabled on battery while in high mode {}, safely downgrading to {}",
            cfg.power_mode,
            target_mode
        );
        cfg.power_mode = target_mode;
        crate::system::config::save_config(&cfg);
        ec_state.lock().set_power_mode(target_mode);
        crate::update_tray_power_mode(target_mode);
        let _ = app.emit("physical-mode-switched", target_mode);
        let _ = app.emit_to("main", "physical-mode-switched", target_mode);
        let _ = app.emit_to("osd", "physical-mode-switched", target_mode);
    }

    let _ = app.emit("unlock-battery-power-limit-changed", enable);
    Ok(enable)
}

#[tauri::command]
pub fn set_autostart_enabled(enabled: bool) -> Result<bool, String> {
    let res = autostart::set_autostart_enabled(enabled);
    if let Ok(val) = res {
        let mut cfg = crate::system::config::load_config();
        cfg.autostart = val;
        crate::system::config::save_config(&cfg);
    }
    res
}

#[tauri::command]
pub fn toggle_oem_service(
    take_over: bool,
    ec_state: State<'_, SharedEcBridge>,
    gpu_state: State<'_, SharedGpuBridge>,
) -> Result<HardwareState, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.takeover_oem = take_over;
    crate::system::config::save_config(&cfg);

    if take_over {
        let _ = service_mgr::pause_oem_services();

        let has_cooler = crate::system::config::is_water_cooler_enabled();
        if has_cooler {
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(600));
                let cooler = crate::driver::ble_cooler::get_global_cooler();
                cooler.start_active_cooling();
            });
        }

        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(600));
            let lighting = crate::driver::ite_lighting::get_global_lighting();
            lighting.invalidate_devices();
            lighting.refresh_devices();
            lighting.restore_all();
            crate::log_info!("LIGHTING", "Seamlessly restored all 3 lighting zones after OEM takeover");

            std::thread::sleep(std::time::Duration::from_millis(1200));
            lighting.restore_all();
        });

        let cfg_clone = cfg.clone();
        let ec_clone = Arc::clone(&ec_state.inner());
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(700));
            if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_AP_OEM_BYTE) {
                if (val & 0x01) == 0 {
                    let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AP_OEM_BYTE, val | 0x01);
                }
            }
            let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
            let effective_mode = if is_dc {
                match cfg_clone.power_mode_ac {
                    1..=4 => cfg_clone.power_mode_ac,
                    _ => 3,
                }
            } else if cfg_clone.power_mode_battery == 1 {
                1
            } else {
                2
            };
            {
                let ec = ec_clone.lock();
                ec.set_power_mode(effective_mode);
                ec.set_battery_limit(cfg_clone.battery_limit);
            }
            if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY) {
                let mut new_v = if cfg_clone.ac_recovery_enabled { v | crate::driver::wmi_ec::AC_RECOVERY_BITMASK } else { v & !crate::driver::wmi_ec::AC_RECOVERY_BITMASK };
                if effective_mode == 4 {
                    new_v |= crate::driver::wmi_ec::MASK_QKEY_WHITE_1830;
                } else {
                    new_v &= !crate::driver::wmi_ec::MASK_QKEY_WHITE_1830;
                }
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY, new_v);
            }
            if effective_mode == 4 {
                if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF) {
                    crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF, v | crate::driver::wmi_ec::MASK_QKEY_WHITE_1831);
                }
            }
            if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
                let new_v = if cfg_clone.usb_charge_enabled { v | crate::driver::wmi_ec::TRIGGER_USB_CHARGE } else { v & !crate::driver::wmi_ec::TRIGGER_USB_CHARGE };
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, new_v);
            }
            if cfg_clone.high_perf_scheme {
                crate::system::power_mode::apply_windows_power_overlay(cfg_clone.power_mode);
            }
            crate::log_info!("TAKEOVER", "Successfully synchronized power mode and battery limit after takeover");
        });
    } else {
        // 顺序不能反：先让键盘彻底松手，再恢复 OEM 服务。
        // 流式线程退出时会发 stop_mcu（MCU 离开 0x33 自定义模式）并释放独占的 HID 句柄；
        // 若只调 resume_oem_services()，官方控制台即使服务已起来也抢不回灯控 —— 实机验证，
        // 用户必须手动重启本程序才能恢复。这是 takeover_oem 双向切换的对称性缺口。
        crate::driver::ite_streamer::get_global_streamer().release_to_oem();
        let _ = service_mgr::resume_oem_services();
    }

    ec_state.lock().reload();
    Ok(get_hardware_status(ec_state, gpu_state))
}

#[tauri::command]
pub fn reboot_computer(_app: tauri::AppHandle) -> Result<(), String> {
    crate::driver::uefi_nvram::trigger_graceful_reboot(800)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogStatus {
    pub visible: bool,
    pub enabled: bool,
    pub level: u8,
    pub path: String,
    pub filter: Option<String>,
}

#[tauri::command]
pub fn get_log_status() -> LogStatus {
    let cfg = crate::system::config::load_config();
    let is_cli = crate::is_cli_log_active();
    LogStatus {
        visible: is_cli,
        enabled: is_cli && cfg.log_enabled,
        level: crate::system::logger::get_log_level(),
        path: crate::system::logger::get_log_path().to_string_lossy().to_string(),
        filter: crate::system::logger::get_log_filter(),
    }
}

#[tauri::command]
pub fn set_log_filter(filter: Option<String>) -> LogStatus {
    let mut cfg = crate::system::config::load_config();
    crate::system::logger::set_log_filter(filter.as_deref());
    cfg.log_filter = crate::system::logger::get_log_filter();
    crate::system::config::save_config(&cfg);

    get_log_status()
}

#[tauri::command]
pub fn set_log_level(level: u8) -> LogStatus {
    let mut cfg = crate::system::config::load_config();
    cfg.log_level = level;
    if level == 0 {
        cfg.log_enabled = false;
        crate::set_cli_log_active(false);
        crate::system::logger::set_log_level(0);
    } else {
        cfg.log_enabled = true;
        crate::set_cli_log_active(true);
        crate::system::logger::set_log_level(level);
    }
    crate::system::config::save_config(&cfg);

    if autostart::is_autostart_enabled() {
        let _ = autostart::set_autostart_enabled(true);
    }

    get_log_status()
}

#[tauri::command]
pub fn open_log_file() -> Result<(), String> {
    let path = crate::system::logger::get_log_path();
    if !path.exists() {
        let _ = std::fs::write(&path, b"[OpenRevo Log Initialized]\n");
    }
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", &path.to_string_lossy()])
        .spawn();
    Ok(())
}

#[derive(serde::Serialize)]
pub struct DesensitizedDiagnosticReport {
    pub generator: String,
    pub version: String,
    pub generated_at: String,
    pub system: DesensitizedSystemInfo,
    pub hardware: DesensitizedHardwareInfo,
    pub factory_tuning: DesensitizedFactoryTuning,
    pub capabilities: DesensitizedCapabilities,
}

#[derive(serde::Serialize)]
pub struct DesensitizedSystemInfo {
    pub manufacturer: String,
    pub product_name: String,
    pub board_model: String,
    pub bios_version: String,
    pub system_family: String,
    pub os: String,
}

#[derive(serde::Serialize)]
pub struct DesensitizedHardwareInfo {
    pub cpu_brand: String,
    pub is_intel_13_14_gen: bool,
    pub gpus: Vec<String>,
    pub current_refresh_rate: u32,
    pub available_refresh_rates: Vec<u32>,
}

#[derive(serde::Serialize)]
pub struct DesensitizedFactoryTuning {
    pub office_pl1_w: u16,
    pub balanced_pl1_w: u16,
    pub turbo_pl1_w: u16,
    pub turbo_pl2_w: u16,
    pub turbo_pl4_effective_w: u16,
    pub turbo_pl4_raw: u16,
    pub turbo_tcc_offset_deg: u8,
    pub gpu_dynamic_boost_max_w: u8,
}

#[derive(serde::Serialize)]
pub struct DesensitizedCapabilities {
    pub has_water_cooler: bool,
    pub has_battery_limit: bool,
    pub has_fan_boost: bool,
    pub has_lightbar: bool,
    pub has_win_lock: bool,
    pub is_rgb_keyboard: bool,
    pub is_dc_online: bool,
}

#[tauri::command]
pub fn export_device_diagnostic_report(
    gpu_state: State<'_, SharedGpuBridge>,
) -> Result<String, String> {
    use crate::driver::wmi_ec;
    use crate::system::display;

    let get_reg_sz = |path: &str, key: &str| -> Option<String> {
        #[cfg(windows)]
        unsafe {
            use windows::core::PCWSTR;
            use windows::Win32::System::Registry::{
                RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
            };
            let wide_path: Vec<u16> = format!("{}\0", path).encode_utf16().collect();
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_path.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
                let wide_key: Vec<u16> = format!("{}\0", key).encode_utf16().collect();
                let mut buf = [0u16; 256];
                let mut size = (buf.len() * 2) as u32;
                let mut data_type = REG_SZ;
                let res = RegQueryValueExW(
                    hkey,
                    PCWSTR(wide_key.as_ptr()),
                    None,
                    Some(&mut data_type),
                    Some(buf.as_mut_ptr() as *mut _),
                    Some(&mut size),
                );
                let _ = RegCloseKey(hkey);
                if res.is_ok() {
                    let len = (size as usize / 2).saturating_sub(1);
                    let s = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
        None
    };

    let bios_path = r"HARDWARE\DESCRIPTION\System\BIOS";
    let manufacturer = get_reg_sz(bios_path, "Manufacturer").unwrap_or_else(|| "Unknown".to_string());
    let product_name = get_reg_sz(bios_path, "SystemProductName").unwrap_or_else(|| "Unknown".to_string());
    let board_model = get_reg_sz(bios_path, "BaseBoardProduct").unwrap_or_else(|| "Unknown".to_string());
    let bios_version = get_reg_sz(bios_path, "BIOSVersion").unwrap_or_else(|| "Unknown".to_string());
    let system_family = get_reg_sz(bios_path, "SystemFamily").unwrap_or_else(|| "Unknown".to_string());

    let os_prod = get_reg_sz(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "ProductName")
        .unwrap_or_else(|| "Windows".to_string());
    let os_ver = get_reg_sz(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "DisplayVersion")
        .unwrap_or_default();
    let os = if !os_ver.is_empty() { format!("{} ({})", os_prod, os_ver) } else { os_prod };

    let cpu = get_cpu_monitor();
    let cpu_brand = cpu.get_brand();
    let is_intel_13_14 = cpu.is_intel_13_14_gen();

    let mut gpus = Vec::new();
    let gpu = gpu_state.lock();
    for i in 0..gpu.get_gpu_count() {
        let name = gpu.get_name(i);
        if !name.is_empty() && !gpus.contains(&name) {
            gpus.push(name);
        }
    }

    let current_refresh_rate = display::get_refresh_rate();
    let available_refresh_rates = display::get_available_refresh_rates();

    let off_pl1 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_OFFICE_PL1).unwrap_or(0) as u16;
    let bal_pl1 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_BALANCE_PL1).unwrap_or(0) as u16;
    let tur_pl1 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TURBO_PL1).unwrap_or(0) as u16;
    let tur_pl2 = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TURBO_PL2).unwrap_or(0) as u16;
    let tur_pl4_raw = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TURBO_PL4).unwrap_or(0) as u16;
    let tur_pl4_effective = tur_pl4_raw;
    let tur_tcc = wmi_ec::wmi_read_ec(wmi_ec::ADDR_TURBO_TCC).unwrap_or(0);
    let gpu_db_max = wmi_ec::wmi_read_ec(wmi_ec::ADDR_GPU_DB_MAX).unwrap_or(25);

    let flags = wmi_ec::get_hardware_support_flags();
    let is_dc = is_dc_round_plugged_in();

    let now_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let report = DesensitizedDiagnosticReport {
        generator: "OpenRevo Machine Diagnostic Report (Safe Desensitized)".to_string(),
        version: "v0.8.0".to_string(),
        generated_at: format!("UnixEpoch_{}", now_sec),
        system: DesensitizedSystemInfo {
            manufacturer,
            product_name,
            board_model: board_model.clone(),
            bios_version,
            system_family,
            os,
        },
        hardware: DesensitizedHardwareInfo {
            cpu_brand,
            is_intel_13_14_gen: is_intel_13_14,
            gpus,
            current_refresh_rate,
            available_refresh_rates,
        },
        factory_tuning: DesensitizedFactoryTuning {
            office_pl1_w: off_pl1,
            balanced_pl1_w: bal_pl1,
            turbo_pl1_w: tur_pl1,
            turbo_pl2_w: tur_pl2,
            turbo_pl4_effective_w: tur_pl4_effective,
            turbo_pl4_raw: tur_pl4_raw,
            turbo_tcc_offset_deg: tur_tcc,
            gpu_dynamic_boost_max_w: gpu_db_max,
        },
        capabilities: DesensitizedCapabilities {
            has_water_cooler: flags.has_water_cooler,
            has_battery_limit: flags.has_battery_limit,
            has_fan_boost: flags.has_fan_boost,
            has_lightbar: flags.has_lightbar,
            has_win_lock: flags.has_win_lock,
            is_rgb_keyboard: flags.is_rgb_keyboard,
            is_dc_online: is_dc,
        },
    };

    let json_str = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("JSON 序列化失败: {}", e))?;

    let desktop_dir = std::env::var("USERPROFILE")
        .map(|p| std::path::PathBuf::from(p).join("Desktop"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let sanitized_board = board_model.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
    let file_name = format!("OpenRevo_Report_{}_{}.json", sanitized_board, now_sec % 1000000);
    let target_path = desktop_dir.join(&file_name);

    std::fs::write(&target_path, json_str)
        .map_err(|e| format!("写入报告到桌面失败: {}", e))?;

    let path_str = target_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", path_str))
            .spawn();
    }

    crate::log_info!("SYSTEM", "Diagnostic report exported cleanly to {}", path_str);
    Ok(path_str)
}

#[tauri::command]
pub fn get_module_toggles() -> std::collections::HashMap<String, bool> {
    crate::system::config::get_module_toggles()
}

#[tauri::command]
pub fn set_module_enabled(id: String, enabled: bool) -> Result<bool, String> {
    let res = crate::system::config::set_module_enabled(&id, enabled);
    Ok(res)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserDataDirInfo {
    pub path: String,
    pub mode: String, // "standard", "portable", "custom"
}

#[tauri::command]
pub fn get_user_data_dir_info() -> Result<UserDataDirInfo, String> {
    let path = crate::system::config::get_config_dir().to_string_lossy().to_string();
    let mode = match crate::system::config::get_data_dir_mode() {
        crate::system::config::DataDirMode::Standard => "standard",
        crate::system::config::DataDirMode::Portable => "portable",
        crate::system::config::DataDirMode::Custom => "custom",
    };
    Ok(UserDataDirInfo {
        path,
        mode: mode.to_string(),
    })
}

#[tauri::command]
pub fn migrate_user_data_dir(target_path: String) -> Result<String, String> {
    let p = std::path::PathBuf::from(target_path.trim());
    crate::system::config::migrate_data_dir(&p)?;
    Ok(p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_user_data_dir() -> Result<(), String> {
    let dir = crate::system::config::get_config_dir();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(dir.to_string_lossy().to_string())
            .spawn();
    }
    Ok(())
}

#[tauri::command]
pub fn clear_user_data_dir() -> Result<String, String> {
    let dir = crate::system::config::get_config_dir();

    // Inviolable security boundaries:
    // Must be absolute, must not touch root drive, must match active config dir
    if !dir.is_absolute() {
        return Err("安全拦截: 路径不是绝对路径".into());
    }
    if dir.parent().is_none() || dir.to_string_lossy().len() <= 3 {
        return Err("安全拦截: 拒绝删除磁盘根目录".into());
    }

    if !dir.exists() {
        return Ok("数据目录已为空".into());
    }

    // Safely iterate through entries in OpenRevo directory
    let read_res = std::fs::read_dir(&dir).map_err(|e| format!("无法读取数据目录: {}", e))?;

    let mut deleted_count = 0;
    for entry_res in read_res {
        if let Ok(entry) = entry_res {
            let entry_path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Do not delete currently executing binaries or drivers if locked by OS
            if file_name.ends_with(".exe") || file_name == "driver" {
                continue;
            }

            if entry_path.is_dir() {
                let _ = std::fs::remove_dir_all(&entry_path);
                deleted_count += 1;
            } else if entry_path.is_file() {
                let _ = std::fs::remove_file(&entry_path);
                deleted_count += 1;
            }
        }
    }

    // Re-create pristine base directories
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::create_dir_all(crate::system::config::get_profiles_dir());

    crate::log_info!("SYSTEM", "User data cleared successfully. Total {} items removed from {:?}", deleted_count, dir);
    Ok("用户数据已成功清除".into())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginTelemetryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, alias = "desiredIntervalMs")]
    pub desired_interval_ms: Option<u64>,
    #[serde(default)]
    pub sensors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginWindowConfig {
    #[serde(default = "default_widget_width")]
    pub width: f64,
    #[serde(default = "default_widget_height")]
    pub height: f64,
    #[serde(default = "default_true")]
    pub transparent: bool,
    #[serde(default = "default_true", alias = "alwaysOnTop")]
    pub always_on_top: bool,
    #[serde(default)]
    pub resizable: bool,
    #[serde(default)]
    pub decorations: bool,
    #[serde(default = "default_true", alias = "skipTaskbar")]
    pub skip_taskbar: bool,
}

fn default_widget_width() -> f64 { 240.0 }
fn default_widget_height() -> f64 { 320.0 }
fn default_true() -> bool { true }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomPluginItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub plugin_type: String,
    pub enabled: bool,
    pub dir_name: String,
    pub window: Option<PluginWindowConfig>,
    pub telemetry: Option<PluginTelemetryConfig>,
    pub entry: Option<String>,
    pub target_slot: Option<String>,
    pub slot_height: Option<String>,
    pub service: Option<crate::system::plugin_service::PluginServiceConfig>,
    pub shortcuts: Option<Vec<crate::system::shortcuts::ShortcutBinding>>,
    pub watchers: Option<Vec<crate::system::watcher_hub::WatcherDefinition>>,
    pub permissions: Option<Vec<String>>,
    pub min_host_version: Option<String>,
    pub is_compatible: bool,
    pub compatibility_notice: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawPluginManifest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default, alias = "type", alias = "plugin_type")]
    pub plugin_type: Option<String>,
    pub entry: Option<String>,
    #[serde(default, alias = "slot", alias = "targetSlot")]
    pub target_slot: Option<String>,
    #[serde(default, alias = "slotHeight")]
    pub slot_height: Option<String>,
    #[serde(default)]
    pub theme_css: Option<String>,
    #[serde(default)]
    pub background_css: Option<String>,
    #[serde(default, alias = "wallpaper", alias = "background_image")]
    pub image: Option<String>,
    pub window: Option<PluginWindowConfig>,
    pub telemetry: Option<PluginTelemetryConfig>,
    pub service: Option<crate::system::plugin_service::PluginServiceConfig>,
    pub shortcuts: Option<Vec<crate::system::shortcuts::ShortcutBinding>>,
    #[serde(default, alias = "rules")]
    pub watchers: Option<Vec<crate::system::watcher_hub::WatcherDefinition>>,
    #[serde(default)]
    pub permissions: Option<Vec<String>>,
    #[serde(default, alias = "openrevo_version", alias = "min_version")]
    pub min_host_version: Option<String>,
}

pub fn check_version_compatibility(min_host_version: Option<&str>) -> (bool, Option<String>) {
    let req_str = match min_host_version {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => return (true, None),
    };

    let clean_req = req_str.trim_start_matches(|c| c == 'v' || c == 'V').trim();
    let parts: Vec<&str> = clean_req.split('.').collect();
    let normalized = if parts.len() == 2 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
        format!("{}.0", clean_req)
    } else if parts.len() == 1 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
        format!("{}.0.0", clean_req)
    } else {
        clean_req.to_string()
    };

    let host_ver_str = env!("CARGO_PKG_VERSION");
    let host_ver = match semver::Version::parse(host_ver_str) {
        Ok(v) => v,
        Err(_) => return (true, None),
    };

    if let Ok(req_ver) = semver::Version::parse(&normalized) {
        if host_ver >= req_ver {
            (true, None)
        } else {
            (false, Some(format!("需 OpenRevo v{}+ (当前 v{})", req_str, host_ver_str)))
        }
    } else if let Ok(req) = semver::VersionReq::parse(clean_req) {
        if req.matches(&host_ver) {
            (true, None)
        } else {
            (false, Some(format!("需 OpenRevo {} (当前 v{})", req_str, host_ver_str)))
        }
    } else {
        (true, None)
    }
}

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn to_base64(data: &[u8]) -> String {
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(B64_CHARS[(triple >> 18) & 0x3F] as char);
        result.push(B64_CHARS[(triple >> 12) & 0x3F] as char);
        if chunk.len() > 1 {
            result.push(B64_CHARS[(triple >> 6) & 0x3F] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(B64_CHARS[triple & 0x3F] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn get_mime_type(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

fn resolve_plugin_css_assets(css: &str, plugin_dir: &std::path::Path) -> String {
    let mut out = String::with_capacity(css.len());
    let mut remaining = css;

    while let Some(start_idx) = remaining.find("url(") {
        out.push_str(&remaining[..start_idx]);
        let after_url = &remaining[start_idx + 4..];
        if let Some(end_idx) = after_url.find(')') {
            let raw_arg = after_url[..end_idx].trim();
            let clean_path = raw_arg.trim_matches(|c| c == '\'' || c == '"').trim();

            if !clean_path.starts_with("http:")
                && !clean_path.starts_with("https:")
                && !clean_path.starts_with("data:")
                && !clean_path.starts_with('#')
                && !clean_path.starts_with("tauri:")
                && !clean_path.starts_with("asset:")
            {
                let file_path = plugin_dir.join(clean_path.trim_start_matches("./"));
                if file_path.exists() && file_path.is_file() {
                    if let Ok(bytes) = std::fs::read(&file_path) {
                        let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        let mime = get_mime_type(ext);
                        let b64 = to_base64(&bytes);
                        out.push_str(&format!("url(\"data:{};base64,{}\")", mime, b64));
                        remaining = &after_url[end_idx + 1..];
                        continue;
                    }
                }
            }

            out.push_str("url(");
            out.push_str(&after_url[..=end_idx]);
            remaining = &after_url[end_idx + 1..];
        } else {
            out.push_str("url(");
            remaining = after_url;
        }
    }
    out.push_str(remaining);
    out
}

pub fn bundle_plugin_html(raw_html: &str, plugin_dir: &std::path::Path) -> String {
    let mut html = raw_html.to_string();

    // 1. Inline linked local stylesheets: <link rel="stylesheet" href="...">
    let mut processed_html = String::with_capacity(html.len());
    let mut cursor = 0;

    while let Some(link_pos) = html[cursor..].find("<link") {
        let actual_start = cursor + link_pos;
        processed_html.push_str(&html[cursor..actual_start]);

        if let Some(tag_end) = html[actual_start..].find('>') {
            let full_tag = &html[actual_start..=actual_start + tag_end];
            let lower_tag = full_tag.to_ascii_lowercase();

            if lower_tag.contains("stylesheet") {
                if let Some(href_val) = extract_attribute_value(full_tag, "href") {
                    let clean_href = href_val.trim_matches(|c| c == '\'' || c == '"').trim();
                    if !clean_href.starts_with("http:")
                        && !clean_href.starts_with("https:")
                        && !clean_href.starts_with("data:")
                        && !clean_href.starts_with("//")
                    {
                        let css_path = plugin_dir.join(clean_href.trim_start_matches("./"));
                        if css_path.exists() && css_path.is_file() {
                            if let Ok(raw_css) = std::fs::read_to_string(&css_path) {
                                let bundled_css = resolve_plugin_css_assets(&raw_css, plugin_dir);
                                processed_html.push_str(&format!("<style data-inlined=\"{}\">{}</style>", clean_href, bundled_css));
                                cursor = actual_start + tag_end + 1;
                                continue;
                            }
                        }
                    }
                }
            }

            processed_html.push_str(full_tag);
            cursor = actual_start + tag_end + 1;
        } else {
            processed_html.push_str(&html[actual_start..]);
            cursor = html.len();
            break;
        }
    }
    processed_html.push_str(&html[cursor..]);
    html = processed_html;

    // 2. Inline local script tags: <script src="..."></script>
    let mut script_inlined_html = String::with_capacity(html.len());
    cursor = 0;
    while let Some(script_pos) = html[cursor..].find("<script") {
        let actual_start = cursor + script_pos;
        script_inlined_html.push_str(&html[cursor..actual_start]);

        if let Some(tag_end) = html[actual_start..].find('>') {
            let open_tag = &html[actual_start..=actual_start + tag_end];
            if let Some(src_val) = extract_attribute_value(open_tag, "src") {
                let clean_src = src_val.trim_matches(|c| c == '\'' || c == '"').trim();
                if !clean_src.starts_with("http:")
                    && !clean_src.starts_with("https:")
                    && !clean_src.starts_with("data:")
                    && !clean_src.starts_with("//")
                {
                    let js_path = plugin_dir.join(clean_src.trim_start_matches("./"));
                    if js_path.exists() && js_path.is_file() {
                        if let Ok(js_content) = std::fs::read_to_string(&js_path) {
                            let is_module = open_tag.to_ascii_lowercase().contains("module");
                            let close_tag_target = "</script>";
                            let after_open = actual_start + tag_end + 1;
                            let end_of_script = if let Some(close_pos) = html[after_open..].find(close_tag_target) {
                                after_open + close_pos + close_tag_target.len()
                            } else {
                                after_open
                            };

                            let module_attr = if is_module { " type=\"module\"" } else { "" };
                            script_inlined_html.push_str(&format!(
                                "<script{} data-inlined=\"{}\">\n{}\n</script>",
                                module_attr, clean_src, js_content
                            ));
                            cursor = end_of_script;
                            continue;
                        }
                    }
                }
            }

            script_inlined_html.push_str(open_tag);
            cursor = actual_start + tag_end + 1;
        } else {
            script_inlined_html.push_str(&html[actual_start..]);
            cursor = html.len();
            break;
        }
    }
    script_inlined_html.push_str(&html[cursor..]);
    html = script_inlined_html;

    // 3. Resolve inline <style>...</style> blocks url(...)
    let mut style_resolved_html = String::with_capacity(html.len());
    cursor = 0;
    while let Some(start) = html[cursor..].find("<style") {
        let tag_start = cursor + start;
        style_resolved_html.push_str(&html[cursor..tag_start]);
        if let Some(open_close) = html[tag_start..].find('>') {
            let after_open = tag_start + open_close + 1;
            if let Some(style_end) = html[after_open..].find("</style>") {
                let style_content = &html[after_open..after_open + style_end];
                let resolved_css = resolve_plugin_css_assets(style_content, plugin_dir);
                style_resolved_html.push_str(&html[tag_start..=tag_start + open_close]);
                style_resolved_html.push_str(&resolved_css);
                style_resolved_html.push_str("</style>");
                cursor = after_open + style_end + "</style>".len();
                continue;
            }
        }
        style_resolved_html.push_str(&html[tag_start..]);
        cursor = html.len();
        break;
    }
    style_resolved_html.push_str(&html[cursor..]);
    html = style_resolved_html;

    // 4. Resolve media tags: src="..." in <img, <audio, <video, <source
    let mut asset_inlined_html = String::with_capacity(html.len());
    cursor = 0;
    let markers = ["<img ", "<audio ", "<video ", "<source "];

    while cursor < html.len() {
        let next_match = markers.iter()
            .filter_map(|m| html[cursor..].find(m).map(|pos| (cursor + pos, *m)))
            .min_by_key(|(pos, _)| *pos);

        if let Some((tag_start, _marker)) = next_match {
            asset_inlined_html.push_str(&html[cursor..tag_start]);
            if let Some(tag_end) = html[tag_start..].find('>') {
                let full_tag = &html[tag_start..=tag_start + tag_end];
                let mut replaced_tag = full_tag.to_string();

                if let Some(src_val) = extract_attribute_value(full_tag, "src") {
                    let clean_src = src_val.trim_matches(|c| c == '\'' || c == '"').trim();
                    if !clean_src.starts_with("http:")
                        && !clean_src.starts_with("https:")
                        && !clean_src.starts_with("data:")
                        && !clean_src.starts_with("//")
                    {
                        let asset_path = plugin_dir.join(clean_src.trim_start_matches("./"));
                        if asset_path.exists() && asset_path.is_file() {
                            if let Ok(bytes) = std::fs::read(&asset_path) {
                                let ext = asset_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                                let mime = get_mime_type(ext);
                                let b64 = to_base64(&bytes);
                                let data_uri = format!("data:{};base64,{}", mime, b64);
                                replaced_tag = replaced_tag.replace(
                                    &format!("src=\"{}\"", src_val),
                                    &format!("src=\"{}\"", data_uri),
                                );
                                replaced_tag = replaced_tag.replace(
                                    &format!("src='{}'", src_val),
                                    &format!("src=\"{}\"", data_uri),
                                );
                            }
                        }
                    }
                }

                asset_inlined_html.push_str(&replaced_tag);
                cursor = tag_start + tag_end + 1;
            } else {
                asset_inlined_html.push_str(&html[tag_start..]);
                break;
            }
        } else {
            asset_inlined_html.push_str(&html[cursor..]);
            break;
        }
    }

    asset_inlined_html
}

fn extract_attribute_value<'a>(tag: &'a str, attr_name: &str) -> Option<&'a str> {
    let pattern = format!("{}=", attr_name);
    let lower_tag = tag.to_ascii_lowercase();
    let idx = lower_tag.find(&pattern)?;
    let val_start = idx + pattern.len();
    let remaining = &tag[val_start..];
    let first_char = remaining.chars().next()?;

    if first_char == '"' || first_char == '\'' {
        let quote = first_char;
        let content_start = val_start + 1;
        let end_idx = tag[content_start..].find(quote)?;
        Some(&tag[content_start..content_start + end_idx])
    } else {
        let end_idx = remaining.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(remaining.len());
        Some(&remaining[..end_idx])
    }
}

#[tauri::command]
pub fn open_plugins_dir() -> Result<(), String> {
    let dir = crate::system::config::get_config_dir().join("plugins");
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(dir.to_string_lossy().to_string())
            .spawn();
    }
    Ok(())
}

#[tauri::command]
pub fn get_custom_plugins() -> Result<Vec<CustomPluginItem>, String> {
    let plugins_dir = crate::system::config::get_config_dir().join("plugins");
    let _ = std::fs::create_dir_all(&plugins_dir);

    let cfg = crate::system::config::load_config();
    let mut list = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                let manifest_path = path.join("manifest.json");

                if manifest_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                            let id = manifest.id.unwrap_or_else(|| dir_name.clone());
                            let plugin_type = manifest.plugin_type.unwrap_or_else(|| "extension".into());
                            let (is_compatible, compatibility_notice) = check_version_compatibility(manifest.min_host_version.as_deref());
                            let enabled = if !is_compatible {
                                false
                            } else if plugin_type == "skin" {
                                cfg.active_skin.as_ref() == Some(&id)
                            } else if plugin_type == "background" {
                                cfg.active_background.as_ref() == Some(&id)
                            } else {
                                cfg.custom_plugin_toggles.get(&id).copied().unwrap_or(true)
                            };
                            list.push(CustomPluginItem {
                                id: id.clone(),
                                name: manifest.name.unwrap_or_else(|| dir_name.clone()),
                                version: manifest.version.unwrap_or_else(|| "1.0.0".into()),
                                author: manifest.author.unwrap_or_else(|| "第三方开发者".into()),
                                description: manifest.description.unwrap_or_else(|| "无插件描述".into()),
                                plugin_type,
                                enabled,
                                dir_name: dir_name.clone(),
                                window: manifest.window,
                                telemetry: manifest.telemetry,
                                entry: manifest.entry,
                                target_slot: manifest.target_slot,
                                slot_height: manifest.slot_height,
                                service: manifest.service,
                                shortcuts: manifest.shortcuts.clone(),
                                watchers: manifest.watchers.clone(),
                                permissions: manifest.permissions.clone(),
                                min_host_version: manifest.min_host_version.clone(),
                                is_compatible,
                                compatibility_notice,
                            });
                            if enabled && is_compatible {
                                if let Some(ref scs) = manifest.shortcuts {
                                    crate::system::shortcuts::load_plugin_shortcuts(&id, scs.clone());
                                }
                                if let Some(ref wts) = manifest.watchers {
                                    crate::system::watcher_hub::load_plugin_watchers(&id, wts.clone());
                                }
                            }
                            continue;
                        }
                    }
                }

                // If folder exists but without standard manifest
                let enabled = cfg.custom_plugin_toggles.get(&dir_name).copied().unwrap_or(true);
                list.push(CustomPluginItem {
                    id: dir_name.clone(),
                    name: dir_name.clone(),
                    version: "0.1.0".into(),
                    author: "本地".into(),
                    description: "本地扩展目录 (未检测到标准 manifest.json)".into(),
                    plugin_type: "directory".into(),
                    enabled,
                    dir_name,
                    window: None,
                    telemetry: None,
                    entry: None,
                    target_slot: None,
                    slot_height: None,
                    service: None,
                    shortcuts: None,
                    watchers: None,
                    permissions: None,
                    min_host_version: None,
                    is_compatible: true,
                    compatibility_notice: None,
                });
            }
        }
    }

    Ok(list)
}

#[tauri::command]
pub fn start_plugin_service(plugin_id: String) -> Result<crate::system::plugin_service::PluginServiceStatus, String> {
    crate::system::plugin_service::start_service(&plugin_id)
}

#[tauri::command]
pub fn stop_plugin_service(plugin_id: String) -> Result<crate::system::plugin_service::PluginServiceStatus, String> {
    crate::system::plugin_service::stop_service(&plugin_id)
}

#[tauri::command]
pub fn get_plugin_service_status(plugin_id: String) -> Result<crate::system::plugin_service::PluginServiceStatus, String> {
    Ok(crate::system::plugin_service::get_service_status(&plugin_id))
}

#[tauri::command]
pub fn set_custom_plugin_enabled(id: String, enabled: bool) -> Result<bool, String> {
    if enabled {
        let p_dir = find_plugin_dir(&id);
        let m_path = p_dir.join("manifest.json");
        if let Ok(content) = std::fs::read_to_string(&m_path) {
            if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                let (is_compatible, notice) = check_version_compatibility(manifest.min_host_version.as_deref());
                if !is_compatible {
                    return Err(notice.unwrap_or_else(|| "当前宿主版本低于插件要求版本".into()));
                }
            }
        }
    }

    let mut cfg = crate::system::config::load_config();
    cfg.custom_plugin_toggles.insert(id.clone(), enabled);
    crate::system::config::save_config(&cfg);
    if !enabled {
        let _ = crate::system::plugin_service::stop_service(&id);
        crate::system::shortcuts::unload_plugin_shortcuts(&id);
        crate::system::watcher_hub::unregister_by_plugin(&id);
    } else {
        let p_dir = find_plugin_dir(&id);
        let m_path = p_dir.join("manifest.json");
        if let Ok(content) = std::fs::read_to_string(&m_path) {
            if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                if let Some(scs) = manifest.shortcuts {
                    crate::system::shortcuts::load_plugin_shortcuts(&id, scs);
                }
                if let Some(wts) = manifest.watchers {
                    crate::system::watcher_hub::load_plugin_watchers(&id, wts);
                }
            }
        }
    }
    Ok(true)
}

pub fn find_plugin_dir(plugin_id: &str) -> std::path::PathBuf {
    let base = crate::system::config::get_config_dir().join("plugins");
    let direct = base.join(plugin_id);
    if direct.exists() {
        return direct;
    }
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                        if manifest.id.as_deref() == Some(plugin_id) {
                            return path;
                        }
                    }
                }
            }
        }
    }
    direct
}

#[tauri::command]
pub fn load_plugin_theme_css(plugin_id: String) -> Result<String, String> {
    let plugins_dir = find_plugin_dir(&plugin_id);
    let manifest_path = plugins_dir.join("manifest.json");
    let css_file_name = if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                manifest.theme_css.unwrap_or_else(|| "theme.css".into())
            } else {
                "theme.css".into()
            }
        } else {
            "theme.css".into()
        }
    } else {
        "theme.css".into()
    };

    let css_path = plugins_dir.join(&css_file_name);
    if !css_path.exists() {
        return Err(format!("'{}' not found in plugin '{}'", css_file_name, plugin_id));
    }
    let raw_css = std::fs::read_to_string(&css_path).map_err(|e| format!("Failed to read theme CSS: {}", e))?;
    Ok(resolve_plugin_css_assets(&raw_css, &plugins_dir))
}

#[tauri::command]
pub fn load_plugin_background_css(plugin_id: String) -> Result<String, String> {
    let plugins_dir = find_plugin_dir(&plugin_id);
    let manifest_path = plugins_dir.join("manifest.json");
    let css_file_name = if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                manifest.background_css.or(manifest.theme_css).unwrap_or_else(|| "background.css".into())
            } else {
                "background.css".into()
            }
        } else {
            "background.css".into()
        }
    } else {
        "background.css".into()
    };

    let css_path = plugins_dir.join(&css_file_name);
    if css_path.exists() {
        let raw_css = std::fs::read_to_string(&css_path).map_err(|e| format!("Failed to read background CSS: {}", e))?;
        return Ok(resolve_plugin_css_assets(&raw_css, &plugins_dir));
    }

    // Fallback: try theme.css
    let fallback_path = plugins_dir.join("theme.css");
    if fallback_path.exists() {
        let raw_css = std::fs::read_to_string(&fallback_path).map_err(|e| format!("Failed to read fallback theme.css: {}", e))?;
        return Ok(resolve_plugin_css_assets(&raw_css, &plugins_dir));
    }

    // Fallback 2: Check manifest.image or discover image file in plugin_dir
    let manifest_opt = if manifest_path.exists() {
        std::fs::read_to_string(&manifest_path).ok().and_then(|c| serde_json::from_str::<RawPluginManifest>(&c).ok())
    } else {
        None
    };

    let image_rel_path = manifest_opt.as_ref().and_then(|m| m.image.clone()).or_else(|| {
        let candidates = [
            "background.jpg", "background.jpeg", "background.png", "background.webp",
            "wallpaper.jpg", "wallpaper.jpeg", "wallpaper.png", "wallpaper.webp",
            "bg.jpg", "bg.jpeg", "bg.png", "bg.webp",
        ];
        candidates.into_iter().find(|name| plugins_dir.join(name).exists()).map(|s| s.to_string())
    });

    if let Some(rel) = image_rel_path {
        let img_path = plugins_dir.join(rel.trim_start_matches("./"));
        if img_path.exists() && img_path.is_file() {
            if let Ok(bytes) = std::fs::read(&img_path) {
                let ext = img_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let mime = get_mime_type(ext);
                let b64 = to_base64(&bytes);
                let generated_css = format!(
                    "/* Auto-generated by OpenRevo Background Engine */\n.openrevo-shell {{\n    background-image: url(\"data:{};base64,{}\") !important;\n    background-size: cover !important;\n    background-position: center !important;\n    background-repeat: no-repeat !important;\n}}\n.mini-drawer-root {{\n    background-image: url(\"data:{};base64,{}\") !important;\n    background-size: cover !important;\n    background-position: center !important;\n    background-repeat: no-repeat !important;\n}}\n",
                    mime, b64, mime, b64
                );
                return Ok(generated_css);
            }
        }
    }

    Err(format!("Neither background CSS ('{}' / 'theme.css') nor image asset found in plugin '{}'", css_file_name, plugin_id))
}

#[tauri::command]
pub fn set_active_skin(skin_id: Option<String>) -> Result<(), String> {
    let mut cfg = crate::system::config::load_config();
    cfg.active_skin = skin_id;
    crate::system::config::save_config(&cfg);
    Ok(())
}

#[tauri::command]
pub fn get_active_skin() -> Result<Option<String>, String> {
    let cfg = crate::system::config::load_config();
    Ok(cfg.active_skin)
}

#[tauri::command]
pub fn set_active_background(background_id: Option<String>) -> Result<(), String> {
    let mut cfg = crate::system::config::load_config();
    cfg.active_background = background_id;
    crate::system::config::save_config(&cfg);
    Ok(())
}

#[tauri::command]
pub fn get_active_background() -> Result<Option<String>, String> {
    let cfg = crate::system::config::load_config();
    Ok(cfg.active_background)
}

#[tauri::command]
pub fn load_plugin_html(plugin_id: String) -> Result<String, String> {
    let plugins_dir = find_plugin_dir(&plugin_id);
    let manifest_path = plugins_dir.join("manifest.json");

    let entry_file = if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<RawPluginManifest>(&content) {
                manifest.entry.unwrap_or_else(|| "index.html".into())
            } else {
                "index.html".into()
            }
        } else {
            "index.html".into()
        }
    } else {
        "index.html".into()
    };

    let html_path = plugins_dir.join(&entry_file);
    if !html_path.exists() {
        return Err(format!("Entry file '{}' not found in plugin '{}'", entry_file, plugin_id));
    }

    let raw_html = std::fs::read_to_string(&html_path)
        .map_err(|e| format!("Failed to read '{}': {}", entry_file, e))?;

    Ok(bundle_plugin_html(&raw_html, &plugins_dir))
}

#[tauri::command]
pub fn get_model_capabilities() -> serde_json::Value {
    let is_debug = crate::system::logger::is_debug_mode();
    let (cpu_name, is_amd) = crate::driver::uefi_nvram::get_cpu_info();
    let detected_cpu_vendor = if is_amd { "amd" } else { "intel" };

    // 1. If declarative driver is loaded, return its identity and capabilities
    if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        let cpu_vendor = manifest.capabilities.cpu_vendor
            .clone()
            .unwrap_or_else(|| detected_cpu_vendor.to_string());
        let cpu_tuning_type = manifest.capabilities.cpu_tuning_type
            .clone()
            .unwrap_or_else(|| if is_amd { "amd_smu".to_string() } else { "intel_pl".to_string() });
        let cpu_supports_fppt = manifest.capabilities.cpu_supports_fppt
            .unwrap_or(!manifest.id.contains("bld"));
        let gpu_power_tuning_mode = manifest.capabilities.gpu_power_tuning_mode
            .clone()
            .unwrap_or_else(|| if manifest.id.contains("bld") { "mode_locked".to_string() } else { "none".to_string() });
        let fan_control_mode = manifest.capabilities.fan_control_mode
            .clone()
            .unwrap_or_else(|| if manifest.capabilities.fan_custom_curve { "discrete_table_16".to_string() } else { "fixed_boost_only".to_string() });
        let fan_boost_mode = manifest.capabilities.fan_boost_mode
            .clone()
            .unwrap_or_else(|| if manifest.capabilities.fan_custom_curve { "hardware_toggle".to_string() } else { "none".to_string() });

        // Dynamic EC Calibration with Manifest Baseline Fallback:
        // On Tongfang ODM platforms, physical EC registers natively burn factory preset PL1 targets.
        // If the ACPI driver is active and returns valid non-zero wattage, prioritize real hardware truth.
        let effective_pmw = {
            let mut watts_map = manifest.capabilities.power_mode_watts.clone().unwrap_or_default();
            if crate::driver::wmi_ec::is_acpi_driver_available() {
                let off_ec = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL1).unwrap_or(0);
                let bal_ec = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL1).unwrap_or(0);
                let tur_ec = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL1).unwrap_or(0);
                if off_ec > 0 { watts_map.insert("office".to_string(), off_ec as u32); }
                if bal_ec > 0 { watts_map.insert("balance".to_string(), bal_ec as u32); }
                if tur_ec > 0 {
                    watts_map.insert("turbo".to_string(), tur_ec as u32);
                    watts_map.insert("custom".to_string(), tur_ec as u32);
                }
            }
            if !watts_map.is_empty() {
                Some(watts_map)
            } else {
                None
            }
        };

        return serde_json::json!({
            "id": manifest.id,
            "name": manifest.name,
            "form_factor": manifest.identity.form_factor,
            "ui_preset": manifest.identity.ui_preset,
            "discrete_gpu": manifest.capabilities.discrete_gpu,
            "water_cooling": manifest.capabilities.water_cooling,
            "fan_custom_curve": manifest.capabilities.fan_custom_curve,
            "fan_independent_channels": manifest.capabilities.fan_independent_channels,
            "fan_ramp_rate": manifest.capabilities.fan_ramp_rate,
            "fan_control_mode": fan_control_mode,
            "fan_boost_mode": fan_boost_mode,
            "cpu_vendor": cpu_vendor,
            "cpu_tuning_type": cpu_tuning_type,
            "cpu_supports_fppt": cpu_supports_fppt,
            "gpu_power_tuning_mode": gpu_power_tuning_mode,
            "cpu_name": cpu_name,
            "battery_charge_limit": manifest.capabilities.battery_charge_limit,
            "power_modes": manifest.capabilities.power_modes,
            "power_mode_watts": effective_pmw,
            "rgb_keyboard": manifest.capabilities.rgb_keyboard,
            "is_adapted": true,
            "is_debug_mode": is_debug,
        });
    }

    let platform_channel = crate::hal::platform_probe::get_platform_channel();

    // 2. Unadapted Hardware Guard:
    // If the machine matches neither a declarative manifest nor known Tongfang / Bitland / IP3 communication channels,
    // strictly report is_adapted = false and disable custom tuning to prevent hardware risk and phantom controls!
    if platform_channel == crate::hal::platform_probe::PlatformChannelKind::Unknown {
        return serde_json::json!({
            "id": "unadapted_generic",
            "name": format!("未适配机型 ({})", cpu_name),
            "form_factor": "unknown",
            "ui_preset": "unadapted_safe",
            "discrete_gpu": crate::driver::gpu_bridge::is_nvidia_dgpu_available(),
            "water_cooling": false,
            "fan_custom_curve": false,
            "fan_independent_channels": false,
            "fan_ramp_rate": false,
            "fan_control_mode": "none",
            "fan_boost_mode": "none",
            "cpu_vendor": detected_cpu_vendor,
            "cpu_tuning_type": "none",
            "cpu_supports_fppt": false,
            "gpu_power_tuning_mode": "none",
            "cpu_name": cpu_name,
            "battery_charge_limit": false,
            "power_modes": Vec::<&str>::new(),
            "rgb_keyboard": {
                "supported": false,
                "type": "none",
                "sleep_timeout_supported": false,
                "timeout_options_seconds": [0],
                "steps": Vec::<u8>::new()
            },
            "is_adapted": false,
            "is_debug_mode": is_debug,
        });
    }

    // 3. Default fallback capabilities based on unified platform probe
    let is_thin_light = crate::hal::platform_probe::is_ultrabook();
    let has_water_cooler = if is_thin_light {
        false
    } else {
        crate::driver::wmi_ec::get_hardware_support_flags().has_water_cooler
    };

    serde_json::json!({
        "id": if is_thin_light { "builtin_thin_light" } else { "builtin_tongfang_gaming" },
        "name": if is_thin_light { "MECHREVO Thin & Light Laptop" } else { "MECHREVO Gaming Laptop" },
        "form_factor": if is_thin_light { "thin_light" } else { "gaming" },
        "ui_preset": if is_thin_light { "ultrabook_compact" } else { "gaming_full" },
        "discrete_gpu": !is_thin_light,
        "water_cooling": has_water_cooler,
        "fan_custom_curve": !is_thin_light,
        "fan_independent_channels": true,
        "fan_ramp_rate": true,
        "fan_control_mode": if is_thin_light { "none" } else { "discrete_table_16" },
        "fan_boost_mode": if is_thin_light { "none" } else { "hardware_toggle" },
        "cpu_vendor": detected_cpu_vendor,
        "cpu_tuning_type": if is_thin_light { "none" } else if is_amd { "amd_smu" } else { "intel_pl" },
        "cpu_supports_fppt": is_amd,
        "gpu_power_tuning_mode": if is_thin_light { "none" } else { "discrete_manual" },
        "cpu_name": cpu_name,
        "battery_charge_limit": true,
        "power_modes": if is_thin_light { vec!["office", "balance", "turbo"] } else { vec!["office", "balance", "turbo", "custom"] },
        "rgb_keyboard": {
            "supported": !is_thin_light,
            "type": if is_thin_light { "none" } else { "per_key_rgb" },
            "sleep_timeout_supported": is_thin_light,
            "timeout_options_seconds": [0, 10, 15, 30, 60],
            "steps": if is_thin_light { Vec::<u8>::new() } else { Vec::<u8>::new() }
        },
        "is_adapted": true,
        "is_debug_mode": is_debug,
    })
}




