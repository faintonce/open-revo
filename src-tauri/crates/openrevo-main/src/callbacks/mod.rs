use std::sync::{Arc, Mutex};
use crate::window::SafeHwnd;
use crate::MainWindow;

pub mod gpu_display;
pub mod lighting;
pub mod plugins;
pub mod power;
pub mod shell;
pub mod switches;
pub mod tuning;
pub mod water;

pub fn register_all(
    window: &MainWindow,
    ui_weak: slint::Weak<MainWindow>,
    hwnd_holder: Arc<Mutex<Option<SafeHwnd>>>,
) {
    power::register(window, ui_weak.clone());
    gpu_display::register(window, ui_weak.clone());
    switches::register(window, ui_weak.clone());
    tuning::register(window, ui_weak.clone());
    lighting::register(window, ui_weak.clone());
    water::register(window, ui_weak.clone());
    plugins::register(window, ui_weak.clone());
    shell::register(window, ui_weak.clone(), hwnd_holder);
}

