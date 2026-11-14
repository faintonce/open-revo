use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use slint::{Color, ComponentHandle, RgbaColor};
pub use slint;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_DONOTROUND,
};
use windows::Win32::System::Threading::GetCurrentThreadId;

use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumThreadWindows, GetParent, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW,
    SetWindowPos, SetWindowTextW, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
};

pub mod ipc;

fn osd_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("C:\\Users\\faint\\.openrevo_osd.log") {
        let _ = writeln!(f, "[{}] {}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(), msg);
    }
}


slint::include_modules!();

#[derive(Debug, Clone)]
pub struct NativeOsdConfig {
    pub enabled: bool,
    pub theme: String,    // "pill" | "fade" | "cinema"
    pub scale: String,    // "80%" | "100%" | "120%" | "140%"
    pub position: String, // "top-left" | "bottom-center" | "center"
}

impl Default for NativeOsdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            theme: "pill".into(),
            scale: "100%".into(),
            position: "top-left".into(),
        }
    }
}

pub fn parse_scale_factor(scale_str: &str) -> f32 {
    match scale_str {
        "80%" => 0.8,
        "120%" => 1.2,
        "140%" => 1.4,
        _ => 1.0,
    }
}

/// Send & Sync wrapper around Windows HWND handle
#[derive(Debug, Clone, Copy)]
pub struct SafeHwnd(pub isize);

unsafe impl Send for SafeHwnd {}
unsafe impl Sync for SafeHwnd {}

impl SafeHwnd {
    pub fn from_hwnd(hwnd: HWND) -> Self {
        Self(hwnd.0 as isize)
    }

    pub fn as_hwnd(&self) -> HWND {
        HWND(self.0 as *mut _)
    }
}

unsafe extern "system" fn enum_thread_wnd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if let Ok(parent) = GetParent(hwnd) {
        if !parent.0.is_null() {
            return BOOL(1);
        }
    }

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;

    osd_log(&format!("enum_thread_wnd: hwnd={:?}, w={}, h={}", hwnd.0, w, h));

    // Top-level Slint OsdWindow has non-zero dimensions
    if w >= 100 && h >= 10 {
        let target = lparam.0 as *mut HWND;
        *target = hwnd;
        return BOOL(0); // Found target window, stop enumeration
    }
    BOOL(1) // Continue enumeration
}

fn find_thread_hwnd() -> Option<HWND> {
    let mut found_hwnd = HWND(std::ptr::null_mut());
    unsafe {
        let thread_id = GetCurrentThreadId();
        let _ = EnumThreadWindows(
            thread_id,
            Some(enum_thread_wnd_proc),
            LPARAM(&mut found_hwnd as *mut HWND as isize),
        );
    }
    if found_hwnd.0.is_null() {
        None
    } else {
        Some(found_hwnd)
    }
}

