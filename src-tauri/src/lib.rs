#![recursion_limit = "256"]

use parking_lot::Mutex;
use std::sync::Arc;
use tauri::Manager;

pub mod commands;
pub mod driver;
pub mod hal;
pub mod system;
pub mod updater;

pub use openrevo_core;

use crate::driver::ec_bridge::{EcBridge, SharedEcBridge};
use crate::driver::gpu_bridge::SharedGpuBridge;

// Re-export common tray, window, and logger utilities for 100% backward compatibility
pub use crate::system::logger::{is_cli_log_active, set_cli_log_active, CLI_LOG_MODE};
pub use crate::system::tray::{
    update_tray_autostart_state, update_tray_fan_boost, update_tray_log_state,
    update_tray_power_mode, TrayKeeper, TrayMenuControls,
};
pub use crate::system::window::{ensure_window_alive, restore_and_sync_window};

struct TauriEventSink {
    app: tauri::AppHandle,
}

impl TauriEventSink {
    fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl openrevo_core::CoreEventSink for TauriEventSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        use tauri::Emitter;
        let _ = self.app.emit(event, payload.clone());
        crate::system::native_wry::broadcast_event(event, &payload);

        match event {
            "win-lock-state-changed" => {
                let locked = payload.as_bool().unwrap_or(false);
                crate::system::osd_window::show_key_lock_toast("win", locked);
            }
            "fn-lock-state-changed" => {
                let locked = payload.as_bool().unwrap_or(false);
                crate::system::osd_window::show_key_lock_toast("fn", locked);
            }
            _ => {}
        }
    }

    fn on_osd_notify(&self, feature: &str, value: &str) {
        crate::log_info!("OSD", "on_osd_notify received: feature={}, value={}", feature, value);
        use tauri::Emitter;
        let payload = serde_json::json!({
            "feature": feature,
            "value": value
        });
        let _ = self.app.emit("osd-notify", payload.clone());
        crate::system::native_wry::broadcast_event("osd-notify", &payload);
    }

    fn on_power_mode_changed(&self, mode: u8) {
        crate::log_info!("OSD", "on_power_mode_changed: mode={}", mode);
        crate::update_tray_power_mode(mode);
        crate::system::osd_window::show_power_mode(mode);
        crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
            "action": "sync_state",
            "power_mode": mode
        }));
        use tauri::Emitter;
        let _ = self.app.emit("physical-mode-switched", mode);
        let _ = self.app.emit_to("main", "physical-mode-switched", mode);
        crate::system::native_wry::broadcast_event("power-mode-changed", &serde_json::json!({ "mode": mode }));
        crate::system::native_wry::broadcast_event("physical-mode-switched", &serde_json::json!(mode));
    }

    fn on_fan_boost_changed(&self, enabled: bool) {
        crate::log_info!("OSD", "on_fan_boost_changed: enabled={}", enabled);
        crate::update_tray_fan_boost(enabled);
        crate::system::osd_window::show_fan_boost(enabled);
        crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
            "action": "sync_state",
            "fan_boost": enabled
        }));
        use tauri::Emitter;
        let _ = self.app.emit("fan-boost-changed", enabled);
        let _ = self.app.emit_to("main", "fan-boost-changed", enabled);
        let _ = self.app.emit("physical-fan-boost-switched", enabled);
        let _ = self.app.emit_to("main", "physical-fan-boost-switched", enabled);
        crate::system::native_wry::broadcast_event("fan-boost-changed", &serde_json::json!(enabled));
        crate::system::native_wry::broadcast_event("physical-fan-boost-switched", &serde_json::json!(enabled));
    }
}

struct SlintDaemonEventSink;

