// OpenRevo - Universal Shortcut Bus Registry & Fast Lookup Engine
// Manages registered shortcut bindings, index lookups by virtual-key codes, and plugin lifecycle sync.

use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use tauri::AppHandle;
use super::types::{ParsedKeyCombo, ShortcutBinding};
use super::dispatcher::dispatch_action;

struct RegistryEntry {
    binding: ShortcutBinding,
    parsed: ParsedKeyCombo,
    last_triggered: Mutex<Option<std::time::Instant>>,
}

struct ShortcutRegistry {
    entries: HashMap<String, RegistryEntry>,
    lookup: HashMap<u16, Vec<String>>,
}

impl ShortcutRegistry {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            lookup: HashMap::new(),
        }
    }
}

static REGISTRY: RwLock<Option<ShortcutRegistry>> = RwLock::new(None);

fn with_registry_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut ShortcutRegistry) -> R,
{
    let mut guard = REGISTRY.write().unwrap();
    if guard.is_none() {
        *guard = Some(ShortcutRegistry::new());
    }
    f(guard.as_mut().unwrap())
}

/// Register a single shortcut binding
pub fn register_shortcut(binding: ShortcutBinding) -> bool {
    let Some(parsed) = ParsedKeyCombo::parse(&binding.combo) else {
        crate::log_warn!("SHORTCUTS", "Failed to parse shortcut combo: '{}'", binding.combo);
        return false;
    };

    with_registry_mut(|registry| {
        let id = binding.id.clone();
        let vk = parsed.vk_code;

        // If replacing existing by id, remove previous lookup reference
        if let Some(old) = registry.entries.remove(&id) {
            if let Some(ids) = registry.lookup.get_mut(&old.parsed.vk_code) {
                ids.retain(|x| x != &id);
            }
        }

        // Also check if any existing shortcut has identical combo to avoid duplicate key conflicts
        let mut conflicting_ids = Vec::new();
        for (existing_id, entry) in &registry.entries {
            if entry.parsed.matches(parsed.vk_code, parsed.ctrl, parsed.alt, parsed.shift, parsed.win) {
                conflicting_ids.push(existing_id.clone());
            }
        }
        for cid in conflicting_ids {
            if let Some(old) = registry.entries.remove(&cid) {
                if let Some(ids) = registry.lookup.get_mut(&old.parsed.vk_code) {
                    ids.retain(|x| x != &cid);
                }
            }
        }

        registry.lookup.entry(vk).or_default().push(id.clone());
        registry.entries.insert(id, RegistryEntry {
            binding,
            parsed,
            last_triggered: Mutex::new(None),
        });
    });

    true
}

/// Unregister a shortcut by its ID
pub fn unregister_shortcut(id: &str) -> bool {
    with_registry_mut(|registry| {
        if let Some(entry) = registry.entries.remove(id) {
            if let Some(ids) = registry.lookup.get_mut(&entry.parsed.vk_code) {
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
    if let Some(ref registry) = *guard {
        registry.entries.values().map(|e| e.binding.clone()).collect()
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
        if let Some(ref registry) = *guard {
            registry.entries.iter()
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
/// Non-blocking read lock only: NEVER blocks or waits on write lock, completely preventing WH_KEYBOARD_LL hook timeout and deadlock.
/// Returns true if the key event should be swallowed (intercepted).
pub fn check_match_and_trigger(
    app: &AppHandle,
    vk: u16,
    is_ctrl: bool,
    is_alt: bool,
    is_shift: bool,
    is_win: bool,
) -> bool {
    let guard = REGISTRY.read().unwrap();
    let Some(ref registry) = *guard else {
        return false;
    };
    let Some(candidate_ids) = registry.lookup.get(&vk) else {
        return false;
    };

    let now = std::time::Instant::now();
    let mut matched_entry: Option<(ShortcutBinding, bool)> = None;

    for id in candidate_ids {
        if let Some(entry) = registry.entries.get(id) {
            if entry.binding.enabled && entry.parsed.matches(vk, is_ctrl, is_alt, is_shift, is_win) {
                // Filter typematic auto-repeat and rapid re-triggers
                let debounce_ms = match entry.binding.action {
                    super::types::ShortcutAction::ToggleRefreshRate => 1200, // Display mode switches need at least 1.2s to settle
                    _ => 400, // Standard shortcuts debounce 400ms to ignore Windows key repeat
                };

                let mut last_guard = entry.last_triggered.lock().unwrap();
                if let Some(last) = *last_guard {
                    if now.duration_since(last) < std::time::Duration::from_millis(debounce_ms) {
                        // Still within cooldown period, ignore repeat but still respect swallow
                        return entry.binding.swallow;
                    }
                }
                *last_guard = Some(now);
                matched_entry = Some((entry.binding.clone(), entry.binding.swallow));
                break;
            }
        }
    }

    // Drop the registry read lock before dispatching
    drop(guard);

    if let Some((binding, swallow)) = matched_entry {
        crate::log_info!("SHORTCUTS", "Triggered shortcut '{}' ({})", binding.name, binding.combo);
        dispatch_action(app.clone(), binding);
        return swallow;
    }

    false
}
