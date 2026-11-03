// OpenRevo - System Tray Interaction & Menu Management
// Handles tray menu building, power/fan boost state synchronization, and safe exit handovers.

use parking_lot::Mutex;
use std::sync::{Arc, OnceLock};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter,
};

use crate::driver::ec_bridge::EcBridge;

pub fn app_version_display() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let commit = option_env!("OPENREVO_GIT_COMMIT").unwrap_or("");

    if commit.is_empty() {
        format!("PRO v{version}")
    } else {
        format!("PRO v{version}-{commit}")
    }
}

#[cfg(test)]
mod tests {
    use super::app_version_display;

    #[test]
    fn tray_version_uses_package_and_commit_hash() {
        let value = app_version_display();
        assert!(value.starts_with("PRO v"));
        assert!(value.contains(env!("CARGO_PKG_VERSION")));
    }
}

pub struct TrayMenuControls {
    pub full_panel: CheckMenuItem<tauri::Wry>,
    pub mini_drawer: CheckMenuItem<tauri::Wry>,
    pub office: CheckMenuItem<tauri::Wry>,
    pub balance: CheckMenuItem<tauri::Wry>,
    pub beast: CheckMenuItem<tauri::Wry>,
    pub custom: CheckMenuItem<tauri::Wry>,
    pub fan_boost: CheckMenuItem<tauri::Wry>,
}

