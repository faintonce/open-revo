use tauri::AppHandle;
use super::super::sync::sync_slint_lighting_state;
use super::super::dialog::open_file_dialog;

#[derive(Clone, Debug, serde::Serialize)]
pub struct ParsedBrfxScript {
    pub id: String,
    pub file_name: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub default_speed: f64,
    pub default_scale: f64,
    pub default_direction: u8,
    pub default_reactive: bool,
    pub default_color: String,
    pub builtin_effect_id: u8,
    pub is_marquee: bool,
    #[serde(skip)]
    pub content: String,
}

fn parse_brfx_tag(content: &str, tag: &str) -> Option<String> {
    let needle = format!("@{}:", tag);
    for line in content.lines().take(32) {
        if let Some(pos) = line.find(&needle) {
            let val = line[pos + needle.len()..].trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

pub fn load_parsed_brfx_scripts() -> Vec<ParsedBrfxScript> {
    let files = crate::commands::lighting_cmd::get_custom_brfx_list().unwrap_or_default();
    let mut list: Vec<ParsedBrfxScript> = files
        .into_iter()
        .map(|f| {
            let id = std::path::Path::new(&f.file_name)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| f.file_name.clone());
            let low = id.to_lowercase();
            let builtin_effect_id = if low.contains("kamehameha") {
                200
            } else if low.contains("lotus") {
                201
            } else if low.contains("singularity") {
                202
            } else if low.contains("marquee") {
                203
            } else {
                200
            };
            let is_marquee = low.contains("marquee") || f.content.contains("ctx.text");
            ParsedBrfxScript {
                name: parse_brfx_tag(&f.content, "name").unwrap_or_else(|| id.clone()),
                author: parse_brfx_tag(&f.content, "author").unwrap_or_else(|| "OpenRevo Community".to_string()),
                description: parse_brfx_tag(&f.content, "description").unwrap_or_default(),
                default_speed: parse_brfx_tag(&f.content, "default_speed")
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|v| v.clamp(0.2, 3.0))
                    .unwrap_or(1.2),
                default_scale: parse_brfx_tag(&f.content, "default_scale")
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|v| v.clamp(0.5, 2.5))
                    .unwrap_or(1.0),
                default_direction: parse_brfx_tag(&f.content, "default_direction")
                    .and_then(|v| v.parse::<u8>().ok())
                    .map(|v| v.min(3))
                    .unwrap_or(0),
                default_reactive: parse_brfx_tag(&f.content, "default_reactive")
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false),
                default_color: parse_brfx_tag(&f.content, "default_color")
                    .unwrap_or_else(|| "#00F0FF".to_string()),
                builtin_effect_id,
                is_marquee,
                id,
                file_name: f.file_name,
                content: f.content,
            }
        })
        .collect();

    list.sort_by(|a, b| a.builtin_effect_id.cmp(&b.builtin_effect_id).then_with(|| a.name.cmp(&b.name)));
    list
}

pub fn ensure_active_custom_script_loaded(script_id: &str) -> Option<ParsedBrfxScript> {
    let scripts = load_parsed_brfx_scripts();
    let target = scripts
        .iter()
        .find(|s| s.id == script_id)
        .or_else(|| scripts.first())?
        .clone();
    let engine = crate::driver::custom_fx_engine::get_custom_fx_engine();
    if !engine.has_active_script() || engine.get_active_name() != target.name {
        let _ = engine.load_script(&target.name, &target.content);
    }
    Some(target)
}

