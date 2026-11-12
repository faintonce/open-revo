use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC, HDC};

#[link(name = "gdi32")]
extern "system" {
    fn SetDeviceGammaRamp(hdc: HDC, lp_ramp: *const std::ffi::c_void) -> windows::Win32::Foundation::BOOL;
}

/// Individual color preset definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayColorPreset {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub brightness_pct: Option<f32>,
    pub color_temp_k: u32,       // 1000..=12000, standard D65 = 6500
    pub gain_r: f32,             // 0.0..=2.0, default 1.0
    pub gain_g: f32,             // 0.0..=2.0, default 1.0
    pub gain_b: f32,             // 0.0..=2.0, default 1.0
    pub saturation: f32,         // 0.0..=2.5, default 1.0
    pub contrast: f32,           // 0.5..=2.0, default 1.0
    pub gamma: f32,              // 0.5..=2.0, default 1.0
    #[serde(default)]
    pub is_builtin: bool,
}

impl Default for DisplayColorPreset {
    fn default() -> Self {
        Self {
            id: "preset_vibrant".to_string(),
            name: "多彩鲜艳 (Vibrant)".to_string(),
            author: Some("机械革命官方".to_string()),
            brightness_pct: Some(50.0),
            color_temp_k: 6500,
            gain_r: 1.0,
            gain_g: 1.0,
            gain_b: 1.0,
            saturation: 1.5,
            contrast: 1.0,
            gamma: 1.0,
            is_builtin: true,
        }
    }
}

/// Persistent configuration for display tuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayTuningConfig {
    pub enabled: bool,
    pub active_preset_id: String,
    pub custom_preset: DisplayColorPreset,
    #[serde(default)]
    pub preset_overrides: HashMap<String, DisplayColorPreset>,
}

impl Default for DisplayTuningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            active_preset_id: "preset_vibrant".to_string(),
            custom_preset: DisplayColorPreset {
                id: "preset_custom".to_string(),
                name: "用户自定义".to_string(),
                author: Some("用户".to_string()),
                brightness_pct: None,
                color_temp_k: 6500,
                gain_r: 1.0,
                gain_g: 1.0,
                gain_b: 1.0,
                saturation: 1.0,
                contrast: 1.0,
                gamma: 1.0,
                is_builtin: false,
            },
            preset_overrides: HashMap::new(),
        }
    }
}

/// Frontend-facing state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayTuningState {
    pub enabled: bool,
    pub active_preset_id: String,
    pub active_preset: DisplayColorPreset,
    pub presets: Vec<DisplayColorPreset>,
    #[serde(default)]
    pub factory_presets: Vec<DisplayColorPreset>,
}

#[repr(C)]
struct GammaRamp {
    red: [u16; 256],
    green: [u16; 256],
    blue: [u16; 256],
}

#[repr(C)]
struct MagColorEffect {
    transform: [f32; 25],
}

type MagInitFn = unsafe extern "system" fn() -> windows::Win32::Foundation::BOOL;
type MagUninitFn = unsafe extern "system" fn() -> windows::Win32::Foundation::BOOL;
type MagSetEffectFn = unsafe extern "system" fn(*const MagColorEffect) -> windows::Win32::Foundation::BOOL;

static MAG_INITIALIZED: AtomicBool = AtomicBool::new(false);
static MAG_LIB: OnceLock<Option<libloading::Library>> = OnceLock::new();
static ENGINE_LOCK: Mutex<()> = Mutex::new(());

fn get_mag_lib() -> Option<&'static libloading::Library> {
    MAG_LIB.get_or_init(|| unsafe {
        libloading::Library::new("Magnification.dll").ok()
    }).as_ref()
}

