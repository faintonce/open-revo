use tauri::AppHandle;
use super::dialog::get_openrevo_main_hwnd;

pub const EMBEDDED_LABEL: &str = "embedded_plugin_host";

/// Compute the physical pixel coordinates of the plugin area within the Slint client space.
pub fn calc_embedded_geometry(parent_hwnd: windows::Win32::Foundation::HWND) -> (i32, i32, i32, i32, i32) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let mut rect = RECT::default();
    unsafe {
        let _ = GetClientRect(parent_hwnd, &mut rect);
    }
    let total_w = rect.right - rect.left;
    let total_h = rect.bottom - rect.top;

    if total_w <= 0 || total_h <= 0 {
        return (200, 79, 919, 680, 9);
    }

    // Slint full dashboard design base: 1120 x 760
    // Sidebar width: 200px
    // Header height: 40px + 1px divider + 38px plugin-action-strip = 79px total top offset
    let scale_x = total_w as f64 / 1120.0;
    let scale_y = total_h as f64 / 760.0;

    let target_x = (200.0 * scale_x).round() as i32;
    let target_y = (79.0 * scale_y).round() as i32;

    // Leave 1px margin on right and bottom so parent's border_subtle is not covered
    let target_w = (total_w - target_x - 1).max(100);
    let target_h = (total_h - target_y - 1).max(100);

    // Parent container outer border-radius is 10px; with 1px border inner radius is ~9px
    let corner_radius = (9.0 * scale_x).round() as i32;

    (target_x, target_y, target_w, target_h, corner_radius)
}

/// Applies a custom clipping region to the embedded child window so that:
/// - Top-left, Top-right, and Bottom-left remain sharp right angles matching interior seams.
/// - Bottom-right corner is smoothly rounded to fit the Slint container's rounded corner (10px).
/// - Completely strips away standard Win32 rectangular borders/shadows outside the region.
pub fn apply_child_rounded_region(child_hwnd: windows::Win32::Foundation::HWND, w: i32, h: i32, radius: i32) {
    use windows::Win32::Graphics::Gdi::{
        CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, RGN_OR,
    };

    if w <= 0 || h <= 0 {
        return;
    }

    unsafe {
        if radius <= 1 {
            let rgn = CreateRectRgn(0, 0, w, h);
            let _ = SetWindowRgn(child_hwnd, rgn, true);
            return;
        }

        // Region 1: Upper block (covers y from 0 to h - radius across full width w)
        let r1 = CreateRectRgn(0, 0, w, h - radius);

        // Region 2: Lower-left block (covers y from h - radius to h, x from 0 to w - radius)
        let r2 = CreateRectRgn(0, h - radius, w - radius, h);

        // Region 3: Round-corner arc for bottom-right corner [w - 2 * radius .. w, h - 2 * radius .. h]
        let r3 = CreateRoundRectRgn(w - 2 * radius, h - 2 * radius, w + 1, h + 1, radius * 2, radius * 2);

        let main_rgn = CreateRectRgn(0, 0, 0, 0);
        CombineRgn(main_rgn, r1, r2, RGN_OR);
        CombineRgn(main_rgn, main_rgn, r3, RGN_OR);

        let _ = DeleteObject(r1);
        let _ = DeleteObject(r2);
        let _ = DeleteObject(r3);

        // SetWindowRgn takes ownership of main_rgn handle
        let _ = SetWindowRgn(child_hwnd, main_rgn, true);
    }
}