fn get_window_dpi_scale(hwnd: HWND) -> f32 {
    #[link(name = "user32")]
    extern "system" {
        fn GetDpiForWindow(hwnd: HWND) -> u32;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi > 0 {
        dpi as f32 / 96.0
    } else {
        1.0
    }
}

fn get_monitor_bounds(hwnd: HWND) -> (i32, i32, i32, i32) {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    unsafe {
        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if !hmon.is_invalid() {
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(hmon, &mut mi).as_bool() {
                let r = mi.rcMonitor;
                return (r.left, r.top, r.right - r.left, r.bottom - r.top);
            }
        }
        (0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
    }
}

fn calculate_osd_position(hwnd: HWND, position_str: &str, win_width: i32, win_height: i32) -> (i32, i32) {
    let dpi_scale = get_window_dpi_scale(hwnd);
    let (mon_x, mon_y, screen_w, screen_h) = get_monitor_bounds(hwnd);

    let w = if win_width > 0 { win_width } else { (640.0 * dpi_scale).round() as i32 };
    let h = if win_height > 0 { win_height } else { (130.0 * dpi_scale).round() as i32 };

    let margin_x = (36.0 * dpi_scale).round() as i32;
    let margin_top = (36.0 * dpi_scale).round() as i32;
    let margin_bottom = (70.0 * dpi_scale).round() as i32;

    let (rel_x, rel_y) = match position_str {
        "top-center" => ((screen_w - w) / 2, margin_top),
        "top-right" | "right" => (screen_w - w - margin_x, margin_top),
        "bottom-center" | "bottom" => ((screen_w - w) / 2, screen_h - h - margin_bottom),
        "bottom-left" => (margin_x, screen_h - h - margin_bottom),
        "center" => ((screen_w - w) / 2, (screen_h - h) / 2),
        _ => (margin_x, margin_top), // Default: "top-left"
    };

    (mon_x + rel_x, mon_y + rel_y)
}

fn reposition_osd_window(hwnd: HWND, position_str: &str) {
    unsafe {
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let win_width = rect.right - rect.left;
        let win_height = rect.bottom - rect.top;

        let (x, y) = calculate_osd_position(hwnd, position_str, win_width, win_height);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

pub fn apply_osd_window_styles(hwnd: HWND, position_str: &str) {
    unsafe {
        // Clear title string so Windows DWM titlebar renderer has nothing to display
        let empty_title: [u16; 1] = [0];
        let _ = SetWindowTextW(hwnd, windows::core::PCWSTR(empty_title.as_ptr()));

        // Remove caption/border decorations without WS_VISIBLE to prevent flash
        SetWindowLongPtrW(hwnd, GWL_STYLE, WS_POPUP.0 as isize);

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_ex_style = (ex_style
            | (WS_EX_TRANSPARENT.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize)
            & !(0x00040000);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style);

        let corner_pref = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner_pref as *const _ as *const _,
            std::mem::size_of_val(&corner_pref) as u32,
        );

        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let win_width = rect.right - rect.left;
        let win_height = rect.bottom - rect.top;

        let (x, y) = calculate_osd_position(hwnd, position_str, win_width, win_height);

        // Crucial: SWP_FRAMECHANGED forces Windows DWM to discard and recalculate non-client caption bar!
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        let _ = ShowWindow(hwnd, SW_HIDE);
        osd_log(&format!("apply_osd_window_styles applied at ({}, {}) for hwnd={:?}", x, y, hwnd.0));
    }
}


#[derive(Debug, Clone)]
pub struct OsdPalette {
    pub office: Color,
    pub balance: Color,
    pub beast: Color,
    pub geek: Color,
    pub fan: Color,
    pub enabled: Color,
    pub disabled: Color,
    pub neutral: Color,
}

impl Default for OsdPalette {
    fn default() -> Self {
        Self {
            office: Color::from(RgbaColor { red: 16, green: 185, blue: 129, alpha: 255 }),
            balance: Color::from(RgbaColor { red: 6, green: 182, blue: 212, alpha: 255 }),
            beast: Color::from(RgbaColor { red: 239, green: 68, blue: 68, alpha: 255 }),
            geek: Color::from(RgbaColor { red: 168, green: 85, blue: 247, alpha: 255 }),
            fan: Color::from(RgbaColor { red: 56, green: 189, blue: 248, alpha: 255 }),
            enabled: Color::from(RgbaColor { red: 16, green: 185, blue: 129, alpha: 255 }),
            disabled: Color::from(RgbaColor { red: 239, green: 68, blue: 68, alpha: 255 }),
            neutral: Color::from(RgbaColor { red: 148, green: 163, blue: 184, alpha: 255 }),
        }
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let s = hex.trim().strip_prefix('#')?;
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color::from_rgb_u8(r, g, b))
    } else if s.len() == 8 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        let a = u8::from_str_radix(&s[6..8], 16).ok()?;
        Some(Color::from_argb_u8(a, r, g, b))
    } else {
        None
    }
}

fn with_alpha(c: Color, alpha: u8) -> Color {
    let rgba = c.to_argb_u8();
    Color::from(RgbaColor { red: rgba.red, green: rgba.green, blue: rgba.blue, alpha })
}

fn current_palette() -> OsdPalette {
    OSD_STATE
        .get()
        .and_then(|s| s.palette.lock().ok().map(|p| p.clone()))
        .unwrap_or_default()
}

#[allow(dead_code)]
struct OsdGlobalState {
    window: slint::Weak<OsdWindow>,
    hwnd: Arc<Mutex<Option<SafeHwnd>>>,
    config: Arc<Mutex<NativeOsdConfig>>,
    palette: Arc<Mutex<OsdPalette>>,
}

static OSD_STATE: OnceLock<OsdGlobalState> = OnceLock::new();

fn reveal_osd_window(hwnd_opt: Option<SafeHwnd>) {
    if let Some(safe_hwnd) = hwnd_opt {
        let pos = OSD_STATE
            .get()
            .and_then(|s| s.config.lock().ok().map(|c| c.position.clone()))
            .unwrap_or_else(|| "top-left".to_string());
        reposition_osd_window(safe_hwnd.as_hwnd(), &pos);
        unsafe {
            let _ = ShowWindow(safe_hwnd.as_hwnd(), SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                safe_hwnd.as_hwnd(),
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        osd_log(&format!("reveal_osd_window: Shown HWND {:?}", safe_hwnd.0));
    } else {
        osd_log("reveal_osd_window: WARNING safe_hwnd is None");
    }
}

static HIDE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn handle_ipc_message(
    raw: &str,
    ui_weak: &slint::Weak<OsdWindow>,
    hwnd_shared: &Arc<Mutex<Option<SafeHwnd>>>,
) {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else {
        osd_log(&format!("handle_ipc_message: invalid JSON: {}", raw));
        return;
    };

    let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("");
    osd_log(&format!("handle_ipc_message: action = {}", action));
    let weak = ui_weak.clone();
    let hwnd_cloned = hwnd_shared.clone();

    match action {
        "mode" => {
            let mode = val.get("value").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            let gen = HIDE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let hwnd_snap = *hwnd_cloned.lock().unwrap();
                    reveal_osd_window(hwnd_snap);
                    show_power_mode_internal(&ui, mode);
                    schedule_auto_hide_internal(&ui, hwnd_cloned.clone(), gen);
                }
            });
        }
        "fan" => {
            let enabled = val.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
            let gen = HIDE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let hwnd_snap = *hwnd_cloned.lock().unwrap();
                    reveal_osd_window(hwnd_snap);
                    show_fan_boost_internal(&ui, enabled);
                    schedule_auto_hide_internal(&ui, hwnd_cloned.clone(), gen);
                }
            });
        }
        "preview" => {
            let gen = HIDE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let hwnd_snap = *hwnd_cloned.lock().unwrap();
                    reveal_osd_window(hwnd_snap);
                    show_power_mode_internal(&ui, 3);
                    schedule_auto_hide_internal(&ui, hwnd_cloned.clone(), gen);
                }
            });
        }
        "key_lock" => {
            let name = val.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let locked = val.get("locked").and_then(|l| l.as_bool()).unwrap_or(false);
            let gen = HIDE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let hwnd_snap = *hwnd_cloned.lock().unwrap();
                    reveal_osd_window(hwnd_snap);
                    show_key_lock_toast_internal(&ui, &name, locked);
                    schedule_auto_hide_internal(&ui, hwnd_cloned.clone(), gen);
                }
            });
        }
        "device_switch" => {
            let name = val.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let enabled = val.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
            let gen = HIDE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let hwnd_snap = *hwnd_cloned.lock().unwrap();
                    reveal_osd_window(hwnd_snap);
                    show_device_switch_internal(&ui, &name, enabled);
                    schedule_auto_hide_internal(&ui, hwnd_cloned.clone(), gen);
                }
            });
        }
        "kb_brightness" => {
            let level = val.get("level").and_then(|l| l.as_u64()).unwrap_or(0) as u8;
            let gen = HIDE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let hwnd_snap = *hwnd_cloned.lock().unwrap();
                    reveal_osd_window(hwnd_snap);
                    show_kb_brightness_internal(&ui, level);
                    schedule_auto_hide_internal(&ui, hwnd_cloned.clone(), gen);
                }
            });
        }
        "kb_sleep" => {
            let seconds = val.get("seconds").and_then(|s| s.as_u64()).unwrap_or(0) as u32;
            let gen = HIDE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let hwnd_snap = *hwnd_cloned.lock().unwrap();
                    reveal_osd_window(hwnd_snap);
                    show_kb_sleep_internal(&ui, seconds);
                    schedule_auto_hide_internal(&ui, hwnd_cloned.clone(), gen);
                }
            });
        }

        "config" => {
            let theme = val.get("theme").and_then(|t| t.as_str()).unwrap_or("pill").to_string();
            let scale = val.get("scale").and_then(|s| s.as_str()).unwrap_or("100%").to_string();
            let position = val.get("position").and_then(|p| p.as_str()).unwrap_or("top-left").to_string();
            let scale_factor = parse_scale_factor(&scale);
            osd_log(&format!("handle_ipc_message config: theme={}, scale={}, pos={}", theme, scale, position));
            if let Some(state) = OSD_STATE.get() {
                let mut cfg = state.config.lock().unwrap();
                cfg.theme = theme.clone();
                cfg.scale = scale.clone();
                cfg.position = position.clone();
            }
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_osd_theme(theme.into());
                    ui.set_osd_scale(scale_factor);
                    ui.set_osd_position(position.clone().into());
                    if let Some(safe_hwnd) = *hwnd_cloned.lock().unwrap() {
                        apply_osd_window_styles(safe_hwnd.as_hwnd(), &position);
                    }
                }
            });
        }
        "theme_tokens" => {
            if let Some(tokens) = val.get("tokens").cloned() {
                if let Some(state) = OSD_STATE.get() {
                    if let Ok(mut pal) = state.palette.lock() {
                        if let Some(c) = tokens.get("mode_office").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            pal.office = c;
                            pal.enabled = c;
                        }
                        if let Some(c) = tokens.get("mode_balance").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            pal.balance = c;
                        }
                        if let Some(c) = tokens.get("mode_beast").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            pal.beast = c;
                            pal.disabled = c;
                        }
                        if let Some(c) = tokens.get("accent_brand").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            pal.geek = c;
                        }
                        if let Some(c) = tokens.get("accent_fan").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            pal.fan = c;
                        }
                        if let Some(c) = tokens.get("text_muted").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            pal.neutral = c;
                        }
                    }
                }
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        if let Some(ff) = tokens.get("font_family").and_then(|v| v.as_str()) {
                            ui.set_font_family(ff.into());
                        }
                        if let Some(c) = tokens.get("surface_elevated").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            let rgba = c.to_argb_u8();
                            let r = rgba.red;
                            let g = rgba.green;
                            let b = rgba.blue;
                            let base_alpha = if rgba.alpha == 255 { 242 } else { rgba.alpha };
                            let mid_alpha = (base_alpha as f32 * 0.45) as u8;

                            ui.set_card_bg_color(Color::from(RgbaColor { red: r, green: g, blue: b, alpha: base_alpha }));
                            ui.set_card_bg_mid(Color::from(RgbaColor { red: r, green: g, blue: b, alpha: mid_alpha }));
                            ui.set_card_bg_end(Color::from(RgbaColor { red: r, green: g, blue: b, alpha: 0 }));
                        }
                        if let Some(c) = tokens.get("text_primary").and_then(|v| v.as_str()).and_then(parse_hex_color) {
                            ui.set_text_primary_color(c);
                        }
                        if let Some(r) = tokens.get("radius_sm").and_then(|v| v.as_f64()) {
                            ui.set_custom_radius_sm(r as f32);
                        }
                        if let Some(r) = tokens.get("radius_md").and_then(|v| v.as_f64()) {
                            ui.set_custom_radius_md(r as f32);
                        }
                    }
                });
            }
        }
        "quit" | "shutdown" | "exit" => {
            std::process::exit(0);
        }
        _ => {}
    }
}



