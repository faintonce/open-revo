use crate::ipc;
use crate::MainWindow;

pub fn register(window: &MainWindow, ui_weak: slint::Weak<MainWindow>) {
    window.on_kbd_effect_cycled(move || {
        ipc::send_action_to_host("cycle_kbd_effect", serde_json::json!({}));
    });
    window.on_kbd_color_cycled(move || {
        ipc::send_action_to_host("cycle_kbd_color", serde_json::json!({}));
    });
    window.on_kbd_more_clicked(move || {
        ipc::send_action_to_host("open_lighting_studio", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_set_lighting_sleep_timer(move |mins| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_sleep_minutes(mins);
            ipc::send_action_to_host("set_lighting_sleep_timer", serde_json::json!({ "minutes": mins }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_ceremony(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_ceremony_enabled();
            ui.set_ceremony_enabled(next);
            ipc::send_action_to_host("set_ceremony_enabled", serde_json::json!({ "enabled": next }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_preview_ceremony(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_previewing_ceremony(true);
            ipc::send_action_to_host("trigger_lighting_welcome", serde_json::json!({}));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_save_lighting_as_startup(move || {
        ipc::send_action_to_host("save_current_lighting_as_startup", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_lighting_save_success(true);
        }
    });

    window.on_reset_lighting_defaults(move |tab| {
        ipc::send_action_to_host("reset_lighting_defaults", serde_json::json!({ "sub_tab": tab }));
    });

    // Keyboard Callbacks
    let ui_cb = ui_weak.clone();
    window.on_toggle_kb_enabled(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_kb_enabled();
            ui.set_kb_enabled(next);
            ipc::send_action_to_host("apply_keyboard_lighting_patch", serde_json::json!({
                "enabled": next,
                "engine": if ui.get_kb_engine() == 1 { "streamer" } else { "hardware" },
                "fps": ui.get_kb_fps(),
                "effect": ui.get_kb_effect(),
                "direction": ui.get_kb_direction(),
                "brightness": ui.get_kb_brightness(),
                "speed": ui.get_kb_speed(),
                "color": ui.get_kb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_kb_engine(move |eng| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_kb_engine(eng);
            ipc::send_action_to_host("apply_keyboard_lighting_patch", serde_json::json!({
                "enabled": ui.get_kb_enabled(),
                "engine": if eng == 1 { "streamer" } else { "hardware" },
                "fps": ui.get_kb_fps(),
                "effect": ui.get_kb_effect(),
                "direction": ui.get_kb_direction(),
                "brightness": ui.get_kb_brightness(),
                "speed": ui.get_kb_speed(),
                "color": ui.get_kb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_kb_fps(move |fps| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_kb_fps(fps);
            ipc::send_action_to_host("set_streamer_fps", serde_json::json!({ "fps": fps }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_kb_effect(move |eff| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_kb_effect(eff);
            ipc::send_action_to_host("apply_keyboard_lighting_patch", serde_json::json!({
                "enabled": ui.get_kb_enabled(),
                "engine": if ui.get_kb_engine() == 1 { "streamer" } else { "hardware" },
                "fps": ui.get_kb_fps(),
                "effect": eff,
                "direction": ui.get_kb_direction(),
                "brightness": ui.get_kb_brightness(),
                "speed": ui.get_kb_speed(),
                "color": ui.get_kb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_kb_direction(move |dir| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_kb_direction(dir);
            ipc::send_action_to_host("apply_keyboard_lighting_patch", serde_json::json!({
                "enabled": ui.get_kb_enabled(),
                "engine": if ui.get_kb_engine() == 1 { "streamer" } else { "hardware" },
                "fps": ui.get_kb_fps(),
                "effect": ui.get_kb_effect(),
                "direction": dir,
                "brightness": ui.get_kb_brightness(),
                "speed": ui.get_kb_speed(),
                "color": ui.get_kb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_kb_brightness(move |b| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_kb_brightness(b);
            ipc::send_action_to_host("apply_keyboard_lighting_patch", serde_json::json!({
                "enabled": ui.get_kb_enabled(),
                "engine": if ui.get_kb_engine() == 1 { "streamer" } else { "hardware" },
                "fps": ui.get_kb_fps(),
                "effect": ui.get_kb_effect(),
                "direction": ui.get_kb_direction(),
                "brightness": b,
                "speed": ui.get_kb_speed(),
                "color": ui.get_kb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_kb_speed(move |s| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_kb_speed(s);
            ipc::send_action_to_host("apply_keyboard_lighting_patch", serde_json::json!({
                "enabled": ui.get_kb_enabled(),
                "engine": if ui.get_kb_engine() == 1 { "streamer" } else { "hardware" },
                "fps": ui.get_kb_fps(),
                "effect": ui.get_kb_effect(),
                "direction": ui.get_kb_direction(),
                "brightness": ui.get_kb_brightness(),
                "speed": s,
                "color": ui.get_kb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_kb_color(move |hex, _c| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_kb_color_hex(hex.clone());
            ipc::send_action_to_host("apply_keyboard_lighting_patch", serde_json::json!({
                "enabled": ui.get_kb_enabled(),
                "engine": if ui.get_kb_engine() == 1 { "streamer" } else { "hardware" },
                "fps": ui.get_kb_fps(),
                "effect": ui.get_kb_effect(),
                "direction": ui.get_kb_direction(),
                "brightness": ui.get_kb_brightness(),
                "speed": ui.get_kb_speed(),
                "color": hex.as_str(),
            }));
        }
    });

    // Lightbar Callbacks
    let ui_cb = ui_weak.clone();
    window.on_toggle_lb_enabled(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_lb_enabled();
            ui.set_lb_enabled(next);
            ipc::send_action_to_host("apply_lightbar_lighting", serde_json::json!({
                "enabled": next,
                "effect": ui.get_lb_effect(),
                "brightness": ui.get_lb_brightness(),
                "speed": ui.get_lb_speed(),
                "color": ui.get_lb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_lb_effect(move |eff| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_lb_effect(eff);
            ipc::send_action_to_host("apply_lightbar_lighting", serde_json::json!({
                "enabled": ui.get_lb_enabled(),
                "effect": eff,
                "brightness": ui.get_lb_brightness(),
                "speed": ui.get_lb_speed(),
                "color": ui.get_lb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_lb_brightness(move |b| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_lb_brightness(b);
            ipc::send_action_to_host("apply_lightbar_lighting", serde_json::json!({
                "enabled": ui.get_lb_enabled(),
                "effect": ui.get_lb_effect(),
                "brightness": b,
                "speed": ui.get_lb_speed(),
                "color": ui.get_lb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_lb_speed(move |s| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_lb_speed(s);
            ipc::send_action_to_host("apply_lightbar_lighting", serde_json::json!({
                "enabled": ui.get_lb_enabled(),
                "effect": ui.get_lb_effect(),
                "brightness": ui.get_lb_brightness(),
                "speed": s,
                "color": ui.get_lb_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_lb_color(move |hex, _c| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_lb_color_hex(hex.clone());
            ipc::send_action_to_host("apply_lightbar_lighting", serde_json::json!({
                "enabled": ui.get_lb_enabled(),
                "effect": ui.get_lb_effect(),
                "brightness": ui.get_lb_brightness(),
                "speed": ui.get_lb_speed(),
                "color": hex.as_str(),
            }));
        }
    });

    // Ambient / Hinge Callbacks
    let ui_cb = ui_weak.clone();
    window.on_toggle_hinge_enabled(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_hinge_enabled();
            ui.set_hinge_enabled(next);
            ipc::send_action_to_host("apply_hinge_lighting", serde_json::json!({
                "enabled": next,
                "effect": ui.get_hinge_effect(),
                "brightness": ui.get_hinge_brightness(),
                "speed": ui.get_hinge_speed(),
                "color": ui.get_hinge_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_hinge_effect(move |eff| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_hinge_effect(eff);
            ipc::send_action_to_host("apply_hinge_lighting", serde_json::json!({
                "enabled": ui.get_hinge_enabled(),
                "effect": eff,
                "brightness": ui.get_hinge_brightness(),
                "speed": ui.get_hinge_speed(),
                "color": ui.get_hinge_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_hinge_brightness(move |b| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_hinge_brightness(b);
            ipc::send_action_to_host("apply_hinge_lighting", serde_json::json!({
                "enabled": ui.get_hinge_enabled(),
                "effect": ui.get_hinge_effect(),
                "brightness": b,
                "speed": ui.get_hinge_speed(),
                "color": ui.get_hinge_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_hinge_speed(move |s| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_hinge_speed(s);
            ipc::send_action_to_host("apply_hinge_lighting", serde_json::json!({
                "enabled": ui.get_hinge_enabled(),
                "effect": ui.get_hinge_effect(),
                "brightness": ui.get_hinge_brightness(),
                "speed": s,
                "color": ui.get_hinge_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_hinge_color(move |hex, _c| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_hinge_color_hex(hex.clone());
            ipc::send_action_to_host("apply_hinge_lighting", serde_json::json!({
                "enabled": ui.get_hinge_enabled(),
                "effect": ui.get_hinge_effect(),
                "brightness": ui.get_hinge_brightness(),
                "speed": ui.get_hinge_speed(),
                "color": hex.as_str(),
            }));
        }
    });

    // Logo Callbacks
    let ui_cb = ui_weak.clone();
    window.on_toggle_logo_enabled(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_logo_enabled();
            ui.set_logo_enabled(next);
            ipc::send_action_to_host("apply_logo_lighting", serde_json::json!({
                "enabled": next,
                "effect": ui.get_logo_effect(),
                "brightness": ui.get_logo_brightness(),
                "color": ui.get_logo_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_logo_effect(move |eff| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_logo_effect(eff);
            ipc::send_action_to_host("apply_logo_lighting", serde_json::json!({
                "enabled": ui.get_logo_enabled(),
                "effect": eff,
                "brightness": ui.get_logo_brightness(),
                "color": ui.get_logo_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_logo_brightness(move |b| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_logo_brightness(b);
            ipc::send_action_to_host("apply_logo_lighting", serde_json::json!({
                "enabled": ui.get_logo_enabled(),
                "effect": ui.get_logo_effect(),
                "brightness": b,
                "color": ui.get_logo_color_hex().as_str(),
            }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_set_logo_color(move |hex, _c| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_logo_color_hex(hex.clone());
            ipc::send_action_to_host("apply_logo_lighting", serde_json::json!({
                "enabled": ui.get_logo_enabled(),
                "effect": ui.get_logo_effect(),
                "brightness": ui.get_logo_brightness(),
                "color": hex.as_str(),
            }));
        }
    });

    // Topology Callbacks
    window.on_apply_lighting_topology(move |en, kb, lb, amb, logo| {
        let kb_str = match kb {
            1 => "per_key",
            2 => "four_zone",
            3 => "single_zone",
            4 => "single_white",
            _ => "auto",
        };
        let lb_str = match lb {
            1 => "present",
            2 => "none",
            _ => "auto",
        };
        let amb_str = match amb {
            1 => "vent",
            2 => "none",
            _ => "auto",
        };
        let logo_str = match logo {
            1 => "letter",
            2 => "shield",
            3 => "none",
            _ => "auto",
        };
        ipc::send_action_to_host("apply_lighting_hardware_override", serde_json::json!({
            "override_cfg": {
                "enabled": en,
                "keyboard": kb_str,
                "front_bar": lb_str,
                "ambient": amb_str,
                "logo": logo_str,
            }
        }));
    });

    window.on_reset_lighting_topology(move || {
        ipc::send_action_to_host("reset_lighting_hardware_override", serde_json::json!({}));
    });
}