impl openrevo_core::CoreEventSink for SlintDaemonEventSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        crate::system::native_wry::broadcast_event(event, &payload);
        match event {
            "win-lock-state-changed" => {
                let locked = payload.as_bool().unwrap_or(false);
                crate::system::osd_window::show_key_lock_toast("win", locked);
            }
            "fn-lock-state-changed" => {
                let locked = payload.as_bool().unwrap_or(false);
                crate::system::osd_window::show_key_lock_toast("fn", locked);
            }
            _ => {}
        }
    }

    fn on_osd_notify(&self, feature: &str, value: &str) {
        crate::log_info!("OSD", "on_osd_notify (daemon): feature={}, value={}", feature, value);
        crate::system::native_wry::broadcast_event("osd-notify", &serde_json::json!({ "feature": feature, "value": value }));
    }

    fn on_power_mode_changed(&self, mode: u8) {
        crate::log_info!("OSD", "on_power_mode_changed (daemon): mode={}", mode);
        crate::system::osd_window::show_power_mode(mode);
        crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
            "action": "sync_state",
            "power_mode": mode
        }));
        crate::system::native_wry::broadcast_event("power-mode-changed", &serde_json::json!({ "mode": mode }));
        crate::system::native_wry::broadcast_event("physical-mode-switched", &serde_json::json!(mode));
    }

    fn on_fan_boost_changed(&self, enabled: bool) {
        crate::log_info!("OSD", "on_fan_boost_changed (daemon): enabled={}", enabled);
        crate::system::osd_window::show_fan_boost(enabled);
        crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
            "action": "sync_state",
            "fan_boost": enabled
        }));
        crate::system::native_wry::broadcast_event("fan-boost-changed", &serde_json::json!(enabled));
        crate::system::native_wry::broadcast_event("physical-fan-boost-switched", &serde_json::json!(enabled));
    }
}

pub fn run() {
    std::env::set_var("WEBVIEW2_DEFAULT_BACKGROUND_COLOR", "0");
    let cfg = crate::system::config::load_config();
    if cfg.use_classic_webview {
        run_tauri_runtime();
    } else {
        run_slint_pure_daemon();
    }
}

fn run_slint_pure_daemon() {
    // 0. Ensure Host process is Per-Monitor DPI Aware V2 so Win32 coordinate queries (GetClientRect) match Slint physical resolution
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "user32")]
        extern "system" {
            fn SetProcessDpiAwarenessContext(value: isize) -> i32;
        }
        const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // 0. Single Instance Lock (Native Win32 Mutex)
    unsafe {
        use windows::Win32::System::Threading::CreateMutexW;
        use windows::Win32::Foundation::GetLastError;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};
        use windows::core::w;

        let _mutex = CreateMutexW(None, true, w!("Global\\OpenRevo_Core_SingleInstance_Mutex"));
        if GetLastError() == windows::Win32::Foundation::ERROR_ALREADY_EXISTS {
            crate::log_info!("SINGLE_INST", "Another instance running. Sending wake message to existing instance...");
            if let Ok(hwnd) = FindWindowW(w!("OpenRevo_Tray_WndClass"), None) {
                if !hwnd.0.is_null() {
                    let _ = PostMessageW(
                        hwnd,
                        crate::system::win32_tray::WM_TRAY_WAKE_INSTANCE,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
            return;
        }
    }

    // 1. Install unconditional emergency panic hook immediately on process startup
    crate::system::logger::install_panic_hook();
    crate::system::boot_trace::init_boot_trace();
    crate::system::boot_trace::boot_log("BOOT_START", "OpenRevo Slint Pure Daemon engine starting up (0MB WebView2/Tao runtime)...");
    crate::system::logger::init_from_cli();

    // 2. Hardware Capability Snapshot Cache Warmup
    std::thread::spawn(|| {
        if crate::hal::platform_probe::is_ultrabook() {
            return;
        }
        let _ = crate::system::hardware_probe::get_hardware_snapshot_cached();
        let _ = crate::driver::gpu_bridge::query_nvidia_power_limits_static();
        let _ = crate::system::power_mode::refresh_factory_power_presets(true);
    });

    // 3. ACPI Kernel Driver check
    if !crate::hal::platform_probe::needs_acpi_kernel_driver() {
        crate::system::boot_trace::boot_log("DRIVER_OK", "Platform uses native WMI/driverless mailbox channel.");
    } else {
        let _ = crate::system::service_mgr::ensure_acpi_driver_ready();
    }

    // 4. Initialize Hardware Bridges
    let ec_instance = crate::driver::ec_bridge::get_global_ec();
    let gpu_instance = crate::driver::gpu_bridge::get_global_gpu();
    let _lighting_instance = crate::driver::ite_lighting::get_global_lighting();

    let ec_for_tray = Arc::clone(&ec_instance);
    let ec_for_restore = Arc::clone(&ec_instance);
    let ec_for_wmi = Arc::clone(&ec_instance);
    let ec_for_watcher = Arc::clone(&ec_instance);
    let ec_for_power_listener = Arc::clone(&ec_instance);
    let ec_for_telemetry = Arc::clone(&ec_instance);
    let gpu_for_telemetry = Arc::clone(&gpu_instance);

    // 5. Global Event Sink
    openrevo_core::set_global_event_sink(Arc::new(SlintDaemonEventSink));

    // 6. Keyboard Hooks (Win Lock, Caps/Num Lock OSD)
    crate::system::hooks::init_hooks(None);

    // 7. Power Listener (Sleep, Resume, Lid Close)
    crate::system::power_listener::init_power_event_watcher(ec_for_power_listener);

    // 8. WMI Physical ACPI Key Listener (Q-Key, Fn+1)
    if !crate::hal::platform_probe::is_ultrabook() && crate::hal::platform_probe::has_acpi_physical_key_events() {
        crate::driver::wmi_event_listener::start_wmi_event_listener(None, ec_for_wmi);
    }

    // 9. Touchpad Hardware Listener
    crate::system::device_switches::start_touchpad_listener(None);

    // 10. Hardware State Watcher (Bluetooth, Wi-Fi, Camera)
    crate::system::device_switches::start_hardware_state_watcher(None, ec_for_watcher);

    // 11. Display Tuning & Color Calibration
    crate::system::display_tuning::init_display_tuning();
    crate::system::color_calibration::init_color_calibration();

    // 12. Hardware State Restore
    crate::system::power::spawn_hardware_state_restore(ec_for_restore);

    // 13. Telemetry Loop
    crate::system::telemetry::start_telemetry_loop(None, ec_for_telemetry, gpu_for_telemetry);

    // 14. Watcher Hub & Enabled Plugins Bootstrap
    crate::commands::system_cmd::bootstrap_enabled_plugins();

    // 15. Autostart Alignment
    crate::system::autostart::sync_autostart_target_path();

    // 16. Check Launch Visibility
    let args: Vec<String> = std::env::args().collect();
    let cli_visible = args.iter().any(|a| a == "--visible" || a == "--show");
    let cfg = crate::system::config::load_config();
    if cli_visible || cfg.launch_visible {
        crate::system::slint_window::show_slint_window();
    } else {
        crate::log_info!("BOOT", "Zero-footprint standby: Slint main companion will spawn on demand upon tray click.");
    }

    // Asynchronously trim cold working set memory after startup stabilization
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(2500));
        crate::system::process::trim_process_working_set();
    });

    // 16. Enter Pure Win32 Tray Loop (blocks main thread until exit)
    crate::system::win32_tray::run_tray_loop(ec_for_tray);
}