fn show_power_mode_internal(ui: &OsdWindow, mode: u8) {
    let pal = current_palette();
    let (title, icon, color) = match mode {
        1 => ("办公模式", "🌱", pal.office),
        2 => ("均衡模式", "⚖️", pal.balance),
        3 => ("狂暴模式", "⚡", pal.beast),
        _ => ("极客调校", "💧", pal.geek),
    };

    ui.set_mode_title(title.into());
    ui.set_icon_symbol(icon.into());
    ui.set_accent_color(color);
    ui.set_accent_bg(with_alpha(color, 35));
    ui.set_is_active(true);
}

fn show_fan_boost_internal(ui: &OsdWindow, enabled: bool) {
    let pal = current_palette();
    let (title, icon, color) = if enabled {
        ("强冷模式 开启", "🌪️", pal.fan)
    } else {
        ("强冷模式 关闭", "💨", pal.neutral)
    };

    ui.set_mode_title(title.into());
    ui.set_icon_symbol(icon.into());
    ui.set_accent_color(color);
    ui.set_accent_bg(with_alpha(color, 35));
    ui.set_is_active(true);
}

fn show_key_lock_toast_internal(ui: &OsdWindow, name: &str, locked: bool) {
    let pal = current_palette();
    let (title, icon, color) = match name {
        "win" => {
            if locked {
                ("Win 键锁定 开启", "🔒", pal.disabled)
            } else {
                ("Win 键锁定 关闭", "🔓", pal.enabled)
            }
        }
        "fn" => {
            if locked {
                ("Fn 键锁定 开启", "⚡", pal.balance)
            } else {
                ("Fn 键锁定 关闭", "⌨️", pal.neutral)
            }
        }
        "touchpad" => {
            if locked {
                ("触摸板 禁用", "🚫", pal.disabled)
            } else {
                ("触摸板 启用", "🖱️", pal.enabled)
            }
        }
        "caps" => {
            if locked {
                ("大写锁定 开启", "🔠", pal.fan)
            } else {
                ("大写锁定 关闭", "🔡", pal.neutral)
            }
        }
        "num" => {
            if locked {
                ("数字锁定 开启", "🔢", pal.fan)
            } else {
                ("数字锁定 关闭", "⌨️", pal.neutral)
            }
        }
        "scroll" => {
            if locked {
                ("滚动锁定 开启", "📜", pal.fan)
            } else {
                ("滚动锁定 关闭", "📄", pal.neutral)
            }
        }
        _ => {
            if locked {
                ("锁定已生效", "🔒", pal.disabled)
            } else {
                ("解锁已生效", "🔓", pal.enabled)
            }
        }
    };

    ui.set_mode_title(title.into());
    ui.set_icon_symbol(icon.into());
    ui.set_accent_color(color);
    ui.set_accent_bg(with_alpha(color, 30));
    ui.set_is_active(true);
}

