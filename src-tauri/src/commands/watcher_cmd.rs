// OpenRevo - Watcher Hub IPC Commands
// Exposes microkernel monitoring registration, unregistration, querying, and manual triggers to frontend/plugins.

use crate::system::watcher_hub::{
    get_all_watchers, register_watcher as reg_watcher, unregister_watcher as unreg_watcher,
    WatcherDefinition, WatcherInfo, dispatch_watcher_trigger,
};

#[tauri::command]
pub fn get_registered_watchers() -> Vec<WatcherInfo> {
    get_all_watchers()
}

#[tauri::command]
pub fn register_custom_watcher(watcher: WatcherDefinition) -> Result<bool, String> {
    if watcher.id.trim().is_empty() {
        return Err("Watcher ID не может быть пустым".to_string());
    }
    if watcher.name.trim().is_empty() {
        return Err("Имя Watcher не может быть пустым".to_string());
    }
    if reg_watcher(watcher) {
        Ok(true)
    } else {
        Err("Не удалось зарегистрировать публичный слушатель".to_string())
    }
}

#[tauri::command]
pub fn unregister_custom_watcher(id: String) -> Result<bool, String> {
    if unreg_watcher(&id) {
        Ok(true)
    } else {
        Err(format!("Не найден публичный слушатель с ID '{}'", id))
    }
}

#[tauri::command]
pub fn trigger_watcher_manual(id: String) -> Result<bool, String> {
    let list = get_all_watchers();
    if let Some(_item) = list.iter().find(|w| w.id == id) {
        // Find full definition if possible
        let dummy_def = WatcherDefinition {
            id: id.clone(),
            name: format!("Manual: {}", id),
            plugin_id: None,
            enabled: true,
            cooldown_ms: 0,
            condition: crate::system::watcher_hub::WatcherCondition::PowerEvent { event: "manual".into() },
            actions: vec![],
        };
        dispatch_watcher_trigger(dummy_def, "manual", serde_json::json!({ "triggered_by": "user" }));
        Ok(true)
    } else {
        Err(format!("Не найден публичный слушатель с ID '{}'", id))
    }
}
