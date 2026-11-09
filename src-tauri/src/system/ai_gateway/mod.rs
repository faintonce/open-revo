// OpenRevo - Universal AI Gateway Engine
// Handles OpenAI compatible API connectivity, SSE streaming, and Hardware Tool Calling loop

pub mod types;
pub mod tools;

use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::Duration;
use serde_json::json;

use types::*;

const PLUGIN_ID: &str = "revo-llm-hub";

/// Constructs an HTTP/HTTPS Agent pre-configured with native Windows SChannel TLS
fn get_http_agent() -> ureq::Agent {
    let mut builder = ureq::builder()
        .timeout(Duration::from_secs(45));

    if let Ok(tls) = ureq::native_tls::TlsConnector::new() {
        builder = builder.tls_connector(Arc::new(tls));
    }

    builder.build()
}

/// Loads LLM configuration from the sandboxed plugin storage of "revo-llm-hub"
pub fn get_gateway_config() -> AiGatewayConfig {
    match crate::system::plugin_storage::get_plugin_storage(PLUGIN_ID) {
        Ok(val) => serde_json::from_value(val).unwrap_or_default(),
        Err(_) => AiGatewayConfig::default(),
    }
}

/// Saves LLM configuration to the sandboxed plugin storage
pub fn save_gateway_config(config: &AiGatewayConfig) -> Result<(), String> {
    let val = serde_json::to_value(config).map_err(|e| e.to_string())?;
    crate::system::plugin_storage::set_plugin_storage(PLUGIN_ID, val)?;
    crate::system::broadcast::broadcast_all("llm:config-updated", &json!({ "model": &config.model }));
    Ok(())
}

/// Normalizes Base URL to ensure standard /v1 endpoint format
fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "https://api.deepseek.com/v1".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Queries the /v1/models endpoint of the provider to list available models
pub fn fetch_models(base_url_opt: Option<String>, api_key_opt: Option<String>) -> Result<Vec<String>, String> {
    let cfg = get_gateway_config();
    let base_url = normalize_base_url(&base_url_opt.unwrap_or(cfg.base_url));
    let api_key = api_key_opt.unwrap_or(cfg.api_key);

    let url = format!("{}/models", base_url);
    crate::log_info!("AI_GATEWAY", "Fetching available models from: {}", url);

    let agent = get_http_agent();
    let mut req = agent.get(&url);

    if !api_key.trim().is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", api_key.trim()));
    }

    let resp_res = req.call();

    let mut list: Vec<String> = Vec::new();

    match resp_res {
        Ok(resp) => {
            if let Ok(model_res) = resp.into_json::<ModelListResponse>() {
                let items = model_res.data.or(model_res.models).unwrap_or_default();
                for m in items {
                    let clean = m.id.trim();
                    if !clean.is_empty() && !list.contains(&clean.to_string()) {
                        list.push(clean.to_string());
                    }
                }
            }
        }
        Err(e) => {
            // If it's localhost Ollama, try native /api/tags endpoint as fallback
            if base_url.contains("11434") {
                crate::log_info!("AI_GATEWAY", "Fallback probing Ollama native /api/tags endpoint");
                let native_url = "http://localhost:11434/api/tags";
                if let Ok(native_resp) = agent.get(native_url).call() {
                    if let Ok(model_res) = native_resp.into_json::<ModelListResponse>() {
                        let items = model_res.models.or(model_res.data).unwrap_or_default();
                        for m in items {
                            let clean = m.id.trim();
                            if !clean.is_empty() && !list.contains(&clean.to_string()) {
                                list.push(clean.to_string());
                            }
                        }
                    }
                }
            }
            if list.is_empty() {
                return Err(format!("无法连接大模型服务: {}: {}", url, e));
            }
        }
    }

    // Secondary fallback for Ollama when /v1/models returned empty data array
    if list.is_empty() && base_url.contains("11434") {
        let native_url = "http://localhost:11434/api/tags";
        if let Ok(native_resp) = agent.get(native_url).call() {
            if let Ok(model_res) = native_resp.into_json::<ModelListResponse>() {
                let items = model_res.models.or(model_res.data).unwrap_or_default();
                for m in items {
                    let clean = m.id.trim();
                    if !clean.is_empty() && !list.contains(&clean.to_string()) {
                        list.push(clean.to_string());
                    }
                }
            }
        }
    }

    // Smart sort: Prioritize conversational/instruction LLMs over embeddings/rerankers/image models
    list.sort_by(|a, b| {
        let score_model = |name: &str| -> i32 {
            let lower = name.to_ascii_lowercase();
            if lower.contains("bge-") || lower.contains("embed") || lower.contains("rerank") || lower.contains("flux") || lower.contains("whisper") || lower.contains("tts") {
                return 100; // Deprioritize non-chat models
            }
            if lower.contains("deepseek-v3") || lower.contains("deepseek-chat") {
                return 1;
            }
            if lower.contains("deepseek") {
                return 2;
            }
            if lower.contains("qwen2.5") {
                return 3;
            }
            if lower.contains("chat") || lower.contains("instruct") {
                return 5;
            }
            10
        };
        let sa = score_model(a);
        let sb = score_model(b);
        sa.cmp(&sb).then_with(|| a.cmp(b))
    });
    Ok(list)
}

