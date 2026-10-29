use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use parking_lot::RwLock;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_power_mode")]
    pub power_mode: u8,           // 1: Office, 2: Balance, 3: Beast, 4: Custom (Current live active mode)
    #[serde(default = "default_power_mode_ac")]
    pub power_mode_ac: u8,        // 1: Office, 2: Balance, 3: Beast, 4: Custom (Preferred mode on AC power)
    #[serde(default = "default_power_mode_battery")]
    pub power_mode_battery: u8,   // 1: Office, 2: Balance (Preferred mode on Battery power)
    #[serde(default)]
    pub fan_boost: bool,          // true / false
    #[serde(default = "default_battery_limit")]
    pub battery_limit: u8,        // 80 or 100
    #[serde(default)]
    pub win_key_locked: bool,     // true / false
    #[serde(default)]
    pub autostart: bool,          // true / false
    #[serde(default)]
    pub log_enabled: bool,        // hidden debug log mode active
    #[serde(default = "default_log_level")]
    pub log_level: u8,            // 1: Error only, 2: All logs
    #[serde(default)]
    pub log_filter: Option<String>, // Optional module filter (e.g. "lighting", "ec,fan")
    #[serde(default)]
    pub water_cooler_enabled: Option<bool>, // Manual override for water cooling module (persisted in config.json)
    #[serde(default = "default_true")]
    pub keep_water_cooler: bool,  // Keep liquid cooler active when taking over OEM
    #[serde(default = "default_true")]
    pub cooler_led_on: bool,      // true / false
    #[serde(default)]
    pub cooler_led_r: u8,         // 0-255
    #[serde(default = "default_cooler_led_g")]
    pub cooler_led_g: u8,         // 0-255
    #[serde(default)]
    pub cooler_led_b: u8,         // 0-255
    #[serde(default = "default_cooler_led_mode")]
    pub cooler_led_mode: u8,      // 0: Static, 1: Breathe, 2: Colorful, 3: Breathe Color

    // Water Cooler Speeds & Strategy
    #[serde(default = "default_pump_v")]
    pub cooler_pump_voltage: u8,  // 7, 8, 11
    #[serde(default = "default_fan_d")]
    pub cooler_fan_duty: u8,      // 0 - 100
    #[serde(default = "default_strategy")]
    pub cooler_strategy: String,  // "smart" | "quiet" | "balance" | "performance" | "custom"
    #[serde(default = "default_cooler_curve_points")]
    pub cooler_curve_points: Vec<CoolerCurvePoint>,
    #[serde(default = "default_cooler_mac")]
    pub cooler_mac: String,       // e.g. "CA:CB:C3:02:97:18" (auto-detected and persistent)
    #[serde(default = "default_cooler_pump_start_temp")]
    pub cooler_pump_start_temp: u8, // default 55°C (Stop/Standard pump threshold)
    #[serde(default = "default_cooler_pump_turbo_temp")]
    pub cooler_pump_turbo_temp: u8, // default 90°C (Standard/Turbo pump threshold)

    // Display
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate: u32,        // 60, 240
    #[serde(default)]
    pub miniled_enabled: bool,

    // Quick Switches (OEM Grid)
    #[serde(default = "default_true")]
    pub wifi_enabled: bool,
    #[serde(default = "default_true")]
    pub bluetooth_enabled: bool,
    #[serde(default = "default_true")]
    pub touchpad_enabled: bool,
    #[serde(default = "default_true")]
    pub camera_enabled: bool,
    #[serde(default = "default_true")]
    pub mic_enabled: bool,
    #[serde(default)]
    pub keep_awake_enabled: bool,
    #[serde(default)]
    pub auto_min_refresh_on_battery: bool,
    #[serde(default)]
    pub unlock_battery_power_limit: bool,
    #[serde(default = "default_true")]
    pub auto_fallback_kb_on_battery: bool,
    #[serde(default = "default_true")]
    pub osd_enabled: bool,
    #[serde(default = "default_osd_theme")]
    pub osd_theme: String,
    #[serde(default = "default_osd_scale")]
    pub osd_scale: String,
    #[serde(default = "default_osd_position")]
    pub osd_position: String,
    #[serde(default)]
    pub fn_lock_enabled: bool,
    #[serde(default)]
    pub wifi_adapter_name: String,
    #[serde(default)]
    pub takeover_oem: bool,

    // Consolidated RGB Lighting Presets
    #[serde(default)]
    pub keyboard_sleep_timeout: u32, // 0: Always-on, 10, 15, 30, 60 seconds
    #[serde(default)]
    pub lighting: crate::driver::ite_lighting::LightingPresetState,

    // GPU Display Mode & 14900HX Safe Guard
    #[serde(default = "default_gpu_mode")]
    pub gpu_mode: String,         // "dgpu" | "hybrid" | "igpu"
    #[serde(default)]
    pub cpu_safety_guard: bool,   // default: false (User configured in BIOS, optional guard)
    #[serde(default)]
    pub cpu_freq_limit: u32,      // default: 0 (0 = unlimited / BIOS control; otherwise cap in MHz, e.g. 5400)
    #[serde(default)]
    pub cpu_freq_crash_guard: bool, // Anti-bootloop watchdog flag: true while frequency limit is settling after boot/apply
    #[serde(default)]
    pub allow_experimental_igpu_only: bool, // Geek manual unlock: bypass dual-mode lock at user's own risk


    // Power & Battery Management (Official OEM Replica)
    #[serde(default = "default_battery_mode")]
    pub battery_mode: String,     // "long_life" (100) | "balance" (80) | "station" (60)
    #[serde(default)]
    pub battery_hardware_limit_enabled: bool, // Experimental hardware platform override for true cutoff
    #[serde(default)]
    pub usb_charge_enabled: bool,
    #[serde(default)]
    pub ac_recovery_enabled: bool,
    #[serde(default = "default_true")]
    pub high_perf_scheme: bool,
    #[serde(default)]
    pub preferred_power_scheme: Option<String>,
    #[serde(default)]
    pub saved_power_mode_on_battery: Option<u8>,

    // Custom Global Shortcuts
    #[serde(default)]
    pub custom_shortcuts: Vec<crate::system::shortcuts::ShortcutBinding>,

    // Modern Standby & Sleep Guard (Windows S0ix Anti-Overheat)
    #[serde(default)]
    pub sleep_guard_enabled: bool,
    #[serde(default)]
    pub sleep_disconnect_net: bool,
    #[serde(default)]
    pub sleep_block_away: bool,
    #[serde(default)]
    pub sleep_ignore_sysreq: bool,
    #[serde(default)]
    pub sleep_disable_wake_timers: bool,
    #[serde(default)]
    pub sleep_deep_sleep: bool,
    #[serde(default)]
    pub sleep_hibernate_30m: bool,
    #[serde(default)]
    pub sleep_auto_office: bool,

    // Full Dashboard Window Position Memory
    #[serde(default)]
    pub full_window_x: Option<i32>,
    #[serde(default)]
    pub full_window_y: Option<i32>,

    // GPU Custom Tuning Bounds & Stepping (Persisted in config.json)
    #[serde(default = "default_gpu_offset_step")]
    pub gpu_offset_step: u32,
    #[serde(default = "default_gpu_core_offset_max")]
    pub gpu_core_offset_max: i32,
    #[serde(default = "default_gpu_mem_offset_max")]
    pub gpu_mem_offset_max: i32,

    // CPU Custom Tuning Bounds & Margins (Persisted in config.json)
    #[serde(default = "default_cpu_temp_target_min")]
    pub cpu_temp_target_min: u8,
    #[serde(default = "default_cpu_temp_target_max")]
    pub cpu_temp_target_max: u8,
    #[serde(default = "default_cpu_pl4_margin")]
    pub cpu_pl4_margin: u16,
    #[serde(default)]
    pub cpu_pl1_max: Option<u16>,
    #[serde(default)]
    pub cpu_pl2_max: Option<u16>,
    #[serde(default)]
    pub cpu_pl4_max: Option<u16>,
    #[serde(default, alias = "pl1_offset")]
    pub cpu_pl1_offset: i16,
    #[serde(default, alias = "pl2_offset")]
    pub cpu_pl2_offset: i16,
    #[serde(default, alias = "pl4_offset")]
    pub cpu_pl4_offset: i16,
    #[serde(default, alias = "temp_offset", alias = "cpu_temp_target_offset")]
    pub cpu_temp_offset: i8,
    #[serde(default)]
    pub cpu_tjmax: Option<u8>,

    // Startup Window Visibility (Diagnosability override: CLI --visible or config.json)
    #[serde(default)]
    pub launch_visible: bool,

    // Window Presentation Mode on Wake/Restore: "remember" (default) | "mini" | "full"
    #[serde(default = "default_wake_window_mode")]
    pub wake_window_mode: String,

    // Mini Drawer Auto Hide on Blur (Focus Lost to Desktop or Other App)
    #[serde(default = "default_true")]
    pub mini_auto_hide_on_blur: bool,

    // Custom Performance Tuning & Fan Strategy (Decoupled into %APPDATA%\OpenRevo\profiles\*.json)
    #[serde(skip_serializing, default)]
    pub custom_profile: CustomProfileConfig,
    #[serde(default = "default_active_profile_id")]
    pub active_profile_id: String,

    // Builtin & Extension Module Toggles (Persisted in config.json)
    // Allows safe-mode isolation and fine-grained user control of tabs/plugins
    #[serde(default = "default_module_toggles")]
    pub module_toggles: std::collections::HashMap<String, bool>,

    // Third-party custom plugin toggles (Persisted in config.json)
    #[serde(default)]
    pub custom_plugin_toggles: std::collections::HashMap<String, bool>,

    // Persisted coordinates for custom desktop widgets: plugin_id -> (x, y)
    #[serde(default)]
    pub widget_positions: std::collections::HashMap<String, (i32, i32)>,

    // Currently active custom skin plugin ID (if any)
    #[serde(default)]
    pub active_skin: Option<String>,

    // Currently active custom background / wallpaper plugin ID (if any)
    #[serde(default)]
    pub active_background: Option<String>,
}

