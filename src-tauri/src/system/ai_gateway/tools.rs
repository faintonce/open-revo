// OpenRevo - AI Gateway Hardware Tools & ReAct Engine
// Maps LLM Tool Calling to OpenRevo HAL & Semantic Hardware APIs

use serde_json::json;

/// Returns standard OpenAI tools schema for all exposed OpenRevo hardware capabilities
pub fn get_hardware_tools_schema() -> Vec<serde_json::Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "get_system_telemetry",
                "description": "获取当前笔记本的硬件状态快照，包括性能模式、全速强冷状态、电池养护上限等。",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "set_fan_boost",
                "description": "开启或关闭机械革命笔记本的全速强冷模式 (Fan Boost / 暴走散热)。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "enable": {
                            "type": "boolean",
                            "description": "true 为开启全速强冷，false 为恢复正常智能温控"
                        }
                    },
                    "required": ["enable"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "set_power_mode",
                "description": "切换笔记本性能模式 (Power Mode)。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "mode": {
                            "type": "integer",
                            "enum": [1, 2, 3, 4],
                            "description": "1: 办公模式 (Office/省电静音), 2: 均衡模式 (Balance/日常流畅), 3: 狂暴模式 (Turbo/Beast/游戏电竞性能释放), 4: 自定义模式"
                        }
                    },
                    "required": ["mode"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "switch_refresh_rate",
                "description": "切换主显示屏的屏幕刷新率 (Hz)。通常离电省电推荐 60Hz，游戏电竞推荐 165Hz 或 240Hz。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "hz": {
                            "type": "integer",
                            "description": "目标屏幕刷新率，如 60, 165, 240"
                        }
                    },
                    "required": ["hz"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "set_battery_limit",
                "description": "设置笔记本电池健康养护充电上限。达到上限后硬件直供断充，防止电池持续高压鼓包损耗寿命。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "enum": [60, 80, 100],
                            "description": "养护限额百分比：60 (极致养护长插电), 80 (日常推荐平衡), 100 (充满远行)"
                        }
                    },
                    "required": ["limit"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "set_device_switch",
                "description": "控制笔记本外设硬件软开关（摄像头、麦克风、触控板、Win键锁定）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "device": {
                            "type": "string",
                            "enum": ["camera", "mic", "touchpad", "winlock"],
                            "description": "目标外设：camera (摄像头), mic (麦克风), touchpad (触控板), winlock (Windows键锁定)"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "true 为开启启用，false 为禁用关闭"
                        }
                    },
                    "required": ["device", "enabled"]
                }
            }
        })
    ]
}

/// Executes a tool call directly on OpenRevo microkernel
pub fn execute_hardware_tool(name: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    crate::log_info!("AI_GATEWAY", "Executing hardware tool '{}' with args: {}", name, args);

    match name {
        "get_system_telemetry" => {
            let cfg = crate::system::config::load_config();
            let switches = crate::system::switches::get_switches_state();
            Ok(json!({
                "power_mode": cfg.power_mode,
                "fan_boost": cfg.fan_boost,
                "battery_limit": cfg.battery_limit,
                "switches": {
                    "camera": switches.camera,
                    "touchpad": switches.touchpad,
                    "mic": switches.mic,
                    "win_key": switches.win_key
                }
            }))
        }
        "set_fan_boost" => {
            let enable = args.get("enable")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "参数 'enable' (boolean) 缺失".to_string())?;

            let hal = crate::hal::get_hardware_driver();
            let _res = hal.set_fan_boost(enable).map_err(|e| format!("硬件下发强冷失败: {}", e))?;
            
            let mut cfg = crate::system::config::load_config();
            cfg.fan_boost = enable;
            crate::system::config::save_config(&cfg);

            crate::system::broadcast::broadcast_all("fan-boost-changed", &json!(enable));

            Ok(json!({
                "success": true,
                "fan_boost": enable,
                "message": if enable { "已成功开启全速强冷模式 (Fan Boost 暴走中)" } else { "已成功关闭强冷，恢复智能温控" }
            }))
        }
        "set_power_mode" => {
            let mode = args.get("mode")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8)
                .ok_or_else(|| "参数 'mode' (1~4) 缺失".to_string())?;

            if mode < 1 || mode > 4 {
                return Err("模式参数越界，仅支持 1 (办公), 2 (均衡), 3 (狂暴), 4 (自定义)".to_string());
            }

            crate::system::power_mode::apply_full_power_mode(mode);

            let mode_name = match mode {
                1 => "办公模式 (Office / 静音省电)",
                2 => "均衡模式 (Balance / 日常平衡)",
                3 => "狂暴模式 (Turbo / 极速性能释放)",
                4 => "自定义模式 (Custom)",
                _ => "未知模式"
            };

            crate::system::broadcast::broadcast_all("physical-mode-switched", &json!(mode));

            Ok(json!({
                "success": true,
                "power_mode": mode,
                "mode_name": mode_name,
                "message": format!("已成功切换为{}", mode_name)
            }))
        }
        "switch_refresh_rate" => {
            let hz = args.get("hz")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| "参数 'hz' 缺失".to_string())?;

            crate::system::display::set_refresh_rate(hz)
                .map_err(|e| format!("切换显示器刷新率失败: {}", e))?;

            Ok(json!({
                "success": true,
                "refresh_rate_hz": hz,
                "message": format!("已成功将屏幕刷新率切换为 {} Hz", hz)
            }))
        }
        "set_battery_limit" => {
            let limit = args.get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8)
                .ok_or_else(|| "参数 'limit' 缺失".to_string())?;

            let health_mode = match limit {
                100 => crate::hal::traits::BatteryHealthMode::LongLife,
                80 => crate::hal::traits::BatteryHealthMode::Balance,
                60 => crate::hal::traits::BatteryHealthMode::Station,
                _ => return Err("电池养护限额仅支持 60, 80 或 100".to_string()),
            };

            let hal = crate::hal::get_hardware_driver();
            hal.set_battery_health(health_mode).map_err(|e| format!("硬件设置电池养护限额失败: {:?}", e))?;

            let mut cfg = crate::system::config::load_config();
            cfg.battery_limit = limit;
            crate::system::config::save_config(&cfg);

            crate::system::broadcast::broadcast_all("battery-limit-changed", &json!(limit));

            Ok(json!({
                "success": true,
                "battery_limit": limit,
                "message": format!("已成功将电池充电养护上限设置为 {}%", limit)
            }))
        }
        "set_device_switch" => {
            let device = args.get("device")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "参数 'device' 缺失".to_string())?;
            let enabled = args.get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "参数 'enabled' 缺失".to_string())?;

            let switch_key = match device {
                "camera" => "camera",
                "mic" | "microphone" => "microphone",
                "touchpad" => "touchpad",
                "winlock" | "win_lock" => "winlock",
                _ => return Err(format!("不支持的设备开关: {}", device))
            };

            crate::system::switches::set_switch(switch_key, enabled)
                .map_err(|e| format!("切换设备开关失败: {}", e))?;

            Ok(json!({
                "success": true,
                "device": device,
                "enabled": enabled,
                "message": format!("已成功{}设备 {}", if enabled { "开启" } else { "关闭" }, device)
            }))
        }
        _ => Err(format!("未识别的硬件工具指令: '{}'", name))
    }
}
