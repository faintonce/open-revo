use tauri::AppHandle;
use tauri::Manager;
use super::super::sync::sync_slint_lighting_state;

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    match action {
        "apply_keyboard_lighting_patch" => {
            if let Some(app) = app {
                let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(101) as u8;
                let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
                let speed = val.get("speed").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
                let direction = val.get("direction").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
                let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#6366F1").to_string();
                let engine = val.get("engine").and_then(|v| v.as_str()).map(|s| s.to_string());
                let fps = val.get("fps").and_then(|v| v.as_i64()).map(|f| f as u32);
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();

                let _ = crate::commands::lighting_cmd::apply_keyboard_lighting(
                    app.clone(),
                    enabled,
                    effect,
                    brightness,
                    speed,
                    direction,
                    color,
                    engine,
                    fps,
                    Some(effect),
                    Some(effect),
                    None,
                    None,
                    lm,
                );
                sync_slint_lighting_state();
            }
            true
        }
        "apply_lightbar_lighting" => {
            if let Some(app) = app {
                let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
                let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
                let speed = val.get("speed").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
                let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#00FFFF").to_string();
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::apply_lightbar_lighting(
                    enabled,
                    effect,
                    brightness,
                    speed,
                    color,
                    lm,
                );
                sync_slint_lighting_state();
            }
            true
        }
        "apply_hinge_lighting" => {
            if let Some(app) = app {
                let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(1) as u8;
                let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
                let speed = val.get("speed").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
                let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#00FFFF").to_string();
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::apply_hinge_lighting(
                    enabled,
                    effect,
                    brightness,
                    speed,
                    color,
                    lm,
                );
                sync_slint_lighting_state();
            }
            true
        }
        "apply_logo_lighting" => {
            if let Some(app) = app {
                let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(1) as u8;
                let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
                let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#00FFFF").to_string();
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::apply_logo_lighting(
                    enabled,
                    effect,
                    brightness,
                    color,
                    lm,
                );
                sync_slint_lighting_state();
            }
            true
        }
        "set_lighting_sleep_timer" => {
            if let (Some(app), Some(minutes)) = (app, val.get("minutes").and_then(|v| v.as_i64())) {
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::set_lighting_sleep_timer(minutes as u8, lm);
                sync_slint_lighting_state();
            }
            true
        }
        "set_ceremony_enabled" => {
            if let (Some(app), Some(enabled)) = (app, val.get("enabled").and_then(|v| v.as_bool())) {
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::set_ceremony_enabled(enabled, lm);
                sync_slint_lighting_state();
            }
            true
        }
        "trigger_lighting_welcome" => {
            if let Some(app) = app {
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::trigger_lighting_welcome(lm);
            }
            true
        }
        "save_current_lighting_as_startup" => {
            if let Some(app) = app {
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::save_current_lighting_as_startup(lm);
                sync_slint_lighting_state();
            }
            true
        }
        "reset_lighting_defaults" => {
            if let Some(app) = app {
                let tab = val.get("sub_tab").and_then(|v| v.as_i64()).unwrap_or(0);
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                match tab {
                    0 => {
                        let _ = crate::commands::lighting_cmd::apply_keyboard_lighting(
                            app.clone(),
                            true,
                            3,
                            3,
                            2,
                            0,
                            "#6366F1".to_string(),
                            Some("hardware".to_string()),
                            Some(30),
                            Some(101),
                            Some(101),
                            None,
                            None,
                            lm,
                        );
                    }
                    1 => {
                        let _ = crate::commands::lighting_cmd::apply_lightbar_lighting(
                            true,
                            3,
                            3,
                            2,
                            "#00FFFF".to_string(),
                            lm,
                        );
                    }
                    2 => {
                        let _ = crate::commands::lighting_cmd::apply_hinge_lighting(
                            true,
                            1,
                            3,
                            2,
                            "#00FFFF".to_string(),
                            lm,
                        );
                    }
                    _ => {
                        let _ = crate::commands::lighting_cmd::apply_logo_lighting(
                            true,
                            1,
                            3,
                            "#00FFFF".to_string(),
                            lm,
                        );
                    }
                }
                sync_slint_lighting_state();
            }
            true
        }
        "set_streamer_fps" => {
            if let (Some(app), Some(fps)) = (app, val.get("fps").and_then(|v| v.as_i64())) {
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::set_streamer_fps(fps as u32, lm);
                sync_slint_lighting_state();
            }
            true
        }
        "apply_lighting_hardware_override" => {
            if let Some(app) = app {
                if let Some(ov_json) = val.get("override_cfg") {
                    if let Ok(cfg) = serde_json::from_value::<crate::driver::ite_lighting::LightingHardwareOverride>(ov_json.clone()) {
                        let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                        let _ = crate::commands::lighting_cmd::apply_lighting_hardware_override(cfg, lm, app.clone());
                        sync_slint_lighting_state();
                    }
                }
            }
            true
        }
        "reset_lighting_hardware_override" => {
            if let Some(app) = app {
                let default_ov = crate::driver::ite_lighting::LightingHardwareOverride {
                    enabled: false,
                    keyboard: "auto".into(),
                    front_bar: "auto".into(),
                    ambient: "auto".into(),
                    logo: "auto".into(),
                };
                let lm: tauri::State<crate::driver::ite_lighting::SharedIteLightingManager> = app.state();
                let _ = crate::commands::lighting_cmd::apply_lighting_hardware_override(default_ov, lm, app.clone());
                sync_slint_lighting_state();
            }
            true
        }
        _ => false,
    }
}