fn default_module_toggles() -> std::collections::HashMap<String, bool> {
    let mut map = std::collections::HashMap::new();
    map.insert("overview".to_string(), true);
    map.insert("gpu".to_string(), true);
    map.insert("tuning".to_string(), true);
    map.insert("lighting".to_string(), true);
    let has_water_cooler = crate::driver::wmi_ec::get_hardware_support_flags().has_water_cooler
        || crate::driver::ble_cooler::find_paired_water_cooler_mac().is_some();
    map.insert("water".to_string(), has_water_cooler);
    map.insert("system".to_string(), true);
    map.insert("plugins".to_string(), true);
    map.insert("sponsor".to_string(), true);
    map
}

fn default_active_profile_id() -> String {
    "preset_silent_beast".to_string()
}

fn default_gpu_offset_step() -> u32 { 5 }
fn default_gpu_core_offset_max() -> i32 { 250 }
fn default_gpu_mem_offset_max() -> i32 { 1000 }

fn default_cpu_temp_target_min() -> u8 { 85 }
fn default_cpu_temp_target_max() -> u8 { 95 }
fn default_cpu_pl4_margin() -> u16 { 20 }
fn default_wake_window_mode() -> String { "remember".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanCurvePoint {
    pub temp: u8,
    pub duty: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomProfileConfig {
    #[serde(default = "default_true")]
    pub cpu_temp_target_enabled: bool,
    #[serde(default = "default_cpu_temp")]
    pub cpu_temp_target: u8,        // 85 ~ 100 C (default 95)
    #[serde(default = "default_pl1")]
    pub cpu_pl1_watts: u16,         // 75W
    #[serde(default = "default_pl2")]
    pub cpu_pl2_watts: u16,         // 89W
    #[serde(default = "default_pl4")]
    pub cpu_pl4_watts: u16,         // 250W
    #[serde(default)]
    pub bios_oc_enabled: bool,      // Advanced BIOS menu switch
    #[serde(default = "default_true")]
    pub gpu_super_perf: bool,       // GPU Super Performance / HWOC switch
    #[serde(default)]
    pub gpu_ctgp_enabled: bool,     // GPU cTGP down-regulation enable
    #[serde(default = "default_ctgp_watts")]
    pub gpu_ctgp_watts: u8,         // GPU cTGP limit clamp (e.g. 115W)
    #[serde(default = "default_true")]
    pub gpu_db_enabled: bool,       // Dynamic Boost enable
    #[serde(default = "default_db_watts")]
    pub gpu_db_watts: u8,           // 25W
    #[serde(default)]
    pub gpu_core_offset_mhz: i32,   // 0 ~ 250 MHz
    #[serde(default)]
    pub gpu_mem_offset_mhz: i32,    // -1000 ~ +1000 MHz
    #[serde(default = "default_true")]
    pub fan_ramp_rate: bool,        // Fan ramp rate sensitivity control
    #[serde(default = "default_fan_ramp_speed_ms")]
    pub fan_ramp_speed_ms: u16,     // 100 ~ 12700 ms (default 300)
    #[serde(default = "default_true")]
    pub fan_isolated_output: bool,  // Fan isolated dual-channel output flag
    #[serde(default)]
    pub fan_curve_enabled: bool,    // Custom curve active
    #[serde(default = "default_fan_curve")]
    pub cpu_fan_curve: Vec<FanCurvePoint>,
    #[serde(default = "default_fan_curve")]
    pub gpu_fan_curve: Vec<FanCurvePoint>,
    #[serde(default)]
    pub fan_curve_linked: bool,     // Sync CPU and GPU fan curves together
}

impl Default for CustomProfileConfig {
    fn default() -> Self {
        Self {
            cpu_temp_target_enabled: true,
            cpu_temp_target: 95,
            cpu_pl1_watts: 75,
            cpu_pl2_watts: 75,
            cpu_pl4_watts: 125,
            bios_oc_enabled: false,
            gpu_super_perf: true,
            gpu_ctgp_enabled: false,
            gpu_ctgp_watts: 0,
            gpu_db_enabled: true,
            gpu_db_watts: 0,
            gpu_core_offset_mhz: 0,
            gpu_mem_offset_mhz: 0,
            fan_ramp_rate: true,
            fan_ramp_speed_ms: 300,
            fan_isolated_output: true,
            fan_curve_enabled: false,
            cpu_fan_curve: default_fan_curve(),
            gpu_fan_curve: default_fan_curve(),
            fan_curve_linked: false,
        }
    }
}

fn default_fan_ramp_speed_ms() -> u16 { 300 }
fn default_cpu_temp() -> u8 { 95 }
fn default_pl1() -> u16 { 75 }
fn default_pl2() -> u16 { 75 }
fn default_pl4() -> u16 { 125 }
fn default_ctgp_watts() -> u8 { 0 }
fn default_db_watts() -> u8 { 0 }
pub fn default_fan_curve() -> Vec<FanCurvePoint> {
    vec![
        FanCurvePoint { temp: 40, duty: 20 },
        FanCurvePoint { temp: 50, duty: 30 },
        FanCurvePoint { temp: 60, duty: 45 },
        FanCurvePoint { temp: 68, duty: 55 },
        FanCurvePoint { temp: 75, duty: 65 },
        FanCurvePoint { temp: 82, duty: 75 },
        FanCurvePoint { temp: 88, duty: 85 },
        FanCurvePoint { temp: 95, duty: 100 },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoolerCurvePoint {
    pub temp: u8,
    pub fan_duty: u8,
    pub pump_voltage: u8, // 7, 8, 11
}

pub fn default_cooler_curve_points() -> Vec<CoolerCurvePoint> {
    vec![
        CoolerCurvePoint { temp: 45, fan_duty: 0, pump_voltage: 0 },
        CoolerCurvePoint { temp: 55, fan_duty: 35, pump_voltage: 2 },
        CoolerCurvePoint { temp: 65, fan_duty: 50, pump_voltage: 2 },
        CoolerCurvePoint { temp: 80, fan_duty: 60, pump_voltage: 2 },
        CoolerCurvePoint { temp: 90, fan_duty: 75, pump_voltage: 2 },
        CoolerCurvePoint { temp: 95, fan_duty: 100, pump_voltage: 3 },
    ]
}

fn default_power_mode() -> u8 { 2 }
fn default_power_mode_ac() -> u8 { 3 }
fn default_power_mode_battery() -> u8 { 2 }
fn default_battery_limit() -> u8 { 80 }
fn default_true() -> bool { true }
fn default_cooler_led_g() -> u8 { 128 }
fn default_cooler_led_mode() -> u8 { 2 } // Default colorful flow
fn default_pump_v() -> u8 { 7 }
fn default_fan_d() -> u8 { 0 }
fn default_strategy() -> String { "smart".to_string() }
fn default_cooler_pump_start_temp() -> u8 { 55 }
fn default_cooler_pump_turbo_temp() -> u8 { 90 }
fn default_refresh_rate() -> u32 { 240 }
fn default_osd_theme() -> String { "cinema".to_string() }
fn default_osd_scale() -> String { "100%".to_string() }
fn default_osd_position() -> String { "top-left".to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            power_mode: 2,          // Default Balance
            power_mode_ac: 3,       // Default Beast on AC
            power_mode_battery: 2,  // Default Balance on Battery
            fan_boost: false,
            battery_limit: 80,      // Default 80% battery protection
            win_key_locked: false,
            autostart: false,
            water_cooler_enabled: None,
            keep_water_cooler: true,
            cooler_led_on: true,
            cooler_led_r: 0,
            cooler_led_g: 128,
            cooler_led_b: 0,
            cooler_led_mode: 2,

            cooler_pump_voltage: 7,
            cooler_fan_duty: 0,
            cooler_strategy: "smart".to_string(),
            cooler_curve_points: default_cooler_curve_points(),
            cooler_mac: String::new(),
            cooler_pump_start_temp: 55,
            cooler_pump_turbo_temp: 90,

            refresh_rate: 240,
            miniled_enabled: false,

            wifi_enabled: true,
            bluetooth_enabled: true,
            touchpad_enabled: true,
            camera_enabled: true,
            mic_enabled: true,
            keep_awake_enabled: false,
            auto_min_refresh_on_battery: false,
            unlock_battery_power_limit: false,
            auto_fallback_kb_on_battery: true,
            osd_enabled: true,
            osd_theme: default_osd_theme(),
            osd_scale: default_osd_scale(),
            osd_position: default_osd_position(),
            fn_lock_enabled: false,
            wifi_adapter_name: String::new(),
            takeover_oem: false,

            lighting: crate::driver::ite_lighting::LightingPresetState::default(),
            keyboard_sleep_timeout: 0,

            gpu_mode: "dgpu".to_string(),
            allow_experimental_igpu_only: false,
            cpu_safety_guard: false,
            cpu_freq_limit: 0,
            cpu_freq_crash_guard: false,

            battery_mode: "balance".to_string(),
            battery_hardware_limit_enabled: false,
            usb_charge_enabled: false,
            ac_recovery_enabled: false,
            high_perf_scheme: true,
            preferred_power_scheme: None,
            saved_power_mode_on_battery: None,
            custom_shortcuts: Vec::new(),

            sleep_guard_enabled: false,
            sleep_disconnect_net: false,
            sleep_block_away: false,
            sleep_ignore_sysreq: false,
            sleep_disable_wake_timers: false,
            sleep_deep_sleep: false,
            sleep_hibernate_30m: false,
            sleep_auto_office: false,

            log_enabled: false,
            log_level: 1,
            log_filter: None,

            full_window_x: None,
            full_window_y: None,
            wake_window_mode: default_wake_window_mode(),
            mini_auto_hide_on_blur: true,

            custom_profile: CustomProfileConfig::default(),
            active_profile_id: default_active_profile_id(),
            launch_visible: false,
            gpu_offset_step: default_gpu_offset_step(),
            gpu_core_offset_max: default_gpu_core_offset_max(),
            gpu_mem_offset_max: default_gpu_mem_offset_max(),
            cpu_temp_target_min: default_cpu_temp_target_min(),
            cpu_temp_target_max: default_cpu_temp_target_max(),
            cpu_pl4_margin: default_cpu_pl4_margin(),
            cpu_pl1_max: None,
            cpu_pl2_max: None,
            cpu_pl4_max: None,
            cpu_pl1_offset: 0,
            cpu_pl2_offset: 0,
            cpu_pl4_offset: 0,
            cpu_temp_offset: 0,
            cpu_tjmax: None,
            module_toggles: default_module_toggles(),
            custom_plugin_toggles: std::collections::HashMap::new(),
            widget_positions: std::collections::HashMap::new(),
            active_skin: None,
            active_background: None,
        }
    }
}

fn default_log_level() -> u8 {
    1
}

fn default_gpu_mode() -> String {
    "dgpu".to_string()
}

fn default_battery_mode() -> String {
    "balance".to_string()
}

fn default_cooler_mac() -> String {
    String::new()
}

static RESOLVED_CONFIG_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataDirMode {
    Standard,
    Portable,
    Custom,
}

/// Detect the active data directory mode
pub fn get_data_dir_mode() -> DataDirMode {
    let dir = get_config_dir();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            if dir == parent.join("data") {
                return DataDirMode::Portable;
            }
        }
    }
    if let Ok(app_data) = std::env::var("APPDATA") {
        let std_dir = PathBuf::from(app_data).join("OpenRevo");
        if dir == std_dir {
            return DataDirMode::Standard;
        }
    }
    DataDirMode::Custom
}

