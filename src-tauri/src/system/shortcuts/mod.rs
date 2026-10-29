// OpenRevo - Universal Shortcut Bus (Route 2 Implementation)
// Provides zero-overhead, declarative global shortcuts for plugins and system automation.

pub mod types;
pub mod dispatcher;
pub mod manager;

pub use types::{ParsedKeyCombo, ShortcutAction, ShortcutBinding};
pub use manager::{
    check_match_and_trigger, get_all_shortcuts, load_plugin_shortcuts, register_shortcut,
    unload_plugin_shortcuts, unregister_shortcut,
};

/// Initialize the Universal Shortcut Bus subsystem.
/// Zero hardcoded default shortcuts: all shortcut bindings are completely opt-in and plugin/user-driven.
pub fn init_shortcuts() {
    crate::log_info!(
        "SHORTCUTS",
        "Initialized OpenRevo Universal Shortcut Bus (0 default shortcuts, fully plugin & user-driven)"
    );
}
