use crate::ipc;
use crate::MainWindow;

pub fn register(window: &MainWindow, ui_weak: slint::Weak<MainWindow>) {
    let ui_cb = ui_weak.clone();
    window.on_power_mode_clicked(move |mode| {
        if let Some(ui) = ui_cb.upgrade() {
            if (mode == 3 || mode == 4) && ui.get_is_power_mode_restricted() {
                // Restricted on battery/Type-C without unlocking: prevent action and preserve current mode
                return;
            }
            ipc::send_action_to_host("set_power_mode", serde_json::json!({ "mode": mode }));
            ui.set_power_mode(mode);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_sleep_auto_office(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_sleep_auto_office();
            ui.set_sleep_auto_office(next);
            ipc::send_action_to_host("toggle_sleep_auto_office", serde_json::json!({ "enable": next }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_master_sleep_guard(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let next = !ui.get_sleep_guard_enabled();
            ui.set_sleep_guard_enabled(next);
            ui.set_sleep_disconnect_net(next);
            ui.set_sleep_block_away(next);
            ui.set_sleep_ignore_sysreq(next);
            ui.set_sleep_disable_wake_timers(next);
            ui.set_sleep_deep_sleep(next);
            ui.set_sleep_hibernate_30m(next);
            ipc::send_action_to_host("toggle_master_sleep_guard", serde_json::json!({ "enable": next }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_sleep_guard_option(move |opt| {
        if let Some(ui) = ui_cb.upgrade() {
            let opt_str = opt.as_str();
            let next = match opt_str {
                "disconnect_net" => {
                    let n = !ui.get_sleep_disconnect_net();
                    ui.set_sleep_disconnect_net(n);
                    n
                }
                "block_away" => {
                    let n = !ui.get_sleep_block_away();
                    ui.set_sleep_block_away(n);
                    n
                }
                "ignore_sysreq" => {
                    let n = !ui.get_sleep_ignore_sysreq();
                    ui.set_sleep_ignore_sysreq(n);
                    n
                }
                "disable_wake_timers" => {
                    let n = !ui.get_sleep_disable_wake_timers();
                    ui.set_sleep_disable_wake_timers(n);
                    n
                }
                "deep_sleep" => {
                    let n = !ui.get_sleep_deep_sleep();
                    ui.set_sleep_deep_sleep(n);
                    n
                }
                "hibernate_30m" => {
                    let n = !ui.get_sleep_hibernate_30m();
                    ui.set_sleep_hibernate_30m(n);
                    n
                }
                _ => false,
            };
            ipc::send_action_to_host("toggle_sleep_guard_option", serde_json::json!({ "option": opt_str, "enable": next }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_power_option(move |opt| {
        if let Some(ui) = ui_cb.upgrade() {
            let opt_str = opt.as_str();
            let next = match opt_str {
                "usb_charge" => {
                    let n = !ui.get_usb_charge_enabled();
                    ui.set_usb_charge_enabled(n);
                    n
                }
                "ac_recovery" => {
                    let n = !ui.get_ac_recovery_enabled();
                    ui.set_ac_recovery_enabled(n);
                    n
                }
                "high_perf" => {
                    let n = !ui.get_high_perf_scheme();
                    ui.set_high_perf_scheme(n);
                    n
                }
                _ => false,
            };
            ipc::send_action_to_host("toggle_power_option", serde_json::json!({ "option": opt_str, "enable": next }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_select_battery_mode(move |mode| {
        if let Some(ui) = ui_cb.upgrade() {
            let m_str = mode.as_str();
            let limit = match m_str {
                "long_life" => 100,
                "station" => 60,
                _ => 80,
            };
            ui.set_battery_mode(mode.clone());
            ui.set_battery_limit(limit);
            ipc::send_action_to_host("select_battery_mode", serde_json::json!({ "mode": m_str }));
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_battery_limit_clicked(move |limit| {
        ipc::send_action_to_host("set_battery_limit", serde_json::json!({ "battery_limit": limit }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_battery_limit(limit);
        }
    });

    window.on_switch_power_scheme(move |guid| {
        ipc::send_action_to_host("switch_power_scheme", serde_json::json!({ "guid": guid.as_str() }));
    });

    window.on_refresh_power_schemes(move || {
        ipc::send_action_to_host("refresh_power_schemes", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_confirm_restore_power_plan(move || {
        ipc::send_action_to_host("restore_official_power_plan", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_show_restore_plan_confirm(false);
        }
    });
}

