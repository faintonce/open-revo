// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--osd-companion") {
        openrevo_osd::run_osd_standalone();
        return;
    }
    if std::env::args().any(|arg| arg == "--slint-main") {
        openrevo_main::run_main_standalone();
        return;
    }
    open_revo_lib::run();
}