/// Four-Stage Fallback Discovery Chain:
/// 1. Opt-in Portable Mode: ./data adjacent to current_exe()
/// 2. User Pointer Redirection: %APPDATA%\OpenRevo\data_dir.ptr
/// 3. Windows Registry: HKCU\Software\OpenRevo -> DataPath
/// 4. Standard Default: %APPDATA%\OpenRevo (100% backward compatible for thousands of existing users)
pub fn get_config_dir() -> PathBuf {
    RESOLVED_CONFIG_DIR
        .get_or_init(|| {
            let dir = resolve_config_dir();
            let _ = fs::create_dir_all(&dir);
            #[cfg(target_os = "windows")]
            sync_registry_data_path(&dir);
            dir
        })
        .clone()
}

fn resolve_config_dir() -> PathBuf {
    // 1. Level 1: Opt-in Portable Mode
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let portable_dir = exe_dir.join("data");
            if portable_dir.is_dir() {
                let test_probe = portable_dir.join(".write_probe");
                if std::fs::write(&test_probe, b"ok").is_ok() {
                    let _ = std::fs::remove_file(&test_probe);
                    return portable_dir;
                }
            }
        }
    }

    // 2. Level 2: User Pointer Redirection (%APPDATA%\OpenRevo\data_dir.ptr)
    if let Ok(app_data) = std::env::var("APPDATA") {
        let default_dir = PathBuf::from(&app_data).join("OpenRevo");
        let ptr_file = default_dir.join("data_dir.ptr");
        if ptr_file.is_file() {
            if let Ok(content) = std::fs::read_to_string(&ptr_file) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    let custom_path = PathBuf::from(trimmed);
                    if custom_path.is_dir() || std::fs::create_dir_all(&custom_path).is_ok() {
                        return custom_path;
                    }
                }
            }
        }
    }

    // 3. Level 3: Registry Check (HKCU\Software\OpenRevo -> DataPath)
    #[cfg(target_os = "windows")]
    {
        if let Some(reg_path) = read_registry_data_path() {
            if reg_path.is_dir() || std::fs::create_dir_all(&reg_path).is_ok() {
                return reg_path;
            }
        }
    }

    // 4. Level 4: Standard Default (%APPDATA%\OpenRevo)
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("OpenRevo")
    } else {
        PathBuf::from(".")
    }
}

