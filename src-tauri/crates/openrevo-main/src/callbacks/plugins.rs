use crate::ipc;
use crate::window::HWND_HOLDER;
use crate::MainWindow;

pub fn register(window: &MainWindow, ui_weak: slint::Weak<MainWindow>) {
    // Plugin Center callbacks
    let ui_cb = ui_weak.clone();
    window.on_toggle_builtin_module(move |id, enabled| {
        if let Some(ui) = ui_cb.upgrade() {
            match id.as_str() {
                "overview" => ui.set_is_overview_enabled(enabled),
                "gpu" => ui.set_is_gpu_enabled(enabled),
                "tuning" => ui.set_is_tuning_enabled(enabled),
                "lighting" => ui.set_is_lighting_enabled(enabled),
                "water" => ui.set_is_water_enabled(enabled),
                _ => {}
            }
            ipc::send_action_to_host("toggle_builtin_module", serde_json::json!({
                "id": id.as_str(),
                "enabled": enabled,
            }));
        }
    });

    window.on_toggle_custom_plugin(move |id, enabled| {
        ipc::send_action_to_host("toggle_custom_plugin", serde_json::json!({
            "id": id.as_str(),
            "enabled": enabled,
        }));
    });

    window.on_open_plugins_dir(move || {
        ipc::send_action_to_host("open_plugins_dir", serde_json::json!({}));
    });

    window.on_reload_plugins(move || {
        ipc::send_action_to_host("reload_plugins", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_select_plugin_tab(move |plugin_id| {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_active_plugin_id(plugin_id.clone());
            let hwnd_val = HWND_HOLDER.lock().unwrap().map(|h| h.0).unwrap_or(0);
            ipc::send_action_to_host("select_plugin_tab", serde_json::json!({
                "plugin_id": plugin_id.as_str(),
                "hwnd": hwnd_val,
            }));
            if plugin_id.is_empty() {
                if let Some(h) = *HWND_HOLDER.lock().unwrap() {
                    crate::window::trigger_full_redraw(h.to_hwnd());
                }
            }
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_launch_plugin_window(move |plugin_id| {
        if let Some(ui) = ui_cb.upgrade() {
            // Automatically reset the embedded view and return to Plugin Center tab
            ui.set_active_plugin_id(slint::SharedString::from(""));
            ui.set_active_tab(5);

            let hwnd_val = HWND_HOLDER.lock().unwrap().map(|h| h.0).unwrap_or(0);
            ipc::send_action_to_host("select_plugin_tab", serde_json::json!({
                "plugin_id": "",
                "hwnd": hwnd_val,
            }));

            if let Some(h) = *HWND_HOLDER.lock().unwrap() {
                crate::window::trigger_full_redraw(h.to_hwnd());
            }
        }
        ipc::send_action_to_host("open_custom_widget", serde_json::json!({
            "plugin_id": plugin_id.as_str(),
        }));
    });

    window.on_open_plugin_dir(move |plugin_id| {
        ipc::send_action_to_host("open_plugin_dir", serde_json::json!({
            "plugin_id": plugin_id.as_str(),
        }));
    });
}