pub fn run_tauri_runtime() {
    // 0. Disable WebView2 background throttling & renderer suspension for HUD overlay and widgets
    #[cfg(windows)]
    {
        let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
        let disable_throttling_flags = "--disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding";
        if existing.is_empty() {
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", disable_throttling_flags);
        } else if !existing.contains("--disable-background-timer-throttling") {
            std::env::set_var(
                "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                format!("{} {}", existing, disable_throttling_flags),
            );
        }
    }

    // 1. Install unconditional emergency panic hook immediately on process startup
    crate::system::logger::install_panic_hook();
    crate::system::boot_trace::init_boot_trace();
    crate::system::boot_trace::boot_log("BOOT_START", "OpenRevo engine starting up...");
    crate::system::logger::init_from_cli();

    // Asynchronously warm up in-memory hardware capability snapshot cache & VBIOS power limits (zero disk persistence)
    std::thread::spawn(|| {
        if crate::hal::platform_probe::is_ultrabook() {
            crate::log_info!("MAIN", "[Hardware Probe] Ultrabook detected: skipping gaming dGPU limits & EC power preset probe.");
            return;
        }
        crate::log_info!("MAIN", "[Hardware Probe] Warming up in-memory hardware capability snapshot cache...");
        let _ = crate::system::hardware_probe::get_hardware_snapshot_cached();
        let _ = crate::driver::gpu_bridge::query_nvidia_power_limits_static();
        let _ = crate::system::power_mode::refresh_factory_power_presets(true);
    });

    // Step 1: Ensure ACPI kernel driver & OEM static capability registry
    crate::system::boot_trace::boot_scope("Step 1: ACPI Driver Readiness", || {
        if !crate::hal::platform_probe::needs_acpi_kernel_driver() {
            crate::system::boot_trace::boot_log("DRIVER_OK", "Platform uses native WMI/driverless mailbox channel. Bypassing UWACPIDriver.sys.");
        } else if crate::driver::wmi_ec::is_acpi_driver_available() {
            crate::system::boot_trace::boot_log("DRIVER_OK", "ACPI Driver (\\\\.\\ACPIDriver) is active and accessible (0ms fast path)");
            let _ = crate::system::service_mgr::ensure_acpi_driver_ready();
        } else {
            crate::system::boot_trace::boot_log("DRIVER_WARN", "ACPI Driver unavailable. Executing PnP self-healing (UWACPIDriver.sys)...");
            crate::log_warn!("MAIN", "ACPI Driver unavailable on startup. Executing PnP self-healing synchronously to restore physical driver...");
            let _ = crate::system::service_mgr::ensure_acpi_driver_ready();
        }
    });

    // Step 2: Initialize EC driver bridge
    let ec_instance: SharedEcBridge = crate::system::boot_trace::boot_scope("Step 2: EcBridge Auto-load", || {
        Arc::new(Mutex::new(EcBridge::auto_load()))
    });

    // Step 3: Initialize GPU telemetry bridge
    let gpu_instance: SharedGpuBridge = crate::system::boot_trace::boot_scope("Step 3: GpuBridge Telemetry", || {
        crate::driver::gpu_bridge::get_global_gpu()
    });

    // Step 4: Initialize ITE lighting manager
    let lighting_instance: crate::driver::ite_lighting::SharedIteLightingManager =
        crate::system::boot_trace::boot_scope("Step 4: IteLightingManager", || {
            crate::driver::ite_lighting::get_global_lighting()
        });

    let ec_for_manage = Arc::clone(&ec_instance);
    let ec_for_tray = Arc::clone(&ec_instance);
    let ec_for_restore = Arc::clone(&ec_instance);
    let ec_for_wmi = Arc::clone(&ec_instance);
    let ec_for_watcher = Arc::clone(&ec_instance);
    let ec_for_power_listener = Arc::clone(&ec_instance);
    let ec_for_telemetry = Arc::clone(&ec_instance);
    let gpu_for_telemetry = Arc::clone(&gpu_instance);

    let autostart_plugin = tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        Some(vec!["--minimized"]),
    );

    let single_instance_plugin = crate::system::boot_trace::boot_scope("Step 5: SingleInstance Plugin", || {
        tauri_plugin_single_instance::init(|app, argv, _cwd| {
            crate::system::boot_trace::boot_log("SINGLE_INST", &format!("Another instance attempted launch with args: {:?}", argv));
            let cfg = crate::system::config::load_config();
            if cfg.use_classic_webview {
                if let Some(window) = crate::system::window::ensure_window_alive(app) {
                    crate::system::window::restore_and_sync_window(&window);
                }
            } else {
                crate::system::slint_window::show_slint_window();
            }
        })
    });

    tauri::Builder::default()
        .manage(ec_for_manage)
        .manage(gpu_instance)
        .manage(lighting_instance)
        .plugin(autostart_plugin)
        .plugin(single_instance_plugin)
        .invoke_handler(tauri::generate_handler![
            commands::hardware_cmd::get_model_capabilities,
            commands::hardware_cmd::get_hardware_status,
            commands::hardware_cmd::set_power_mode,
            commands::hardware_cmd::set_fan_boost,
            commands::hardware_cmd::set_battery_limit,
            commands::hardware_cmd::switch_refresh_rate,
            commands::hardware_cmd::get_display_monitors,
            commands::hardware_cmd::set_display_monitor_refresh_rate,
            commands::hardware_cmd::set_auto_min_refresh_on_battery,
            commands::hardware_cmd::set_unlock_battery_power_limit,
            commands::hardware_cmd::get_display_brightness,
            commands::hardware_cmd::set_display_brightness,
            commands::hardware_cmd::get_miniled_support,
            commands::hardware_cmd::get_miniled_status,
            commands::hardware_cmd::set_miniled_status,
            commands::hardware_cmd::turn_off_screen,
            commands::hardware_cmd::set_win_key_locked,
            commands::hardware_cmd::resize_window,
            commands::hardware_cmd::fit_mini_window_height,
            commands::hardware_cmd::toggle_maximize_window,
            commands::hardware_cmd::get_wake_window_mode,
            commands::hardware_cmd::set_wake_window_mode,
            commands::hardware_cmd::get_mini_auto_hide,
            commands::hardware_cmd::set_mini_auto_hide,
            commands::hardware_cmd::toggle_oem_service,
            commands::hardware_cmd::set_autostart_enabled,
            commands::hardware_cmd::start_water_cooler,
            commands::hardware_cmd::get_water_cooler_status,
            commands::hardware_cmd::set_water_cooler_speed,
            commands::hardware_cmd::set_water_cooler_strategy,
            commands::hardware_cmd::set_water_cooler_pump_thresholds,
            commands::hardware_cmd::set_water_cooler_led,
            commands::hardware_cmd::reconnect_water_cooler,
            commands::hardware_cmd::pair_water_cooler,
            commands::hardware_cmd::fill_water_cooler,
            commands::hardware_cmd::reset_water_cooler,
            commands::hardware_cmd::stop_water_cooler,
            commands::hardware_cmd::get_water_cooler_enabled,
            commands::hardware_cmd::set_water_cooler_enabled,
            commands::hardware_cmd::scan_water_cooler_devices,
            commands::hardware_cmd::select_water_cooler,
            commands::hardware_cmd::get_module_toggles,
            commands::hardware_cmd::set_module_enabled,
            commands::hardware_cmd::get_lighting_status,
            commands::hardware_cmd::get_lighting_state,
            commands::hardware_cmd::get_lighting_runtime_status,
            commands::hardware_cmd::apply_lighting_hardware_override,
            commands::hardware_cmd::get_lighting_hardware_override,
            commands::hardware_cmd::get_keyboard_hardware_info,
            commands::hardware_cmd::apply_keyboard_lighting,
            commands::hardware_cmd::apply_lightbar_lighting,
            commands::hardware_cmd::apply_hinge_lighting,
            commands::hardware_cmd::apply_four_zone_colors,
            commands::hardware_cmd::apply_logo_lighting,
            commands::hardware_cmd::set_lighting_sleep_timer,
            commands::hardware_cmd::trigger_lighting_welcome,
            commands::hardware_cmd::set_ceremony_enabled,
            commands::hardware_cmd::save_current_lighting_as_startup,
            commands::hardware_cmd::get_custom_brfx_list,
            commands::hardware_cmd::save_custom_brfx,
            commands::hardware_cmd::open_brfx_in_notepad,
            commands::hardware_cmd::apply_custom_brfx_script,
            commands::hardware_cmd::clear_custom_brfx_script,
            commands::hardware_cmd::set_custom_fx_text,
            commands::hardware_cmd::get_custom_fx_text,
            commands::hardware_cmd::get_keyboard_sleep_timeout,
            commands::hardware_cmd::set_keyboard_sleep_timeout,
            commands::hardware_cmd::get_app_config,
            commands::hardware_cmd::save_app_config,
            commands::hardware_cmd::get_classic_webview,
            commands::hardware_cmd::set_classic_webview,
            commands::system_cmd::get_update_channel,
            commands::system_cmd::set_update_channel,
            commands::hardware_cmd::restart_app,
            commands::hardware_cmd::get_device_switches,
            commands::hardware_cmd::set_device_switch,
            commands::hardware_cmd::get_gpu_mode_info,
            commands::hardware_cmd::set_gpu_mode,
            commands::hardware_cmd::hot_switch_gpu_mode,
            commands::hardware_cmd::set_experimental_igpu_unlock,
            commands::hardware_cmd::get_cpu_freq_info,
            commands::hardware_cmd::set_cpu_freq_limit,
            commands::hardware_cmd::reset_cpu_freq_limit,
            commands::hardware_cmd::set_cpu_safety_guard,
            commands::hardware_cmd::get_power_settings,
            commands::hardware_cmd::set_battery_mode,
            commands::hardware_cmd::set_battery_hardware_limit,
            commands::hardware_cmd::set_power_option,
            commands::hardware_cmd::set_sleep_guard_option,
            commands::hardware_cmd::set_master_sleep_guard,
            commands::hardware_cmd::set_sleep_auto_office,
            commands::hardware_cmd::get_custom_profile,
            commands::hardware_cmd::reset_custom_profile_to_defaults,
            commands::hardware_cmd::apply_custom_profile,
            commands::hardware_cmd::apply_fan_curve_live,
            commands::hardware_cmd::toggle_fan_isolated_output,
            commands::hardware_cmd::toggle_fan_ramp_rate,
            commands::hardware_cmd::toggle_fan_curve_control,
            commands::hardware_cmd::get_profiles_list,
            commands::hardware_cmd::get_default_builtin_presets,
            commands::hardware_cmd::get_active_profile_id,
            commands::hardware_cmd::set_active_profile_id,
            commands::hardware_cmd::save_profile_preset,
            commands::hardware_cmd::create_profile_preset,
            commands::hardware_cmd::reset_profile_preset,
            commands::hardware_cmd::heal_gpu_power_contract,
            commands::hardware_cmd::heal_cpu_power_contract,
            commands::hardware_cmd::rename_profile_preset,
            commands::hardware_cmd::delete_profile_preset,
            commands::hardware_cmd::open_profiles_folder,
            commands::hardware_cmd::import_profile_preset,
            commands::hardware_cmd::export_profile_preset,
            commands::hardware_cmd::apply_live_custom_tweak,
            commands::hardware_cmd::set_bld_fan_speed_cap,
            commands::hardware_cmd::get_gpu_tuning_capabilities,
            commands::hardware_cmd::get_bios_advanced_menu_status,
            commands::hardware_cmd::toggle_bios_advanced_menu,
            commands::hardware_cmd::reboot_computer,
            commands::hardware_cmd::get_log_status,
            commands::hardware_cmd::set_log_level,
            commands::hardware_cmd::set_log_filter,
            commands::hardware_cmd::open_log_file,
            commands::hardware_cmd::set_keyboard_engine,
            commands::hardware_cmd::set_streamer_fps,
            commands::hardware_cmd::set_auto_fallback_kb_on_battery,
            commands::hardware_cmd::get_auto_fallback_kb_on_battery,
            commands::hardware_cmd::get_model_profile_info,
            commands::hardware_cmd::get_models_file_path,
            commands::hardware_cmd::reload_model_profiles,
            commands::hardware_cmd::open_models_in_notepad,
            commands::hardware_cmd::export_models_json,
            commands::hardware_cmd::import_models_json,
            commands::hardware_cmd::open_user_data_dir,
            commands::hardware_cmd::clear_user_data_dir,
            commands::hardware_cmd::get_user_data_dir_info,
            commands::hardware_cmd::migrate_user_data_dir,
            updater::check_for_updates_cmd,
            updater::download_and_install_update_cmd,
            commands::hardware_cmd::open_plugins_dir,
            commands::hardware_cmd::get_custom_plugins,
            commands::hardware_cmd::set_custom_plugin_enabled,
            commands::hardware_cmd::start_plugin_service,
            commands::hardware_cmd::stop_plugin_service,
            commands::hardware_cmd::get_plugin_service_status,
            commands::hardware_cmd::get_plugin_host_capabilities,
            commands::hardware_cmd::open_external_url,
            commands::hardware_cmd::write_clipboard,
            commands::hardware_cmd::reload_plugins,
            commands::hardware_cmd::get_plugin_manifest,
            commands::hardware_cmd::set_plugin_enabled_unified,
            commands::hardware_cmd::uninstall_plugin,
            commands::hardware_cmd::install_plugin_from_url,
            commands::system_cmd::register_telemetry_consumer,
            commands::system_cmd::unregister_telemetry_consumer,
            commands::system_cmd::emit_plugin_broadcast,
            commands::system_cmd::load_plugin_theme_css,
            commands::system_cmd::set_active_skin,
            commands::system_cmd::get_active_skin,
            commands::system_cmd::load_plugin_background_css,
            commands::system_cmd::set_active_background,
            commands::system_cmd::get_active_background,
            commands::system_cmd::load_plugin_html,
            commands::hardware_cmd::get_windows_power_schemes,
            commands::hardware_cmd::set_active_windows_power_scheme,
            commands::hardware_cmd::restore_official_power_plan,
            crate::system::osd_window::get_osd_config,
            crate::system::osd_window::set_osd_config,
            crate::system::osd_window::save_osd_config,
            crate::system::osd_window::trigger_osd_preview,
            crate::system::osd_window::show_osd_window,
            crate::system::osd_window::hide_osd_window,
            crate::system::boot_trace::get_boot_log,
            crate::system::boot_trace::frontend_ready,
            commands::hardware_cmd::get_display_tuning_state,
            commands::hardware_cmd::set_display_tuning_enabled,
            commands::hardware_cmd::apply_display_color_preset,
            commands::hardware_cmd::import_color_preset_content,
            commands::hardware_cmd::reset_display_tuning,
            commands::hardware_cmd::reset_color_preset_to_default,
            commands::hardware_cmd::get_color_calibration_state,
            commands::hardware_cmd::apply_color_profile,
            commands::hardware_cmd::set_color_calibration_enabled,
            commands::hardware_cmd::import_custom_color_profile,
            commands::hardware_cmd::delete_custom_color_profile,
            commands::hardware_cmd::get_cloud_icc_identity,
            commands::hardware_cmd::fetch_cloud_icc_profile,
            commands::hardware_cmd::export_device_diagnostic_report,
            commands::hardware_cmd::apply_gpu_tuning_state,
            commands::hardware_cmd::get_gpu_power_state_truth,
            commands::hardware_cmd::emergency_hard_unlock_gpu,
            crate::system::telemetry::report_window_visibility,
            crate::system::telemetry::get_telemetry_heartbeat_status,
            crate::system::widget_manager::open_custom_widget,
            crate::system::widget_manager::close_custom_widget,
            crate::system::widget_manager::get_running_widgets,
            crate::system::widget_manager::widget_ready,
            commands::shortcut_cmd::get_registered_shortcuts,
            commands::shortcut_cmd::check_hotkey_conflict,
            commands::shortcut_cmd::register_custom_shortcut,
            commands::shortcut_cmd::unregister_custom_shortcut,
            commands::shortcut_cmd::trigger_shortcut_action,
            commands::raw_ec_cmd::get_raw_ec_capabilities,
            commands::raw_ec_cmd::raw_ec_read,
            commands::raw_ec_cmd::raw_ec_write,
            commands::raw_ec_cmd::raw_mailbox_invoke,
            commands::watcher_cmd::get_registered_watchers,
            commands::watcher_cmd::register_custom_watcher,
            commands::watcher_cmd::unregister_custom_watcher,
            commands::watcher_cmd::trigger_watcher_manual,
            commands::plugin_storage_cmd::get_plugin_storage,
            commands::plugin_storage_cmd::set_plugin_storage,
            commands::plugin_storage_cmd::read_plugin_file,
            commands::plugin_storage_cmd::write_plugin_file,
            commands::plugin_storage_cmd::delete_plugin_file,
            commands::plugin_storage_cmd::list_plugin_files,
            commands::ai_cmd::ai_get_config,
            commands::ai_cmd::ai_save_config,
            commands::ai_cmd::ai_fetch_models,
            commands::ai_cmd::ai_chat_send,
            commands::ai_cmd::ai_get_system_skills,
            commands::tuning_cmd::get_gpu_tuning_info,
            commands::tuning_cmd::set_gpu_clock_offsets,
            commands::tuning_cmd::set_gpu_vf_curve_deltas,
            commands::tuning_cmd::set_gpu_undervolt_curve,
            commands::tuning_cmd::reset_gpu_vf_curve,
            commands::tuning_cmd::reset_gpu_tuning,
            commands::power_cmd::get_turbo_submode_state,
            commands::power_cmd::set_turbo_submode,
            commands::power_cmd::apply_turbo_profile,
            commands::tuning_cmd::apply_gpu_oc,
            commands::tuning_cmd::get_gpu_oc_state,
        ])
        .setup(move |app| {
            openrevo_core::set_global_event_sink(Arc::new(TauriEventSink::new(app.handle().clone())));
            crate::system::boot_trace::boot_log("SETUP", "Tauri setup closure entered");
            crate::system::actuator::UnifiedPowerActuator::global().bind_app_handle(app.handle().clone());
            crate::system::webview_orchestrator::WebviewOrchestrator::global().bind_app_handle(app.handle().clone());

            // Step 6.1: Initialize main WebviewWindow (positioning above taskbar)
            crate::system::boot_trace::boot_scope("Step 6.1: Main Window Positioning", || {
                let cfg = crate::system::config::load_config();
                if cfg.use_classic_webview {
                    if let Some(window) = app.get_webview_window("main").or_else(|| crate::system::window::create_main_webview_window(&app.handle())) {
                        crate::system::window::init_main_window(&window);
                    }
                } else {
                    crate::log_info!("WINDOW", "Slint native UI mode active; zero background WebView2 windows spawned on boot.");
                }
            });

            // Step 6.2: Initialize permanent keyboard hook (Caps Lock, Num Lock, Scroll Lock, Win Lock)
            crate::system::boot_trace::boot_scope("Step 6.2: Keyboard Hooks", || {
                crate::system::hooks::init_hooks(Some(&app.handle()));
            });

            // Step 6.3: Enqueue OSD overlay window into WebviewOrchestrator (active only after main window confirms ready)
            crate::system::boot_trace::boot_scope("Step 6.3: OSD Orchestrator Enqueue", || {
                crate::system::osd_window::init_osd_window(&app.handle());
            });

            // Step 6.3b: Initialize Slint native main companion if enabled
            crate::system::boot_trace::boot_scope("Step 6.3b: Slint Main Companion", || {
                if let Ok(mut g) = crate::system::slint_window::SLINT_APP_HANDLE.lock() {
                    *g = Some(app.handle().clone());
                }
                let args: Vec<String> = std::env::args().collect();
                let cli_visible = args.iter().any(|a| a == "--visible" || a == "--show");
                let cfg = crate::system::config::load_config();
                if !cfg.use_classic_webview && (cli_visible || cfg.launch_visible) {
                    crate::system::slint_window::show_slint_window();
                } else {
                    crate::log_info!("BOOT", "Zero-footprint standby: Slint main companion will spawn on demand upon tray click.");
                }
            });

            // Step 6.4: Start dedicated Windows power event watcher (Sleep, Hibernate, Lid Close, Shutdown)
            crate::system::boot_trace::boot_scope("Step 6.4: Power Listener", || {
                crate::system::power_listener::init_power_event_watcher(ec_for_power_listener);
            });

            // Step 6.5: Start dedicated WMI ACPI event watcher for physical Q-Key mode switch and Fan Boost
            crate::system::boot_trace::boot_scope("Step 6.5: WMI ACPI Listener", || {
                if crate::hal::platform_probe::is_ultrabook() || !crate::hal::platform_probe::has_acpi_physical_key_events() {
                    crate::system::boot_trace::boot_log("LISTENER_BYPASS", "Ultrabook detected: AcpiTest_EventULong companion listener bypassed.");
                    return;
                }
                crate::driver::wmi_event_listener::start_wmi_event_listener(Some(app.handle().clone()), ec_for_wmi);
            });

            // Step 6.6: Start dedicated Precision Touchpad registry listener for hardware Fn+F5 sync
            crate::system::boot_trace::boot_scope("Step 6.6: Touchpad Listener", || {
                crate::system::device_switches::start_touchpad_listener(Some(app.handle().clone()));
            });

            // Step 6.7: Start dedicated background watcher for external Bluetooth, Wi-Fi, and Camera state changes
            crate::system::boot_trace::boot_scope("Step 6.7: Hardware State Watcher", || {
                crate::system::device_switches::start_hardware_state_watcher(Some(app.handle().clone()), ec_for_watcher);
            });

            // Step 6.8: Initialize display tuning engine
            crate::system::boot_trace::boot_scope("Step 6.8: Display Tuning", || {
                crate::system::display_tuning::init_display_tuning();
                crate::system::color_calibration::init_color_calibration();
            });

            // Step 6.9: Restore user configuration and hardware states from last session immediately
            crate::system::power::spawn_hardware_state_restore(ec_for_restore);

            // Step 6.10: Build and mount system tray menu and listeners
            let tray = crate::system::tray::build_tray(&app.handle(), ec_for_tray)?;
            app.manage(crate::system::tray::TrayKeeper(tray));

            // Step 6.11: Start adaptive dynamic telemetry heartbeat broadcaster
            crate::system::boot_trace::boot_scope("Step 6.11: Telemetry Heartbeat", || {
                crate::system::telemetry::start_telemetry_loop(
                    Some(app.handle().clone()),
                    ec_for_telemetry,
                    gpu_for_telemetry,
                );
            });

            // Step 6.12: Enqueue enabled widget windows into WebviewOrchestrator for safe serial startup
            crate::system::boot_trace::boot_scope("Step 6.12: Widgets Orchestrator Enqueue", || {
                crate::system::widget_manager::restore_active_widgets_on_boot(Some(&app.handle()));
            });

            // Step 6.13: Verify & auto-heal autostart scheduled task to align with current executable
            crate::system::boot_trace::boot_scope("Step 6.13: Autostart Alignment", || {
                crate::system::autostart::sync_autostart_target_path();
            });

            // Step 6.14: Initialize Universal Watcher Hub and auto-load plugin watchers
            crate::system::boot_trace::boot_scope("Step 6.14: Watcher Hub Initialization", || {
                crate::system::watcher_hub::init_watcher_hub(app.handle().clone());
                crate::system::broadcast::bind_broadcast_app(app.handle().clone());
                crate::commands::system_cmd::bootstrap_enabled_plugins();
            });

            crate::system::boot_trace::boot_log("TRAY", "System tray mounted and listeners active");
            crate::system::boot_trace::record_stage0_ui_ready();

            Ok(())
        })
        .on_window_event(crate::system::window::handle_window_event)
        .build(tauri::generate_context!())
        .expect("error while building OpenRevo application")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !crate::system::tray::IS_EXPLICIT_EXIT.load(std::sync::atomic::Ordering::SeqCst) {
                    // Prevent app from quitting when no webview windows exist (system tray background daemon)
                    api.prevent_exit();
                }
            }
            tauri::RunEvent::Exit => {
                println!("[OpenRevo] Process exiting. Executing clean webview shutdown and hardware safety shutdown...");
                crate::system::native_wry::shutdown_all();
                crate::system::webview_orchestrator::WebviewOrchestrator::global().safe_shutdown_all_subviews();
                crate::system::tray::safe_hardware_shutdown_on_exit();
                crate::system::plugin_service::shutdown_all_plugin_services();
                std::process::exit(0);
            }
            _ => {}
        });
}