/// Sends a prompt and streams the output via OpenRevo universal broadcast bus
pub fn send_chat_stream(
    prompt: String,
    session_id_opt: Option<String>,
    enable_tools_opt: Option<bool>,
    history_opt: Option<Vec<ChatMessage>>,
    config_override: Option<AiGatewayConfig>,
) -> Result<String, String> {
    let session_id = session_id_opt.unwrap_or_else(|| format!("sess_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    let enable_tools = enable_tools_opt.unwrap_or(true);
    let cfg = config_override.unwrap_or_else(|| get_gateway_config());

    if cfg.api_key.trim().is_empty() && !cfg.base_url.contains("localhost") && !cfg.base_url.contains("127.0.0.1") {
        let err_msg = "未配置 API Key，请在 API Key 输入框填入密钥".to_string();
        crate::system::broadcast::broadcast_all("llm:chat:error", &json!({
            "session_id": &session_id,
            "error": &err_msg
        }));
        return Err(err_msg);
    }

    let sid_clone = session_id.clone();
    std::thread::Builder::new()
        .name(format!("ai-stream-{}", session_id))
        .spawn(move || {
            if let Err(e) = execute_chat_stream_loop(&sid_clone, &cfg, prompt, history_opt, enable_tools) {
                crate::log_warn!("AI_GATEWAY", "Chat stream error: {}", e);
                crate::system::broadcast::broadcast_all("llm:chat:error", &json!({
                    "session_id": &sid_clone,
                    "error": e
                }));
            }
        })
        .map_err(|e| format!("启动流式线程失败: {}", e))?;

    Ok(session_id)
}

/// Internal execution loop supporting ReAct tool call cycles
fn execute_chat_stream_loop(
    session_id: &str,
    cfg: &AiGatewayConfig,
    user_prompt: String,
    history_opt: Option<Vec<ChatMessage>>,
    enable_tools: bool,
) -> Result<(), String> {
    let mut messages: Vec<ChatMessage> = Vec::new();

    // 1. System Prompt
    if let Some(ref sys) = cfg.system_prompt {
        if !sys.trim().is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(sys.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    // 2. History
    if let Some(history) = history_opt {
        messages.extend(history);
    }

    // 3. Current User Message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: Some(user_prompt),
        tool_calls: None,
        tool_call_id: None,
    });

    let base_url = normalize_base_url(&cfg.base_url);
    let completions_url = format!("{}/chat/completions", base_url);

    let tools_schema = if enable_tools {
        Some(tools::get_hardware_tools_schema())
    } else {
        None
    };

    let mut full_accumulated_text = String::new();
    let max_rounds = 4; // Prevent infinite tool call loops
    let mut current_round = 0;

    while current_round < max_rounds {
        current_round += 1;

        let req_body = ChatCompletionRequest {
            model: cfg.model.clone(),
            messages: messages.clone(),
            stream: true,
            temperature: cfg.temperature,
            tools: tools_schema.clone(),
        };

        crate::log_info!("AI_GATEWAY", "[{}] Sending round {} to {}", session_id, current_round, completions_url);

        let mut req = get_http_agent().post(&completions_url)
            .set("Content-Type", "application/json");

        if !cfg.api_key.trim().is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", cfg.api_key.trim()));
        }

        let resp = match req.send_json(&req_body) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                let detail = if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&err_body) {
                    err_json.get("message")
                        .or_else(|| err_json.get("error").and_then(|e| e.get("message")))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or(err_body)
                } else {
                    err_body
                };
                return Err(format!("大模型端点响应 HTTP {}: {}", code, detail));
            }
            Err(e) => {
                return Err(format!("请求大模型失败: {}", e));
            }
        };

        let reader = resp.into_reader();
        let buf_reader = BufReader::new(reader);

        // Track tool call parts during SSE stream
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
        let mut assistant_content = String::new();

        for line_res in buf_reader.lines() {
            let line = line_res.map_err(|e| format!("读取流数据中断: {}", e))?;
            let line_trimmed = line.trim();

            if line_trimmed.is_empty() || line_trimmed.starts_with(':') {
                continue;
            }

            if line_trimmed == "data: [DONE]" {
                break;
            }

            if let Some(json_str) = line_trimmed.strip_prefix("data:") {
                let parsed: serde_json::Value = match serde_json::from_str(json_str.trim()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let choices = match parsed.get("choices").and_then(|c| c.as_array()) {
                    Some(c) if !c.is_empty() => c,
                    _ => continue,
                };

                let delta = match choices[0].get("delta") {
                    Some(d) => d,
                    _ => continue,
                };

                // Normal content delta
                if let Some(content) = delta.get("content").and_then(|s| s.as_str()) {
                    if !content.is_empty() {
                        assistant_content.push_str(content);
                        full_accumulated_text.push_str(content);
                        crate::system::broadcast::broadcast_all("llm:stream:chunk", &json!({
                            "session_id": session_id,
                            "delta": content,
                            "is_reasoning": false
                        }));
                    }
                }

                // DeepSeek reasoning content delta
                if let Some(reasoning) = delta.get("reasoning_content").and_then(|s| s.as_str()) {
                    if !reasoning.is_empty() {
                        crate::system::broadcast::broadcast_all("llm:stream:chunk", &json!({
                            "session_id": session_id,
                            "delta": reasoning,
                            "is_reasoning": true
                        }));
                    }
                }

                // Tool calls delta accumulation
                if let Some(tc_array) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                    for tc in tc_array {
                        let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                        while pending_tool_calls.len() <= idx {
                            pending_tool_calls.push(PendingToolCall::default());
                        }

                        if let Some(id) = tc.get("id").and_then(|s| s.as_str()) {
                            pending_tool_calls[idx].id.push_str(id);
                        }
                        if let Some(func) = tc.get("function") {
                            if let Some(name) = func.get("name").and_then(|s| s.as_str()) {
                                pending_tool_calls[idx].name.push_str(name);
                            }
                            if let Some(args) = func.get("arguments").and_then(|s| s.as_str()) {
                                pending_tool_calls[idx].arguments.push_str(args);
                            }
                        }
                    }
                }
            }
        }

        // If no tool calls were requested, round is complete!
        if pending_tool_calls.is_empty() {
            break;
        }

        // Reconstruct tool_calls for messages history
        let mut executed_calls: Vec<ToolCall> = Vec::new();
        let mut tool_results: Vec<(String, String)> = Vec::new();

        for (i, tc) in pending_tool_calls.into_iter().enumerate() {
            let call_id = if tc.id.is_empty() { format!("call_{}_{}", session_id, i) } else { tc.id };
            let tool_name = tc.name;
            let args_json: serde_json::Value = serde_json::from_str(&tc.arguments).unwrap_or(json!({}));

            // Execute hardware tool via HAL
            let exec_result = tools::execute_hardware_tool(&tool_name, &args_json);
            let result_val = match exec_result {
                Ok(res) => res,
                Err(err) => json!({ "success": false, "error": err })
            };

            // Broadcast executing feedback to widgets
            crate::system::broadcast::broadcast_all("llm:action:executing", &json!({
                "session_id": session_id,
                "tool": &tool_name,
                "args": &args_json,
                "result": &result_val
            }));

            executed_calls.push(ToolCall {
                id: call_id.clone(),
                r#type: "function".to_string(),
                function: ToolCallFunction {
                    name: tool_name,
                    arguments: tc.arguments,
                },
            });

            tool_results.push((call_id, result_val.to_string()));
        }

        // Add assistant's tool_calls step to message chain
        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: if assistant_content.is_empty() { None } else { Some(assistant_content) },
            tool_calls: Some(executed_calls),
            tool_call_id: None,
        });

        // Add tool results to message chain for model to generate final answer
        for (call_id, res_str) in tool_results {
            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(res_str),
                tool_calls: None,
                tool_call_id: Some(call_id),
            });
        }
    }

    // Final completion broadcast
    crate::system::broadcast::broadcast_all("llm:chat:done", &json!({
        "session_id": session_id,
        "full_text": full_accumulated_text
    }));

    Ok(())
}

#[derive(Default, Debug)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