pub fn attach_or_switch_embedded_webview(app: &AppHandle, plugin_id: &str) {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetParent, SetWindowLongW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, GWL_STYLE, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOW,
        WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_CONTROLPARENT, WS_VISIBLE,
    };

    crate::log_info!("EMBEDDED_WEBVIEW", "attach_or_switch_embedded_webview requested for plugin: '{}'", plugin_id);

    let parent_hwnd_raw = get_openrevo_main_hwnd();
    crate::log_info!("EMBEDDED_WEBVIEW", "Resolved Slint parent HWND: 0x{:X}", parent_hwnd_raw);

    if parent_hwnd_raw == 0 {
        crate::log_warn!("EMBEDDED_WEBVIEW", "Cannot locate Slint main window HWND (is 0)! Aborting dock.");
        return;
    }
    let parent_hwnd = HWND(parent_hwnd_raw as *mut _);
    let (target_x, target_y, target_w, target_h, corner_radius) = calc_embedded_geometry(parent_hwnd);
    crate::log_info!("EMBEDDED_WEBVIEW", "Geometry calculated: x={}, y={}, w={}, h={}, radius={}", target_x, target_y, target_w, target_h, corner_radius);

    // 1. Ensure parent Slint window has WS_CLIPCHILDREN so its background repainting doesn't overwrite WebView2!
    unsafe {
        let p_style = GetWindowLongW(parent_hwnd, GWL_STYLE) as u32;
        if (p_style & WS_CLIPCHILDREN.0) == 0 {
            let _ = SetWindowLongW(parent_hwnd, GWL_STYLE, (p_style | WS_CLIPCHILDREN.0) as i32);
            crate::log_info!("EMBEDDED_WEBVIEW", "Applied WS_CLIPCHILDREN to parent Slint window");
        }
    }

    // 2. If the embedded window already exists, switch hash and reposition
    if let Some(win) = app.get_webview_window(EMBEDDED_LABEL) {
        crate::log_info!("EMBEDDED_WEBVIEW", "Window '{}' already exists. Switching hash...", EMBEDDED_LABEL);
        let script = format!(
            "window.__OPENREVO_WIDGET_ID__ = '{0}'; window.location.hash = '#/widget/{0}'; window.dispatchEvent(new CustomEvent('openrevo-switch-widget', {{ detail: {{ pluginId: '{0}' }} }}));",
            plugin_id
        );
        let _ = win.eval(&script);
        let _ = win.set_size(tauri::PhysicalSize::new(target_w as u32, target_h as u32));
        if let Ok(child_hwnd_raw) = win.hwnd() {
            let child_hwnd = HWND(child_hwnd_raw.0 as *mut _);
            unsafe {
                let _ = ShowWindow(child_hwnd, SW_SHOW);
                let _ = SetWindowPos(
                    child_hwnd,
                    HWND_TOP,
                    target_x,
                    target_y,
                    target_w,
                    target_h,
                    SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                );
            }
            apply_child_rounded_region(child_hwnd, target_w, target_h, corner_radius);
        }
        let _ = win.show();
        crate::log_info!("EMBEDDED_WEBVIEW", "Existing window presented and navigated to plugin '{}'", plugin_id);
        return;
    }

    // 3. Create the child WebviewWindow lazily with clean index.html URL
    let target_url = WebviewUrl::App("index.html".into());
    let init_script = format!(
        "window.__OPENREVO_WIDGET_ID__ = '{0}'; window.location.hash = '#/widget/{0}'; \
        const s = document.createElement('style'); \
        s.textContent = 'html, body, #root {{ background: #0c1017 !important; background-color: #0c1017 !important; margin: 0; padding: 0; overflow: hidden; }}'; \
        if (document.head) {{ document.head.appendChild(s); }} else {{ document.addEventListener('DOMContentLoaded', () => document.head.appendChild(s)); }}",
        plugin_id
    );

    crate::log_info!("EMBEDDED_WEBVIEW", "Building new WebviewWindow for plugin '{}' into parent 0x{:X}", plugin_id, parent_hwnd_raw);

    let builder = WebviewWindowBuilder::new(app, EMBEDDED_LABEL, target_url)
        .title("OpenRevo Embedded Plugin")
        .inner_size(target_w as f64, target_h as f64)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .drag_and_drop(false)
        .initialization_script(&init_script)
        .visible(false);

    match builder.build() {
        Ok(win) => {
            crate::log_info!("EMBEDDED_WEBVIEW", "WebviewWindow built successfully, configuring Win32 child styles...");
            let _ = win.set_size(tauri::PhysicalSize::new(target_w as u32, target_h as u32));
            if let Ok(child_hwnd_raw) = win.hwnd() {
                let child_hwnd = HWND(child_hwnd_raw.0 as *mut _);
                crate::log_info!("EMBEDDED_WEBVIEW", "Configuring Child HWND: 0x{:X}", child_hwnd_raw.0 as usize);
                unsafe {
                    let style = GetWindowLongW(child_hwnd, GWL_STYLE) as u32;
                    let remove_mask = 0x80000000 | 0x00C00000 | 0x00040000 | 0x00020000 | 0x00010000 | 0x00800000 | 0x00400000;
                    let new_style = ((style & !remove_mask) | WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0) as i32;
                    let _ = SetWindowLongW(child_hwnd, GWL_STYLE, new_style);

                    let ex_style = GetWindowLongW(child_hwnd, GWL_EXSTYLE) as u32;
                    let remove_ex = 0x00040000 | 0x00000100 | 0x00000200 | 0x00020000 | 0x00000001;
                    let new_ex_style = ((ex_style & !remove_ex) | WS_EX_CONTROLPARENT.0) as i32;
                    let _ = SetWindowLongW(child_hwnd, GWL_EXSTYLE, new_ex_style);

                    let _ = SetParent(child_hwnd, parent_hwnd);
                    let _ = SetWindowPos(
                        child_hwnd,
                        HWND_TOP,
                        target_x,
                        target_y,
                        target_w,
                        target_h,
                        SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                    let _ = ShowWindow(child_hwnd, SW_SHOW);
                }

                apply_child_rounded_region(child_hwnd, target_w, target_h, corner_radius);
            }
            let _ = win.show();
            crate::log_info!("EMBEDDED_WEBVIEW", "Child WebviewWindow mounted and shown smoothly for '{}'", plugin_id);
        }
        Err(e) => {
            crate::log_error!("EMBEDDED_WEBVIEW", "Failed to build embedded WebviewWindow for '{}': {:?}", plugin_id, e);
        }
    }
}

pub fn hide_embedded_webview(app: &AppHandle) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    use tauri::Manager;

    crate::log_info!("EMBEDDED_WEBVIEW", "hide_embedded_webview called");
    if let Some(win) = app.get_webview_window(EMBEDDED_LABEL) {
        if let Ok(child_hwnd_raw) = win.hwnd() {
            let child_hwnd = HWND(child_hwnd_raw.0 as *mut _);
            unsafe {
                let _ = ShowWindow(child_hwnd, SW_HIDE);
            }
            crate::log_info!("EMBEDDED_WEBVIEW", "Embedded child window 0x{:X} hidden with SW_HIDE", child_hwnd_raw.0 as usize);
        }
    }
}

