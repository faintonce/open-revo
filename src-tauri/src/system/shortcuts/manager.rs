// OpenRevo - Universal Shortcut Bus Registry & Fast Lookup Engine
// Manages registered shortcut bindings, index lookups by virtual-key codes, and plugin lifecycle sync.

use std::collections::HashMap;
use std::sync::RwLock;
use tauri::AppHandle;
use super::types::{ParsedKeyCombo, ShortcutBinding};
use super::dispatcher::dispatch_action;

struct RegistryEntry {
    binding: ShortcutBinding,
    parsed: ParsedKeyCombo,
    last_triggered: Option<std::time::Instant>,
}

static REGISTRY: RwLock<Option<HashMap<String, RegistryEntry>>> = RwLock::new(None);
static LOOKUP: RwLock<Option<HashMap<u16, Vec<String>>>> = RwLock::new(None);

fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, RegistryEntry>, &mut HashMap<u16, Vec<String>>) -> R,
{
    let mut reg_guard = REGISTRY.write().unwrap();
    let mut look_guard = LOOKUP.write().unwrap();

    if reg_guard.is_none() {
        *reg_guard = Some(HashMap::new());
    }
    if look_guard.is_none() {
        *look_guard = Some(HashMap::new());
    }

    f(reg_guard.as_mut().unwrap(), look_guard.as_mut().unwrap())
}

/// Register a single shortcut binding
pub fn register_shortcut(binding: ShortcutBinding) -> bool {
    let Some(parsed) = ParsedKeyCombo::parse(&binding.combo) else {
        crate::log_warn!("SHORTCUTS", "Failed to parse shortcut combo: '{}'", binding.combo);
        return false;
    };

    with_registry(|reg, look| {
        let id = binding.id.clone();
        let vk = parsed.vk_code;

        // If replacing existing, remove previous lookup reference
        if let Some(old) = reg.remove(&id) {
            if let Some(ids) = look.get_mut(&old.parsed.vk_code) {
                ids.retain(|x| x != &id);
            }
        }

        look.entry(vk).or_default().push(id.clone());
        reg.insert(id, RegistryEntry {
            binding,
            parsed,
            last_triggered: None,
        });
    });

    true
}

/// Unregister a shortcut by its ID
pub fn unregister_shortcut(id: &str) -> bool {
    with_registry(|reg, look| {
        if let Some(entry) = reg.remove(id) {
            if let Some(ids) = look.get_mut(&entry.parsed.vk_code) {
                ids.retain(|x| x != id);
            }
            true
        } else {
            false
        }
    })
}

/// Retrieve all currently registered shortcuts
pub fn get_all_shortcuts() -> Vec<ShortcutBinding> {
    let guard = REGISTRY.read().unwrap();
    if let Some(ref map) = *guard {
        map.values().map(|e| e.binding.clone()).collect()
    } else {
        Vec::new()
    }
}

/// Register all shortcuts declared by a specific plugin
pub fn load_plugin_shortcuts(plugin_id: &str, shortcuts: Vec<ShortcutBinding>) {
    crate::log_info!("SHORTCUTS", "Loading {} shortcuts for plugin '{}'", shortcuts.len(), plugin_id);
    for mut s in shortcuts {
        s.plugin_id = Some(plugin_id.to_string());
        register_shortcut(s);
    }
}

/// Unregister all shortcuts belonging to a specific plugin
pub fn unload_plugin_shortcuts(plugin_id: &str) {
    let to_remove: Vec<String> = {
        let guard = REGISTRY.read().unwrap();
        if let Some(ref map) = *guard {
            map.iter()
                .filter(|(_, e)| e.binding.plugin_id.as_deref() == Some(plugin_id))
                .map(|(k, _)| k.clone())
                .collect()
        } else {
            Vec::new()
        }
    };

    for id in to_remove {
        unregister_shortcut(&id);
    }
    crate::log_info!("SHORTCUTS", "Unloaded shortcuts for plugin '{}'", plugin_id);
}

/// Hot-path: match active key event and trigger action.
/// Returns true if the key event should be swallowed (intercepted).
pub fn check_match_and_trigger(
    app: &AppHandle,
    vk: u16,
    is_ctrl: bool,
    is_alt: bool,
    is_shift: bool,
    is_win: bool,
) -> bool {
    let matched_entry: Option<(ShortcutBinding, bool)> = {
        let look_guard = LOOKUP.read().unwrap();
        let Some(ref look) = *look_guard else {
            return false;
        };
        let Some(candidate_ids) = look.get(&vk) else {
            return false;
        };

        let mut reg_guard = REGISTRY.write().unwrap();
        let Some(ref mut reg) = *reg_guard else {
            return false;
        };

        let now = std::time::Instant::now();
        let mut found = None;
        for id in candidate_ids {
            if let Some(entry) = reg.get_mut(id) {
                if entry.binding.enabled && entry.parsed.matches(vk, is_ctrl, is_alt, is_shift, is_win) {
                    // Filter typematic auto-repeat and rapid re-triggers
                    let debounce_ms = match entry.binding.action {
                        super::types::ShortcutAction::ToggleRefreshRate => 1200, // Display mode switches need at least 1.2s to settle
                        _ => 400, // Standard shortcuts debounce 400ms to ignore Windows key repeat
                    };
                    if let Some(last) = entry.last_triggered {
                        if now.duration_since(last) < std::time::Duration::from_millis(debounce_ms) {
                            // Still within cooldown period, ignore repeat but still respect swallow
                            return entry.binding.swallow;
                        }
                    }
                    entry.last_triggered = Some(now);
                    found = Some((entry.binding.clone(), entry.binding.swallow));
                    break;
                }
            }
        }
        found
    };

    if let Some((binding, swallow)) = matched_entry {
        crate::log_info!("SHORTCUTS", "Triggered shortcut '{}' ({})", binding.name, binding.combo);
        dispatch_action(app.clone(), binding);
        return swallow;
    }

    false
}