/// Convert color temperature (Kelvin) to normalized RGB gain relative to standard D65 (6500K)
pub fn kelvin_to_rgb(kelvin: u32) -> (f32, f32, f32) {
    if kelvin == 6500 {
        return (1.0, 1.0, 1.0);
    }

    fn raw_blackbody(k: u32) -> (f32, f32, f32) {
        let temp = (k.clamp(1000, 40000) as f32) / 100.0;

        let r = if temp <= 66.0 {
            255.0
        } else {
            329.698727446 * (temp - 60.0).powf(-0.1332047592)
        };

        let g = if temp <= 66.0 {
            99.4708025861 * temp.ln() - 161.1195681661
        } else {
            288.1221695283 * (temp - 60.0).powf(-0.0755148492)
        };

        let b = if temp >= 66.0 {
            255.0
        } else if temp <= 19.0 {
            0.0
        } else {
            138.5177312231 * (temp - 10.0).ln() - 305.0447927307
        };

        (
            r.clamp(0.0, 255.0),
            g.clamp(0.0, 255.0),
            b.clamp(0.0, 255.0),
        )
    }

    let (d65_r, d65_g, d65_b) = raw_blackbody(6500);
    let (raw_r, raw_g, raw_b) = raw_blackbody(kelvin);

    (
        (raw_r / d65_r).clamp(0.0, 2.0),
        (raw_g / d65_g).clamp(0.0, 2.0),
        (raw_b / d65_b).clamp(0.0, 2.0),
    )
}

/// Apply hardware Gamma Ramp via Win32 GDI SetDeviceGammaRamp (Best-effort, does not fail if unsupported).
/// Preserves any active Windows/OpenRevo ICC profile's VCGT/MHC2 calibration curve as the base ramp.
pub fn apply_gamma_ramp(preset: &DisplayColorPreset) {
    let (temp_r, temp_g, temp_b) = kelvin_to_rgb(preset.color_temp_k);
    let final_r = preset.gain_r.clamp(0.0, 2.0) * temp_r;
    let final_g = preset.gain_g.clamp(0.0, 2.0) * temp_g;
    let final_b = preset.gain_b.clamp(0.0, 2.0) * temp_b;

    let gamma = preset.gamma.clamp(0.5, 2.0);
    let contrast = preset.contrast.clamp(0.5, 2.0);

    let build_channel = |base_val: f32, gain: f32| -> u16 {
        let curved = base_val.powf(1.0 / gamma);
        let adjusted = (curved - 0.5) * contrast + 0.5;
        let val = (adjusted * gain).clamp(0.0, 1.0);
        (val * 65535.0).round() as u16
    };

    let devices = crate::system::color_calibration::enumerate_active_display_devices();
    if !devices.is_empty() {
        for dev in devices {
            let base_opt = crate::system::color_calibration::get_effective_base_ramp_for_device(&dev);
            let mut ramp = crate::system::color_calibration::GammaRamp {
                red: [0; 256],
                green: [0; 256],
                blue: [0; 256],
            };
            for i in 0..256 {
                let linear_x = (i as f32) / 255.0;
                let (bx_r, bx_g, bx_b) = if let Some(ref base) = base_opt {
                    (
                        (base.red[i] as f32) / 65535.0,
                        (base.green[i] as f32) / 65535.0,
                        (base.blue[i] as f32) / 65535.0,
                    )
                } else {
                    (linear_x, linear_x, linear_x)
                };
                ramp.red[i] = build_channel(bx_r, final_r);
                ramp.green[i] = build_channel(bx_g, final_g);
                ramp.blue[i] = build_channel(bx_b, final_b);
            }
            let _ = crate::system::color_calibration::apply_hardware_ramp_for_device(&dev, &ramp);
        }
        return;
    }

    let mut ramp = GammaRamp {
        red: [0; 256],
        green: [0; 256],
        blue: [0; 256],
    };
    for i in 0..256 {
        let x = (i as f32) / 255.0;
        ramp.red[i] = build_channel(x, final_r);
        ramp.green[i] = build_channel(x, final_g);
        ramp.blue[i] = build_channel(x, final_b);
    }

    unsafe {
        let hdc: HDC = GetDC(HWND::default());
        if !hdc.0.is_null() {
            let _ = SetDeviceGammaRamp(hdc, &ramp as *const _ as *const _);
            let _ = ReleaseDC(HWND::default(), hdc);
        }
    }
}