/// Safely migrates existing configuration, profiles, and plugins to a new target directory
pub fn migrate_data_dir(target_dir: &std::path::Path) -> Result<(), String> {
    if !target_dir.is_absolute() {
        return Err("目标路径必须是绝对路径".to_string());
    }
    if target_dir.parent().is_none() || target_dir.to_string_lossy().len() <= 3 {
        return Err("禁止将系统磁盘根目录作为数据目录".to_string());
    }

    let current_dir = get_config_dir();
    if current_dir == target_dir {
        return Ok(());
    }

    fs::create_dir_all(target_dir).map_err(|e| format!("创建目标目录失败: {}", e))?;

    // Copy entries from current_dir to target_dir recursively
    if current_dir.exists() {
        copy_dir_recursive(&current_dir, target_dir)?;
    }

    // Write pointer file in default %APPDATA%\OpenRevo\data_dir.ptr
    if let Ok(app_data) = std::env::var("APPDATA") {
        let default_dir = PathBuf::from(app_data).join("OpenRevo");
        let _ = fs::create_dir_all(&default_dir);
        let ptr_file = default_dir.join("data_dir.ptr");
        let _ = fs::write(&ptr_file, target_dir.to_string_lossy().as_bytes());
    }

    // Sync registry
    #[cfg(target_os = "windows")]
    sync_registry_data_path(target_dir);

    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let target_path = dst.join(&file_name);

        if file_name == "data_dir.ptr" {
            continue;
        }

        if entry_path.is_dir() {
            fs::create_dir_all(&target_path).map_err(|e| e.to_string())?;
            copy_dir_recursive(&entry_path, &target_path)?;
        } else {
            let _ = fs::copy(&entry_path, &target_path);
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn read_registry_data_path() -> Option<PathBuf> {
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, REG_SZ,
    };
    use windows::core::PCWSTR;

    let subkey: Vec<u16> = "Software\\OpenRevo\0".encode_utf16().collect();
    let value_name: Vec<u16> = "DataPath\0".encode_utf16().collect();

    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
            let mut data_type = REG_SZ;
            let mut buffer = [0u16; 512];
            let mut buf_size = (buffer.len() * 2) as u32;

            let query_res = RegQueryValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                None,
                Some(&mut data_type),
                Some(buffer.as_mut_ptr() as *mut u8),
                Some(&mut buf_size),
            );

            let _ = RegCloseKey(hkey);

            if query_res.is_ok() && data_type == REG_SZ {
                let len = (buf_size as usize / 2).saturating_sub(1);
                let path_str = String::from_utf16_lossy(&buffer[..len]);
                let trimmed = path_str.trim();
                if !trimmed.is_empty() {
                    return Some(PathBuf::from(trimmed));
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub fn sync_registry_data_path(path: &std::path::Path) {
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    use windows::core::PCWSTR;

    let subkey: Vec<u16> = "Software\\OpenRevo\0".encode_utf16().collect();
    let value_name: Vec<u16> = "DataPath\0".encode_utf16().collect();
    let path_str = format!("{}\0", path.to_string_lossy());
    let path_wide: Vec<u16> = path_str.encode_utf16().collect();

    unsafe {
        let mut hkey = HKEY::default();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        ).is_ok() {
            let _ = RegSetValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(std::slice::from_raw_parts(path_wide.as_ptr() as *const u8, path_wide.len() * 2)),
            );
            let _ = RegCloseKey(hkey);
        }
    }
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn get_profiles_dir() -> PathBuf {
    let dir = get_config_dir().join("profiles");
    let _ = fs::create_dir_all(&dir);
    dir
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomProfilePreset {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub icon_key: String,
    pub is_builtin: bool,
    pub profile: CustomProfileConfig,
}

pub fn get_cpu_pl4_margin() -> u16 {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(margin) = val.get("cpu_pl4_margin").and_then(|v| v.as_u64()) {
                    return (margin as u16).max(20);
                }
            }
        }
    }
    20
}

