// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--osd-companion") {
        openrevo_osd::run_osd_standalone();
        return;
    }
    open_revo_lib::run();
}
