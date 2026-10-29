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
/// Loads persistent custom shortcuts saved in host config.json.
pub fn init_shortcuts() {
    let cfg = crate::system::config::load_config();
    let count = cfg.custom_shortcuts.len();
    for binding in cfg.custom_shortcuts {
        register_shortcut(binding);
    }
    crate::log_info!(
        "SHORTCUTS",
        "Initialized OpenRevo Universal Shortcut Bus (loaded {} persistent custom shortcuts)",
        count
    );
}

/// Probes whether a key combination is currently occupied:
/// 1. By OpenRevo's internal shortcut bus
/// 2. By third-party applications (QQ, WeChat, Snipaste, etc.) via Win32 RegisterHotKey probe
#[cfg(target_os = "windows")]
pub fn probe_system_hotkey_occupied(combo_str: &str) -> (bool, Option<String>) {
    let Some(parsed) = types::ParsedKeyCombo::parse(combo_str) else {
        return (false, None);
    };

    // 1. Check OpenRevo internal registration
    let openrevo_occupant = {
        let all = manager::get_all_shortcuts();
        all.into_iter().find(|s| {
            if let Some(p) = types::ParsedKeyCombo::parse(&s.combo) {
                p.matches(parsed.vk_code, parsed.ctrl, parsed.alt, parsed.shift, parsed.win)
            } else {
                false
            }
        }).map(|s| s.name)
    };

    // 2. Check third-party Windows RegisterHotKey occupancy
    let is_third_party = unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS};

        let mut mods = 0u32;
        if parsed.alt { mods |= 0x0001; }
        if parsed.ctrl { mods |= 0x0002; }
        if parsed.shift { mods |= 0x0004; }
        if parsed.win { mods |= 0x0008; }

        const PROBE_ID: i32 = 0x7E57; // Dummy probe ID
        let res = RegisterHotKey(HWND(std::ptr::null_mut()), PROBE_ID, HOT_KEY_MODIFIERS(mods), parsed.vk_code as u32);
        if res.is_ok() {
            let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), PROBE_ID);
            false // Free
        } else {
            true // Occupied by external apps (QQ, WeChat, Snipaste, etc.)
        }
    };

    (is_third_party, openrevo_occupant)
}

#[cfg(not(target_os = "windows"))]
pub fn probe_system_hotkey_occupied(_combo_str: &str) -> (bool, Option<String>) {
    (false, None)
}
