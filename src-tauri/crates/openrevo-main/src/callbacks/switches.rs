use crate::ipc;
use crate::MainWindow;

pub fn register(window: &MainWindow, ui_weak: slint::Weak<MainWindow>) {
    let ui_cb = ui_weak.clone();
    window.on_fan_boost_toggled(move || {
        ipc::send_action_to_host("toggle_fan_boost", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_fan_boost(!ui.get_fan_boost());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_bld_fan_lock_toggled(move |rpm| {
        ipc::send_action_to_host("toggle_bld_fan_lock", serde_json::json!({ "rpm": rpm }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_fan_boost(!ui.get_fan_boost());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_bld_fan_rpm_changed(move |rpm| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_bld_fan_rpm(rpm);
        }
        ipc::send_action_to_host("set_bld_fan_rpm", serde_json::json!({ "rpm": rpm }));
    });

    let ui_cb = ui_weak.clone();
    window.on_camera_toggled(move || {
        ipc::send_action_to_host("toggle_camera", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_camera_enabled(!ui.get_camera_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_mic_toggled(move || {
        ipc::send_action_to_host("toggle_mic", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_mic_enabled(!ui.get_mic_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_win_lock_toggled(move || {
        ipc::send_action_to_host("toggle_win_lock", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_win_locked(!ui.get_win_locked());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_keep_awake_toggled(move || {
        ipc::send_action_to_host("toggle_keep_awake", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_keep_awake(!ui.get_keep_awake());
        }
    });

    window.on_turn_off_screen_clicked(move || {
        ipc::send_action_to_host("turn_off_screen", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_wifi_toggled(move || {
        ipc::send_action_to_host("toggle_wifi", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_wifi_enabled(!ui.get_wifi_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_bt_toggled(move || {
        ipc::send_action_to_host("toggle_bt", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_bt_enabled(!ui.get_bt_enabled());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_touchpad_toggled(move || {
        ipc::send_action_to_host("toggle_touchpad", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_touchpad_locked(!ui.get_touchpad_locked());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_osd_toggled(move || {
        ipc::send_action_to_host("toggle_osd", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_osd_enabled();
            ui.set_osd_enabled(next);
            let mut cfg = ui.get_osd_config();
            cfg.enabled = next;
            ui.set_osd_config(cfg);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_fn_lock_toggled(move || {
        ipc::send_action_to_host("toggle_fn_lock", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_fn_locked(!ui.get_fn_locked());
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_autostart_toggled(move || {
        ipc::send_action_to_host("toggle_autostart", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_autostart(!ui.get_autostart());
        }
    });

    let ui_oem = ui_weak.clone();
    window.on_oem_toggled(move || {
        let cur_running = if let Some(ui) = ui_oem.upgrade() {
            ui.get_oem_running()
        } else {
            false
        };
        // When OEM is currently running (cur_running=true), user clicks "接管官方控制台", take_over must be true.
        // When OEM is currently stopped/taken over (cur_running=false), user clicks "恢复官方控制台", take_over must be false.
        ipc::send_action_to_host("toggle_oem", serde_json::json!({ "take_over": cur_running }));
        if let Some(ui) = ui_oem.upgrade() {
            ui.set_oem_running(!cur_running);
        }
    });

    // GPU Boost Handlers
    window.on_toggle_balanced_db(move |en| {
        ipc::send_action_to_host("set_balanced_db", serde_json::json!({ "enabled": en }));
    });
    let ui_cb = ui_weak.clone();
    window.on_change_oc_core(move |val| {
        ipc::send_action_to_host("set_gpu_oc_core", serde_json::json!({ "mhz": val }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_oc_save_hint("已保存".into());
        }
    });
    let ui_cb = ui_weak.clone();
    window.on_change_oc_mem(move |val| {
        ipc::send_action_to_host("set_gpu_oc_mem", serde_json::json!({ "mhz": val }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_oc_save_hint("已保存".into());
        }
    });

    // System Settings Handlers
    let ui_cb = ui_weak.clone();
    window.on_change_osd_theme(move |t| {
        ipc::send_action_to_host("set_osd_theme", serde_json::json!({ "theme": t.as_str() }));
        if let Some(ui) = ui_cb.upgrade() {
            let mut cfg = ui.get_osd_config();
            cfg.theme = t;
            ui.set_osd_config(cfg);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_change_osd_scale(move |s| {
        ipc::send_action_to_host("set_osd_scale", serde_json::json!({ "scale": s.as_str() }));
        if let Some(ui) = ui_cb.upgrade() {
            let mut cfg = ui.get_osd_config();
            cfg.scale = s;
            ui.set_osd_config(cfg);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_change_osd_position(move |p| {
        ipc::send_action_to_host("set_osd_position", serde_json::json!({ "position": p.as_str() }));
        if let Some(ui) = ui_cb.upgrade() {
            let mut cfg = ui.get_osd_config();
            cfg.position = p;
            ui.set_osd_config(cfg);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_osd_enabled(move |en| {
        ipc::send_action_to_host("set_osd_enabled", serde_json::json!({ "enabled": en }));
        if let Some(ui) = ui_cb.upgrade() {
            let mut cfg = ui.get_osd_config();
            cfg.enabled = en;
            ui.set_osd_config(cfg);
        }
    });

    window.on_trigger_osd_preview(move || {
        ipc::send_action_to_host("trigger_osd_preview", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_confirm_unlock_battery(move || {
        ipc::send_action_to_host("set_unlock_battery_limit", serde_json::json!({ "enabled": true }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_unlock_battery_limit(true);
            ui.set_show_unlock_battery_confirm(false);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_disable_unlock_battery(move || {
        ipc::send_action_to_host("set_unlock_battery_limit", serde_json::json!({ "enabled": false }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_unlock_battery_limit(false);
        }
    });
}