fn show_device_switch_internal(ui: &OsdWindow, name: &str, enabled: bool) {
    let pal = current_palette();
    let (title, icon, color) = match name {
        "camera" => {
            if enabled {
                ("摄像头 开启", "📷", pal.enabled)
            } else {
                ("摄像头 关闭", "🚫", pal.disabled)
            }
        }
        "mic" => {
            if enabled {
                ("麦克风 开启", "🎙️", pal.enabled)
            } else {
                ("麦克风 静音", "🔇", pal.disabled)
            }
        }
        "wifi" => {
            if enabled {
                ("Wi-Fi 无线网 开启", "📶", pal.fan)
            } else {
                ("Wi-Fi 无线网 关闭", "✈️", pal.disabled)
            }
        }
        "bluetooth" => {
            if enabled {
                ("蓝牙 开启", "ᛒ", pal.fan)
            } else {
                ("蓝牙 关闭", "📵", pal.disabled)
            }
        }
        "keep_awake" => {
            if enabled {
                ("始终保持唤醒 开启", "☕", pal.fan)
            } else {
                ("始终保持唤醒 关闭", "💤", pal.neutral)
            }
        }
        "screen_flip" => {
            if enabled {
                ("屏幕已翻转 (180°)", "🔄", pal.fan)
            } else {
                ("屏幕方向已恢复", "🖥️", pal.enabled)
            }
        }
        _ => {
            if enabled {
                ("功能已开启", "✅", pal.enabled)
            } else {
                ("功能已关闭", "❌", pal.disabled)
            }
        }
    };

    ui.set_mode_title(title.into());
    ui.set_icon_symbol(icon.into());
    ui.set_accent_color(color);
    ui.set_accent_bg(with_alpha(color, 30));
    ui.set_is_active(true);
}

