//! OpenRevo Hardware Peripheral & Device Switches Subsystem Facade
//! Re-exports unified switches engine from `openrevo_core` and hosts the Tauri watcher.

pub use openrevo_core::system::switches::*;

pub mod audio;
pub mod camera;
pub mod radio;
pub mod touchpad;
pub mod watcher;

pub use watcher::start_hardware_state_watcher;