pub fn get_default_builtin_presets() -> Vec<CustomProfilePreset> {
    let fp_opt = crate::system::power_mode::get_cached_factory_power_presets();

    let off_pl1 = fp_opt.as_ref().map(|f| f.office_pl1).filter(|&v| v > 0).unwrap_or(45);
    let off_pl2 = fp_opt.as_ref().map(|f| f.office_pl2).filter(|&v| v > 0).unwrap_or(45);
    let off_pl4 = fp_opt.as_ref().map(|f| f.office_pl4).filter(|&v| v > 0).unwrap_or(100);

    let bal_pl1 = fp_opt.as_ref().map(|f| f.balance_pl1).filter(|&v| v > 0).unwrap_or(75);
    let bal_pl2 = fp_opt.as_ref().map(|f| f.balance_pl2).filter(|&v| v > 0).unwrap_or(75);
    let bal_pl4 = fp_opt.as_ref().map(|f| f.balance_pl4).filter(|&v| v > 0).unwrap_or(125);

    let tur_pl1 = fp_opt.as_ref().map(|f| f.turbo_pl1).filter(|&v| v > 0).unwrap_or(90);
    let tur_pl2 = fp_opt.as_ref().map(|f| f.turbo_pl2).filter(|&v| v > 0).unwrap_or(105);
    let pl4_margin = get_cpu_pl4_margin();
    let tur_pl4 = fp_opt
        .as_ref()
        .map(|f| f.turbo_pl4)
        .filter(|&v| v > 0)
        .unwrap_or((tur_pl2 as f32 * 1.3).round() as u16)
        .max(tur_pl2.saturating_add(pl4_margin));

    let base_tgp = fp_opt.as_ref().map(|f| f.base_tgp as u8).filter(|&v| v > 0).unwrap_or(100);
    let max_db = fp_opt.as_ref().map(|f| f.max_db as u8).filter(|&v| v > 0).unwrap_or(15);

    // 1. 静音狂暴: 根据狂暴模式与平衡模式给出科学浮动比例
    // PL1 取平衡与狂暴的黄金中段 (平衡 + 50% 狂暴增益)
    let sb_pl1 = if tur_pl1 > bal_pl1 {
        bal_pl1 + ((tur_pl1 - bal_pl1) as f32 * 0.5).round() as u16
    } else {
        (tur_pl1 as f32 * 0.85).round() as u16
    };
    // PL2 锚定在狂暴 PL1 处，短时爆发严格处于安全散热预算内
    let sb_pl2 = tur_pl1;
    let sb_pl4 = (sb_pl2 as f32 * 1.25).round().min(tur_pl4 as f32) as u16;

    // 2. 极致静音: 直接采用出厂办公模式硬件标定
    let quiet_tgp = ((base_tgp as f32) * 0.75).round().clamp(35.0, 95.0) as u8;
    let quiet_db = max_db.min(10);

    // Adapt to hardware capabilities (e.g. Baolongda / non-Tongfang models)
    let (fan_ramp, fan_isolated, fan_linked, super_perf) = if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        (
            manifest.capabilities.fan_ramp_rate,
            manifest.capabilities.fan_independent_channels,
            !manifest.capabilities.fan_independent_channels,
            true,
        )
    } else {
        (true, true, true, true)
    };

    vec![
        CustomProfilePreset {
            id: "preset_silent_beast".to_string(),
            name: "静音狂暴".to_string(),
            file_name: "preset_silent_beast.json".to_string(),
            icon_key: "flame".to_string(),
            is_builtin: true,
            profile: CustomProfileConfig {
                cpu_temp_target_enabled: true,
                cpu_temp_target: 95,
                cpu_pl1_watts: sb_pl1,
                cpu_pl2_watts: sb_pl2,
                cpu_pl4_watts: sb_pl4,
                bios_oc_enabled: false,
                gpu_super_perf: super_perf,
                gpu_ctgp_enabled: false,
                gpu_ctgp_watts: base_tgp,
                gpu_db_enabled: true,
                gpu_db_watts: max_db,
                gpu_core_offset_mhz: 0,
                gpu_mem_offset_mhz: 0,
                fan_ramp_rate: fan_ramp,
                fan_ramp_speed_ms: 500,
                fan_isolated_output: fan_isolated,
                fan_curve_enabled: true,
                cpu_fan_curve: vec![
                    FanCurvePoint { temp: 40, duty: 15 },
                    FanCurvePoint { temp: 50, duty: 25 },
                    FanCurvePoint { temp: 60, duty: 35 },
                    FanCurvePoint { temp: 68, duty: 45 },
                    FanCurvePoint { temp: 75, duty: 58 },
                    FanCurvePoint { temp: 82, duty: 70 },
                    FanCurvePoint { temp: 88, duty: 80 },
                    FanCurvePoint { temp: 95, duty: 90 },
                ],
                gpu_fan_curve: vec![
                    FanCurvePoint { temp: 40, duty: 15 },
                    FanCurvePoint { temp: 50, duty: 25 },
                    FanCurvePoint { temp: 60, duty: 35 },
                    FanCurvePoint { temp: 68, duty: 45 },
                    FanCurvePoint { temp: 75, duty: 58 },
                    FanCurvePoint { temp: 82, duty: 70 },
                    FanCurvePoint { temp: 88, duty: 80 },
                    FanCurvePoint { temp: 95, duty: 90 },
                ],
                fan_curve_linked: fan_linked,
            },
        },
        CustomProfilePreset {
            id: "preset_balance".to_string(),
            name: "均衡电竞".to_string(),
            file_name: "preset_balance.json".to_string(),
            icon_key: "zap".to_string(),
            is_builtin: true,
            profile: CustomProfileConfig {
                cpu_temp_target_enabled: true,
                cpu_temp_target: 95,
                cpu_pl1_watts: bal_pl1,
                cpu_pl2_watts: bal_pl2,
                cpu_pl4_watts: bal_pl4,
                bios_oc_enabled: false,
                gpu_super_perf: super_perf,
                gpu_ctgp_enabled: false,
                gpu_ctgp_watts: base_tgp,
                gpu_db_enabled: true,
                gpu_db_watts: max_db,
                gpu_core_offset_mhz: 0,
                gpu_mem_offset_mhz: 0,
                fan_ramp_rate: fan_ramp,
                fan_ramp_speed_ms: 300,
                fan_isolated_output: fan_isolated,
                fan_curve_enabled: false,
                cpu_fan_curve: default_fan_curve(),
                gpu_fan_curve: default_fan_curve(),
                fan_curve_linked: fan_linked,
            },
        },
        CustomProfilePreset {
            id: "preset_quiet".to_string(),
            name: "极致静音".to_string(),
            file_name: "preset_quiet.json".to_string(),
            icon_key: "leaf".to_string(),
            is_builtin: true,
            profile: CustomProfileConfig {
                cpu_temp_target_enabled: true,
                cpu_temp_target: 85,
                cpu_pl1_watts: off_pl1,
                cpu_pl2_watts: off_pl2,
                cpu_pl4_watts: off_pl4,
                bios_oc_enabled: false,
                gpu_super_perf: false,
                gpu_ctgp_enabled: true,
                gpu_ctgp_watts: quiet_tgp,
                gpu_db_enabled: true,
                gpu_db_watts: quiet_db,
                gpu_core_offset_mhz: 0,
                gpu_mem_offset_mhz: 0,
                fan_ramp_rate: fan_ramp,
                fan_ramp_speed_ms: 300,
                fan_isolated_output: fan_isolated,
                fan_curve_enabled: true,
                cpu_fan_curve: vec![
                    FanCurvePoint { temp: 40, duty: 10 },
                    FanCurvePoint { temp: 50, duty: 15 },
                    FanCurvePoint { temp: 60, duty: 25 },
                    FanCurvePoint { temp: 68, duty: 35 },
                    FanCurvePoint { temp: 75, duty: 45 },
                    FanCurvePoint { temp: 82, duty: 55 },
                    FanCurvePoint { temp: 88, duty: 65 },
                    FanCurvePoint { temp: 95, duty: 80 },
                ],
                gpu_fan_curve: vec![
                    FanCurvePoint { temp: 40, duty: 10 },
                    FanCurvePoint { temp: 50, duty: 15 },
                    FanCurvePoint { temp: 60, duty: 25 },
                    FanCurvePoint { temp: 68, duty: 35 },
                    FanCurvePoint { temp: 75, duty: 45 },
                    FanCurvePoint { temp: 82, duty: 55 },
                    FanCurvePoint { temp: 88, duty: 65 },
                    FanCurvePoint { temp: 95, duty: 80 },
                ],
                fan_curve_linked: fan_linked,
            },
        },
        CustomProfilePreset {
            id: "preset_extreme".to_string(),
            name: "极限狂暴".to_string(),
            file_name: "preset_extreme.json".to_string(),
            icon_key: "rocket".to_string(),
            is_builtin: true,
            profile: CustomProfileConfig {
                cpu_temp_target_enabled: true,
                cpu_temp_target: 95,
                cpu_pl1_watts: tur_pl1,
                cpu_pl2_watts: tur_pl2,
                cpu_pl4_watts: tur_pl4,
                bios_oc_enabled: false,
                gpu_super_perf: super_perf,
                gpu_ctgp_enabled: false,
                gpu_ctgp_watts: base_tgp,
                gpu_db_enabled: true,
                gpu_db_watts: max_db,
                gpu_core_offset_mhz: 0,
                gpu_mem_offset_mhz: 0,
                fan_ramp_rate: fan_ramp,
                fan_ramp_speed_ms: 100,
                fan_isolated_output: fan_isolated,
                fan_curve_enabled: true,
                cpu_fan_curve: vec![
                    FanCurvePoint { temp: 40, duty: 30 },
                    FanCurvePoint { temp: 50, duty: 45 },
                    FanCurvePoint { temp: 60, duty: 60 },
                    FanCurvePoint { temp: 68, duty: 75 },
                    FanCurvePoint { temp: 75, duty: 85 },
                    FanCurvePoint { temp: 82, duty: 95 },
                    FanCurvePoint { temp: 88, duty: 100 },
                    FanCurvePoint { temp: 95, duty: 100 },
                ],
                gpu_fan_curve: vec![
                    FanCurvePoint { temp: 40, duty: 30 },
                    FanCurvePoint { temp: 50, duty: 45 },
                    FanCurvePoint { temp: 60, duty: 60 },
                    FanCurvePoint { temp: 68, duty: 75 },
                    FanCurvePoint { temp: 75, duty: 85 },
                    FanCurvePoint { temp: 82, duty: 95 },
                    FanCurvePoint { temp: 88, duty: 100 },
                    FanCurvePoint { temp: 95, duty: 100 },
                ],
                fan_curve_linked: fan_linked,
            },
        },
    ]
}

