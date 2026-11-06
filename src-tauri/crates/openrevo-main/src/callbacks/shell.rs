use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, ShowWindow, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
};

use crate::ipc;
use crate::window::{
    align_mini_drawer_to_tray, expand_native_window, SafeHwnd, SAVED_FULL_POS,
    WAS_MINIMIZED,
};
use crate::MainWindow;

pub fn register(
    window: &MainWindow,
    ui_weak: slint::Weak<MainWindow>,
    hwnd_holder: Arc<Mutex<Option<SafeHwnd>>>,
) {
    let ui_cb = ui_weak.clone();
    window.on_change_wake_window_mode(move |m| {
        ipc::send_action_to_host("set_wake_window_mode", serde_json::json!({ "mode": m.as_str() }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_wake_window_mode(m);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_mini_auto_hide(move |en| {
        ipc::send_action_to_host("set_mini_auto_hide", serde_json::json!({ "enabled": en }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_mini_auto_hide(en);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_toggle_classic_webview(move |en| {
        ipc::send_action_to_host("set_classic_webview", serde_json::json!({ "enabled": en }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_use_classic_webview(en);
        }
    });

    window.on_restart_openrevo(move || {
        ipc::send_action_to_host("restart_openrevo", serde_json::json!({}));
    });

    window.on_open_user_data_dir(move || {
        ipc::send_action_to_host("open_user_data_dir", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_confirm_clear_user_data(move || {
        ipc::send_action_to_host("clear_user_data_dir", serde_json::json!({}));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_show_clear_confirm(false);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_confirm_migrate_user_data(move |path| {
        ipc::send_action_to_host("migrate_user_data_dir", serde_json::json!({ "target_path": path.as_str() }));
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_show_migrate_modal(false);
        }
    });

    let ui_cb = ui_weak.clone();
    window.on_check_for_updates(move || {
        if let Some(ui) = ui_cb.upgrade() {
            ui.set_is_checking_update(true);
        }
        ipc::send_action_to_host("check_for_updates", serde_json::json!({}));
    });

    let ui_cb = ui_weak.clone();
    window.on_start_download_update(move || {
        if let Some(ui) = ui_cb.upgrade() {
            let mut d = ui.get_update_data();
            d.downloading = true;
            ui.set_update_data(d);
        }
        ipc::send_action_to_host("start_download_update", serde_json::json!({}));
    });

    let hwnd_expand = hwnd_holder.clone();
    let ui_expand = ui_weak.clone();
    window.on_expand_dashboard_clicked(move || {
        if let Some(h) = *hwnd_expand.lock().unwrap() {
            let saved = *SAVED_FULL_POS.lock().unwrap();
            expand_native_window(h.to_hwnd(), 1120, 760, saved);
        }
        if let Some(ui) = ui_expand.upgrade() {
            ui.set_is_expanded(true);
        }
        ipc::send_action_to_host("expand_dashboard", serde_json::json!({ "is_expanded": true }));
    });

    let hwnd_collapse = hwnd_holder.clone();
    let ui_collapse = ui_weak.clone();
    window.on_collapse_dashboard_clicked(move || {
        if let Some(h) = *hwnd_collapse.lock().unwrap() {
            align_mini_drawer_to_tray(h.to_hwnd(), 395, 632);
        }
        if let Some(ui) = ui_collapse.upgrade() {
            ui.set_is_expanded(false);
        }
        ipc::send_action_to_host("collapse_dashboard", serde_json::json!({ "is_expanded": false }));
    });

    let hwnd_fs = hwnd_holder.clone();
    let is_fullscreen = Arc::new(Mutex::new(false));
    let is_fs_clone = is_fullscreen.clone();
    window.on_toggle_fullscreen_clicked(move || {
        if let Some(h) = *hwnd_fs.lock().unwrap() {
            let mut fs = is_fs_clone.lock().unwrap();
            unsafe {
                if *fs {
                    let _ = ShowWindow(h.to_hwnd(), SW_RESTORE);
                    *fs = false;
                } else {
                    let _ = ShowWindow(h.to_hwnd(), SW_MAXIMIZE);
                    *fs = true;
                }
            }
        }
    });

    window.on_open_sponsor_clicked(move || {
        ipc::send_action_to_host("open_sponsor", serde_json::json!({}));
    });

    // Window controls
    let hwnd_min = hwnd_holder.clone();
    window.on_minimize_clicked(move || {
        if let Some(h) = *hwnd_min.lock().unwrap() {
            unsafe {
                WAS_MINIMIZED.store(true, Ordering::SeqCst);
                let _ = ShowWindow(h.to_hwnd(), SW_MINIMIZE);
            }
        }
    });

    let hwnd_close = hwnd_holder.clone();
    let ui_close = ui_weak.clone();
    window.on_close_clicked(move || {
        if let Some(h) = *hwnd_close.lock().unwrap() {
            unsafe {
                let hwnd = h.to_hwnd();
                if let Some(ui) = ui_close.upgrade() {
                    if ui.get_is_expanded() {
                        let mut rect = RECT::default();
                        if GetWindowRect(hwnd, &mut rect).is_ok() {
                            *SAVED_FULL_POS.lock().unwrap() = Some((rect.left, rect.top));
                            ipc::send_action_to_host("save_full_window_pos", serde_json::json!({ "x": rect.left, "y": rect.top }));
                        }
                    }
                }
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    });

    let ui_theme = ui_weak.clone();
    window.on_toggle_theme_clicked(move || {
        if let Some(ui) = ui_theme.upgrade() {
            let next = (ui.get_theme_mode() + 1) % 3;
            ui.set_theme_mode(next);
            ipc::send_action_to_host("set_theme_mode", serde_json::json!({ "theme_mode": next }));
        }
    });
}