/// Apply full 5x5 color transform matrix via Magnification.dll (DWM GPU compositor)
pub fn apply_color_matrix(preset: &DisplayColorPreset) -> Result<(), String> {
    let lib = get_mag_lib().ok_or_else(|| "Failed to load Magnification.dll".to_string())?;

    let s = preset.saturation.clamp(0.0, 2.5);
    let (temp_r, temp_g, temp_b) = kelvin_to_rgb(preset.color_temp_k);
    let gr = preset.gain_r.clamp(0.0, 2.0) * temp_r;
    let gg = preset.gain_g.clamp(0.0, 2.0) * temp_g;
    let gb = preset.gain_b.clamp(0.0, 2.0) * temp_b;
    let contrast = preset.contrast.clamp(0.5, 2.0);

    let wr = 0.2126f32;
    let wg = 0.7152f32;
    let wb = 0.0722f32;

    let mut matrix = MagColorEffect {
        transform: [0.0; 25],
    };

    // Magnification.dll vector-matrix convention:
    // [R', G', B', A', 1] = [R, G, B, A, 1] * Matrix
    // Output R' = R * transform[0] + G * transform[5] + B * transform[10] + transform[20]
    // Output G' = R * transform[1] + G * transform[6] + B * transform[11] + transform[21]
    // Output B' = R * transform[2] + G * transform[7] + B * transform[12] + transform[22]

    // Column 0: Red Output
    matrix.transform[0] = ((1.0 - s) * wr + s) * gr * contrast;
    matrix.transform[5] = ((1.0 - s) * wg)     * gr * contrast;
    matrix.transform[10]= ((1.0 - s) * wb)    * gr * contrast;

    // Column 1: Green Output
    matrix.transform[1] = ((1.0 - s) * wr)     * gg * contrast;
    matrix.transform[6] = ((1.0 - s) * wg + s) * gg * contrast;
    matrix.transform[11]= ((1.0 - s) * wb)    * gg * contrast;

    // Column 2: Blue Output
    matrix.transform[2] = ((1.0 - s) * wr)     * gb * contrast;
    matrix.transform[7] = ((1.0 - s) * wg)     * gb * contrast;
    matrix.transform[12]= ((1.0 - s) * wb + s)* gb * contrast;

    // Row 3: Alpha (Identity)
    matrix.transform[18] = 1.0;

    // Row 4: Contrast offset & W (Identity)
    let offset = 0.5 * (1.0 - contrast);
    matrix.transform[20] = offset;
    matrix.transform[21] = offset;
    matrix.transform[22] = offset;
    matrix.transform[24] = 1.0;

    unsafe {
        let init: MagInitFn = *lib.get(b"MagInitialize\0").map_err(|e| e.to_string())?;
        let set_effect: MagSetEffectFn = *lib
            .get(b"MagSetFullscreenColorEffect\0")
            .map_err(|e| e.to_string())?;

        if !MAG_INITIALIZED.swap(true, Ordering::SeqCst) {
            let _ = init();
        }

        let res = set_effect(&matrix);
        if !res.as_bool() {
            return Err("MagSetFullscreenColorEffect returned false".to_string());
        }
    }
    Ok(())
}

