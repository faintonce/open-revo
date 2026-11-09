// OpenRevo - Universal Watcher & Trigger Hub (公共监听与轮询注册中心)
// Provides a centralized, microkernel-managed reactive monitoring engine.
// Allows plugins to register telemetry threshold conditions, foreground application watchers,
// power/hardware edge events, and raw EC diff tracking without running dedicated polling loops in Webview.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::commands::system_cmd::HardwareState;

fn default_true() -> bool {
    true
}

fn default_cooldown() -> u64 {
    5000 // 5 seconds default cooldown between triggers
}

/// Helper deserializer to parse u16 from either an integer or hex/dec string (e.g. 1873, "0x0751", "751", "1873")
pub fn deserialize_hex_or_dec_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct HexOrDecVisitor;
    impl<'de> serde::de::Visitor<'de> for HexOrDecVisitor {
        type Value = u16;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or hex/dec string like '0x0751', '751' or '1873'")
        }
        fn visit_u64<E>(self, value: u64) -> Result<u16, E>
        where
            E: serde::de::Error,
        {
            if value <= 0xFFFF {
                Ok(value as u16)
            } else {
                Err(E::custom(format!("u16 overflow: {}", value)))
            }
        }
        fn visit_i64<E>(self, value: i64) -> Result<u16, E>
        where
            E: serde::de::Error,
        {
            if value >= 0 && value <= 0xFFFF {
                Ok(value as u16)
            } else {
                Err(E::custom(format!("u16 overflow or negative: {}", value)))
            }
        }
        fn visit_str<E>(self, value: &str) -> Result<u16, E>
        where
            E: serde::de::Error,
        {
            let trimmed = value.trim();
            if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                u16::from_str_radix(&trimmed[2..], 16).map_err(E::custom)
            } else if trimmed.chars().all(|c| c.is_ascii_hexdigit()) && trimmed.len() <= 4 && trimmed.chars().any(|c| c.is_alphabetic()) {
                u16::from_str_radix(trimmed, 16).map_err(E::custom)
            } else {
                trimmed.parse::<u16>().map_err(E::custom)
            }
        }
    }
    deserializer.deserialize_any(HexOrDecVisitor)
}