fn show_kb_brightness_internal(ui: &OsdWindow, level: u8) {
    let pal = current_palette();
    let (title, icon, color) = if level == 0 {
        ("键盘背光 关闭".to_string(), "💡", pal.neutral)
    } else {
        let pct = match level {
            1 => 25,
            2 => 50,
            3 => 75,
            _ => 100,
        };
        (format!("键盘背光 {}%", pct), "💡", pal.fan)
    };

    ui.set_mode_title(title.into());
    ui.set_icon_symbol(icon.into());
    ui.set_accent_color(color);
    ui.set_accent_bg(with_alpha(color, 30));
    ui.set_is_active(true);
}

fn show_kb_sleep_internal(ui: &OsdWindow, seconds: u32) {
    let pal = current_palette();
    let title = if seconds == 0 {
        "键盘背光 常亮".to_string()
    } else {
        format!("背光 {}秒熄灭", seconds)
    };

    ui.set_mode_title(title.into());
    ui.set_icon_symbol("⏳".into());
    ui.set_accent_color(pal.fan);
    ui.set_accent_bg(with_alpha(pal.fan, 35));
    ui.set_is_active(true);
}

fn schedule_auto_hide_internal(ui: &OsdWindow, hwnd_shared: Arc<Mutex<Option<SafeHwnd>>>, generation: u64) {
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(1800), move || {
        if HIDE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
            return; // Superseded by a newer OSD event, abort hiding
        }
        if let Some(target) = weak.upgrade() {
            target.set_is_active(false);

            let hide_hwnd = hwnd_shared.clone();
            slint::Timer::single_shot(Duration::from_millis(260), move || {
                if HIDE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
                    return; // Superseded by a newer OSD event, keep shown
                }
                if let Some(safe_hwnd) = *hide_hwnd.lock().unwrap() {
                    unsafe {
                        let _ = ShowWindow(safe_hwnd.as_hwnd(), SW_HIDE);
                    }
                    osd_log("schedule_auto_hide: window hidden via SW_HIDE");
                }
            });
        }
    });
}


