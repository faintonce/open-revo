// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--osd-companion") {
        openrevo_osd::run_osd_standalone();
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
