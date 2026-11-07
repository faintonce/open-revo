use crate::MainWindow;
use super::parse_hex_color;

pub fn handle(action: &str, val: &serde_json::Value, ui: &MainWindow) -> bool {
    match action {
        "sync_lighting_state" => {
            if let Some(sub_tab) = val.get("sub_tab").and_then(|v| v.as_i64()) {
                ui.set_lighting_sub_tab(sub_tab as i32);
            }
            if let Some(sleep) = val.get("sleep_minutes").and_then(|v| v.as_i64()) {
                ui.set_sleep_minutes(sleep as i32);
            }
            if let Some(ceremony) = val.get("ceremony_enabled").and_then(|v| v.as_bool()) {
                ui.set_ceremony_enabled(ceremony);
            }
            if let Some(save_ok) = val.get("save_success").and_then(|v| v.as_bool()) {
                ui.set_lighting_save_success(save_ok);
            }

            // Keyboard
            if let Some(en) = val.get("kb_enabled").and_then(|v| v.as_bool()) {
                ui.set_kb_enabled(en);
            }
            if let Some(eng) = val.get("kb_engine").and_then(|v| v.as_str()) {
                let eng_code = match eng {
                    "hardware" => 0,
                    "streamer" => 1,
                    "custom" => 2,
                    _ => 1,
                };
                ui.set_kb_engine(eng_code);
            }
            if let Some(fps) = val.get("kb_fps").and_then(|v| v.as_i64()) {
                ui.set_kb_fps(fps as i32);
            }
            if let Some(eff) = val.get("kb_effect").and_then(|v| v.as_i64()) {
                ui.set_kb_effect(eff as i32);
            }
            if let Some(dir) = val.get("kb_direction").and_then(|v| v.as_i64()) {
                ui.set_kb_direction(dir as i32);
            }
            if let Some(b) = val.get("kb_brightness").and_then(|v| v.as_i64()) {
                ui.set_kb_brightness(b as i32);
            }
            if let Some(s) = val.get("kb_speed").and_then(|v| v.as_i64()) {
                ui.set_kb_speed(s as i32);
            }
            if let Some(c) = val.get("kb_color").and_then(|v| v.as_str()) {
                ui.set_kb_color_hex(c.into());
                let color = slint::Color::from_argb_encoded(parse_hex_color(c));
                ui.set_kb_color(color);
            }

            // Lightbar
            if let Some(en) = val.get("lb_enabled").and_then(|v| v.as_bool()) {
                ui.set_lb_enabled(en);
            }
            if let Some(eff) = val.get("lb_effect").and_then(|v| v.as_i64()) {
                ui.set_lb_effect(eff as i32);
            }
            if let Some(b) = val.get("lb_brightness").and_then(|v| v.as_i64()) {
                ui.set_lb_brightness(b as i32);
            }
            if let Some(s) = val.get("lb_speed").and_then(|v| v.as_i64()) {
                ui.set_lb_speed(s as i32);
            }
            if let Some(c) = val.get("lb_color").and_then(|v| v.as_str()) {
                ui.set_lb_color_hex(c.into());
                let color = slint::Color::from_argb_encoded(parse_hex_color(c));
                ui.set_lb_color(color);
            }

            // Ambient / Hinge
            if let Some(en) = val.get("hinge_enabled").and_then(|v| v.as_bool()) {
                ui.set_hinge_enabled(en);
            }
            if let Some(eff) = val.get("hinge_effect").and_then(|v| v.as_i64()) {
                ui.set_hinge_effect(eff as i32);
            }
            if let Some(b) = val.get("hinge_brightness").and_then(|v| v.as_i64()) {
                ui.set_hinge_brightness(b as i32);
            }
            if let Some(s) = val.get("hinge_speed").and_then(|v| v.as_i64()) {
                ui.set_hinge_speed(s as i32);
            }
            if let Some(c) = val.get("hinge_color").and_then(|v| v.as_str()) {
                ui.set_hinge_color_hex(c.into());
                let color = slint::Color::from_argb_encoded(parse_hex_color(c));
                ui.set_hinge_color(color);
            }

            // Logo
            if let Some(en) = val.get("logo_enabled").and_then(|v| v.as_bool()) {
                ui.set_logo_enabled(en);
            }
            if let Some(eff) = val.get("logo_effect").and_then(|v| v.as_i64()) {
                ui.set_logo_effect(eff as i32);
            }
            if let Some(b) = val.get("logo_brightness").and_then(|v| v.as_i64()) {
                ui.set_logo_brightness(b as i32);
            }
            if let Some(c) = val.get("logo_color").and_then(|v| v.as_str()) {
                ui.set_logo_color_hex(c.into());
                let color = slint::Color::from_argb_encoded(parse_hex_color(c));
                ui.set_logo_color(color);
            }
            true
        }
        "sync_lighting_hardware_info" => {
            if let Some(hlb) = val.get("has_front_lightbar").and_then(|v| v.as_bool()) {
                ui.set_has_lightbar(hlb);
            }
            if let Some(hhinge) = val.get("has_hinge").and_then(|v| v.as_bool()) {
                ui.set_has_hinge(hhinge);
            }
            if let Some(hlogo) = val.get("has_logo").and_then(|v| v.as_bool()) {
                ui.set_has_logo(hlogo);
            }
            if let Some(eclogo) = val.get("is_ec_logo").and_then(|v| v.as_bool()) {
                ui.set_is_ec_logo(eclogo);
            }
            if let Some(desc) = val.get("description").and_then(|v| v.as_str()) {
                ui.set_kb_hw_desc(desc.into());
            }
            if let Some(ht) = val.get("hardware_type").and_then(|v| v.as_str()) {
                ui.set_kb_hardware_type(ht.into());
            }
            if let Some(fz) = val.get("is_four_zone").and_then(|v| v.as_bool()) {
                ui.set_is_four_zone(fz);
            }
            if let Some(sz) = val.get("is_single_zone").and_then(|v| v.as_bool()) {
                ui.set_is_single_zone(sz);
            }
            if let Some(sc) = val.get("is_single_color").and_then(|v| v.as_bool()) {
                ui.set_is_single_color(sc);
            }
            true
        }
        _ => false,
    }
}