pub fn handle(action: &str, val: &serde_json::Value, app: Option<&AppHandle>) -> bool {
    let global_lm = crate::driver::ite_lighting::get_global_lighting();
    match action {
        "set_kb_engine" => {
            let engine = val.get("engine").and_then(|v| v.as_str()).unwrap_or("hardware").to_string();
            let st = global_lm.state.lock().clone();
            let (target_eff, streamer_eff, script_id_opt, tuning_override) = match engine.as_str() {
                "streamer" => {
                    crate::driver::custom_fx_engine::get_custom_fx_engine().clear_script();
                    let se = if st.streamer_effect >= 100 && st.streamer_effect < 200 { st.streamer_effect } else { 101 };
                    (se, Some(se), None, None)
                }
                "custom" => {
                    if let Some(script) = ensure_active_custom_script_loaded(&st.custom_script_id) {
                        let fx = crate::driver::custom_fx_engine::get_custom_fx_engine();
                        fx.set_tuning(Some(script.default_speed), script.default_scale, script.default_reactive);
                        (
                            script.builtin_effect_id,
                            Some(script.builtin_effect_id),
                            Some(script.id.clone()),
                            Some((
                                ((script.default_speed * 2.0).round() as u8).min(4),
                                script.default_direction,
                                script.default_color,
                            )),
                        )
                    } else {
                        let se = if st.streamer_effect >= 200 { st.streamer_effect } else { 200 };
                        (se, Some(se), Some(st.custom_script_id.clone()), None)
                    }
                }
                _ => {
                    crate::driver::custom_fx_engine::get_custom_fx_engine().clear_script();
                    let hw = if st.kb_effect > 0 && st.kb_effect < 100 { st.kb_effect } else { 3 };
                    (hw, None, None, None)
                }
            };
            let (kb_speed, kb_dir, kb_col) = tuning_override.unwrap_or((st.kb_speed, st.kb_direction, st.kb_color));
            let csi = script_id_opt.unwrap_or(st.custom_script_id);
            let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                app,
                st.kb_enabled,
                target_eff,
                st.kb_brightness,
                kb_speed,
                kb_dir,
                kb_col,
                Some(engine),
                Some(st.kb_fps),
                streamer_eff,
                streamer_eff,
                Some(csi.clone()),
                Some(csi),
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
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
            let filename = val
                .get("file_name")
                .or_else(|| val.get("filename"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or("kamehameha");
                    format!("{}.brfx", id)
                });
            let _ = crate::commands::lighting_cmd::open_brfx_in_notepad(filename);
            true
        }
        "select_custom_script" => {
            let req_id = val.get("id").and_then(|v| v.as_str()).unwrap_or("kamehameha");
            let scripts = load_parsed_brfx_scripts();
            if let Some(script) = scripts.into_iter().find(|s| s.id == req_id) {
                let fx = crate::driver::custom_fx_engine::get_custom_fx_engine();
                let _ = fx.load_script(&script.name, &script.content);
                fx.set_tuning(Some(script.default_speed), script.default_scale, script.default_reactive);
                let st = global_lm.state.lock().clone();
                let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                    app,
                    st.kb_enabled,
                    script.builtin_effect_id,
                    st.kb_brightness,
                    ((script.default_speed * 2.0).round() as u8).min(4),
                    script.default_direction,
                    script.default_color,
                    Some("custom".to_string()),
                    Some(st.kb_fps),
                    Some(script.builtin_effect_id),
                    Some(script.builtin_effect_id),
                    Some(script.id.clone()),
                    Some(script.id),
                    &global_lm,
                );
            }
            sync_slint_lighting_state();
            true
        }
        "update_custom_tuning" => {
            let st = global_lm.state.lock().clone();
            let fx = crate::driver::custom_fx_engine::get_custom_fx_engine();
            let cur_speed = val.get("speed").and_then(|v| v.as_f64()).unwrap_or_else(|| fx.get_tuning_speed().unwrap_or(1.2));
            let cur_scale = val.get("scale").and_then(|v| v.as_f64()).unwrap_or_else(|| fx.get_tuning_scale());
            let cur_reactive = val.get("reactive").and_then(|v| v.as_bool()).unwrap_or_else(|| fx.get_tuning_reactive());
            fx.set_tuning(Some(cur_speed.clamp(0.2, 3.0)), cur_scale.clamp(0.5, 2.5), cur_reactive);
            if let Some(txt) = val.get("marquee_text").and_then(|v| v.as_str()) {
                fx.set_active_text(txt);
            }
            let dir = val.get("direction").and_then(|v| v.as_i64()).map(|d| d as u8).unwrap_or(st.kb_direction);
            let col = val.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or(st.kb_color);
            let eff = if st.streamer_effect >= 200 { st.streamer_effect } else { 200 };
            let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                app,
                st.kb_enabled,
                eff,
                st.kb_brightness,
                ((cur_speed * 2.0).round() as u8).min(4),
                dir,
                col,
                Some("custom".to_string()),
                Some(st.kb_fps),
                Some(eff),
                Some(eff),
                Some(st.custom_script_id.clone()),
                Some(st.custom_script_id),
                &global_lm,
            );
            sync_slint_lighting_state();
            true
        }
        "refresh_custom_scripts" => {
            let st = global_lm.state.lock().clone();
            let scripts = load_parsed_brfx_scripts();
            if let Some(script) = scripts.iter().find(|s| s.id == st.custom_script_id).or_else(|| scripts.first()) {
                let fx = crate::driver::custom_fx_engine::get_custom_fx_engine();
                let _ = fx.load_script(&script.name, &script.content);
            }
            sync_slint_lighting_state();
            true
        }
        "import_custom_script" => {
            let app_clone = app.cloned();
            std::thread::spawn(move || {
                if let Some(file_path) = open_file_dialog("BRFX 动效脚本", "brfx;*.revo-fx;*.rhai;*.txt") {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let fname = std::path::Path::new(&file_path)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "imported.brfx".to_string());
                        if crate::commands::lighting_cmd::save_custom_brfx(fname.clone(), content.clone()).is_ok() {
                            let id = std::path::Path::new(&fname)
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "imported".to_string());
                            let scripts = load_parsed_brfx_scripts();
                            if let Some(script) = scripts.into_iter().find(|s| s.id == id) {
                                let fx = crate::driver::custom_fx_engine::get_custom_fx_engine();
                                let _ = fx.load_script(&script.name, &script.content);
                                fx.set_tuning(Some(script.default_speed), script.default_scale, script.default_reactive);
                                let global_lm = crate::driver::ite_lighting::get_global_lighting();
                                let st = global_lm.state.lock().clone();
                                let _ = crate::commands::lighting_cmd::apply_keyboard_lighting_internal(
                                    app_clone.as_ref(),
                                    st.kb_enabled,
                                    script.builtin_effect_id,
                                    st.kb_brightness,
                                    ((script.default_speed * 2.0).round() as u8).min(4),
                                    script.default_direction,
                                    script.default_color,
                                    Some("custom".to_string()),
                                    Some(st.kb_fps),
                                    Some(script.builtin_effect_id),
                                    Some(script.builtin_effect_id),
                                    Some(script.id.clone()),
                                    Some(script.id),
                                    &global_lm,
                                );
                            }
                            sync_slint_lighting_state();
                        }
                    }
                }
            });
            true
        }
        "export_custom_script" => {
            let req_id = val.get("id").and_then(|v| v.as_str()).unwrap_or("kamehameha").to_string();
            std::thread::spawn(move || {
                let scripts = load_parsed_brfx_scripts();
                if let Some(script) = scripts.into_iter().find(|s| s.id == req_id) {
                    let default_name = format!("{}.brfx", script.id);
                    if let Some(dest) = crate::commands::tuning_cmd::save_file_dialog(&default_name, "BRFX 动效脚本", "brfx") {
                        let _ = std::fs::write(dest, script.content);
                    }
                }
            });
            true
        }
        _ => false,
    }
}