/// Reset display tuning back to active ICC calibration (or 100% linear default if none)
pub fn reset_display_tuning_to_default() {
    let _lock = ENGINE_LOCK.lock();
    // 1. Restore each display's active ICC hardware gamma ramp (fallback to linear identity ramp)
    if !crate::system::color_calibration::restore_active_icc_ramps_for_all_devices() {
        let mut linear_ramp = GammaRamp {
            red: [0; 256],
            green: [0; 256],
            blue: [0; 256],
        };
        for i in 0..256 {
            let val = ((i as f32 / 255.0) * 65535.0).round() as u16;
            linear_ramp.red[i] = val;
            linear_ramp.green[i] = val;
            linear_ramp.blue[i] = val;
        }
        unsafe {
            let hdc = GetDC(HWND::default());
            if !hdc.0.is_null() {
                let _ = SetDeviceGammaRamp(hdc, &linear_ramp as *const _ as *const _);
                let _ = ReleaseDC(HWND::default(), hdc);
            }
        }
    }

    // 2. Restore 5x5 identity matrix
    let mut identity = MagColorEffect {
        transform: [0.0; 25],
    };
    identity.transform[0] = 1.0;
    identity.transform[6] = 1.0;
    identity.transform[12] = 1.0;
    identity.transform[18] = 1.0;
    identity.transform[24] = 1.0;

    if let Some(lib) = get_mag_lib() {
        unsafe {
            if let Ok(set_effect) = lib.get::<MagSetEffectFn>(b"MagSetFullscreenColorEffect\0") {
                let _ = set_effect(&identity);
            }
            if MAG_INITIALIZED.swap(false, Ordering::SeqCst) {
                if let Ok(uninit) = lib.get::<MagUninitFn>(b"MagUninitialize\0") {
                    let _ = uninit();
                }
            }
        }
    }
}

/// Apply full color preset (both hardware gamma ramp and magnification matrix)
pub fn apply_color_preset(preset: &DisplayColorPreset) -> Result<(), String> {
    let _lock = ENGINE_LOCK.lock();
    apply_gamma_ramp(preset);
    apply_color_matrix(preset)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// Preset & Configuration Storage Management
// ----------------------------------------------------------------------------

pub fn get_color_profiles_dir() -> PathBuf {
    let dir = crate::system::config::get_config_dir().join("color_profiles");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_tuning_config_path() -> PathBuf {
    crate::system::config::get_config_dir().join("display_tuning.json")
}

pub fn get_builtin_presets() -> Vec<DisplayColorPreset> {
    vec![
        DisplayColorPreset {
            id: "preset_vibrant".to_string(),
            name: "多彩鲜艳 (Vibrant)".to_string(),
            author: Some("机械革命官方".to_string()),
            brightness_pct: Some(50.0),
            color_temp_k: 6500,
            gain_r: 1.0,
            gain_g: 1.0,
            gain_b: 1.0,
            saturation: 1.5,
            contrast: 1.0,
            gamma: 1.0,
            is_builtin: true,
        },
        DisplayColorPreset {
            id: "preset_internet".to_string(),
            name: "上网模式 (Internet)".to_string(),
            author: Some("机械革命官方".to_string()),
            brightness_pct: Some(55.0),
            color_temp_k: 6500,
            gain_r: 98.0 / 128.0,
            gain_g: 88.0 / 128.0,
            gain_b: 114.0 / 128.0,
            saturation: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            is_builtin: true,
        },
        DisplayColorPreset {
            id: "preset_video".to_string(),
            name: "视频模式 (Video)".to_string(),
            author: Some("机械革命官方".to_string()),
            brightness_pct: Some(55.0),
            color_temp_k: 6500,
            gain_r: 110.0 / 128.0,
            gain_g: 119.0 / 128.0,
            gain_b: 114.0 / 128.0,
            saturation: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            is_builtin: true,
        },
        DisplayColorPreset {
            id: "preset_lowblue".to_string(),
            name: "低蓝光护眼 (Low Blue)".to_string(),
            author: Some("机械革命官方".to_string()),
            brightness_pct: Some(50.0),
            color_temp_k: 5000,
            gain_r: 1.0,
            gain_g: 1.0,
            gain_b: 1.0,
            saturation: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            is_builtin: true,
        },
        DisplayColorPreset {
            id: "preset_cinema".to_string(),
            name: "影院模式 (Cinema)".to_string(),
            author: Some("机械革命官方".to_string()),
            brightness_pct: Some(55.0),
            color_temp_k: 6000,
            gain_r: 107.0 / 128.0,
            gain_g: 119.0 / 128.0,
            gain_b: 116.0 / 128.0,
            saturation: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            is_builtin: true,
        },
        DisplayColorPreset {
            id: "preset_photo".to_string(),
            name: "照片模式 (Photo)".to_string(),
            author: Some("机械革命官方".to_string()),
            brightness_pct: Some(55.0),
            color_temp_k: 6500,
            gain_r: 98.0 / 128.0,
            gain_g: 129.0 / 128.0,
            gain_b: 114.0 / 128.0,
            saturation: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            is_builtin: true,
        },
    ]
}

pub fn load_display_tuning_config() -> DisplayTuningConfig {
    let path = get_tuning_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<DisplayTuningConfig>(&content) {
                return cfg;
            }
        }
    }
    DisplayTuningConfig::default()
}

