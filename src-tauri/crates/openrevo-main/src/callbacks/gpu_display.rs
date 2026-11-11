use crate::ipc;
use crate::MainWindow;

pub fn register(window: &MainWindow, ui_weak: slint::Weak<MainWindow>) {
    // GPU mode
    let ui_cb = ui_weak.clone();
    window.on_gpu_mode_clicked(move |mode| {
        ipc::send_action_to_host("set_gpu_mode", serde_json::json!({ "gpu_mode": mode.as_str() }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_gpu_mode(mode);
        }
    });

    // Refresh Rate & Display Monitor
    let ui_cb = ui_weak.clone();
    window.on_refresh_rate_switched(move |hz| {
        ipc::send_action_to_host("switch_refresh_rate", serde_json::json!({ "hz": hz }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_refresh_rate(hz);
        }
    });

    window.on_select_monitor_index(move |idx| {
        ipc::send_action_to_host("select_monitor_index", serde_json::json!({ "index": idx }));
    });

    // Auto min hz
    let ui_cb = ui_weak.clone();
    window.on_auto_min_hz_toggled(move || {
        ipc::send_action_to_host("toggle_auto_min_hz", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_auto_min_hz(!ui.get_auto_min_hz());
        }
    });

    // Brightness
    let ui_cb = ui_weak.clone();
    window.on_brightness_changed(move |b| {
        ipc::send_action_to_host("set_brightness", serde_json::json!({ "brightness": b }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_brightness(b);
        }
    });

    // Mini-LED local dimming toggle
    let ui_miniled = ui_weak.clone();
    window.on_toggle_miniled(move || {
        if let Some(ui) = ui_miniled.upgrade() {
            let next = !ui.get_miniled_enabled();
            ui.set_miniled_enabled(next);
            ipc::send_action_to_host("set_miniled_status", serde_json::json!({ "enabled": next }));
        }
    });

    // Display Tuning & Color Management
    let ui_tuning_en = ui_weak.clone();
    window.on_set_tuning_enabled(move |en| {
        ipc::send_action_to_host("set_display_tuning_enabled", serde_json::json!({ "enabled": en }));
        if let Some(ui) = ui_tuning_en.upgrade() {
            ui.set_tuning_enabled(en);
        }
    });
    window.on_toggle_tuning_enabled(move || {
        ipc::send_action_to_host("toggle_display_tuning", serde_json::json!({}));
    });
    window.on_apply_tuning_preset(move |id| {
        ipc::send_action_to_host("apply_display_tuning_preset", serde_json::json!({ "preset_id": id.as_str() }));
    });
    window.on_update_tuning_slider(move |key, val| {
        ipc::send_action_to_host("update_display_tuning_slider", serde_json::json!({ "key": key.as_str(), "value": val }));
    });
    window.on_reset_tuning_preset(move || {
        ipc::send_action_to_host("reset_display_tuning_preset", serde_json::json!({}));
    });
    window.on_import_tuning_file(move || {
        ipc::send_action_to_host("import_display_tuning_file", serde_json::json!({}));
    });
    window.on_export_tuning_file(move || {
        ipc::send_action_to_host("export_display_tuning_file", serde_json::json!({}));
    });
    window.on_import_color_profile(move || {
        ipc::send_action_to_host("import_color_profile", serde_json::json!({}));
    });
    window.on_open_cloud_icc(move || {
        ipc::send_action_to_host("open_cloud_icc", serde_json::json!({}));
    });
    window.on_refresh_cloud_icc_identity(move || {
        ipc::send_action_to_host("refresh_cloud_icc_identity", serde_json::json!({}));
    });
    window.on_start_cloud_icc_search(move |mode| {
        ipc::send_action_to_host("start_cloud_icc_search", serde_json::json!({ "mode": mode.as_str() }));
    });
    window.on_apply_cloud_icc_profile(move |idx| {
        ipc::send_action_to_host("apply_cloud_icc_profile", serde_json::json!({ "index": idx }));
    });
    window.on_apply_all_cloud_icc_profiles(move || {
        ipc::send_action_to_host("apply_all_cloud_icc_profiles", serde_json::json!({}));
    });
    window.on_apply_color_profile(move |prof| {
        ipc::send_action_to_host("apply_color_profile", serde_json::json!({ "profile_id": prof.as_str() }));
    });
}

