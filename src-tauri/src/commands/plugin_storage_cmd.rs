// OpenRevo - Plugin Sandboxed Storage IPC Commands
// Provides sandboxed JSON/Key-Value and file storage APIs for plugins.

#[tauri::command]
pub fn get_plugin_storage(plugin_id: String) -> Result<serde_json::Value, String> {
    crate::system::plugin_storage::get_plugin_storage(&plugin_id)
}

#[tauri::command]
pub fn set_plugin_storage(plugin_id: String, data: serde_json::Value) -> Result<(), String> {
    crate::system::plugin_storage::set_plugin_storage(&plugin_id, data)
}

#[tauri::command]
pub fn read_plugin_file(plugin_id: String, path: String) -> Result<String, String> {
    crate::system::plugin_storage::read_plugin_file(&plugin_id, &path)
}

#[tauri::command]
pub fn write_plugin_file(plugin_id: String, path: String, content: String) -> Result<(), String> {
    crate::system::plugin_storage::write_plugin_file(&plugin_id, &path, &content)
}

#[tauri::command]
pub fn delete_plugin_file(plugin_id: String, path: String) -> Result<bool, String> {
    crate::system::plugin_storage::delete_plugin_file(&plugin_id, &path)
}

#[tauri::command]
pub fn list_plugin_files(plugin_id: String, sub_dir: Option<String>) -> Result<Vec<String>, String> {
    crate::system::plugin_storage::list_plugin_files(&plugin_id, sub_dir)
}

