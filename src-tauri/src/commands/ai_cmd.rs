// OpenRevo - AI Gateway IPC Commands
// Exposes LLM capability and hardware tool orchestrator to all surfaces

use crate::system::ai_gateway::{
    self,
    types::AiGatewayConfig,
    tools,
};

#[tauri::command]
pub fn ai_get_config() -> Result<AiGatewayConfig, String> {
    Ok(ai_gateway::get_gateway_config())
}

#[tauri::command]
pub fn ai_save_config(config: AiGatewayConfig) -> Result<(), String> {
    ai_gateway::save_gateway_config(&config)
}

#[tauri::command]
pub fn ai_fetch_models(base_url: Option<String>, api_key: Option<String>) -> Result<Vec<String>, String> {
    ai_gateway::fetch_models(base_url, api_key)
}

#[tauri::command]
pub fn ai_chat_send(
    prompt: String,
    session_id: Option<String>,
    enable_tools: Option<bool>,
    config_override: Option<AiGatewayConfig>,
) -> Result<String, String> {
    ai_gateway::send_chat_stream(prompt, session_id, enable_tools, None, config_override)
}

#[tauri::command]
pub fn ai_get_system_skills() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "tools": tools::get_hardware_tools_schema(),
        "config": ai_gateway::get_gateway_config(),
    }))
}

