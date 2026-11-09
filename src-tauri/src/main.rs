// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--osd-companion") {
        openrevo_osd::run_osd_standalone();
        return;
    }
    if let Some(pos) = std::env::args().position(|arg| arg == "--slint-widget") {
        let args: Vec<String> = std::env::args().collect();
        let plugin_id = args.get(pos + 1).cloned().unwrap_or_default();
        if let Some(embed_pos) = args.iter().position(|a| a == "--embed") {
            if args.len() >= embed_pos + 6 {
                let parent_hwnd: isize = args[embed_pos + 1].parse().unwrap_or(0);
                let x: i32 = args[embed_pos + 2].parse().unwrap_or(0);
                let y: i32 = args[embed_pos + 3].parse().unwrap_or(0);
                let w: i32 = args[embed_pos + 4].parse().unwrap_or(600);
                let h: i32 = args[embed_pos + 5].parse().unwrap_or(400);
                openrevo_widget::run_widget_embedded(&plugin_id, parent_hwnd, (x, y, w, h));
                return;
            }
        }
        openrevo_widget::run_widget_standalone(&plugin_id);
        return;
    }
    if std::env::args().any(|arg| arg == "--slint-main") {
        #[cfg(target_os = "windows")]
        unsafe {
            #[link(name = "user32")]
            extern "system" {
                fn SetProcessDpiAwarenessContext(value: isize) -> i32;
            }
            const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        openrevo_main::run_main_standalone();
        return;
    }
    open_revo_lib::run();
}
