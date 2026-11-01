// OpenRevo - Universal Shortcut Bus (Route 2 Implementation)
// Provides zero-overhead, declarative global shortcuts for plugins and system automation.

pub mod types;
pub mod dispatcher;
pub mod manager;
pub mod settings;

pub use types::{ParsedKeyCombo, ShortcutAction, ShortcutBinding};
pub use manager::{
    check_match_and_trigger, get_all_shortcuts, load_plugin_shortcuts, register_shortcut,
    unload_plugin_shortcuts, unregister_shortcut,
};
pub use settings::{get_system_shortcut_settings, set_system_shortcut_enabled, SystemShortcutSetting};

/// Initialize default system shortcuts and load plugin shortcuts
pub fn init_shortcuts() {
    crate::log_info!("SHORTCUTS", "Initializing OpenRevo Universal Shortcut Bus...");

    settings::init_system_shortcuts();
}