pub fn save_display_tuning_config(cfg: &DisplayTuningConfig) {
    let path = get_tuning_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, json);
    }
}

pub fn reset_preset_override(preset_id: &str) {
    let mut cfg = load_display_tuning_config();
    cfg.preset_overrides.remove(preset_id);
    if preset_id == "preset_custom" {
        cfg.custom_preset = DisplayColorPreset::default();
    }
    save_display_tuning_config(&cfg);
}

pub fn list_all_presets() -> Vec<DisplayColorPreset> {
    let cfg = load_display_tuning_config();
    let mut list = get_builtin_presets();
    for p in list.iter_mut() {
        if let Some(user_ov) = cfg.preset_overrides.get(&p.id) {
            p.color_temp_k = user_ov.color_temp_k;
            p.brightness_pct = user_ov.brightness_pct;
            p.gain_r = user_ov.gain_r;
            p.gain_g = user_ov.gain_g;
            p.gain_b = user_ov.gain_b;
            p.saturation = user_ov.saturation;
            p.contrast = user_ov.contrast;
            p.gamma = user_ov.gamma;
        }
    }
    let dir = get_color_profiles_dir();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&p) {
                    if let Ok(mut preset) = serde_json::from_str::<DisplayColorPreset>(&content) {
                        preset.is_builtin = false;
                        list.push(preset);
                    }
                }
            }
        }
    }
    list
}

pub fn get_current_tuning_state() -> DisplayTuningState {
    let cfg = load_display_tuning_config();
    let all_presets = list_all_presets();

    let active_preset = if cfg.active_preset_id == "preset_custom" {
        cfg.custom_preset.clone()
    } else {
        all_presets
            .iter()
            .find(|p| p.id == cfg.active_preset_id)
            .cloned()
            .unwrap_or_else(|| all_presets.first().cloned().unwrap_or_default())
    };

    DisplayTuningState {
        enabled: cfg.enabled,
        active_preset_id: cfg.active_preset_id,
        active_preset,
        presets: all_presets,
        factory_presets: get_builtin_presets(),
    }
}