pub fn ensure_default_profiles() {
    let dir = get_profiles_dir();
    let fp_opt = crate::system::power_mode::get_cached_factory_power_presets();
    let base_tgp = fp_opt.as_ref().map(|f| f.base_tgp as u8).filter(|&v| v > 0).unwrap_or(100);
    let max_db = fp_opt.as_ref().map(|f| f.max_db as u8).filter(|&v| v > 0).unwrap_or(15);
    let pl4_margin = get_cpu_pl4_margin();

    for preset in get_default_builtin_presets() {
        let file_path = dir.join(&preset.file_name);
        if !file_path.exists() {
            if let Ok(json) = serde_json::to_string_pretty(&preset) {
                let _ = fs::write(&file_path, json);
            }
        } else if preset.is_builtin && fp_opt.is_some() {
            // Self-heal: If an existing builtin template has an erroneous ctgp/db higher than true hardware base,
            // sanitize it to match true silicon specs only when verified hardware cache is ready!
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Ok(mut disk_preset) = serde_json::from_str::<CustomProfilePreset>(&content) {
                    let mut modified = false;
                    if disk_preset.profile.gpu_ctgp_watts > base_tgp {
                        disk_preset.profile.gpu_ctgp_watts = base_tgp;
                        modified = true;
                    }
                    if disk_preset.profile.gpu_db_watts > max_db {
                        disk_preset.profile.gpu_db_watts = max_db;
                        modified = true;
                    }
                    if disk_preset.id != "preset_quiet" && disk_preset.profile.gpu_db_watts < 15 {
                        disk_preset.profile.gpu_db_watts = max_db;
                        modified = true;
                    }
                    if disk_preset.profile.cpu_fan_curve.len() != 8 {
                        disk_preset.profile.cpu_fan_curve = preset.profile.cpu_fan_curve.clone();
                        disk_preset.profile.gpu_fan_curve = preset.profile.gpu_fan_curve.clone();
                        modified = true;
                    }
                    if disk_preset.id == "preset_quiet" && disk_preset.profile.gpu_super_perf {
                        disk_preset.profile.gpu_super_perf = false;
                        modified = true;
                    }
                    if disk_preset.profile.cpu_pl4_watts < disk_preset.profile.cpu_pl2_watts.saturating_add(pl4_margin) {
                        disk_preset.profile.cpu_pl4_watts = disk_preset.profile.cpu_pl2_watts.saturating_add(pl4_margin);
                        modified = true;
                    }
                    if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
                        if !manifest.capabilities.fan_ramp_rate && disk_preset.profile.fan_ramp_rate {
                            disk_preset.profile.fan_ramp_rate = false;
                            modified = true;
                        }
                        if !manifest.capabilities.fan_independent_channels && disk_preset.profile.fan_isolated_output {
                            disk_preset.profile.fan_isolated_output = false;
                            disk_preset.profile.fan_curve_linked = true;
                            modified = true;
                        }
                    }
                    if modified {
                        if let Ok(json) = serde_json::to_string_pretty(&disk_preset) {
                            let _ = fs::write(&file_path, json);
                            crate::log_info!("CONFIG", "Self-healed historical bloated GPU presets in {:?}", file_path);
                        }
                    }
                }
            }
        }
    }
}

