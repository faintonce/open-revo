use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfoSnapshot {
    pub board_model: String,
    pub system_product: String,
    pub bios_version: String,
    pub cpu_name: String,
    pub os_caption: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuEnvironmentSnapshot {
    pub topology_type: String,
    pub all_gpu_devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayPanelSnapshot {
    pub internal_panel: String,
    pub all_displays: Vec<String>,
    pub supported_refresh_rates_hz: Vec<u32>,
    pub max_refresh_rate_hz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiosOverclockSnapshot {
    pub status: String,
    pub nvram_support: String,
    pub nvram_switch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyboardLightingSnapshot {
    pub hardware_type: String,
    pub description: String,
    pub has_ite8291_controller: bool,
    pub has_ite7001_controller: bool,
    pub supports_single_zone_rgb: bool,
    pub supports_single_color_white: bool,
    pub board_project_code: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeripheralsPowerSnapshot {
    pub water_cooler_supported: bool,
    pub ec_access_status: String,
    pub battery_mode: String,
    pub charge_limit_upper_pct: u8,
    pub charge_limit_lower_pct: u8,
    pub power_off_usb_charging: String,
    pub ac_power_recovery: String,
    pub fn_key_mode: String,
    pub win_key_lock: String,
    pub camera_power: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProbeMetadata {
    pub last_probe_time: String,
    pub openrevo_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowerLimitGroup {
    pub pl1_w: u8,
    pub pl2_w: u8,
    pub pl4_w: u16,
    pub tcc_offset_deg: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuLimitGroup {
    pub dynamic_boost_max_w: u8,
    pub configurable_tgp_w: u8,
    pub db_control_byte: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactoryPowerPresetsSnapshot {
    pub ec_hardware_accessible: bool,
    pub office_mode: PowerLimitGroup,
    pub balanced_mode: PowerLimitGroup,
    pub turbo_mode: PowerLimitGroup,
    pub current_active: PowerLimitGroup,
    pub gpu_limits: GpuLimitGroup,
    pub oem_profiles_detected: bool,
    pub diagnosis_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareSnapshot {
    pub system_info: SystemInfoSnapshot,
    pub gpu_environment: GpuEnvironmentSnapshot,
    pub display_panel: DisplayPanelSnapshot,
    pub bios_overclock_menu: BiosOverclockSnapshot,
    pub keyboard_lighting: KeyboardLightingSnapshot,
    pub peripherals_and_power: PeripheralsPowerSnapshot,
    pub factory_power_presets: FactoryPowerPresetsSnapshot,
    pub ec_registers_snapshot: BTreeMap<String, String>,
    pub probe_metadata: ProbeMetadata,
}

#[derive(Default)]
struct NativeHardwareProbeResult {
    board_model: String,
    system_product: String,
    bios_version: String,
    cpu_name: String,
    os_caption: String,
    gpus: Vec<String>,
    internal_panel: String,
    all_displays: Vec<String>,
    refresh_rates: Vec<u32>,
    oc_status: String,
    oc_support: String,
    oc_switch: String,
}

fn get_reg_sz(subkey: &str, val_name: &str) -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    let wide_sub: Vec<u16> = format!("{}\0", subkey).encode_utf16().collect();
    let wide_val: Vec<u16> = format!("{}\0", val_name).encode_utf16().collect();
    let mut hkey = HKEY::default();

    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_sub.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
            let mut buf = [0u16; 512];
            let mut size = (buf.len() * 2) as u32;
            let mut data_type = REG_SZ;
            let res = RegQueryValueExW(
                hkey,
                PCWSTR(wide_val.as_ptr()),
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
}

fn query_native_probe() -> NativeHardwareProbeResult {
    // 1. Motherboard & BIOS Information
    let board_model = get_reg_sz("HARDWARE\\DESCRIPTION\\System\\BIOS", "BaseBoardProduct")
        .unwrap_or_else(|| "Unknown".to_string());
    let system_product = get_reg_sz("HARDWARE\\DESCRIPTION\\System\\BIOS", "SystemProductName")
        .unwrap_or_else(|| "Unknown".to_string());
    let bios_version = get_reg_sz("HARDWARE\\DESCRIPTION\\System\\BIOS", "BIOSVersion")
        .unwrap_or_else(|| "Unknown".to_string());

    // 2. CPU Architecture & Specification
    let (cpu_name, is_amd) = crate::driver::uefi_nvram::get_cpu_info();

    // 3. Operating System Info
    let os_prod = get_reg_sz("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", "ProductName")
        .unwrap_or_else(|| "Windows 11".to_string());
    let os_ver = get_reg_sz("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", "DisplayVersion")
        .unwrap_or_default();
    let os_build = get_reg_sz("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", "CurrentBuild")
        .and_then(|b| b.parse::<u32>().ok())
        .unwrap_or(0);
    let os_caption = if os_build >= 22000 && os_prod.starts_with("Windows 10") {
        let patched = os_prod.replacen("Windows 10", "Windows 11", 1);
        if !os_ver.is_empty() {
            format!("{} ({})", patched, os_ver)
        } else {
            patched
        }
    } else if !os_ver.is_empty() {
        format!("{} ({})", os_prod, os_ver)
    } else {
        os_prod
    };

    // 4. Video Controller Devices (Dynamic PnP Enumeration)
    let mut gpus = Vec::new();
    unsafe {
        use windows::core::{PCWSTR, PWSTR};
        use windows::Win32::System::Registry::{
            RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
        };
        let base_class_str: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\0".encode_utf16().collect();
        let mut h_base = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(base_class_str.as_ptr()), 0, KEY_READ, &mut h_base).is_ok() {
            let mut index = 0;
            loop {
                let mut name_buf = [0u16; 64];
                let mut name_len = name_buf.len() as u32;
                let ret = RegEnumKeyExW(
                    h_base,
                    index,
                    PWSTR(name_buf.as_mut_ptr()),
                    &mut name_len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                );
                if ret.is_err() {
                    break;
                }

                let sub_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                if sub_name.len() == 4 && sub_name.chars().all(|c| c.is_ascii_digit()) {
                    let subkey = format!("SYSTEM\\CurrentControlSet\\Control\\Class\\{{4d36e968-e325-11ce-bfc1-08002be10318}}\\{}\0", sub_name);
                    let wide_sub: Vec<u16> = subkey.encode_utf16().collect();
                    let mut hkey = HKEY::default();
                    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_sub.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
                        let val_name: Vec<u16> = "DriverDesc\0".encode_utf16().collect();
                        let mut buf = [0u16; 256];
                        let mut size = (buf.len() * 2) as u32;
                        let mut data_type = REG_SZ;
                        if RegQueryValueExW(
                            hkey,
                            PCWSTR(val_name.as_ptr()),
                            None,
                            Some(&mut data_type),
                            Some(buf.as_mut_ptr() as *mut _),
                            Some(&mut size),
                        ).is_ok() {
                            let len = (size as usize / 2).saturating_sub(1);
                            let desc = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                            let upper = desc.to_uppercase();
                            if !desc.is_empty()
                                && !upper.contains("BASIC")
                                && !upper.contains("MIRAGE")
                                && !upper.contains("REMOTE")
                                && !upper.contains("VIRTUAL")
                            {
                                if !gpus.contains(&desc) {
                                    gpus.push(desc);
                                }
                            }
                        }
                        let _ = RegCloseKey(hkey);
                    }
                }
                index += 1;
            }
            let _ = RegCloseKey(h_base);
        }
    }

    // Ensure discrete GPU is represented even if display class was polluted by virtual adapters
    #[cfg(windows)]
    if !gpus.iter().any(|g| {
        let u = g.to_uppercase();
        u.contains("NVIDIA") || u.contains("GEFORCE") || u.contains("RTX") || u.contains("RADEON RX")
    }) {
        if let Some((disc_name, _)) = crate::driver::gpu_bridge::detect_discrete_gpu() {
            gpus.push(disc_name);
        }
    }

    // 5. Display Panels & Refresh Rates
    let monitors = crate::system::display::get_all_monitors();
    let mut all_displays = Vec::new();
    let mut internal_panel = String::new();
    let mut refresh_rates = Vec::new();

    for m in &monitors {
        all_displays.push(m.friendly_name.clone());
        if m.is_internal && internal_panel.is_empty() {
            internal_panel = m.friendly_name.clone();
            refresh_rates = m.supported_rates.clone();
        }
    }
    if internal_panel.is_empty() {
        if let Some(first) = monitors.first() {
            internal_panel = first.friendly_name.clone();
            refresh_rates = first.supported_rates.clone();
        }
    }
    if refresh_rates.is_empty() {
        refresh_rates = vec![60];
    }

    // 6. UEFI NVRAM Overclock Menu Status
    let (oc_status, oc_support, oc_switch) = if is_amd {
        ("AMD_NOT_APPLICABLE".to_string(), "N/A".to_string(), "N/A".to_string())
    } else {
        let status = crate::driver::uefi_nvram::read_bios_advanced_menu_status();
        if status.supported {
            if status.enabled {
                ("ENABLED".to_string(), "0x01".to_string(), "0x01".to_string())
            } else {
                ("DISABLED".to_string(), "0x01".to_string(), "0x00".to_string())
            }
        } else if status.error.is_some() {
            ("NOT_FOUND_OR_RESTRICTED".to_string(), "0x00".to_string(), "0x00".to_string())
        } else {
            ("UNSUPPORTED".to_string(), "0x00".to_string(), "0x00".to_string())
        }
    };

    NativeHardwareProbeResult {
        board_model,
        system_product,
        bios_version,
        cpu_name,
        os_caption,
        gpus,
        internal_panel,
        all_displays,
        refresh_rates,
        oc_status,
        oc_support,
        oc_switch,
    }
}

static SNAPSHOT_CACHE: std::sync::OnceLock<HardwareSnapshot> = std::sync::OnceLock::new();

/// Retrieve or initialize in-memory hardware snapshot without disk persistence
pub fn get_hardware_snapshot_cached() -> &'static HardwareSnapshot {
    SNAPSHOT_CACHE.get_or_init(|| {
        collect_hardware_snapshot()
    })
}

/// Collect complete hardware diagnostic snapshot matching probe_machine_hardware.ps1
pub fn collect_hardware_snapshot() -> HardwareSnapshot {
    let probe_res = query_native_probe();

    let board_model = probe_res.board_model;
    let system_product = probe_res.system_product;
    let bios_version = probe_res.bios_version;
    let cpu_name = probe_res.cpu_name;
    let os_caption = probe_res.os_caption;
    let gpus = probe_res.gpus;

    // 1. System Info
    let sys_info = SystemInfoSnapshot {
        board_model: board_model.clone(),
        system_product: system_product.clone(),
        bios_version,
        cpu_name,
        os_caption,
    };

    // 2. GPU Environment
    let topology = if gpus.iter().any(|g| g.contains("NVIDIA") || g.contains("Radeon RX")) {
        "独显游戏本/全能本 (Hybrid/dGPU)".to_string()
    } else {
        "核显轻薄本 (iGPU Only)".to_string()
    };
    let gpu_env = GpuEnvironmentSnapshot {
        topology_type: topology,
        all_gpu_devices: gpus,
    };

    // 3. Display Panel
    let friendly_internal = if probe_res.internal_panel.is_empty() {
        "内建标准面板".to_string()
    } else {
        probe_res.internal_panel
    };

    let all_displays = probe_res.all_displays;
    let rates = probe_res.refresh_rates;
    let max_rate = *rates.iter().max().unwrap_or(&60);
    let disp_panel = DisplayPanelSnapshot {
        internal_panel: friendly_internal,
        all_displays,
        supported_refresh_rates_hz: rates,
        max_refresh_rate_hz: max_rate,
    };

    // 4. BIOS Overclock Menu
    let oc_code = probe_res.oc_status;
    let oc_desc = match oc_code.as_str() {
        "ENABLED" => "硬件支持且当前已开启 [ENABLED] (开机按 F2/Del 可直接进入高级超频菜单)".to_string(),
        "DISABLED" => "硬件支持但当前已隐藏 [DISABLED] (可通过 OpenRevo 免进 BIOS 一键解锁)".to_string(),
        "AMD_NOT_APPLICABLE" => "不适用 (AMD 平台采用 SMU 硬件功耗调度与 PBO/CBS 架构，无 Intel UniWillVariable 菜单结构)".to_string(),
        "UNSUPPORTED" => "当前主板固件未声明开启超频菜单能力".to_string(),
        "NOT_FOUND_OR_RESTRICTED" => "主板 NVRAM 未找到 OemMagicVariable / UniWillVariable 变量或需要管理员权限".to_string(),
        _ => "未探测到超频状态".to_string(),
    };

    let oc_menu = BiosOverclockSnapshot {
        status: oc_desc,
        nvram_support: probe_res.oc_support,
        nvram_switch: probe_res.oc_switch,
    };

    // 5. Keyboard Lighting Hardware Diagnosis
    let has_ite8291 = crate::driver::ite_lighting::IteLightingManager::probe_keyboard_device_exists();
    let has_ite7001 = crate::driver::ite_lighting::IteLightingManager::probe_lightbar_device_exists();
    let raw_kb_support = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_SUPPORT_BYTE2).unwrap_or(0);
    let raw_bl_white = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BACKLIGHT_WHITE).unwrap_or(0);
    let raw_project_id = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_PROJECT_ID).unwrap_or(0);

    let is_single_zone = (raw_kb_support & 0x04) != 0;
    let is_single_color = (raw_bl_white & 0x01) != 0;

    let (hw_type, desc) = if has_ite8291 {
        ("PerKey_RGB".to_string(), "独立 ITE 8291 USB 全彩逐键/4区 RGB 键盘 [支持 16 种炫彩流动特效与调色盘]".to_string())
    } else if is_single_zone {
        ("SingleZone_RGB".to_string(), "主板 EC 单区 RGB 键盘 (全键同色调光)".to_string())
    } else if is_single_color {
        ("SingleColor".to_string(), "单色白光背光键盘 (支持4档亮度调节)".to_string())
    } else {
        ("SingleColor".to_string(), "通用单色背光键盘".to_string())
    };

    let kb_snap = KeyboardLightingSnapshot {
        hardware_type: hw_type,
        description: desc,
        has_ite8291_controller: has_ite8291,
        has_ite7001_controller: has_ite7001,
        supports_single_zone_rgb: is_single_zone,
        supports_single_color_white: is_single_color,
        board_project_code: raw_project_id,
    };

    // 6. Peripherals & Power Specs (EC RAM)
    let raw_pwr_trigger = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE).unwrap_or(0);
    let raw_ac_recovery = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_AC_RECOVERY).unwrap_or(0);
    let raw_fn_mode = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_FN_LOCK).unwrap_or(0);
    let raw_hw_status = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_STATUS_BYTE).unwrap_or(0);
    let raw_camera_power = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF).unwrap_or(0);
    let raw_bat_mode = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE).unwrap_or(0);
    let raw_bat_limit_up = (crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BATTERY_LIMIT_UP).unwrap_or(80)) & 0x7F;
    let raw_bat_limit_down = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BATTERY_LIMIT_DOWN).unwrap_or(75);

    let is_water_cooler = crate::driver::wmi_ec::get_hardware_support_flags().has_water_cooler;

    let bat_mode_desc = match raw_bat_mode {
        0x21 => "工作站模式".to_string(),
        0x11 => "平衡模式".to_string(),
        0x01 => "长效模式".to_string(),
        _ => format!("模式码: 0x{:02X}", raw_bat_mode),
    };

    let pwr_snap = PeripheralsPowerSnapshot {
        water_cooler_supported: is_water_cooler,
        ec_access_status: "正常直通 [ACPIDriver_IOCTL / WMI]".to_string(),
        battery_mode: bat_mode_desc,
        charge_limit_upper_pct: if raw_bat_limit_up > 0 { raw_bat_limit_up } else { 80 },
        charge_limit_lower_pct: if raw_bat_limit_down > 0 { raw_bat_limit_down } else { 75 },
        power_off_usb_charging: if (raw_pwr_trigger & 0x10) != 0 { "开启 [0x10]".to_string() } else { "关闭 [0x00] (节能防耗)".to_string() },
        ac_power_recovery: if (raw_ac_recovery & crate::driver::wmi_ec::AC_RECOVERY_BITMASK) != 0 { "开启 [0x08]".to_string() } else { "关闭 [0x00]".to_string() },
        fn_key_mode: if raw_fn_mode == 0x11 { "Fn 键锁定 [0x11] (F1~F12为功能键)".to_string() } else { "标准功能键 [0x01]".to_string() },
        win_key_lock: if (raw_hw_status & crate::driver::wmi_ec::STATUS_WINLOCK) != 0 { "硬件锁定 [0x01]".to_string() } else { "正常开启 [0x00]".to_string() },
        camera_power: if (raw_camera_power & crate::driver::wmi_ec::CAMERA_CUTOFF_BITMASK) != 0 { "物理断电 [0x08]".to_string() } else { "正常供电 [0x00]".to_string() },
    };

    // 7. Factory Power Presets Snapshot (EC Power Tables)
    let off_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL1).unwrap_or(0);
    let off_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL2).unwrap_or(0);
    let off_pl4 = crate::driver::wmi_ec::pl4_reg_to_watts(crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL4).unwrap_or(0));
    let off_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_TCC).unwrap_or(0);

    let bal_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL1).unwrap_or(0);
    let bal_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL2).unwrap_or(0);
    let bal_pl4 = crate::driver::wmi_ec::pl4_reg_to_watts(crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL4).unwrap_or(0));
    let bal_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_TCC).unwrap_or(0);

    let tur_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL1).unwrap_or(0);
    let tur_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL2).unwrap_or(0);
    let tur_pl4 = crate::driver::wmi_ec::pl4_reg_to_watts(crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL4).unwrap_or(0));
    let tur_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_TCC).unwrap_or(0);

    let cur_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL1).unwrap_or(0);
    let cur_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL2).unwrap_or(0);
    let cur_pl4 = crate::driver::wmi_ec::pl4_reg_to_watts(crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_CUSTOM_PL4).unwrap_or(0));

    let gpu_db_ctrl = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_DB_CTRL1).unwrap_or(0);
    let gpu_tgp = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TGP_OFFSET).unwrap_or(0);
    let gpu_db_max = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_GPU_DB_MAX).unwrap_or(0);

    let ec_accessible = off_pl1 > 0 || bal_pl1 > 0 || tur_pl1 > 0;

    let oem_profiles_exist = std::path::Path::new("C:\\Program Files\\OEM")
        .join("AiStoneService")
        .exists()
        || glob_oem_profiles_exist();

    let diag_summary = if ec_accessible {
        format!(
            "出厂模具预设: 办公 {}W | 平衡 {}W | 狂暴 {}W | GPU DB +{}W (当前运行: {}W)",
            off_pl1, bal_pl1, tur_pl1, gpu_db_max, cur_pl1
        )
    } else {
        "未能通过 EC 硬件通道直读出厂预设功耗表".to_string()
    };

    let pwr_presets = FactoryPowerPresetsSnapshot {
        ec_hardware_accessible: ec_accessible,
        office_mode: PowerLimitGroup {
            pl1_w: off_pl1,
            pl2_w: off_pl2,
            pl4_w: off_pl4,
            tcc_offset_deg: off_tcc,
        },
        balanced_mode: PowerLimitGroup {
            pl1_w: bal_pl1,
            pl2_w: bal_pl2,
            pl4_w: bal_pl4,
            tcc_offset_deg: bal_tcc,
        },
        turbo_mode: PowerLimitGroup {
            pl1_w: tur_pl1,
            pl2_w: tur_pl2,
            pl4_w: tur_pl4,
            tcc_offset_deg: tur_tcc,
        },
        current_active: PowerLimitGroup {
            pl1_w: cur_pl1,
            pl2_w: cur_pl2,
            pl4_w: cur_pl4,
            tcc_offset_deg: 0,
        },
        gpu_limits: GpuLimitGroup {
            dynamic_boost_max_w: gpu_db_max,
            configurable_tgp_w: gpu_tgp,
            db_control_byte: gpu_db_ctrl,
        },
        oem_profiles_detected: oem_profiles_exist,
        diagnosis_summary: diag_summary,
    };

    // 8. Full EC RAM Core Registers Snapshot
    let mut ec_map: BTreeMap<String, String> = BTreeMap::new();
    let ec_regs: &[(u16, &str)] = &[
        (crate::driver::wmi_ec::ADDR_PROJECT_ID, "ProjectID"),
        (crate::driver::wmi_ec::ADDR_SUPPORT_BYTE1, "HardwareSupportByte1"),
        (crate::driver::wmi_ec::ADDR_SUPPORT_BYTE2, "HardwareSupportByte2"),
        (crate::driver::wmi_ec::ADDR_BACKLIGHT_WHITE, "SingleKBL_Enable"),
        (crate::driver::wmi_ec::ADDR_SUPPORT_BYTE3, "HardwareSupportByte3"),
        (crate::driver::wmi_ec::ADDR_AC_RECOVERY, "AC_Recovery"),
        (crate::driver::wmi_ec::ADDR_CAMERA_CUTOFF, "CameraPower"),
        (crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, "PowerOff_USB"),
        (crate::driver::wmi_ec::ADDR_STATUS_BYTE, "WinKey_Lock"),
        (crate::driver::wmi_ec::ADDR_FN_LOCK, "FnKey_Lock"),
        (crate::driver::wmi_ec::ADDR_BATTERY_PROTECTION_MODE, "BatteryMode_Touchpad"),
        (crate::driver::wmi_ec::ADDR_BATTERY_LIMIT_UP, "Charge_Limit_Upper"),
        (crate::driver::wmi_ec::ADDR_BATTERY_LIMIT_DOWN, "Charge_Limit_Lower"),
        (crate::driver::wmi_ec::ADDR_WATER_COOLER_FLAG, "Bluetooth_Status"),
        (crate::driver::wmi_ec::ADDR_MAIN_FAN_RPM_B1, "CpuFanRpm_High"),
        (crate::driver::wmi_ec::ADDR_MAIN_FAN_RPM_B2, "CpuFanRpm_Low"),
        (crate::driver::wmi_ec::ADDR_SECOND_FAN_RPM_B2, "GpuFanRpm_Low"),
        (crate::driver::wmi_ec::ADDR_SECOND_FAN_RPM_B1, "GpuFanRpm_High"),
        // Factory Presets & Active Power
        (crate::driver::wmi_ec::ADDR_OFFICE_PL1, "Office_PL1_Default"),
        (crate::driver::wmi_ec::ADDR_OFFICE_PL2, "Office_PL2_Default"),
        (crate::driver::wmi_ec::ADDR_OFFICE_PL4, "Office_PL4_Default"),
        (crate::driver::wmi_ec::ADDR_OFFICE_TCC, "Office_TccOffset"),
        (crate::driver::wmi_ec::ADDR_BALANCE_PL1, "Balanced_PL1_Default"),
        (crate::driver::wmi_ec::ADDR_BALANCE_PL2, "Balanced_PL2_Default"),
        (crate::driver::wmi_ec::ADDR_BALANCE_PL4, "Balanced_PL4_Default"),
        (crate::driver::wmi_ec::ADDR_BALANCE_TCC, "Balanced_TccOffset"),
        (crate::driver::wmi_ec::ADDR_TURBO_PL1, "Turbo_PL1_Default"),
        (crate::driver::wmi_ec::ADDR_TURBO_PL2, "Turbo_PL2_Default"),
        (crate::driver::wmi_ec::ADDR_TURBO_PL4, "Turbo_PL4_Default"),
        (crate::driver::wmi_ec::ADDR_TURBO_TCC, "Turbo_TccOffset"),
        (crate::driver::wmi_ec::ADDR_CUSTOM_PL1, "Current_PL1_Active"),
        (crate::driver::wmi_ec::ADDR_CUSTOM_PL2, "Current_PL2_Active"),
        (crate::driver::wmi_ec::ADDR_CUSTOM_PL4, "Current_PL4_Active"),
        (crate::driver::wmi_ec::ADDR_DB_CTRL1, "GPU_DB_CtrlByte"),
        (crate::driver::wmi_ec::ADDR_TGP_OFFSET, "GPU_ConfigurableTGP"),
        (crate::driver::wmi_ec::ADDR_GPU_DB_MAX, "GPU_DynamicBoost_MaxTGP"),
    ];

    for &(addr, name) in ec_regs {
        let val = crate::driver::wmi_ec::wmi_read_ec(addr).unwrap_or(0);
        ec_map.insert(name.to_string(), format!("0x{:02X} ({})", val, val));
    }

    // Format local ISO-like timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    HardwareSnapshot {
        system_info: sys_info,
        gpu_environment: gpu_env,
        display_panel: disp_panel,
        bios_overclock_menu: oc_menu,
        keyboard_lighting: kb_snap,
        peripherals_and_power: pwr_snap,
        factory_power_presets: pwr_presets,
        ec_registers_snapshot: ec_map,
        probe_metadata: ProbeMetadata {
            last_probe_time: format!("Unix {}", now),
            openrevo_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    }
}