pub fn save_custom_preset_file(preset: &DisplayColorPreset) -> Result<(), String> {
    let dir = get_color_profiles_dir();
    let safe_id = preset
        .id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>();
    let file_path = dir.join(format!("{}.json", safe_id));
    let json = serde_json::to_string_pretty(preset).map_err(|e| e.to_string())?;
    fs::write(file_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Parse and import any JSON color profile (Universal JSON parser, handles native and official DisplayTable.json)
pub fn import_color_preset_json(
    json_content: &str,
    original_filename: Option<String>,
) -> Result<DisplayColorPreset, String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // 1. Direct match with DisplayColorPreset
    if let Ok(mut p) = serde_json::from_str::<DisplayColorPreset>(json_content) {
        p.id = format!("custom_color_{}", timestamp);
        p.is_builtin = false;
        if let Some(fname) = &original_filename {
            if p.name.is_empty() || p.name == "极客自定义" {
                p.name = fname.replace(".json", "");
            }
        }
        save_custom_preset_file(&p)?;
        return Ok(p);
    }

    // 2. Official Mechrevo DisplayTable.json parsing
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_content) {
        if let Some(list) = val
            .get("DisplayTableSetting")
            .and_then(|s| s.get("ProfileNameList"))
            .and_then(|l| l.as_array())
        {
            let current_selected = val
                .get("DisplayTableSetting")
                .and_then(|s| s.get("CurrentSelectName"))
                .and_then(|n| n.as_str());

            let target_item = current_selected
                .and_then(|sel| {
                    list.iter().find(|it| {
                        it.get("colorProfileName")
                            .and_then(|n| n.as_str())
                            == Some(sel)
                    })
                })
                .or_else(|| list.first());

            if let Some(first_item) = target_item {
                let raw_name = first_item
                    .get("colorProfileName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("官方导入方案");
                let name = match raw_name {
                    "VibrantMode" => "多彩鲜艳 (Vibrant)",
                    "InternetMode" => "上网模式 (Internet)",
                    "VideoMode" => "视频模式 (Video)",
                    "LowBlueMode" => "低蓝光护眼 (Low Blue)",
                    "CinemaMode" => "影院模式 (Cinema)",
                    "PhotoMode" => "照片模式 (Photo)",
                    other => other,
                }.to_string();
                let color_temp = first_item
                    .get("ColorTemp")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(6500) as u32;
                let color_r = first_item
                    .get("ColorR")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(128.0) as f32;
                let color_g = first_item
                    .get("ColorG")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(128.0) as f32;
                let color_b = first_item
                    .get("ColorB")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(128.0) as f32;
                let vibrant = first_item
                    .get("VibrantValue")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;
                let contrast = first_item
                    .get("Constrast")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;
                let gamma = first_item
                    .get("Gamma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;
                let brightness = first_item
                    .get("brightness")
                    .and_then(|v| v.as_f64())
                    .map(|b| b as f32);

                let preset = DisplayColorPreset {
                    id: format!("custom_color_{}", timestamp),
                    name,
                    author: Some("官方预设".to_string()),
                    brightness_pct: brightness,
                    color_temp_k: color_temp.clamp(2000, 12000),
                    gain_r: (color_r / 128.0).clamp(0.0, 2.0),
                    gain_g: (color_g / 128.0).clamp(0.0, 2.0),
                    gain_b: (color_b / 128.0).clamp(0.0, 2.0),
                    saturation: vibrant.clamp(0.0, 2.5),
                    contrast: contrast.clamp(0.5, 2.0),
                    gamma: gamma.clamp(0.5, 2.0),
                    is_builtin: false,
                };
                save_custom_preset_file(&preset)?;
                return Ok(preset);
            }
        }

        // 3. Loose key-value mapping from generic JSON
        let name = original_filename
            .unwrap_or_else(|| "导入方案".to_string())
            .replace(".json", "");
        let temp = val
            .get("color_temp_k")
            .or_else(|| val.get("color_temp"))
            .or_else(|| val.get("ColorTemp"))
            .and_then(|v| v.as_u64())
            .unwrap_or(6500) as u32;
        let sat = val
            .get("saturation")
            .or_else(|| val.get("vibrance"))
            .or_else(|| val.get("VibrantValue"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let con = val
            .get("contrast")
            .or_else(|| val.get("Constrast"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let gam = val
            .get("gamma")
            .or_else(|| val.get("Gamma"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let preset = DisplayColorPreset {
            id: format!("custom_color_{}", timestamp),
            name,
            author: Some("User".to_string()),
            brightness_pct: None,
            color_temp_k: temp.clamp(2000, 12000),
            gain_r: 1.0,
            gain_g: 1.0,
            gain_b: 1.0,
            saturation: sat.clamp(0.0, 2.5),
            contrast: con.clamp(0.5, 2.0),
            gamma: gam.clamp(0.5, 2.0),
            is_builtin: false,
        };
        save_custom_preset_file(&preset)?;
        return Ok(preset);
    }

    Err("无法识别的 JSON 方案文件格式".to_string())
}

/// Initialize display tuning on application startup: restore settings if enabled
pub fn init_display_tuning() {
    let cfg = load_display_tuning_config();
    if cfg.enabled {
        let state = get_current_tuning_state();
        if let Err(e) = apply_color_preset(&state.active_preset) {
            crate::log_warn!("DISPLAY_TUNING", "Failed to apply color preset on startup: {}", e);
        } else {
            crate::log_info!(
                "DISPLAY_TUNING",
                "Successfully restored active color preset: {} ({})",
                state.active_preset.name,
                state.active_preset.id
            );
        }
    }
}
