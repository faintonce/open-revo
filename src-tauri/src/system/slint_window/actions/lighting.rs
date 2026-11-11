use tauri::AppHandle;
use super::super::sync::sync_slint_lighting_state;

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    let global_lm = crate::driver::ite_lighting::get_global_lighting();
    match action {
        "apply_keyboard_lighting_patch" => {
            let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(101) as u8;
            let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
            let speed = val.get("speed").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
            let direction = val.get("direction").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
            let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#6366F1").to_string();
            let engine = val.get("engine").and_then(|v| v.as_str()).map(|s| s.to_string());
            let fps = val.get("fps").and_then(|v| v.as_i64()).map(|f| f as u32);

            let streamer_eff = if effect >= 100 { Some(effect) } else { None };
            let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                app,
                enabled,
                effect,
                brightness,
                speed,
                direction,
                color,
                engine,
                fps,
                streamer_eff,
                streamer_eff,
                None,
                None,
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
        "apply_lightbar_lighting" => {
            let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
            let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
            let speed = val.get("speed").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
            let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#00FFFF").to_string();
            let _ = crate::commands::lighting_cmd::apply_lightbar_lighting_internal(
                enabled,
                effect,
                brightness,
                speed,
                color,
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
        "apply_hinge_lighting" => {
            let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(1) as u8;
            let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
            let speed = val.get("speed").and_then(|v| v.as_i64()).unwrap_or(2) as u8;
            let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#00FFFF").to_string();
            let _ = crate::commands::lighting_cmd::apply_hinge_lighting_internal(
                enabled,
                effect,
                brightness,
                speed,
                color,
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
        "apply_logo_lighting" => {
            let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let effect = val.get("effect").and_then(|v| v.as_i64()).unwrap_or(1) as u8;
            let brightness = val.get("brightness").and_then(|v| v.as_i64()).unwrap_or(3) as u8;
            let color = val.get("color").and_then(|v| v.as_str()).unwrap_or("#00FFFF").to_string();
            let _ = crate::commands::lighting_cmd::apply_logo_lighting_internal(
                enabled,
                effect,
                brightness,
                color,
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
        "set_lighting_sleep_timer" => {
            if let Some(minutes) = val.get("minutes").and_then(|v| v.as_i64()) {
                let _ = crate::commands::lighting_cmd::set_lighting_sleep_timer_internal(minutes as u8, &global_lm);
                sync_slint_lighting_state();
            }
            true
        }
        "set_ceremony_enabled" => {
            if let Some(enabled) = val.get("enabled").and_then(|v| v.as_bool()) {
                let _ = crate::commands::lighting_cmd::set_ceremony_enabled_internal(enabled, &global_lm);
                sync_slint_lighting_state();
            }
            true
        }
        "trigger_lighting_welcome" => {
            let _ = crate::commands::lighting_cmd::trigger_lighting_welcome_internal(&global_lm);
            true
        }
        "save_current_lighting_as_startup" => {
            let _ = crate::commands::lighting_cmd::save_current_lighting_as_startup_internal(&global_lm);
            sync_slint_lighting_state();
            true
        }
        "reset_lighting_defaults" => {
            let tab = val.get("sub_tab").and_then(|v| v.as_i64()).unwrap_or(0);
            match tab {
                0 => {
                    let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                        app,
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
                        &global_lm,
                    );
                }
                1 => {
                    let _ = crate::commands::lighting_cmd::apply_lightbar_lighting_internal(
                        true,
                        3,
                        3,
                        2,
                        "#00FFFF".to_string(),
                        &global_lm,
                    );
                }
                2 => {
                    let _ = crate::commands::lighting_cmd::apply_hinge_lighting_internal(
                        true,
                        1,
                        3,
                        2,
                        "#00FFFF".to_string(),
                        &global_lm,
                    );
                }
                _ => {
                    let _ = crate::commands::lighting_cmd::apply_logo_lighting_internal(
                        true,
                        1,
                        3,
                        "#00FFFF".to_string(),
                        &global_lm,
                    );
                }
            }
            sync_slint_lighting_state();
            true
        }
        "set_streamer_fps" => {
            if let Some(fps) = val.get("fps").and_then(|v| v.as_i64()) {
                let _ = crate::commands::lighting_cmd::set_streamer_fps_internal(fps as u32, &global_lm);
                sync_slint_lighting_state();
            }
            true
        }
        "apply_lighting_hardware_override" => {
            if let Some(ov_json) = val.get("override_cfg") {
                if let Ok(cfg) = serde_json::from_value::<crate::driver::ite_lighting::LightingHardwareOverride>(ov_json.clone()) {
                    let _ = crate::commands::lighting_cmd::apply_lighting_hardware_override_internal(cfg, &global_lm, app);
                    sync_slint_lighting_state();
                }
            }
            true
        }
        "reset_lighting_hardware_override" => {
            let default_ov = crate::driver::ite_lighting::LightingHardwareOverride {
                enabled: false,
                keyboard: "auto".into(),
                front_bar: "auto".into(),
                ambient: "auto".into(),
                logo: "auto".into(),
            };
            let _ = crate::commands::lighting_cmd::apply_lighting_hardware_override_internal(default_ov, &global_lm, app);
            sync_slint_lighting_state();
            true
        }
        "cycle_kbd_effect" => {
            let state = global_lm.state.lock().clone();
            let cur = if state.kb_engine == "streamer" { state.streamer_effect } else { state.kb_effect };
            let effects = [101u8, 102, 103, 104, 105, 106];
            let next = if let Some(pos) = effects.iter().position(|&e| e == cur) {
                effects[(pos + 1) % effects.len()]
            } else {
                101
            };
            let engine = if next >= 100 { "streamer".to_string() } else { "hardware".to_string() };
            let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                app,
                true,
                next,
                state.kb_brightness.max(1),
                state.kb_speed,
                state.kb_direction,
                state.kb_color.clone(),
                Some(engine),
                None,
                Some(next),
                Some(next),
                None,
                None,
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
        "cycle_kbd_color" => {
            let colors = [
                "#6366F1", // Indigo
                "#06B6D4", // Cyan
                "#10B981", // Emerald
                "#EF4444", // Red
                "#F59E0B", // Amber
                "#EC4899", // Pink
                "#8B5CF6", // Purple
                "#FFFFFF", // White
            ];
            let state = global_lm.state.lock().clone();
            let cur_hex = state.kb_color.to_uppercase();
            let next_color = if let Some(pos) = colors.iter().position(|&c| c.eq_ignore_ascii_case(&cur_hex)) {
                colors[(pos + 1) % colors.len()]
            } else {
                colors[0]
            };
            let engine = if state.kb_effect >= 100 { "streamer".to_string() } else { "hardware".to_string() };
            let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                app,
                true,
                state.kb_effect,
                state.kb_brightness.max(1),
                state.kb_speed,
                state.kb_direction,
                next_color.to_string(),
                Some(engine),
                None,
                if state.kb_effect >= 100 { Some(state.kb_effect) } else { None },
                if state.kb_effect >= 100 { Some(state.kb_effect) } else { None },
                None,
                None,
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
        "open_brfx_in_notepad" => {
            let filename = val.get("filename").and_then(|v| v.as_str()).unwrap_or("custom.brfx").to_string();
            let _ = crate::commands::lighting_cmd::open_brfx_in_notepad(filename);
            true
        }
        _ => false,
    }
}