pub struct TrayKeeper(#[allow(dead_code)] pub TrayIcon);

static GLOBAL_TRAY: OnceLock<TrayIcon> = OnceLock::new();
static TRAY_MENU_CONTROLS: OnceLock<TrayMenuControls> = OnceLock::new();

/// Updates the tray window presentation mode checkmarks (Full Panel vs Mini Drawer).
pub fn update_tray_window_mode(is_expanded: bool) {
    if let Some(ctrl) = TRAY_MENU_CONTROLS.get() {
        let _ = ctrl.full_panel.set_checked(is_expanded);
        let _ = ctrl.mini_drawer.set_checked(!is_expanded);
    }
}

/// Updates the tray power mode radio checkmarks.
pub fn update_tray_power_mode(mode: u8) {
    if let Some(ctrl) = TRAY_MENU_CONTROLS.get() {
        let _ = ctrl.office.set_checked(mode == 1);
        let _ = ctrl.balance.set_checked(mode == 2);
        let _ = ctrl.beast.set_checked(mode == 3);
        let _ = ctrl.custom.set_checked(mode == 4);
    }
}

/// Updates the tray fan boost checkmark state.
pub fn update_tray_fan_boost(enabled: bool) {
    if let Some(ctrl) = TRAY_MENU_CONTROLS.get() {
        let _ = ctrl.fan_boost.set_checked(enabled);
    }
}

/// Hook for future autostart tray checkmark synchronization.
pub fn update_tray_autostart_state(_enabled: bool) {
    // Reserved for dedicated tray autostart checkmark
}

/// Hook for future log level tray checkmark synchronization.
pub fn update_tray_log_state(_enabled: bool) {
    // Reserved for dedicated tray log level checkmark
}

/// Graceful hardware shutdown and MCU handover on application exit.
pub fn safe_hardware_shutdown_on_exit() {
    if let Err(e) = crate::hal::cpu_power::shutdown() { crate::log_error!("CPU_POWER", "CPU power restoration failed: {}", e); }
    crate::hal::ptl_pmc::shutdown();
    println!("[OpenRevo] Tray quit requested. Safely shutting down water cooler, handing over logo & keyboard to MCU...");
    crate::driver::ble_cooler::get_global_cooler().stop_and_shutdown();
    crate::driver::ite_lighting::IteLightingManager::handover_logo_on_exit();
    crate::driver::ite_streamer::get_global_streamer().handover_to_hardware_on_exit();
    crate::driver::bld_driver_mgr::shutdown_bld_driver_on_exit();
}

/// Constructs the full tray menu with power modes, fan boost, OEM toggle, and notice items.
pub fn build_tray_menu(
    app: &AppHandle,
    ec: &Arc<Mutex<EcBridge>>,
) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let is_expanded = crate::system::window::ensure_window_alive(app)
        .map(|w| crate::system::window::is_window_expanded(&w))
        .unwrap_or(false);

    let full_panel_item = CheckMenuItem::with_id(app, "mode_full_panel", "🖥️ 完整面板", true, is_expanded, None::<&str>)?;
    let mini_drawer_item = CheckMenuItem::with_id(app, "mode_mini_drawer", "🪟 迷你抽屉", true, !is_expanded, None::<&str>)?;
    let turn_off_screen_item = MenuItem::with_id(app, "turn_off_screen", "🌙 关闭屏幕 (3秒防误触)", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;

    let initial_mode = ec.lock().get_power_mode();
    let initial_fan = ec.lock().is_fan_boost();

    let office_item = CheckMenuItem::with_id(app, "mode_office", "办公模式 (Office)", true, initial_mode == 1, None::<&str>)?;
    let balance_item = CheckMenuItem::with_id(app, "mode_balance", "均衡模式 (Balance)", true, initial_mode == 2, None::<&str>)?;
    let beast_item = CheckMenuItem::with_id(app, "mode_beast", "狂暴模式 (Beast)", true, initial_mode == 3, None::<&str>)?;
    let custom_item = CheckMenuItem::with_id(app, "mode_custom", "极客调校 (Custom)", true, initial_mode == 4, None::<&str>)?;

    let sep2 = PredefinedMenuItem::separator(app)?;

    let fan_boost_item = CheckMenuItem::with_id(app, "toggle_fan", "一键强冷 (Fan Boost)", true, initial_fan, None::<&str>)?;
    let oem_toggle_item = MenuItem::with_id(app, "takeover_toggle", "🔄 接管/恢复 官方后台服务", true, None::<&str>)?;

    let sep3 = PredefinedMenuItem::separator(app)?;

    let notice_item = MenuItem::with_id(app, "notice", &format!("🛡️ {}", app_version_display()), false, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 OpenRevo", true, None::<&str>)?;

    let _ = TRAY_MENU_CONTROLS.set(TrayMenuControls {
        full_panel: full_panel_item.clone(),
        mini_drawer: mini_drawer_item.clone(),
        office: office_item.clone(),
        balance: balance_item.clone(),
        beast: beast_item.clone(),
        custom: custom_item.clone(),
        fan_boost: fan_boost_item.clone(),
    });

    Menu::with_items(
        app,
        &[
            &full_panel_item,
            &mini_drawer_item,
            &turn_off_screen_item,
            &sep1,
            &office_item,
            &balance_item,
            &beast_item,
            &custom_item,
            &sep2,
            &fan_boost_item,
            &oem_toggle_item,
            &sep3,
            &notice_item,
            &quit_item,
        ],
    )
}

/// Centralized event handler for tray menu item clicks.
pub fn handle_tray_menu_event(
    app_handle: &AppHandle,
    event_id: &str,
    ec_clone: &Arc<Mutex<EcBridge>>,
) {
    let native_mode = match event_id { "mode_office" => Some(1), "mode_balance" => Some(2), "mode_beast" => Some(3), "mode_custom" => Some(4), _ => None };
    if let Some(mode) = native_mode.filter(|_| crate::commands::tuning_cmd::uses_native_cpu_power()) {
        let hw = match mode { 1 => crate::hal::HardwarePowerMode::Office, 2 => crate::hal::HardwarePowerMode::Balance, 3 => crate::hal::HardwarePowerMode::Turbo, _ => crate::hal::HardwarePowerMode::Custom };
        if let Err(e) = crate::hal::get_hardware_driver().set_performance_mode(hw) {
            crate::log_error!("CPU_POWER", "Tray mode request failed: {}", e);
            update_tray_power_mode(ec_clone.lock().get_power_mode());
            return;
        }
        ec_clone.lock().set_power_mode_from_hardware(mode);
        let mut cfg = crate::system::config::load_config();
        cfg.power_mode = mode;
        if crate::commands::power_cmd::is_dc_round_plugged_in() { cfg.power_mode_ac = mode; }
        else { cfg.power_mode_battery = mode; }
        crate::system::config::save_config(&cfg);
        update_tray_power_mode(mode);
        let _ = app_handle.emit("physical-mode-switched", mode);
        crate::system::osd_window::show_power_mode(mode);
        return;
    }
    match event_id {
        "mode_full_panel" => {
            crate::log_info!("TRAY", "Menu 'mode_full_panel' (完整面板) clicked");
            crate::system::window::switch_to_full_panel(app_handle);
        }
        "mode_mini_drawer" => {
            crate::log_info!("TRAY", "Menu 'mode_mini_drawer' (迷你抽屉) clicked");
            crate::system::window::switch_to_mini_drawer(app_handle);
        }
        "turn_off_screen" => {
            crate::log_info!("TRAY", "Menu 'turn_off_screen' (关闭屏幕) clicked");
            let _ = crate::system::switches::turn_off_screen();
        }
        "mode_office" => {
            ec_clone.lock().set_power_mode(1);
            let mut cfg = crate::system::config::load_config();
            let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
            cfg.power_mode = 1;
            if is_dc {
                cfg.power_mode_ac = 1;
            } else {
                cfg.power_mode_battery = 1;
            }
            crate::system::config::save_config(&cfg);
            update_tray_power_mode(1);
            crate::system::osd_window::wake_osd_if_enabled(&app_handle);
            let _ = app_handle.emit("physical-mode-switched", 1);
            let _ = app_handle.emit_to("main", "physical-mode-switched", 1);
            let _ = app_handle.emit_to("osd", "physical-mode-switched", 1);
        }
        "mode_balance" => {
            ec_clone.lock().set_power_mode(2);
            let mut cfg = crate::system::config::load_config();
            let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
            cfg.power_mode = 2;
            if is_dc {
                cfg.power_mode_ac = 2;
            } else {
                cfg.power_mode_battery = 2;
            }
            crate::system::config::save_config(&cfg);
            update_tray_power_mode(2);
            crate::system::osd_window::wake_osd_if_enabled(&app_handle);
            let _ = app_handle.emit("physical-mode-switched", 2);
            let _ = app_handle.emit_to("main", "physical-mode-switched", 2);
            let _ = app_handle.emit_to("osd", "physical-mode-switched", 2);
        }
        "mode_beast" => {
            let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
            let mut cfg = crate::system::config::load_config();
            if !is_dc && !cfg.unlock_battery_power_limit {
                crate::log_warn!("TRAY", "Blocked Turbo mode from tray: AC/DC power is not connected!");
                update_tray_power_mode(ec_clone.lock().get_power_mode());
            } else {
                ec_clone.lock().set_power_mode(3);
                cfg.power_mode = 3;
                if is_dc {
                    cfg.power_mode_ac = 3;
                } else {
                    cfg.power_mode_battery = 3;
                }
                crate::system::config::save_config(&cfg);
                update_tray_power_mode(3);
                crate::system::osd_window::wake_osd_if_enabled(&app_handle);
                let _ = app_handle.emit("physical-mode-switched", 3);
                let _ = app_handle.emit_to("main", "physical-mode-switched", 3);
                let _ = app_handle.emit_to("osd", "physical-mode-switched", 3);
            }
        }
        "mode_custom" => {
            let is_dc = crate::commands::hardware_cmd::is_dc_round_plugged_in();
            let mut cfg = crate::system::config::load_config();
            if !is_dc && !cfg.unlock_battery_power_limit {
                crate::log_warn!("TRAY", "Blocked Custom mode from tray: AC/DC power is not connected!");
                update_tray_power_mode(ec_clone.lock().get_power_mode());
            } else {
                ec_clone.lock().set_power_mode(4);
                cfg.power_mode = 4;
                if is_dc {
                    cfg.power_mode_ac = 4;
                } else {
                    cfg.power_mode_battery = 4;
                }
                crate::system::config::save_config(&cfg);
                update_tray_power_mode(4);
                crate::system::osd_window::wake_osd_if_enabled(&app_handle);
                let _ = app_handle.emit("physical-mode-switched", 4);
                let _ = app_handle.emit_to("main", "physical-mode-switched", 4);
                let _ = app_handle.emit_to("osd", "physical-mode-switched", 4);
            }
        }
        "toggle_fan" => {
            let ec = ec_clone.lock();
            let current = ec.is_fan_boost();
            let next = !current;
            ec.set_fan_boost(next);
            let mut cfg = crate::system::config::load_config();
            cfg.fan_boost = next;
            crate::system::config::save_config(&cfg);
            update_tray_fan_boost(next);
            crate::system::osd_window::wake_osd_if_enabled(&app_handle);
            let _ = app_handle.emit("fan-boost-changed", next);
            let _ = app_handle.emit_to("main", "fan-boost-changed", next);
            let _ = app_handle.emit_to("osd", "fan-boost-changed", next);
            let _ = app_handle.emit("physical-fan-boost-switched", next);
            let _ = app_handle.emit_to("main", "physical-fan-boost-switched", next);
            let _ = app_handle.emit_to("osd", "physical-fan-boost-switched", next);
        }
        "takeover_toggle" => {
            let is_running = crate::system::process::is_oem_service_running();
            if is_running {
                let _ = crate::system::service_mgr::pause_oem_services();
            } else {
                let _ = crate::system::service_mgr::resume_oem_services();
            }
            let next_state = !is_running;
            let _ = app_handle.emit("oem-service-status-changed", next_state);
            let _ = app_handle.emit_to("main", "oem-service-status-changed", next_state);
        }
        "quit" => {
            crate::system::webview_orchestrator::WebviewOrchestrator::global().safe_shutdown_all_subviews();
            safe_hardware_shutdown_on_exit();
            app_handle.exit(0);
        }
        _ => {}
    }
}

/// Handles tray icon mouse events (left click and double click to restore window).
pub fn handle_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    crate::log_info!("TRAY", "TrayIconEvent: {:?}", event);
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            ..
        }
        | TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => {
            let app = tray.app_handle();
            if let Some(window) = crate::system::window::ensure_window_alive(app) {
                crate::log_info!(
                    "TRAY",
                    "Left click triggered. Current state: is_visible={:?}, is_minimized={:?}",
                    window.is_visible(),
                    window.is_minimized()
                );
                crate::system::window::restore_and_sync_window(&window);
            } else {
                crate::log_error!("TRAY", "CRITICAL: Unable to obtain or resurrect 'main' WebviewWindow on tray click!");
            }
        }
        _ => {}
    }
}

/// Builds and registers the global system tray icon on startup.
pub fn build_tray(
    app: &AppHandle,
    ec_for_tray: Arc<Mutex<EcBridge>>,
) -> Result<TrayIcon, tauri::Error> {
    let tray_menu = build_tray_menu(app, &ec_for_tray)?;
    let ec_clone = Arc::clone(&ec_for_tray);

    let tray_builder = TrayIconBuilder::with_id("open-revo-tray")
        .menu(&tray_menu)
        .show_menu_on_left_click(false);

    #[cfg(target_os = "windows")]
    let tray_builder = if let Some(icon) = app.default_window_icon() {
        tray_builder.icon(icon.clone())
    } else {
        tray_builder
    };

    let tray = tray_builder
        .on_menu_event(move |app_handle, event| {
            handle_tray_menu_event(app_handle, event.id.as_ref(), &ec_clone);
        })
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app)?;

    let _ = GLOBAL_TRAY.set(tray.clone());
    Ok(tray)
}