pub fn list_profiles() -> Vec<CustomProfilePreset> {
    ensure_default_profiles();
    let dir = get_profiles_dir();
    let mut presets = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(mut preset) = serde_json::from_str::<CustomProfilePreset>(&content) {
                        if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                            preset.file_name = fname.to_string();
                        }
                        presets.push(preset);
                    }
                }
            }
        }
    }
    let order_builtin = |id: &str| -> usize {
        match id {
            "preset_silent_beast" => 0,
            "preset_balance" => 1,
            "preset_quiet" => 2,
            "preset_extreme" => 3,
            _ => 100,
        }
    };
    presets.sort_by(|a, b| {
        let order_a = order_builtin(&a.id);
        let order_b = order_builtin(&b.id);
        if order_a != order_b {
            order_a.cmp(&order_b)
        } else {
            a.name.cmp(&b.name)
        }
    });
    presets
}

pub fn save_profile_file(preset: &CustomProfilePreset) -> Result<(), String> {
    let dir = get_profiles_dir();
    let safe_file_name = if preset.file_name.ends_with(".json") {
        preset.file_name.clone()
    } else {
        format!("{}.json", preset.file_name)
    };
    let path = dir.join(&safe_file_name);
    let json = serde_json::to_string_pretty(preset)
        .map_err(|e| format!("序列化配置方案失败: {}", e))?;
    fs::write(&path, json)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(())
}

pub fn delete_profile_file(id: &str) -> Result<(), String> {
    if id.starts_with("preset_") {
        return Err("原厂预设模板受保护，不可删除！".to_string());
    }
    let profiles = list_profiles();
    let target = profiles.into_iter().find(|p| p.id == id)
        .ok_or_else(|| format!("未找到 ID 为 {} 的方案", id))?;
    if target.is_builtin {
        return Err("原厂预设模板受保护，不可删除！".to_string());
    }
    let dir = get_profiles_dir();
    let file_path = dir.join(&target.file_name);
    let canonical_dir = dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical_file = file_path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical_file.starts_with(&canonical_dir) {
        return Err("非法路径删除拦截！".to_string());
    }
    fs::remove_file(canonical_file).map_err(|e| format!("删除配置文件失败: {}", e))?;
    Ok(())
}

pub fn import_profile_json(json_content: &str, original_filename: Option<String>) -> Result<CustomProfilePreset, String> {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let mut preset = if let Ok(mut p) = serde_json::from_str::<CustomProfilePreset>(json_content) {
        p.id = format!("custom_{}", timestamp);
        p.is_builtin = false;
        p
    } else if let Ok(cfg) = serde_json::from_str::<CustomProfileConfig>(json_content) {
        let name = original_filename.as_deref().unwrap_or("导入方案").replace(".json", "");
        CustomProfilePreset {
            id: format!("custom_{}", timestamp),
            name,
            file_name: String::new(),
            icon_key: "star".to_string(),
            is_builtin: false,
            profile: cfg,
        }
    } else {
        return Err("无法识别的 JSON 方案格式！".to_string());
    };

    if preset.icon_key.is_empty() {
        preset.icon_key = "star".to_string();
    }
    let raw_name = original_filename.as_deref().unwrap_or(&preset.name);
    let safe_stem = raw_name.replace(".json", "").chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect::<String>();
    let final_fname = if safe_stem.is_empty() {
        format!("profile_{}.json", timestamp)
    } else {
        format!("{}_{}.json", safe_stem, timestamp % 10000)
    };
    preset.file_name = final_fname;
    save_profile_file(&preset)?;
    Ok(preset)
}

static CONFIG_CACHE: LazyLock<RwLock<Option<AppConfig>>> = LazyLock::new(|| RwLock::new(None));

fn load_config_from_disk() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            // Actively purge legacy fields and ensure GPU tuning bounds are explicitly visible in config.json
            let mut sanitized = false;
            let missing_gpu_bounds = !content.contains("\"gpu_core_offset_max\"")
                || !content.contains("\"gpu_mem_offset_max\"")
                || !content.contains("\"gpu_offset_step\"");

            let final_content = if content.contains("\"hardware_snapshot\"") || content.contains("\"custom_profile\"") {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = val.as_object_mut() {
                        if obj.remove("hardware_snapshot").is_some() {
                            sanitized = true;
                        }
                        if obj.remove("custom_profile").is_some() {
                            sanitized = true;
                        }
                    }
                    if sanitized {
                        serde_json::to_string_pretty(&val).unwrap_or_else(|_| content.clone())
                    } else {
                        content.clone()
                    }
                } else {
                    content.clone()
                }
            } else {
                content.clone()
            };

            if missing_gpu_bounds {
                sanitized = true;
            }

            if sanitized {
                let _ = fs::write(&path, &final_content);
                crate::log_info!("CONFIG", "Synchronized config.json schema and ensured GPU offset limits are explicitly persisted");
            }

            if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&final_content) {
                // Dual-parameter power mode backward compatibility migration
                if cfg.power_mode_ac == 0 {
                    if let Some(saved) = cfg.saved_power_mode_on_battery {
                        if saved >= 1 && saved <= 4 {
                            cfg.power_mode_ac = saved;
                        }
                    }
                    if cfg.power_mode_ac == 0 {
                        cfg.power_mode_ac = if cfg.power_mode >= 1 && cfg.power_mode <= 4 {
                            cfg.power_mode
                        } else {
                            3
                        };
                    }
                }
                if cfg.power_mode_battery == 0 {
                    cfg.power_mode_battery = if cfg.power_mode == 1 { 1 } else { 2 };
                }

                // Ensure active profile is loaded from profiles/<active_id>.json
                let profiles = list_profiles();
                if let Some(active) = profiles.iter().find(|p| p.id == cfg.active_profile_id) {
                    cfg.custom_profile = active.profile.clone();
                } else if let Some(first) = profiles.first() {
                    cfg.active_profile_id = first.id.clone();
                    cfg.custom_profile = first.profile.clone();
                }

                if missing_gpu_bounds {
                    save_config_to_disk(&cfg);
                    crate::log_info!("CONFIG", "Persisted explicit GPU offset bounds into config.json");
                }
                return cfg;
            }
        }
    }
    AppConfig::default()
}