/// Helper deserializer to parse u8 from either an integer or hex string (e.g. 16, "0x10", "16")
pub fn deserialize_hex_or_dec_u8<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct HexOrDecU8Visitor;
    impl<'de> serde::de::Visitor<'de> for HexOrDecU8Visitor {
        type Value = u8;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or hex string like '0x10' or '16'")
        }
        fn visit_u64<E>(self, value: u64) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            if value <= 0xFF {
                Ok(value as u8)
            } else {
                Err(E::custom(format!("u8 overflow: {}", value)))
            }
        }
        fn visit_i64<E>(self, value: i64) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            if value >= 0 && value <= 0xFF {
                Ok(value as u8)
            } else {
                Err(E::custom(format!("u8 overflow or negative: {}", value)))
            }
        }
        fn visit_str<E>(self, value: &str) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            let trimmed = value.trim();
            if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                u8::from_str_radix(&trimmed[2..], 16).map_err(E::custom)
            } else {
                trimmed.parse::<u8>().map_err(E::custom)
            }
        }
    }
    deserializer.deserialize_any(HexOrDecU8Visitor)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessTriggerMode {
    Enter,
    Exit,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum WatcherCondition {
    /// 1. Telemetry and hardware state threshold condition
    #[serde(rename = "telemetry")]
    Telemetry {
        #[serde(default)]
        power_mode: Option<u8>,
        #[serde(default)]
        power_mode_in: Option<Vec<u8>>,
        #[serde(default)]
        cpu_temp_gt: Option<i32>,
        #[serde(default)]
        cpu_temp_gte: Option<i32>,
        #[serde(default)]
        cpu_temp_lt: Option<i32>,
        #[serde(default)]
        gpu_temp_gt: Option<u32>,
        #[serde(default)]
        gpu_temp_gte: Option<u32>,
        #[serde(default)]
        is_dc_online: Option<bool>,
        #[serde(default)]
        fan_boost: Option<bool>,
    },

    /// 2. Active foreground application or game process
    #[serde(rename = "foreground_process")]
    ForegroundProcess {
        targets: Vec<String>,
        #[serde(default)]
        trigger_on: Option<ProcessTriggerMode>,
    },

    /// 3. Power and AC/DC transition events
    #[serde(rename = "power_event")]
    PowerEvent {
        event: String, // "ac_connected", "ac_disconnected", "screen_off", "screen_on"
    },

    /// 4. Controlled Raw EC register diff / change
    #[serde(rename = "raw_ec_diff")]
    RawEcDiff {
        #[serde(deserialize_with = "deserialize_hex_or_dec_u16")]
        address: u16,
        #[serde(default, deserialize_with = "deserialize_optional_hex_or_dec_u8")]
        mask: Option<u8>,
    },
}

pub fn deserialize_optional_hex_or_dec_u8<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Helper(#[serde(deserialize_with = "deserialize_hex_or_dec_u8")] u8);
    let opt = Option::<Helper>::deserialize(deserializer)?;
    Ok(opt.map(|h| h.0))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action")]
pub enum WatcherAction {
    #[serde(rename = "set_power_mode")]
    SetPowerMode { mode: u8 },

    #[serde(rename = "set_fan_boost")]
    SetFanBoost { enable: bool },

    #[serde(rename = "delay_ms")]
    DelayMs { duration: u64 },

    #[serde(rename = "raw_ec_write")]
    RawEcWrite {
        #[serde(deserialize_with = "deserialize_hex_or_dec_u16")]
        address: u16,
        #[serde(deserialize_with = "deserialize_hex_or_dec_u8")]
        value: u8,
    },

    #[serde(rename = "raw_ec_modify_mask")]
    RawEcModifyMask {
        #[serde(deserialize_with = "deserialize_hex_or_dec_u16")]
        address: u16,
        #[serde(deserialize_with = "deserialize_hex_or_dec_u8")]
        mask: u8,
        value: bool,
    },

    #[serde(rename = "plugin_event")]
    PluginEvent {
        event_name: String,
        payload: serde_json::Value,
    },

    #[serde(rename = "notify_osd")]
    NotifyOsd {
        title: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WatcherDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub plugin_id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_cooldown")]
    pub cooldown_ms: u64,
    #[serde(alias = "when")]
    pub condition: WatcherCondition,
    #[serde(default)]
    pub actions: Vec<WatcherAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherInfo {
    pub id: String,
    pub name: String,
    pub plugin_id: Option<String>,
    pub enabled: bool,
    pub cooldown_ms: u64,
    pub condition_type: String,
    pub action_count: usize,
    pub last_triggered_secs_ago: Option<u64>,
}

struct WatcherRecord {
    definition: WatcherDefinition,
    last_triggered: Option<Instant>,
    last_ec_val: Option<u8>,
    last_matched: bool,
}

static GLOBAL_WATCHERS: OnceLock<Mutex<HashMap<String, WatcherRecord>>> = OnceLock::new();
static GLOBAL_APP: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();
static PROCESS_WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);

fn get_watchers_map() -> &'static Mutex<HashMap<String, WatcherRecord>> {
    GLOBAL_WATCHERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_app_handle() -> Option<AppHandle> {
    let cell = GLOBAL_APP.get_or_init(|| Mutex::new(None));
    cell.lock().clone()
}

pub fn init_watcher_hub(app: AppHandle) {
    let cell = GLOBAL_APP.get_or_init(|| Mutex::new(None));
    *cell.lock() = Some(app);
    start_background_process_watcher();
}

/// Register a watcher definition into the Hub
pub fn register_watcher(def: WatcherDefinition) -> bool {
    let mut map = get_watchers_map().lock();
    let id = def.id.clone();
    crate::log_info!("WATCHER_HUB", "Registered watcher '{}' [{}] for plugin '{:?}'", def.name, id, def.plugin_id);
    map.insert(
        id,
        WatcherRecord {
            definition: def,
            last_triggered: None,
            last_ec_val: None,
            last_matched: false,
        },
    );
    true
}

/// Unregister a single watcher by ID
pub fn unregister_watcher(id: &str) -> bool {
    let mut map = get_watchers_map().lock();
    if map.remove(id).is_some() {
        crate::log_info!("WATCHER_HUB", "Unregistered watcher '{}'", id);
        true
    } else {
        false
    }
}

/// Unregister all watchers belonging to a specific plugin ID
pub fn unregister_by_plugin(plugin_id: &str) -> usize {
    let mut map = get_watchers_map().lock();
    let before = map.len();
    map.retain(|_, record| record.definition.plugin_id.as_deref() != Some(plugin_id));
    let removed = before.saturating_sub(map.len());
    if removed > 0 {
        crate::log_info!("WATCHER_HUB", "Cleaned up {} watchers for unloaded plugin '{}'", removed, plugin_id);
    }
    removed
}

/// Load a batch of watchers from a plugin manifest
pub fn load_plugin_watchers(plugin_id: &str, watchers: Vec<WatcherDefinition>) {
    unregister_by_plugin(plugin_id);
    let count = watchers.len();
    for mut w in watchers {
        w.plugin_id = Some(plugin_id.to_string());
        register_watcher(w);
    }
    crate::log_info!("WATCHER_HUB", "Loaded {} declarative watchers for plugin '{}'", count, plugin_id);
}

/// Query all active registered watchers info for frontend inspection
pub fn get_all_watchers() -> Vec<WatcherInfo> {
    let map = get_watchers_map().lock();
    let now = Instant::now();
    map.values()
        .map(|r| {
            let cond_type = match &r.definition.condition {
                WatcherCondition::Telemetry { .. } => "telemetry",
                WatcherCondition::ForegroundProcess { .. } => "foreground_process",
                WatcherCondition::PowerEvent { .. } => "power_event",
                WatcherCondition::RawEcDiff { .. } => "raw_ec_diff",
            };
            let secs_ago = r.last_triggered.map(|t| now.duration_since(t).as_secs());
            WatcherInfo {
                id: r.definition.id.clone(),
                name: r.definition.name.clone(),
                plugin_id: r.definition.plugin_id.clone(),
                enabled: r.definition.enabled,
                cooldown_ms: r.definition.cooldown_ms,
                condition_type: cond_type.to_string(),
                action_count: r.definition.actions.len(),
                last_triggered_secs_ago: secs_ago,
            }
        })
        .collect()
}

/// Check if any active enabled watcher needs telemetry polling
pub fn has_active_telemetry_watchers() -> bool {
    let map = get_watchers_map().lock();
    map.values().any(|r| {
        r.definition.enabled && matches!(r.definition.condition, WatcherCondition::Telemetry { .. })
    })
}

/// Check if any active telemetry watcher requires GPU sensors
pub fn should_query_gpu_for_watchers() -> bool {
    let map = get_watchers_map().lock();
    map.values().any(|r| {
        if !r.definition.enabled {
            return false;
        }
        if let WatcherCondition::Telemetry { gpu_temp_gt, gpu_temp_gte, .. } = &r.definition.condition {
            gpu_temp_gt.is_some() || gpu_temp_gte.is_some()
        } else {
            false
        }
    })
}

/// Evaluate telemetry watchers against current hardware state
pub fn evaluate_telemetry_watchers(state: &HardwareState) {
    let mut triggers: Vec<(WatcherDefinition, serde_json::Value)> = Vec::new();
    let now = Instant::now();

    {
        let mut map = get_watchers_map().lock();
        for record in map.values_mut() {
            if !record.definition.enabled {
                continue;
            }

            if let WatcherCondition::Telemetry {
                power_mode,
                power_mode_in,
                cpu_temp_gt,
                cpu_temp_gte,
                cpu_temp_lt,
                gpu_temp_gt,
                gpu_temp_gte,
                is_dc_online,
                fan_boost,
            } = &record.definition.condition
            {
                let mut matched = true;

                if let Some(pm) = power_mode {
                    if state.power_mode != *pm {
                        matched = false;
                    }
                }
                if let Some(list) = power_mode_in {
                    if !list.contains(&state.power_mode) {
                        matched = false;
                    }
                }
                if let Some(t) = cpu_temp_gt {
                    if state.cpu_temp <= *t {
                        matched = false;
                    }
                }
                if let Some(t) = cpu_temp_gte {
                    if state.cpu_temp < *t {
                        matched = false;
                    }
                }
                if let Some(t) = cpu_temp_lt {
                    if state.cpu_temp >= *t {
                        matched = false;
                    }
                }
                if let Some(t) = gpu_temp_gt {
                    if state.gpu_temp <= *t {
                        matched = false;
                    }
                }
                if let Some(t) = gpu_temp_gte {
                    if state.gpu_temp < *t {
                        matched = false;
                    }
                }
                if let Some(dc) = is_dc_online {
                    if state.is_dc_online != *dc {
                        matched = false;
                    }
                }
                if let Some(fb) = fan_boost {
                    if state.fan_boost != *fb {
                        matched = false;
                    }
                }

                if matched {
                    let can_trigger = match record.last_triggered {
                        None => true,
                        Some(last) => now.duration_since(last) >= Duration::from_millis(record.definition.cooldown_ms),
                    };

                    if can_trigger {
                        record.last_triggered = Some(now);
                        record.last_matched = true;
                        triggers.push((
                            record.definition.clone(),
                            serde_json::json!({
                                "cpu_temp": state.cpu_temp,
                                "gpu_temp": state.gpu_temp,
                                "power_mode": state.power_mode,
                                "fan_boost": state.fan_boost,
                                "is_dc_online": state.is_dc_online
                            }),
                        ));
                    }
                } else {
                    record.last_matched = false;
                }
            }
        }
    }

    for (def, details) in triggers {
        dispatch_watcher_trigger(def, "telemetry", details);
    }
}

/// Notify Hub of external power events (AC plug, lid, screen sleep)
pub fn notify_power_event(event_name: &str) {
    let mut triggers: Vec<WatcherDefinition> = Vec::new();
    let now = Instant::now();

    {
        let mut map = get_watchers_map().lock();
        for record in map.values_mut() {
            if !record.definition.enabled {
                continue;
            }
            if let WatcherCondition::PowerEvent { event } = &record.definition.condition {
                if event.eq_ignore_ascii_case(event_name) {
                    let can_trigger = match record.last_triggered {
                        None => true,
                        Some(last) => now.duration_since(last) >= Duration::from_millis(record.definition.cooldown_ms),
                    };
                    if can_trigger {
                        record.last_triggered = Some(now);
                        triggers.push(record.definition.clone());
                    }
                }
            }
        }
    }

    for def in triggers {
        dispatch_watcher_trigger(
            def,
            "power_event",
            serde_json::json!({ "event": event_name }),
        );
    }
}

/// Execute watcher actions and broadcast events asynchronously
pub fn dispatch_watcher_trigger(def: WatcherDefinition, condition_type: &'static str, details: serde_json::Value) {
    crate::log_info!(
        "WATCHER_HUB",
        "Watcher '{}' triggered (condition: {}) with {} actions",
        def.name,
        condition_type,
        def.actions.len()
    );

    let app = get_app_handle();

    // 1. Emit universal event to Tauri bus & Native Wry
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let payload = serde_json::json!({
        "watcher_id": def.id,
        "watcher_name": def.name,
        "plugin_id": def.plugin_id,
        "condition_type": condition_type,
        "timestamp_ms": ts,
        "details": details
    });
    if let Some(ref handle) = app {
        let _ = handle.emit("openrevo-watcher-triggered", &payload);
        let _ = handle.emit(&format!("watcher-event:{}", def.id), &payload);
    }
    crate::system::native_wry::broadcast_event("openrevo-watcher-triggered", &payload);
    crate::system::native_wry::broadcast_event(&format!("watcher-event:{}", def.id), &payload);

    // 2. Dispatch sequence of declared actions in separate thread
    if !def.actions.is_empty() {
        let actions = def.actions.clone();
        let watcher_name = def.name.clone();
        std::thread::Builder::new()
            .name(format!("watcher-action-{}", def.id))
            .spawn(move || {
                for act in actions {
                    execute_single_action(&act, &watcher_name);
                }
            })
            .ok();
    }
}

fn detect_active_raw_protocol() -> crate::system::raw_gateway::RawEcProtocol {
    crate::system::raw_gateway::detect_auto_raw_protocol()
}

fn execute_single_action(act: &WatcherAction, watcher_name: &str) {
    match act {
        WatcherAction::SetPowerMode { mode } => {
            crate::log_info!("WATCHER_ACT", "[{}] Setting power mode to {}", watcher_name, mode);
            crate::system::power_mode::apply_full_power_mode(*mode);
        }
        WatcherAction::SetFanBoost { enable } => {
            crate::log_info!("WATCHER_ACT", "[{}] Setting fan boost to {}", watcher_name, enable);
            let hal = crate::hal::get_hardware_driver();
            let mut cfg = crate::system::config::load_config();
            if hal.set_fan_boost(*enable).is_ok() {
                cfg.fan_boost = *enable;
                crate::system::config::save_config(&cfg);
                if let Some(handle) = get_app_handle() {
                    let _ = handle.emit("fan-boost-changed", *enable);
                    let _ = handle.emit_to("osd", "fan-boost-changed", *enable);
                    let _ = handle.emit_to("main", "fan-boost-changed", *enable);
                }
            }
        }
        WatcherAction::DelayMs { duration } => {
            crate::log_debug!("WATCHER_ACT", "[{}] Delaying for {} ms", watcher_name, duration);
            std::thread::sleep(Duration::from_millis(*duration));
        }
        WatcherAction::RawEcWrite { address, value } => {
            crate::log_info!("WATCHER_ACT", "[{}] Raw EC Write 0x{:04X} = 0x{:02X}", watcher_name, address, value);
            let req = crate::system::raw_gateway::RawEcWriteRequest {
                protocol: detect_active_raw_protocol(),
                address: *address,
                value: *value,
            };
            if let Err(e) = crate::system::raw_gateway::execute_raw_write(req) {
                crate::log_warn!("WATCHER_ACT", "[{}] Raw EC write failed: {}", watcher_name, e);
            }
        }
        WatcherAction::RawEcModifyMask { address, mask, value } => {
            if let Err(e) = crate::system::raw_gateway::shield::validate_write_operation(*address, 0) {
                crate::log_warn!("WATCHER_ACT", "[{}] Blast shield blocked EC modify on 0x{:04X}: {}", watcher_name, address, e);
                return;
            }
            let proto = detect_active_raw_protocol();
            let read_req = crate::system::raw_gateway::RawEcReadRequest {
                protocol: proto,
                address: *address,
            };
            if let Ok(cur) = crate::system::raw_gateway::execute_raw_read(read_req) {
                let next = if *value { cur | mask } else { cur & !mask };
                crate::log_info!("WATCHER_ACT", "[{}] Modifying EC 0x{:04X} mask 0x{:02X} (val: {}): 0x{:02X} -> 0x{:02X} via {:?}", watcher_name, address, mask, value, cur, next, proto);
                let write_req = crate::system::raw_gateway::RawEcWriteRequest {
                    protocol: proto,
                    address: *address,
                    value: next,
                };
                let _ = crate::system::raw_gateway::execute_raw_write(write_req);
            } else {
                crate::log_warn!("WATCHER_ACT", "[{}] Failed to read EC 0x{:04X} for bitmask modification", watcher_name, address);
            }
        }
        WatcherAction::PluginEvent { event_name, payload } => {
            if let Some(handle) = get_app_handle() {
                let _ = handle.emit(event_name, payload);
            }
            crate::system::native_wry::broadcast_event(event_name, payload);
        }
        WatcherAction::NotifyOsd { title, message } => {
            if let Some(handle) = get_app_handle() {
                let _ = handle.emit("notify-osd-custom", serde_json::json!({
                    "title": title,
                    "message": message
                }));
            }
        }
    }
}

/// Lightweight thread watching foreground application process & raw EC diffs
fn start_background_process_watcher() {
    if PROCESS_WATCHER_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::Builder::new()
        .name("openrevo-watcher-process-scanner".to_string())
        .spawn(move || {
            crate::log_info!("WATCHER_HUB", "Dedicated process & EC diff watcher loop initialized.");
            let mut last_proc_name = String::new();

            loop {
                // If system reboot is underway, freeze
                if crate::driver::uefi_nvram::is_reboot_in_progress() {
                    std::thread::sleep(Duration::from_millis(2000));
                    continue;
                }

                // Check if any watchers need process or EC diff scanning
                let (has_proc_watchers, has_ec_watchers) = {
                    let map = get_watchers_map().lock();
                    let p = map.values().any(|r| r.definition.enabled && matches!(r.definition.condition, WatcherCondition::ForegroundProcess { .. }));
                    let e = map.values().any(|r| r.definition.enabled && matches!(r.definition.condition, WatcherCondition::RawEcDiff { .. }));
                    (p, e)
                };

                if !has_proc_watchers && !has_ec_watchers {
                    std::thread::sleep(Duration::from_millis(1500));
                    continue;
                }

                // 1. Scan Foreground Window Process
                if has_proc_watchers {
                    let current_proc = get_active_foreground_process_name().unwrap_or_default().to_lowercase();
                    if !current_proc.is_empty() && current_proc != last_proc_name {
                        evaluate_foreground_process_change(&last_proc_name, &current_proc);
                        last_proc_name = current_proc;
                    }
                }

                // 2. Scan Raw EC Diffs
                if has_ec_watchers {
                    evaluate_raw_ec_diffs();
                }

                std::thread::sleep(Duration::from_millis(800));
            }
        })
        .expect("Failed to spawn process watcher thread");
}

fn evaluate_foreground_process_change(prev_proc: &str, curr_proc: &str) {
    let mut triggers: Vec<(WatcherDefinition, serde_json::Value)> = Vec::new();
    let now = Instant::now();

    {
        let mut map = get_watchers_map().lock();
        for record in map.values_mut() {
            if !record.definition.enabled {
                continue;
            }
            if let WatcherCondition::ForegroundProcess { targets, trigger_on } = &record.definition.condition {
                let mode = trigger_on.unwrap_or(ProcessTriggerMode::Enter);
                let matched_curr = targets.iter().any(|t| t.to_lowercase() == curr_proc);
                let matched_prev = targets.iter().any(|t| t.to_lowercase() == prev_proc);

                let should_fire = match mode {
                    ProcessTriggerMode::Enter => matched_curr && !matched_prev,
                    ProcessTriggerMode::Exit => !matched_curr && matched_prev,
                    ProcessTriggerMode::Both => (matched_curr && !matched_prev) || (!matched_curr && matched_prev),
                };

                if should_fire {
                    let can_trigger = match record.last_triggered {
                        None => true,
                        Some(last) => now.duration_since(last) >= Duration::from_millis(record.definition.cooldown_ms),
                    };
                    if can_trigger {
                        record.last_triggered = Some(now);
                        triggers.push((
                            record.definition.clone(),
                            serde_json::json!({
                                "process": curr_proc,
                                "previous_process": prev_proc,
                                "transition": if matched_curr { "enter" } else { "exit" }
                            }),
                        ));
                    }
                }
            }
        }
    }

    for (def, details) in triggers {
        dispatch_watcher_trigger(def, "foreground_process", details);
    }
}

fn evaluate_raw_ec_diffs() {
    let mut triggers: Vec<(WatcherDefinition, serde_json::Value)> = Vec::new();
    let now = Instant::now();

    {
        let mut map = get_watchers_map().lock();
        for record in map.values_mut() {
            if !record.definition.enabled {
                continue;
            }
            if let WatcherCondition::RawEcDiff { address, mask } = &record.definition.condition {
                let proto = detect_active_raw_protocol();
                let read_req = crate::system::raw_gateway::RawEcReadRequest {
                    protocol: proto,
                    address: *address,
                };
                if let Ok(mut val) = crate::system::raw_gateway::execute_raw_read(read_req) {
                    if let Some(m) = mask {
                        val &= m;
                    }

                    match record.last_ec_val {
                        Some(last_val) if last_val != val => {
                            let can_trigger = match record.last_triggered {
                                None => true,
                                Some(last) => now.duration_since(last) >= Duration::from_millis(record.definition.cooldown_ms),
                            };
                            if can_trigger {
                                record.last_triggered = Some(now);
                                record.last_ec_val = Some(val);
                                triggers.push((
                                    record.definition.clone(),
                                    serde_json::json!({
                                        "address": address,
                                        "old_val": last_val,
                                        "new_val": val
                                    }),
                                ));
                            }
                        }
                        None => {
                            record.last_ec_val = Some(val);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    for (def, details) in triggers {
        dispatch_watcher_trigger(def, "raw_ec_diff", details);
    }
}

#[cfg(windows)]
fn get_active_foreground_process_name() -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == std::ptr::null_mut() {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut path_buf = [0u16; 1024];
        let mut size = path_buf.len() as u32;

        let res = QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(path_buf.as_mut_ptr()),
            &mut size,
        );

        let _ = CloseHandle(process_handle);

        if res.is_ok() && size > 0 {
            let full_path = String::from_utf16_lossy(&path_buf[..size as usize]);
            let file_name = std::path::Path::new(&full_path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(&full_path)
                .to_string();
            Some(file_name)
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_active_foreground_process_name() -> Option<String> {
    None
}
