// OpenRevo - AI Gateway Data Contracts & Types
// Standard OpenAI compatible types with Tool Calling support

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiGatewayConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

impl Default for AiGatewayConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.deepseek.com/v1".to_string(),
            api_key: "".to_string(),
            model: "deepseek-chat".to_string(),
            temperature: Some(0.7),
            system_prompt: Some("你是运行在 OpenRevo 笔记本控制中心上的 AI 智能桌宠与硬件助手。你可以理解用户意图，并通过工具控制电脑的性能模式、全速强冷、屏幕刷新率、电池养护和设备开关。若调用了工具，请用简练自然、带有桌宠机甲个性的语气向用户汇报状态。".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    #[serde(default)]
    pub data: Option<Vec<ModelItem>>,
    #[serde(default)]
    pub models: Option<Vec<ModelItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelItem {
    #[serde(default, alias = "name", alias = "model")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkPayload {
    pub session_id: String,
    pub delta: String,
    pub is_reasoning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutingPayload {
    pub session_id: String,
    pub tool: String,
    pub args: serde_json::Value,
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDonePayload {
    pub session_id: String,
    pub full_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatErrorPayload {
    pub session_id: String,
    pub error: String,
}