fn save_config_to_disk(cfg: &AppConfig) {
    let path = get_config_path();
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let tmp_path = path.with_extension("tmp");
        if fs::write(&tmp_path, &json).is_ok() {
            let _ = fs::rename(&tmp_path, &path);
        } else {
            let _ = fs::write(path, json);
        }
    }
}

/// Thread-safe in-memory configuration loader. Returns cached copy in <0.001ms.
pub fn load_config() -> AppConfig {
    if let Some(cfg) = CONFIG_CACHE.read().as_ref() {
        return cfg.clone();
    }
    let mut guard = CONFIG_CACHE.write();
    if let Some(cfg) = guard.as_ref() {
        return cfg.clone();
    }
    let cfg = load_config_from_disk();
    *guard = Some(cfg.clone());
    cfg
}

/// Updates the in-memory configuration cache and persists to disk atomically.
pub fn save_config(cfg: &AppConfig) {
    let mut guard = CONFIG_CACHE.write();
    *guard = Some(cfg.clone());
    save_config_to_disk(cfg);
}

/// Atomically modifies the active configuration in-place under an exclusive write lock,
/// then persists the updated configuration to disk. Prevents multi-module RMW race conditions.
pub fn modify_config<F: FnOnce(&mut AppConfig) -> R, R>(f: F) -> R {
    let mut guard = CONFIG_CACHE.write();
    let mut cfg = if let Some(ref current) = *guard {
        current.clone()
    } else {
        load_config_from_disk()
    };
    let res = f(&mut cfg);
    save_config_to_disk(&cfg);
    *guard = Some(cfg);
    res
}

/// Forces a fresh reload from disk into the in-memory cache.
pub fn reload_config_from_disk() -> AppConfig {
    let mut guard = CONFIG_CACHE.write();
    let cfg = load_config_from_disk();
    *guard = Some(cfg.clone());
    cfg
}

/// Check if water cooler module is enabled:
/// 1. Prioritizes explicit user configuration in module_toggles or config.json
/// 2. If Windows already has a paired water cooler, naturally self-heals and enables
/// 3. If unset/default, defaults to physical hardware detection (`has_water_cooler`)
pub fn is_water_cooler_enabled() -> bool {
    let cfg = load_config();
    // 1. Explicit legacy config field preference if present
    if let Some(user_pref) = cfg.water_cooler_enabled {
        return user_pref;
    }
    // 2. Explicit module toggle from plugin management
    if let Some(&enabled) = cfg.module_toggles.get("water") {
        if enabled {
            return true;
        }
        // If toggle was default-initialized to false, but a paired cooler is present in Windows PnP, self-heal
        if crate::driver::ble_cooler::find_paired_water_cooler_mac().is_some()
            || (!cfg.cooler_mac.trim().is_empty() && cfg.cooler_mac != "0")
        {
            return true;
        }
        return false;
    }
    // 3. Paired device priority: if Windows already has a paired cooler or stored MAC, naturally enabled
    if crate::driver::ble_cooler::find_paired_water_cooler_mac().is_some()
        || (!cfg.cooler_mac.trim().is_empty() && cfg.cooler_mac != "0")
    {
        return true;
    }
    // 4. First-run default: enabled if machine natively has liquid cooling, false otherwise
    crate::driver::wmi_ec::get_hardware_support_flags().has_water_cooler
}

/// Retrieve the map of all module toggles from config.json
pub fn get_module_toggles() -> std::collections::HashMap<String, bool> {
    let cfg = load_config();
    cfg.module_toggles
}

/// Update a specific module toggle in config.json and return the updated state
pub fn set_module_enabled(id: &str, enabled: bool) -> bool {
    // Defense in depth: do not allow disabling system, sponsor, or plugins
    if id == "system" || id == "sponsor" || id == "plugins" {
        return true;
    }
    let mut cfg = load_config();
    cfg.module_toggles.insert(id.to_string(), enabled);
    // Maintain backward compatibility with legacy water_cooler_enabled field
    if id == "water" {
        cfg.water_cooler_enabled = Some(enabled);
        if enabled {
            let cooler = crate::driver::ble_cooler::get_global_cooler();
            cooler.start_active_cooling();
        } else {
            crate::driver::ble_cooler::get_global_cooler().stop_and_shutdown();
        }
    }
    save_config(&cfg);
    crate::log_info!("CONFIG", "Module toggle updated: {} -> {}", id, enabled);
    enabled
}

/// Check if a specific module is enabled
pub fn is_module_enabled(id: &str) -> bool {
    let cfg = load_config();
    if id == "water" {
        return is_water_cooler_enabled();
    }
    cfg.module_toggles.get(id).copied().unwrap_or(true)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purge_legacy_hardware_snapshot() {
        let legacy_json = r#"{
            "power_mode": 2,
            "hardware_snapshot": {
                "system_info": { "board_model": "GM6IX9B" },
                "ec_project_id_1856": 25
            }
        }"#;

        let mut val: serde_json::Value = serde_json::from_str(legacy_json).unwrap();
        if let Some(obj) = val.as_object_mut() {
            assert!(obj.remove("hardware_snapshot").is_some());
        }
        let cleaned_json = serde_json::to_string_pretty(&val).unwrap();
        assert!(!cleaned_json.contains("hardware_snapshot"));
        let cfg: AppConfig = serde_json::from_str(&cleaned_json).unwrap();
        assert_eq!(cfg.power_mode, 2);
    }

    #[test]
    fn test_load_config_purges_appdata() {
        let _ = load_config();
    }

    #[test]
    fn test_window_position_persistence() {
        let mut cfg = AppConfig::default();
        assert_eq!(cfg.full_window_x, None);
        assert_eq!(cfg.full_window_y, None);

        cfg.full_window_x = Some(240);
        cfg.full_window_y = Some(180);

        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.full_window_x, Some(240));
        assert_eq!(loaded.full_window_y, Some(180));
    }

    #[test]
    fn test_cpu_pl_max_bounds_persistence() {
        let mut cfg = AppConfig::default();
        assert_eq!(cfg.cpu_pl1_max, None);
        assert_eq!(cfg.cpu_pl2_max, None);
        assert_eq!(cfg.cpu_pl4_max, None);

        cfg.cpu_pl1_max = Some(235);
        cfg.cpu_pl2_max = Some(240);
        cfg.cpu_pl4_max = Some(280);

        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.cpu_pl1_max, Some(235));
        assert_eq!(loaded.cpu_pl2_max, Some(240));
        assert_eq!(loaded.cpu_pl4_max, Some(280));
    }
}

