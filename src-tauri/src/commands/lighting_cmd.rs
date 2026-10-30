// OpenRevo - Keyboard, Lightbar, Logo, Hinge & Custom BRFX Lighting Commands
// Handles hardware lighting presets, 4-zone colors, BetterRGB streamer modes, and custom BRFX scripts.

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

#[tauri::command]
pub fn get_lighting_status(
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> crate::driver::ite_lighting::LightingPresetState {
    lighting_state.state.lock().clone()
}

#[tauri::command]
pub fn get_lighting_state(
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> crate::driver::ite_lighting::LightingPresetState {
    lighting_state.state.lock().clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardHardwareInfo {
    pub hardware_type: String, // "PerKeyRgb", "FourZoneRgb", "SingleZoneRgb", "SingleColor", "None"
    pub has_ite8291: bool,
    pub has_ite7001: bool,
    #[serde(default)]
    pub has_ite7000: bool,
    #[serde(default)]
    pub has_front_lightbar: bool,
    #[serde(default)]
    pub is_ec_lightbar: bool,
    #[serde(default)]
    pub has_logo: bool,
    #[serde(default)]
    pub has_usb_logo: bool,
    #[serde(default)]
    pub is_ec_logo: bool,
    #[serde(default)]
    pub has_hinge: bool,
    #[serde(default)]
    pub is_ec_hinge: bool,
    #[serde(default)]
    pub is_four_zone: bool,
    pub description: String,
    pub brightness_level: u8,
    pub is_on: bool,
}


#[tauri::command]
pub fn get_lighting_runtime_status(
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> crate::driver::ite_lighting::LightingRuntimeStatus {
    lighting_state.get_runtime_status()
}

#[tauri::command]
pub fn apply_lighting_hardware_override(
    override_cfg: crate::driver::ite_lighting::LightingHardwareOverride,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<crate::driver::ite_lighting::LightingRuntimeStatus, String> {
    lighting_state.apply_hardware_override_and_reinit(override_cfg)?;
    let status = lighting_state.get_runtime_status();
    let _ = app_handle.emit("lighting://runtime_status_changed", &status);
    let _ = app_handle.emit("lighting://hardware_info_changed", ());
    Ok(status)
}

#[tauri::command]
pub fn get_lighting_hardware_override(
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> crate::driver::ite_lighting::LightingHardwareOverride {
    lighting_state.get_runtime_status().hardware_override
}

#[tauri::command]
pub fn get_keyboard_hardware_info(
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> KeyboardHardwareInfo {
    let contract = crate::system::model_engine::get_active_lighting_contract();
    let is_sleep = lighting_state.is_sleeping.load(std::sync::atomic::Ordering::Relaxed);

    // 1. Coastline / Front Lightbar: Strictly guarded by Constitution
    let (has_ite7001, has_ite7000, is_ec_lightbar, has_front_lightbar) = if !contract.has_coastline {
        (false, false, false, false)
    } else {
        let ite7001 = lighting_state.has_usb_base_lightbar();
        let ite7000 = if !ite7001 {
            crate::driver::ite_lighting::IteLightingManager::open_hid_device(
                crate::hal::vault::ite::ITE_VID,
                crate::hal::vault::ite::ITE_PID_LIGHTBAR_HINGE,
                0,
            ).is_some()
        } else {
            false
        };
        let ec_lb = !ite7001 && !ite7000 && crate::driver::lighting::IteLightingManager::is_ec_lightbar_supported();
        let front = ite7001 || ec_lb;
        (ite7001, ite7000, ec_lb, front)
    };

    // 2. Air Vent / Hinge Lightbar: Strictly guarded by Constitution
    let has_hinge = if !contract.has_air_vent {
        false
    } else {
        crate::driver::lighting::IteLightingManager::probe_hinge_device_exists()
    };
    let is_ec_hinge = false;

    // 3. Logo Matrix: Strictly guarded by Constitution
    let (has_logo, has_usb_logo, is_ec_logo) = if !contract.has_logo {
        (false, false, false)
    } else {
        let usb_logo = crate::driver::lighting::IteLightingManager::probe_usb_logo_supported();
        let ec_logo = !usb_logo && crate::driver::lighting::IteLightingManager::is_ec_logo_supported();
        (usb_logo || ec_logo, usb_logo, ec_logo)
    };

    // 4. Keyboard Backlight Matrix: Routed by Constitution
    let st = lighting_state.state.lock();
    let cfg_brightness = st.kb_brightness;
    let is_on_cfg = st.kb_enabled && cfg_brightness > 0;

    match contract.keyboard_type.as_str() {
        "PerKey" => {
            let has_ite8291 = lighting_state.has_perkey_keyboard_device();
            let is_streamer = st.kb_engine == "streamer" || st.kb_engine == "custom";
            let streamer = crate::driver::ite_streamer::get_global_streamer();
            let actually_active = if is_streamer {
                !is_sleep && streamer.is_running() && streamer.get_params().brightness_pct > 0
            } else {
                !is_sleep && is_on_cfg
            };
            KeyboardHardwareInfo {
                hardware_type: "PerKeyRgb".to_string(),
                has_ite8291,
                has_ite7001,
                has_ite7000,
                has_front_lightbar,
                is_ec_lightbar,
                has_logo,
                has_usb_logo,
                is_ec_logo,
                has_hinge,
                is_ec_hinge,
                is_four_zone: false,
                description: "独立 ITE 8291 USB 全彩逐键 RGB 键盘".to_string(),
                brightness_level: cfg_brightness,
                is_on: actually_active,
            }
        }
        "4Zone" => {
            let is_four_zone = lighting_state.has_four_zone_keyboard_device();
            let actually_active = !is_sleep && is_on_cfg;
            KeyboardHardwareInfo {
                hardware_type: "FourZoneRgb".to_string(),
                has_ite8291: false,
                has_ite7001,
                has_ite7000,
                has_front_lightbar,
                is_ec_lightbar,
                has_logo,
                has_usb_logo,
                is_ec_logo,
                has_hinge,
                is_ec_hinge,
                is_four_zone,
                description: "ITE 4区 RGB 幻彩背光键盘 (048D:CE00)".to_string(),
                brightness_level: cfg_brightness,
                is_on: actually_active,
            }
        }
        "SingleZone" | "SingleZone_RGB" => {
            let hal = crate::hal::get_hardware_driver();
            let (is_hw_on, effective_br) = if hal.kind() != crate::hal::DriverKind::TongfangAcpi {
                let lvl = hal.get_monochrome_backlight().unwrap_or(cfg_brightness);
                (is_on_cfg && lvl > 0, lvl)
            } else {
                let raw_backlight_status = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BACKLIGHT_WHITE).unwrap_or(0);
                let on = !is_sleep && (raw_backlight_status & 0x02) == 0;
                let br = if on && cfg_brightness > 0 { cfg_brightness } else if on { 4 } else { 0 };
                (on, br)
            };
            KeyboardHardwareInfo {
                hardware_type: "SingleZoneRgb".to_string(),
                has_ite8291: false,
                has_ite7001,
                has_ite7000,
                has_front_lightbar,
                is_ec_lightbar,
                has_logo,
                has_usb_logo,
                is_ec_logo,
                has_hinge,
                is_ec_hinge,
                is_four_zone: false,
                description: "主板 EC 单区 RGB 键盘 (全键同色调光)".to_string(),
                brightness_level: effective_br,
                is_on: is_hw_on,
            }
        }
        "MonochromeWhite" => {
            let hal = crate::hal::get_hardware_driver();
            let (is_hw_on, effective_br) = if let Some(lvl) = hal.get_monochrome_backlight() {
                let on = !is_sleep && lvl > 0;
                (on, if on { lvl } else { 0 })
            } else {
                let raw_backlight_status = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BACKLIGHT_WHITE).unwrap_or(0);
                let on = !is_sleep && (raw_backlight_status & 0x02) == 0;
                let br = if on && cfg_brightness > 0 { cfg_brightness } else if on { 3 } else { 0 };
                (on, br)
            };
            KeyboardHardwareInfo {
                hardware_type: "SingleColor".to_string(),
                has_ite8291: false,
                has_ite7001,
                has_ite7000,
                has_front_lightbar,
                is_ec_lightbar,
                has_logo,
                has_usb_logo,
                is_ec_logo,
                has_hinge,
                is_ec_hinge,
                is_four_zone: false,
                description: "单色白光背光键盘 (支持档位调节)".to_string(),
                brightness_level: effective_br,
                is_on: is_hw_on,
            }
        }
        _ => {
            // "None" or fallback
            KeyboardHardwareInfo {
                hardware_type: "None".to_string(),
                has_ite8291: false,
                has_ite7001,
                has_ite7000,
                has_front_lightbar,
                is_ec_lightbar,
                has_logo,
                has_usb_logo,
                is_ec_logo,
                has_hinge,
                is_ec_hinge,
                is_four_zone: false,
                description: "未配置或不支持背光调节".to_string(),
                brightness_level: 0,
                is_on: false,
            }
        }
    }
}

#[tauri::command]
pub fn apply_keyboard_lighting(
    app: tauri::AppHandle,
    enabled: bool,
    effect: u8,
    brightness: u8,
    speed: u8,
    direction: u8,
    color: String,
    engine: Option<String>,
    fps: Option<u32>,
    streamer_effect: Option<u8>,
    #[allow(non_snake_case)]
    streamerEffect: Option<u8>,
    #[allow(non_snake_case)]
    customScriptId: Option<String>,
    custom_script_id: Option<String>,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let se_opt = streamerEffect.or(streamer_effect);
    let csi_opt = customScriptId.or(custom_script_id);
    let (st_snapshot, brightness_changed) = {
        let mut st = lighting_state.state.lock();
        let prev_lvl = if st.kb_enabled { st.kb_brightness } else { 0 };
        if brightness > 0 {
            st.kb_brightness = brightness;
        }
        st.kb_enabled = enabled && brightness > 0;
        let new_lvl = if st.kb_enabled { st.kb_brightness } else { 0 };
        let brightness_changed = prev_lvl != new_lvl;
        if let Some(ref eng) = engine {
            st.kb_engine = eng.clone();
        }
        if let Some(csi) = csi_opt {
            st.custom_script_id = csi;
        }
        if st.kb_engine == "streamer" {
            // BetterRGB Streamer Engine (100..199)
            if let Some(se) = se_opt {
                st.streamer_effect = if se >= 100 && se < 200 { se } else if se < 100 { se + 100 } else { 101 };
            } else if effect >= 100 && effect < 200 {
                st.streamer_effect = effect;
            } else if st.streamer_effect >= 200 || st.streamer_effect < 100 {
                st.streamer_effect = 101;
            }
        } else if st.kb_engine == "custom" {
            // Custom BRFX Engine (200..255)
            if let Some(se) = se_opt {
                st.streamer_effect = if se >= 200 { se } else { 200 };
            } else if effect >= 200 {
                st.streamer_effect = effect;
            } else if st.streamer_effect < 200 {
                st.streamer_effect = 200;
            }
        } else {
            // Hardware MCU Engine: update kb_effect ONLY (< 100)
            if effect > 0 && effect < 100 {
                st.kb_effect = effect;
            }
        }
        st.kb_speed = speed;
        st.kb_direction = direction;
        st.kb_color = color.clone();
        if let Some(f) = fps {
            st.kb_fps = f;
        }
        (st.clone(), brightness_changed)
    };
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);

    let is_on = st_snapshot.kb_enabled;
    let target_brightness = if is_on { st_snapshot.kb_brightness } else { 0 };

    let contract = crate::system::model_engine::get_active_lighting_contract();
    if contract.keyboard_type == "PerKey" && (st_snapshot.kb_engine == "streamer" || st_snapshot.kb_engine == "custom") {
        let streamer = crate::driver::ite_streamer::get_global_streamer();
        let fps_to_use = fps.unwrap_or(st_snapshot.kb_fps);
        let brightness_pct = if is_on {
            match target_brightness {
                1 => 20,
                2 => 45,
                3 => 75,
                4 => 100,
                _ => 75,
            }
        } else {
            0
        };
        let sp = crate::driver::ite_streamer::StreamerParams {
            enabled: is_on,
            effect: st_snapshot.streamer_effect,
            fps: fps_to_use,
            brightness_pct,
            speed,
            direction,
            color_hex: color.clone(),
        };
        streamer.update_params(sp.clone());
        crate::log_info!("STREAMER", "BetterRGB streamer params applied: effect={}, fps={}, br={}, enabled={}", sp.effect, sp.fps, sp.brightness_pct, is_on);
    } else {
        let hw_effect = if st_snapshot.kb_effect > 0 && st_snapshot.kb_effect < 100 {
            st_snapshot.kb_effect
        } else {
            3
        };
        // Hand off cleanly to Lighting Arbiter (handles PerKey MCU, 4Zone MCU, SingleZone EC, White EC)
        lighting_state.apply_keyboard(
            is_on,
            hw_effect,
            target_brightness,
            speed,
            direction,
            &color,
        );
    }

    if brightness_changed {
        let emit_lvl = if is_on { st_snapshot.kb_brightness } else { 0 };
        crate::system::osd_window::wake_osd_if_enabled(&app);
        let _ = app.emit("keyboard-brightness-changed", emit_lvl);
        let _ = app.emit_to("main", "keyboard-brightness-changed", emit_lvl);
        let _ = app.emit_to("osd", "keyboard-brightness-changed", emit_lvl);
    }
    Ok(())
}

#[tauri::command]
pub fn apply_lightbar_lighting(
    enabled: bool,
    effect: u8,
    brightness: u8,
    speed: u8,
    color: String,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let contract = crate::system::model_engine::get_active_lighting_contract();
    if !contract.has_coastline {
        crate::log_warn!("LIGHTING_CMD", "apply_lightbar_lighting blocked: Contract declares has_coastline = false");
        return Err("当前模具宪法契约声明无海岸灯硬件，指令已安全拦截".to_string());
    }

    let st_snapshot = {
        let mut st = lighting_state.state.lock();
        st.lb_enabled = enabled;
        st.lb_effect = effect;
        st.lb_brightness = brightness;
        st.lb_speed = speed;
        st.lb_color = color.clone();
        st.clone()
    };
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
    lighting_state.apply_lightbar(enabled, effect, brightness, speed, &color);
    Ok(())
}

#[tauri::command]
pub fn apply_hinge_lighting(
    enabled: bool,
    effect: u8,
    brightness: u8,
    speed: u8,
    color: String,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let contract = crate::system::model_engine::get_active_lighting_contract();
    if !contract.has_air_vent {
        crate::log_warn!("LIGHTING_CMD", "apply_hinge_lighting blocked: Contract declares has_air_vent = false");
        return Err("当前模具宪法契约声明无出风口/转轴灯硬件，指令已安全拦截".to_string());
    }

    let st_snapshot = {
        let mut st = lighting_state.state.lock();
        st.hinge_enabled = enabled;
        st.hinge_effect = effect;
        st.hinge_brightness = brightness;
        st.hinge_speed = speed;
        st.hinge_color = color.clone();
        st.clone()
    };
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
    lighting_state.apply_hinge(enabled, effect, brightness, speed, &color);
    Ok(())
}

#[tauri::command]
pub fn apply_four_zone_colors(
    zone1: String,
    zone2: String,
    zone3: String,
    zone4: String,
    brightness: u8,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let contract = crate::system::model_engine::get_active_lighting_contract();
    if contract.keyboard_type != "4Zone" {
        crate::log_warn!("LIGHTING_CMD", "apply_four_zone_colors blocked: Contract declares keyboard_type != 4Zone");
        return Err("当前模具宪法契约声明非4区RGB键盘，指令已安全拦截".to_string());
    }

    let st_snapshot = {
        let mut st = lighting_state.state.lock();
        st.kb_zones = [zone1, zone2, zone3, zone4];
        if brightness > 0 {
            st.kb_brightness = brightness;
            st.kb_enabled = true;
        }
        st.clone()
    };
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
    lighting_state.apply_four_zone_keyboard(
        st_snapshot.kb_enabled,
        1, // Static with 4 individual zone colors
        st_snapshot.kb_brightness,
        st_snapshot.kb_speed,
        st_snapshot.kb_direction,
        &st_snapshot.kb_color,
        Some(&st_snapshot.kb_zones),
    );
    Ok(())
}

#[tauri::command]
pub fn apply_logo_lighting(
    enabled: bool,
    effect: u8,
    brightness: u8,
    color: String,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let contract = crate::system::model_engine::get_active_lighting_contract();
    if !contract.has_logo {
        crate::log_warn!("LIGHTING_CMD", "apply_logo_lighting blocked: Contract declares has_logo = false. Ghost Logo eliminated!");
        return Err("当前模具宪法契约声明无 A 面 LOGO 硬件，指令已安全拦截".to_string());
    }

    let st_snapshot = {
        let mut st = lighting_state.state.lock();
        st.logo_enabled = enabled;
        st.logo_effect = effect;
        st.logo_brightness = brightness;
        st.logo_color = color.clone();
        st.clone()
    };
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
    let accepted = lighting_state.apply_logo(enabled, effect, brightness, &color);
    if !accepted {
        crate::log_warn!(
            "LIGHTING_CMD",
            "apply_logo_lighting: no lighting channel accepted the command (USB 7001 handle missing?)"
        );
        return Err("未找到可用的 A 面徽标控制通道，指令未下发到硬件".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn set_lighting_sleep_timer(
    minutes: u8,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let st_snapshot = {
        let mut st = lighting_state.state.lock();
        st.sleep_minutes = minutes;
        st.clone()
    };
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
    lighting_state.set_timeout(minutes);
    Ok(())
}

#[tauri::command]
pub fn trigger_lighting_welcome(
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    lighting_state.trigger_welcome();
    Ok(())
}

#[tauri::command]
pub fn set_ceremony_enabled(
    enabled: bool,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let st_snapshot = {
        let mut st = lighting_state.state.lock();
        st.ceremony_enabled = enabled;
        st.clone()
    };
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
    Ok(())
}

#[tauri::command]
pub fn save_current_lighting_as_startup(
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let st_snapshot = lighting_state.state.lock().clone();
    crate::driver::ite_lighting::save_lighting_state(&st_snapshot);
    Ok(())
}

#[tauri::command]
pub fn set_keyboard_engine(
    app: tauri::AppHandle,
    engine: String,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let (enabled, effect, brightness, speed, direction, color, fps, streamer_effect) = {
        let mut st = lighting_state.state.lock();
        st.kb_engine = engine.clone();
        (
            st.kb_enabled,
            st.kb_effect,
            st.kb_brightness,
            st.kb_speed,
            st.kb_direction,
            st.kb_color.clone(),
            st.kb_fps,
            st.streamer_effect,
        )
    };
    apply_keyboard_lighting(
        app,
        enabled,
        effect,
        brightness,
        speed,
        direction,
        color,
        Some(engine),
        Some(fps),
        Some(streamer_effect),
        Some(streamer_effect),
        None,
        None,
        lighting_state,
    )
}

#[tauri::command]
pub fn set_streamer_fps(
    fps: u32,
    lighting_state: State<'_, crate::driver::ite_lighting::SharedIteLightingManager>,
) -> Result<(), String> {
    let streamer = crate::driver::ite_streamer::get_global_streamer();
    streamer.set_fps(fps);
    let mut st = lighting_state.state.lock();
    st.kb_fps = fps;
    crate::driver::ite_lighting::save_lighting_state(&st);
    Ok(())
}

#[tauri::command]
pub fn set_auto_fallback_kb_on_battery(enable: bool) -> Result<bool, String> {
    let mut cfg = crate::system::config::load_config();
    cfg.auto_fallback_kb_on_battery = enable;
    crate::system::config::save_config(&cfg);
    Ok(enable)
}

#[tauri::command]
pub fn get_auto_fallback_kb_on_battery() -> Result<bool, String> {
    let cfg = crate::system::config::load_config();
    Ok(cfg.auto_fallback_kb_on_battery)
}

// --- BetterRGB Custom Lighting (.brfx) Studio Commands ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomBrfxFileInfo {
    pub file_name: String,
    pub content: String,
    pub full_path: String,
}

#[tauri::command]
pub fn get_custom_brfx_list() -> Result<Vec<CustomBrfxFileInfo>, String> {
    let custom_dir = crate::system::config::get_config_dir().join("custom_effects");
    // Ensure all built-in presets exist on disk (seed any missing presets and self-heal corrupted legacy files)
    let presets = super::brfx_presets::get_default_presets();

    let _ = std::fs::create_dir_all(&custom_dir);
    for (fname, content) in &presets {
        let path = custom_dir.join(fname);
        let need_write = if !path.exists() {
            true
        } else if let Ok(existing) = std::fs::read_to_string(&path) {
            // Self-healing: if file contains legacy garbled characters or missing correct marker, repair it!
            existing.contains("榫熸淳") 
                || existing.contains("璧涘崥") 
                || existing.contains("寮曞姏") 
                || existing.contains("鏂囧瓧")
                || existing.contains('\u{FFFD}')
                || (!existing.contains("@name: 龟派气功") 
                    && !existing.contains("@name: 赛博青莲") 
                    && !existing.contains("@name: 引力黑洞") 
                    && !existing.contains("@name: 文字弹幕"))
        } else {
            true
        };

        if need_write {
            let _ = std::fs::write(&path, content);
        }
    }

    let mut entries = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&custom_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ext_lower == "brfx" || ext_lower == "revo-fx" || ext_lower == "rhai" {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            entries.push(CustomBrfxFileInfo {
                                file_name,
                                content,
                                full_path: path.to_string_lossy().to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(entries)
}

#[tauri::command]
pub fn save_custom_brfx(filename: String, content: String) -> Result<String, String> {
    let custom_dir = crate::system::config::get_config_dir().join("custom_effects");
    let _ = std::fs::create_dir_all(&custom_dir);

    let mut safe_name = filename.trim().to_string();
    if !safe_name.ends_with(".brfx") {
        safe_name.push_str(".brfx");
    }
    let clean_name = std::path::Path::new(&safe_name)
        .file_name()
        .ok_or_else(|| "无效的文件名".to_string())?;

    let file_path = custom_dir.join(clean_name);
    std::fs::write(&file_path, &content).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_brfx_in_notepad(filename: String) -> Result<(), String> {
    let custom_dir = crate::system::config::get_config_dir().join("custom_effects");
    let clean_name = std::path::Path::new(&filename)
        .file_name()
        .ok_or_else(|| "无效的文件名".to_string())?;
    let file_path = custom_dir.join(clean_name);
    if !file_path.exists() {
        return Err(format!("目标动效文件不存在: {:?}", file_path));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("notepad.exe")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("启动记事本失败: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = file_path;
        Err("非 Windows 系统不支持记事本启动".to_string())
    }
}

#[tauri::command]
pub fn apply_custom_brfx_script(name: String, content: String) -> Result<(), String> {
    crate::driver::custom_fx_engine::get_custom_fx_engine().load_script(&name, &content)
}

#[tauri::command]
pub fn clear_custom_brfx_script() -> Result<(), String> {
    crate::driver::custom_fx_engine::get_custom_fx_engine().clear_script();
    Ok(())
}

#[tauri::command]
pub fn set_custom_fx_text(text: String) -> Result<(), String> {
    crate::driver::custom_fx_engine::get_custom_fx_engine().set_active_text(&text);
    Ok(())
}

#[tauri::command]
pub fn get_custom_fx_text() -> Result<String, String> {
    Ok(crate::driver::custom_fx_engine::get_custom_fx_engine().get_active_text())
}

#[tauri::command]
pub fn get_keyboard_sleep_timeout() -> u32 {
    let cfg = crate::system::config::load_config();
    cfg.keyboard_sleep_timeout
}

#[tauri::command]
pub fn set_keyboard_sleep_timeout(seconds: u32, app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    let mut cfg = crate::system::config::load_config();
    cfg.keyboard_sleep_timeout = seconds;
    crate::system::config::save_config(&cfg);

    // If active model is Declarative (e.g. Xingyao 14), invoke WMI Mailbox
    let hal = crate::hal::get_hardware_driver();
    if hal.kind() != crate::hal::DriverKind::TongfangAcpi {
        let cmd = if seconds == 0 {
            // Always-on (Disable sleep timeout)
            vec![0x03, 0x0C, 0x01, 0x00]
        } else {
            // Enable hardware power saving auto-timeout
            vec![0x03, 0x0C, 0x00, 0x00]
        };
        crate::log_info!("LIGHTING", "Applying keyboard backlight sleep timeout ({}s) via WMI: {:?}", seconds, cmd);
        let _ = crate::hal::declarative::invoke_raw_mailbox(&cmd);
    }

    crate::system::osd_window::show_keyboard_sleep_timeout(seconds);
    let _ = app.emit("keyboard-sleep-timeout-changed", seconds);
    let _ = app.emit_to("osd", "keyboard-sleep-timeout-changed", seconds);
    Ok(())
}