fn glob_oem_profiles_exist() -> bool {
    if let Ok(entries) = std::fs::read_dir("C:\\Program Files\\OEM") {
        for entry in entries.flatten() {
            if entry.path().join("AiStoneService\\MyControlCenter\\UserPofiles").exists() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_hardware_snapshot_factory_presets() {
        let snapshot = collect_hardware_snapshot();
        println!("\n=== Factory Power Presets Test Output ===");
        println!("EC Accessible: {}", snapshot.factory_power_presets.ec_hardware_accessible);
        println!("Office:  PL1={}W, PL2={}W, PL4={}W, TccOffset={}", 
            snapshot.factory_power_presets.office_mode.pl1_w,
            snapshot.factory_power_presets.office_mode.pl2_w,
            snapshot.factory_power_presets.office_mode.pl4_w,
            snapshot.factory_power_presets.office_mode.tcc_offset_deg
        );
        println!("Balanced: PL1={}W, PL2={}W, PL4={}W, TccOffset={}", 
            snapshot.factory_power_presets.balanced_mode.pl1_w,
            snapshot.factory_power_presets.balanced_mode.pl2_w,
            snapshot.factory_power_presets.balanced_mode.pl4_w,
            snapshot.factory_power_presets.balanced_mode.tcc_offset_deg
        );
        println!("Turbo:   PL1={}W, PL2={}W, PL4={}W, TccOffset={}", 
            snapshot.factory_power_presets.turbo_mode.pl1_w,
            snapshot.factory_power_presets.turbo_mode.pl2_w,
            snapshot.factory_power_presets.turbo_mode.pl4_w,
            snapshot.factory_power_presets.turbo_mode.tcc_offset_deg
        );
        println!("Current Active: PL1={}W, PL2={}W, PL4={}W",
            snapshot.factory_power_presets.current_active.pl1_w,
            snapshot.factory_power_presets.current_active.pl2_w,
            snapshot.factory_power_presets.current_active.pl4_w
        );
        println!("GPU: DB Max=+{}W, TGP={}W",
            snapshot.factory_power_presets.gpu_limits.dynamic_boost_max_w,
            snapshot.factory_power_presets.gpu_limits.configurable_tgp_w
        );
        println!("Summary: {}", snapshot.factory_power_presets.diagnosis_summary);
        
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        assert!(json.contains("factory_power_presets"));
        assert!(json.contains("office_mode"));
        assert!(json.contains("turbo_mode"));
        println!("=== Serialized JSON Sample (excerpt) ===");
        let excerpt: String = json.chars().take(800).collect();
        println!("{}", excerpt);
    }
}
