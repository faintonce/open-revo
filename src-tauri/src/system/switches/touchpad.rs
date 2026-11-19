//! OpenRevo Precision Touchpad Hardware & Kernel Notification Facade
//! Re-exports unified touchpad switch and kernel notification engine from `openrevo_core`.

pub use openrevo_core::system::switches::touchpad::*;

pub fn start_touchpad_listener(_app: Option<tauri::AppHandle>) {
    openrevo_core::system::switches::touchpad::start_touchpad_listener();
}
