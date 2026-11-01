// OpenRevo - Shortcut Bus IPC Commands
// Provides frontend and plugins with APIs to query, register, unregister, and manually trigger shortcuts.

use tauri::AppHandle;
use crate::system::shortcuts::{
    get_all_shortcuts, register_shortcut as reg_shortcut, unregister_shortcut as unreg_shortcut,
    ShortcutAction, ShortcutBinding, SystemShortcutSetting,
};

#[tauri::command]
pub fn get_registered_shortcuts() -> Vec<ShortcutBinding> {
    get_all_shortcuts()
}

#[tauri::command]
pub fn get_system_shortcut_settings() -> Result<Vec<SystemShortcutSetting>, String> {
    crate::system::shortcuts::get_system_shortcut_settings()
}

#[tauri::command]
pub fn set_system_shortcut_enabled(
    id: String,
    enabled: bool,
) -> Result<Vec<SystemShortcutSetting>, String> {
    crate::system::shortcuts::set_system_shortcut_enabled(id, enabled)
}

#[tauri::command]
pub fn register_custom_shortcut(binding: ShortcutBinding) -> Result<bool, String> {
    if binding.combo.trim().is_empty() {
        return Err("快捷键组合不能为空".to_string());
    }
    if reg_shortcut(binding) {
        Ok(true)
    } else {
        Err("解析或注册快捷键失败，请检查按键格式 (如 'Ctrl+Alt+F')".to_string())
    }
}

#[tauri::command]
pub fn unregister_custom_shortcut(id: String) -> Result<bool, String> {
    if unreg_shortcut(&id) {
        Ok(true)
    } else {
        Err(format!("未找到 ID 为 '{}' 的快捷键", id))
    }
}

#[tauri::command]
pub fn trigger_shortcut_action(
    app: AppHandle,
    action: ShortcutAction,
    osd: Option<bool>,
) -> Result<(), String> {
    let dummy_binding = ShortcutBinding {
        id: "manual_trigger".to_string(),
        name: "Manual Trigger".to_string(),
        combo: "".to_string(),
        action,
        osd: osd.unwrap_or(true),
        enabled: true,
        swallow: false,
        plugin_id: None,
    };
    crate::system::shortcuts::dispatcher::dispatch_action(app, dummy_binding);
    Ok(())
}