/// Standalone entrypoint for --osd-companion process mode.
/// Runs exclusively on the true Main Thread without conflicting with foreign event loops.
pub fn run_osd_standalone() {
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "user32")]
        extern "system" {
            fn SetProcessDpiAwarenessContext(value: isize) -> i32;
        }
        const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    std::panic::set_hook(Box::new(|info| {
        osd_log(&format!("FATAL PANIC in OSD Companion: {}", info));
    }));

    osd_log("run_osd_standalone entered");

    let osd = match OsdWindow::new() {
        Ok(w) => w,
        Err(e) => {
            osd_log(&format!("FATAL: Failed to create Slint OsdWindow: {}", e));
            return;
        }
    };

    let osd_weak = osd.as_weak();
    let hwnd_holder = Arc::new(Mutex::new(None));
    let cfg_shared = Arc::new(Mutex::new(NativeOsdConfig::default()));
    let palette_shared = Arc::new(Mutex::new(OsdPalette::default()));

    let _ = OSD_STATE.set(OsdGlobalState {
        window: osd_weak.clone(),
        hwnd: hwnd_holder.clone(),
        config: cfg_shared.clone(),
        palette: palette_shared,
    });

    // Start stdin IPC listener
    ipc::start_ipc_server(osd_weak.clone(), hwnd_holder.clone());

    // Show window once so winit actually creates the HWND
    let _ = osd.show();
    osd_log("Called osd.show() once to force HWND creation");

    // Initial capture and style setup timer
    let timer = slint::Timer::default();
    let applied_once = Arc::new(Mutex::new(false));
    let applied_clone = applied_once.clone();
    let hwnd_setter = hwnd_holder.clone();
    let cfg_getter = cfg_shared.clone();
    let osd_init_weak = osd.as_weak();

    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(20),
        move || {
            let mut done = applied_clone.lock().unwrap();
            if !*done {
                if let Some(hwnd) = find_thread_hwnd() {
                    osd_log(&format!("Captured target HWND: {:?}", hwnd.0));
                    let (pos, theme, scale) = {
                        let c = cfg_getter.lock().unwrap();
                        (c.position.clone(), c.theme.clone(), c.scale.clone())
                    };
                    if let Some(ui) = osd_init_weak.upgrade() {
                        ui.set_osd_theme(theme.clone().into());
                        ui.set_osd_scale(parse_scale_factor(&scale));
                        ui.set_osd_position(pos.clone().into());
                    }
                    apply_osd_window_styles(hwnd, &pos);
                    *hwnd_setter.lock().unwrap() = Some(SafeHwnd::from_hwnd(hwnd));
                    unsafe {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                    }
                    *done = true;
                    osd_log(&format!("apply_osd_window_styles done (initial pos={}, theme={}, scale={}), window primed and hidden", pos, theme, scale));
                }
            }
        },
    );

    osd_log("Entering Slint main event loop (run)");
    let _ = osd.run();
}


