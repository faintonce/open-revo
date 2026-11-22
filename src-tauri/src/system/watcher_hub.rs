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

fn default_pulse_delay() -> u64 {
    200 // 200ms default pulse hold duration
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

/// Helper deserializer to parse u64 from either an integer or hex/dec string
pub fn deserialize_hex_or_dec_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct HexOrDecU64Visitor;
    impl<'de> serde::de::Visitor<'de> for HexOrDecU64Visitor {
        type Value = u64;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or string representing u64")
        }
        fn visit_u64<E>(self, value: u64) -> Result<u64, E>
        where
            E: serde::de::Error,
        {
            Ok(value)
        }
        fn visit_i64<E>(self, value: i64) -> Result<u64, E>
        where
            E: serde::de::Error,
        {
            if value >= 0 {
                Ok(value as u64)
            } else {
                Err(E::custom(format!("u64 cannot be negative: {}", value)))
            }
        }
        fn visit_str<E>(self, value: &str) -> Result<u64, E>
        where
            E: serde::de::Error,
        {
            let trimmed = value.trim();
            if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                u64::from_str_radix(&trimmed[2..], 16).map_err(E::custom)
            } else {
                trimmed.parse::<u64>().map_err(E::custom)
            }
        }
    }
    deserializer.deserialize_any(HexOrDecU64Visitor)
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
        #[serde(default)]
        battery_level_gt: Option<u8>,
        #[serde(default)]
        battery_level_gte: Option<u8>,
        #[serde(default)]
        battery_level_lt: Option<u8>,
        #[serde(default)]
        battery_level_lte: Option<u8>,
        #[serde(default)]
        is_charging: Option<bool>,
        #[serde(default)]
        is_ac_online: Option<bool>,
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

    /// 5. Periodic Interval condition (runs every N ms in microkernel background)
    #[serde(rename = "interval")]
    Interval {
        #[serde(deserialize_with = "deserialize_hex_or_dec_u64")]
        every_ms: u64,
    },

    /// 6. Core Lifecycle condition (runs on microkernel startup readiness and system wake-up)
    #[serde(rename = "startup", alias = "boot", alias = "lifecycle", alias = "always")]
    Startup,

    /// 7. Composite Any condition (Logical OR: fires if ANY child condition matches/triggers)
    #[serde(rename = "any", alias = "or")]
    Any {
        conditions: Vec<WatcherCondition>,
    },

    /// 8. Composite All condition (Logical AND: fires if ALL child conditions match)
    #[serde(rename = "all", alias = "and")]
    All {
        conditions: Vec<WatcherCondition>,
    },
}

impl WatcherCondition {
    pub fn condition_type_str(&self) -> String {
        match self {
            WatcherCondition::Telemetry { .. } => "telemetry".to_string(),
            WatcherCondition::ForegroundProcess { .. } => "foreground_process".to_string(),
            WatcherCondition::PowerEvent { .. } => "power_event".to_string(),
            WatcherCondition::RawEcDiff { .. } => "raw_ec_diff".to_string(),
            WatcherCondition::Interval { .. } => "interval".to_string(),
            WatcherCondition::Startup => "startup".to_string(),
            WatcherCondition::Any { conditions } => {
                let inner = conditions.iter().map(|c| c.condition_type_str()).collect::<Vec<_>>().join("|");
                format!("any({})", inner)
            }
            WatcherCondition::All { conditions } => {
                let inner = conditions.iter().map(|c| c.condition_type_str()).collect::<Vec<_>>().join("&");
                format!("all({})", inner)
            }
        }
    }

    pub fn matches_startup(&self) -> bool {
        match self {
            WatcherCondition::Startup => true,
            WatcherCondition::Any { conditions } => conditions.iter().any(|c| c.matches_startup()),
            WatcherCondition::All { conditions } => conditions.iter().all(|c| c.matches_startup()),
            _ => false,
        }
    }

    pub fn matches_power_event(&self, event_name: &str) -> bool {
        match self {
            WatcherCondition::PowerEvent { event } => event.eq_ignore_ascii_case(event_name),
            WatcherCondition::Any { conditions } => conditions.iter().any(|c| c.matches_power_event(event_name)),
            WatcherCondition::All { conditions } => conditions.iter().any(|c| c.matches_power_event(event_name)),
            _ => false,
        }
    }

    pub fn should_assert_on_lifecycle(&self) -> bool {
        match self {
            WatcherCondition::ForegroundProcess { .. } => false,
            WatcherCondition::RawEcDiff { .. } => false,
            // Periodic interval watchers must NOT fire as one-off startup assertions to avoid racing with startup guards
            WatcherCondition::Interval { .. } => false,
            WatcherCondition::Any { conditions } => conditions.iter().any(|c| c.should_assert_on_lifecycle()),
            WatcherCondition::All { conditions } => conditions.iter().all(|c| c.should_assert_on_lifecycle()),
            _ => true,
        }
    }

    pub fn has_foreground_process(&self) -> bool {
        match self {
            WatcherCondition::ForegroundProcess { .. } => true,
            WatcherCondition::Any { conditions } | WatcherCondition::All { conditions } => {
                conditions.iter().any(|c| c.has_foreground_process())
            }
            _ => false,
        }
    }

    pub fn has_raw_ec_diff(&self) -> bool {
        match self {
            WatcherCondition::RawEcDiff { .. } => true,
            WatcherCondition::Any { conditions } | WatcherCondition::All { conditions } => {
                conditions.iter().any(|c| c.has_raw_ec_diff())
            }
            _ => false,
        }
    }

    pub fn has_interval(&self) -> bool {
        match self {
            WatcherCondition::Interval { .. } => true,
            WatcherCondition::Any { conditions } | WatcherCondition::All { conditions } => {
                conditions.iter().any(|c| c.has_interval())
            }
            _ => false,
        }
    }

    pub fn get_min_interval_ms(&self) -> Option<u64> {
        match self {
            WatcherCondition::Interval { every_ms } => Some(*every_ms),
            WatcherCondition::Any { conditions } | WatcherCondition::All { conditions } => {
                conditions.iter().filter_map(|c| c.get_min_interval_ms()).min()
            }
            _ => None,
        }
    }

    pub fn has_telemetry(&self) -> bool {
        match self {
            WatcherCondition::Telemetry { .. } => true,
            WatcherCondition::Any { conditions } | WatcherCondition::All { conditions } => {
                conditions.iter().any(|c| c.has_telemetry())
            }
            _ => false,
        }
    }

    pub fn needs_gpu_sensor(&self) -> bool {
        match self {
            WatcherCondition::Telemetry { gpu_temp_gt, gpu_temp_gte, .. } => {
                gpu_temp_gt.is_some() || gpu_temp_gte.is_some()
            }
            WatcherCondition::Any { conditions } | WatcherCondition::All { conditions } => {
                conditions.iter().any(|c| c.needs_gpu_sensor())
            }
            _ => false,
        }
    }

    pub fn matches_telemetry(&self, state: &HardwareState) -> bool {
        match self {
            WatcherCondition::Telemetry {
                power_mode,
                power_mode_in,
                cpu_temp_gt,
                cpu_temp_gte,
                cpu_temp_lt,
                gpu_temp_gt,
                gpu_temp_gte,
                is_dc_online,
                fan_boost,
                battery_level_gt,
                battery_level_gte,
                battery_level_lt,
                battery_level_lte,
                is_charging,
                is_ac_online,
            } => {
                let mut matched = true;
                if let Some(pm) = power_mode { if state.power_mode != *pm { matched = false; } }
                if let Some(list) = power_mode_in { if !list.contains(&state.power_mode) { matched = false; } }
                if let Some(t) = cpu_temp_gt { if state.cpu_temp <= *t { matched = false; } }
                if let Some(t) = cpu_temp_gte { if state.cpu_temp < *t { matched = false; } }
                if let Some(t) = cpu_temp_lt { if state.cpu_temp >= *t { matched = false; } }
                if let Some(t) = gpu_temp_gt { if state.gpu_temp <= *t { matched = false; } }
                if let Some(t) = gpu_temp_gte { if state.gpu_temp < *t { matched = false; } }
                if let Some(dc) = is_dc_online { if state.is_dc_online != *dc { matched = false; } }
                if let Some(fb) = fan_boost { if state.fan_boost != *fb { matched = false; } }
                if let Some(t) = battery_level_gt { if state.battery_level <= *t { matched = false; } }
                if let Some(t) = battery_level_gte { if state.battery_level < *t { matched = false; } }
                if let Some(t) = battery_level_lt { if state.battery_level >= *t { matched = false; } }
                if let Some(t) = battery_level_lte { if state.battery_level > *t { matched = false; } }
                if let Some(ch) = is_charging { if state.is_charging != *ch { matched = false; } }
                if let Some(ac) = is_ac_online { if state.is_ac_online != *ac { matched = false; } }
                matched
            }
            WatcherCondition::Any { conditions } => conditions.iter().any(|c| c.matches_telemetry(state)),
            WatcherCondition::All { conditions } => conditions.iter().all(|c| c.matches_telemetry(state)),
            _ => false,
        }
    }

    pub fn matches_foreground_process(&self, prev_proc: &str, curr_proc: &str) -> bool {
        match self {
            WatcherCondition::ForegroundProcess { targets, trigger_on } => {
                let mode = trigger_on.unwrap_or(ProcessTriggerMode::Enter);
                let matched_curr = targets.iter().any(|t| t.to_lowercase() == curr_proc);
                let matched_prev = targets.iter().any(|t| t.to_lowercase() == prev_proc);
                match mode {
                    ProcessTriggerMode::Enter => matched_curr && !matched_prev,
                    ProcessTriggerMode::Exit => !matched_curr && matched_prev,
                    ProcessTriggerMode::Both => (matched_curr && !matched_prev) || (!matched_curr && matched_prev),
                }
            }
            WatcherCondition::Any { conditions } => conditions.iter().any(|c| c.matches_foreground_process(prev_proc, curr_proc)),
            WatcherCondition::All { conditions } => conditions.iter().all(|c| c.matches_foreground_process(prev_proc, curr_proc)),
            _ => false,
        }
    }
}

pub fn deserialize_watcher_condition<'de, D>(deserializer: D) -> Result<WatcherCondition, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ConditionHelper {
        Single(WatcherCondition),
        List(Vec<WatcherCondition>),
    }

    match ConditionHelper::deserialize(deserializer)? {
        ConditionHelper::Single(c) => Ok(c),
        ConditionHelper::List(list) => Ok(WatcherCondition::Any { conditions: list }),
    }
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
    /// Universal IPC/RPC command execution. Directly delegates to OpenRevo's plugin IPC router,
    /// enabling any plugin to trigger any host capability without modifying host Rust code.
    #[serde(rename = "call_command", alias = "invoke_command")]
    CallCommand {
        command: String,
        #[serde(default)]
        args: serde_json::Value,
    },

    #[serde(rename = "set_power_mode")]
    SetPowerMode { mode: u8 },

    #[serde(rename = "set_fan_boost")]
    SetFanBoost { enable: bool },

    #[serde(rename = "set_battery_limit")]
    SetBatteryLimit {
        #[serde(deserialize_with = "deserialize_hex_or_dec_u8")]
        limit: u8,
    },

    #[serde(rename = "delay_ms")]
    DelayMs { duration: u64 },

    #[serde(rename = "raw_ec_write")]
    RawEcWrite {
        #[serde(deserialize_with = "deserialize_hex_or_dec_u16")]
        address: u16,
        #[serde(deserialize_with = "deserialize_hex_or_dec_u8")]
        value: u8,
    },

    /// Native pulse write: write active value -> delay -> write restore value (e.g. 1 -> 200ms -> 0)
    #[serde(rename = "pulse_write")]
    PulseWrite {
        #[serde(deserialize_with = "deserialize_hex_or_dec_u16")]
        address: u16,
        #[serde(deserialize_with = "deserialize_hex_or_dec_u8")]
        active: u8,
        #[serde(deserialize_with = "deserialize_hex_or_dec_u8")]
        restore: u8,
        #[serde(default = "default_pulse_delay", deserialize_with = "deserialize_hex_or_dec_u64")]
        delay_ms: u64,
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
    #[serde(alias = "when", deserialize_with = "deserialize_watcher_condition")]
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
    load_persistent_custom_watchers();
    ensure_background_watcher_started();
}

fn get_custom_watchers_file_path() -> std::path::PathBuf {
    crate::system::config::get_config_dir().join("custom_watchers.json")
}

pub fn save_persistent_custom_watchers() {
    let map = get_watchers_map().lock();
    let persistent_list: Vec<WatcherDefinition> = map
        .values()
        .map(|r| r.definition.clone())
        .collect();

    let path = get_custom_watchers_file_path();
    if let Ok(json) = serde_json::to_string_pretty(&persistent_list) {
        let _ = std::fs::write(path, json);
    }
}

pub fn load_persistent_custom_watchers() {
    let path = get_custom_watchers_file_path();
    if !path.exists() {
        return;
    }
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(list) = serde_json::from_str::<Vec<WatcherDefinition>>(&content) {
            let count = list.len();
            for def in list {
                register_watcher_internal(def, false);
            }
            crate::log_info!("WATCHER_HUB", "Loaded {} persistent custom watchers from disk", count);
        }
    }
}

/// Register a watcher definition into the Hub
pub fn register_watcher(def: WatcherDefinition) -> bool {
    register_watcher_internal(def, true)
}

fn register_watcher_internal(def: WatcherDefinition, persist: bool) -> bool {
    let mut map = get_watchers_map().lock();
    let id = def.id.clone();
    let mod_tag = match &def.plugin_id {
        Some(pid) => format!("PLUGIN:{}:WATCHER", pid),
        None => "WATCHER_HUB".to_string(),
    };
    crate::log_info!(&mod_tag, "Registered watcher '{}' [{}]", def.name, id);
    let is_startup = def.enabled && def.condition.matches_startup();
    let def_to_trigger = if is_startup { Some(def.clone()) } else { None };
    map.insert(
        id,
        WatcherRecord {
            definition: def,
            last_triggered: None,
            last_ec_val: None,
            last_matched: false,
        },
    );
    drop(map);
    if persist {
        save_persistent_custom_watchers();
    }
    if let Some(d) = def_to_trigger {
        crate::log_info!(&mod_tag, "Executing newly registered startup watcher '{}' [{}]", d.name, d.id);
        dispatch_watcher_trigger(d, "startup", serde_json::json!({ "event": "startup_registered" }));
    }
    true
}

/// Unregister a single watcher by ID
pub fn unregister_watcher(id: &str) -> bool {
    let mut map = get_watchers_map().lock();
    let removed = map.remove(id).is_some();
    if removed {
        crate::log_info!("WATCHER_HUB", "Unregistered watcher '{}'", id);
        drop(map);
        save_persistent_custom_watchers();
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
        drop(map);
        save_persistent_custom_watchers();
    }
    removed
}

fn get_plugin_watchers_storage_path(plugin_id: &str) -> Result<std::path::PathBuf, String> {
    let dir = crate::system::plugin_storage::get_plugin_storage_dir(plugin_id)?;
    Ok(dir.join("watchers.json"))
}

pub fn load_plugin_watchers_from_storage(plugin_id: &str) -> Option<Vec<WatcherDefinition>> {
    let path = get_plugin_watchers_storage_path(plugin_id).ok()?;
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Vec<WatcherDefinition>>(&content).ok()
}

pub fn save_plugin_watchers_to_storage(plugin_id: &str, watchers: &[WatcherDefinition]) -> Result<(), String> {
    let path = get_plugin_watchers_storage_path(plugin_id)?;
    let json = serde_json::to_string_pretty(watchers).map_err(|e| format!("序列化 Watcher 规则失败: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("写入 Watcher 配置文件失败: {}", e))?;
    Ok(())
}

/// Dynamically update runtime watchers for a specific plugin and persist to its private sandbox.
/// Immediately unregisters previous watchers, mounts the new rules, and triggers startup watchers if applicable.
pub fn set_plugin_watchers_runtime(plugin_id: &str, watchers: Vec<WatcherDefinition>) -> Result<bool, String> {
    let mod_tag = format!("PLUGIN:{}:WATCHER", plugin_id);
    let mut normalized_watchers = watchers;
    for w in &mut normalized_watchers {
        w.plugin_id = Some(plugin_id.to_string());
    }

    save_plugin_watchers_to_storage(plugin_id, &normalized_watchers)?;
    unregister_by_plugin(plugin_id);

    let count = normalized_watchers.len();
    let mut startup_triggers = Vec::new();
    {
        let mut map = get_watchers_map().lock();
        for w in normalized_watchers {
            let id = w.id.clone();
            let is_startup = w.enabled && w.condition.matches_startup();
            if is_startup {
                startup_triggers.push(w.clone());
            }
            map.insert(
                id,
                WatcherRecord {
                    definition: w,
                    last_triggered: None,
                    last_ec_val: None,
                    last_matched: false,
                },
            );
        }
    }

    crate::log_info!(&mod_tag, "Set {} runtime watchers in sandbox and mounted into microkernel", count);
    for w in startup_triggers {
        crate::log_info!(&mod_tag, "Immediately triggering startup watcher '{}' [{}] after runtime update", w.name, w.id);
        dispatch_watcher_trigger(w, "startup", serde_json::json!({ "event": "runtime_updated" }));
    }
    Ok(true)
}

/// Load a batch of watchers from a plugin manifest.
/// Precedence contract:
/// 1. If plugin_storage/<plugin_id>/watchers.json exists, load those custom rules (user settings).
/// 2. Otherwise, mount baseline watchers from manifest.json (headless or out-of-the-box defaults).
pub fn load_plugin_watchers(plugin_id: &str, manifest_watchers: Vec<WatcherDefinition>) {
    let mod_tag = format!("PLUGIN:{}:WATCHER", plugin_id);
    let watchers_to_load = if let Some(saved) = load_plugin_watchers_from_storage(plugin_id) {
        crate::log_info!(&mod_tag, "Loaded {} custom watchers from plugin storage sandbox", saved.len());
        saved
    } else {
        manifest_watchers
    };

    let mut map = get_watchers_map().lock();
    let mut loaded_baseline = 0;
    for mut w in watchers_to_load {
        w.plugin_id = Some(plugin_id.to_string());
        if !map.contains_key(&w.id) {
            let id = w.id.clone();
            crate::log_info!(&mod_tag, "Mounted watcher '{}' [{}]", w.name, id);
            map.insert(
                id,
                WatcherRecord {
                    definition: w,
                    last_triggered: None,
                    last_ec_val: None,
                    last_matched: false,
                },
            );
            loaded_baseline += 1;
        } else {
            crate::log_info!(&mod_tag, "Watcher '{}' [{}] already present, skipping duplicate mount", w.name, w.id);
        }
    }
    drop(map);
    crate::log_info!(&mod_tag, "Processed plugin watchers ({} mounted)", loaded_baseline);
}

/// Reassert all active enabled watchers during core lifecycle milestones
/// (such as microkernel startup readiness or system resume from sleep/hibernate).
/// Host does not make arbitrary assumptions about power cables or external states;
/// any enabled watcher declared by plugins is faithfully executed on startup and resume.
pub fn reassert_active_watchers(trigger_reason: &str) {
    crate::log_info!("WATCHER_HUB", "Microkernel lifecycle trigger: '{}'. Asserting active watchers.", trigger_reason);
    let now = Instant::now();
    let mut triggers: Vec<WatcherDefinition> = Vec::new();

    {
        let mut map = get_watchers_map().lock();
        for record in map.values_mut() {
            if !record.definition.enabled {
                continue;
            }

            // Foreground process and EC diff conditions are event-driven,
            // while hardware baseline policies should be actively asserted on startup/resume.
            let should_assert = record.definition.condition.should_assert_on_lifecycle();

            if should_assert && !record.definition.actions.is_empty() {
                record.last_triggered = Some(now);
                triggers.push(record.definition.clone());
            }
        }
    }

    let count = triggers.len();
    for def in triggers {
        let tag = match &def.plugin_id {
            Some(pid) => format!("PLUGIN:{}:WATCHER", pid),
            None => "WATCHER_HUB".to_string(),
        };
        crate::log_info!(
            &tag,
            "Asserting active watcher '{}' [{}] on lifecycle '{}'",
            def.name,
            def.id,
            trigger_reason
        );
        dispatch_watcher_trigger(
            def,
            "lifecycle_assertion",
            serde_json::json!({ "reason": trigger_reason }),
        );
    }
    crate::log_info!("WATCHER_HUB", "Dispatched {} active watchers for lifecycle '{}'.", count, trigger_reason);
}

/// Trigger baseline initialization for active watchers on system startup.
pub fn trigger_startup_watchers() {
    ensure_background_watcher_started();
    reassert_active_watchers("system_startup");
}

/// Query all active registered watchers info for frontend inspection
pub fn get_all_watchers() -> Vec<WatcherInfo> {
    let map = get_watchers_map().lock();
    let now = Instant::now();
    map.values()
        .map(|r| {
            let cond_type = r.definition.condition.condition_type_str();
            let secs_ago = r.last_triggered.map(|t| now.duration_since(t).as_secs());
            WatcherInfo {
                id: r.definition.id.clone(),
                name: r.definition.name.clone(),
                plugin_id: r.definition.plugin_id.clone(),
                enabled: r.definition.enabled,
                cooldown_ms: r.definition.cooldown_ms,
                condition_type: cond_type,
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
        r.definition.enabled && r.definition.condition.has_telemetry()
    })
}

/// Check if any active telemetry watcher requires GPU sensors
pub fn should_query_gpu_for_watchers() -> bool {
    let map = get_watchers_map().lock();
    map.values().any(|r| {
        if !r.definition.enabled {
            return false;
        }
        r.definition.condition.needs_gpu_sensor()
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

            if record.definition.condition.matches_telemetry(state) {
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
            if record.definition.condition.matches_power_event(event_name) {
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
    let mod_tag = match &def.plugin_id {
        Some(pid) => format!("PLUGIN:{}:WATCHER", pid),
        None => "WATCHER_HUB".to_string(),
    };
    crate::log_info!(
        &mod_tag,
        "Watcher '{}' [{}] triggered (condition: {}) with {} actions",
        def.name,
        def.id,
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
        let plugin_id = def.plugin_id.clone();
        std::thread::Builder::new()
            .name(format!("watcher-action-{}", def.id))
            .spawn(move || {
                for act in actions {
                    execute_single_action(&act, &watcher_name, plugin_id.as_deref());
                }
            })
            .ok();
    }
}

fn detect_active_raw_protocol() -> crate::system::raw_gateway::RawEcProtocol {
    crate::system::raw_gateway::detect_auto_raw_protocol()
}

fn execute_single_action(act: &WatcherAction, watcher_name: &str, plugin_id: Option<&str>) {
    let mod_tag = match plugin_id {
        Some(pid) => format!("PLUGIN:{}:ACT", pid),
        None => "WATCHER_ACT".to_string(),
    };
    match act {
        WatcherAction::CallCommand { command, args } => {
            let pid = plugin_id.unwrap_or("watcher_hub");
            crate::log_info!(&mod_tag, "[{}] Universal IPC Call: '{}' args={:?}", watcher_name, command, args);
            let cmd_str = command.clone();
            let args_val = args.clone();
            let pid_str = pid.to_string();
            tauri::async_runtime::block_on(async move {
                if let Err(e) = crate::system::native_wry::ipc::dispatch_plugin_ipc(&pid_str, &cmd_str, args_val).await {
                    crate::log_warn!(&mod_tag, "[{}] Universal IPC Call '{}' failed: {}", watcher_name, cmd_str, e);
                }
            });
        }
        WatcherAction::SetPowerMode { mode } => {
            crate::log_info!(&mod_tag, "[{}] Setting power mode to {}", watcher_name, mode);
            crate::system::power_mode::apply_full_power_mode(*mode);
        }
        WatcherAction::SetFanBoost { enable } => {
            crate::log_info!(&mod_tag, "[{}] Setting fan boost to {}", watcher_name, enable);
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
        WatcherAction::SetBatteryLimit { limit } => {
            crate::log_info!(&mod_tag, "[{}] Setting battery limit to {}%", watcher_name, limit);
            let ec = crate::driver::ec_bridge::get_global_ec();
            ec.lock().set_battery_limit(*limit);
        }
        WatcherAction::DelayMs { duration } => {
            crate::log_debug!(&mod_tag, "[{}] Delaying for {} ms", watcher_name, duration);
            std::thread::sleep(Duration::from_millis(*duration));
        }
        WatcherAction::RawEcWrite { address, value } => {
            crate::log_info!(&mod_tag, "[{}] Raw EC Write 0x{:04X} = 0x{:02X}", watcher_name, address, value);
            let req = crate::system::raw_gateway::RawEcWriteRequest {
                protocol: detect_active_raw_protocol(),
                address: *address,
                value: *value,
            };
            if let Err(e) = crate::system::raw_gateway::execute_raw_write(req) {
                crate::log_warn!(&mod_tag, "[{}] Raw EC write failed: {}", watcher_name, e);
            }
        }
        WatcherAction::PulseWrite { address, active, restore, delay_ms } => {
            crate::log_info!(
                &mod_tag,
                "[{}] Pulse EC Write 0x{:04X} active=0x{:02X} -> sleep {}ms -> restore=0x{:02X}",
                watcher_name,
                address,
                active,
                delay_ms,
                restore
            );
            let proto = detect_active_raw_protocol();
            if let Err(e) = crate::system::raw_gateway::execute_raw_write(crate::system::raw_gateway::RawEcWriteRequest {
                protocol: proto,
                address: *address,
                value: *active,
            }) {
                crate::log_warn!(&mod_tag, "[{}] Pulse EC write active failed: {}", watcher_name, e);
            }
            std::thread::sleep(Duration::from_millis(*delay_ms));
            if let Err(e) = crate::system::raw_gateway::execute_raw_write(crate::system::raw_gateway::RawEcWriteRequest {
                protocol: proto,
                address: *address,
                value: *restore,
            }) {
                crate::log_warn!(&mod_tag, "[{}] Pulse EC write restore failed: {}", watcher_name, e);
            }
        }
        WatcherAction::RawEcModifyMask { address, mask, value } => {
            if let Err(e) = crate::system::raw_gateway::shield::validate_write_operation(*address, 0) {
                crate::log_warn!(&mod_tag, "[{}] Blast shield blocked EC modify on 0x{:04X}: {}", watcher_name, address, e);
                return;
            }
            let proto = detect_active_raw_protocol();
            let read_req = crate::system::raw_gateway::RawEcReadRequest {
                protocol: proto,
                address: *address,
            };
            if let Ok(cur) = crate::system::raw_gateway::execute_raw_read(read_req) {
                let next = if *value { cur | mask } else { cur & !mask };
                crate::log_info!(&mod_tag, "[{}] Modifying EC 0x{:04X} mask 0x{:02X} (val: {}): 0x{:02X} -> 0x{:02X} via {:?}", watcher_name, address, mask, value, cur, next, proto);
                let write_req = crate::system::raw_gateway::RawEcWriteRequest {
                    protocol: proto,
                    address: *address,
                    value: next,
                };
                let _ = crate::system::raw_gateway::execute_raw_write(write_req);
            } else {
                crate::log_warn!(&mod_tag, "[{}] Failed to read EC 0x{:04X} for bitmask modification", watcher_name, address);
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
pub fn ensure_background_watcher_started() {
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

                // Check if any watchers need process, EC diff or interval scanning
                let (has_proc_watchers, has_ec_watchers, has_interval_watchers) = {
                    let map = get_watchers_map().lock();
                    let p = map.values().any(|r| r.definition.enabled && r.definition.condition.has_foreground_process());
                    let e = map.values().any(|r| r.definition.enabled && r.definition.condition.has_raw_ec_diff());
                    let i = map.values().any(|r| r.definition.enabled && r.definition.condition.has_interval());
                    (p, e, i)
                };

                if !has_proc_watchers && !has_ec_watchers && !has_interval_watchers {
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

                // 3. Scan Interval Watchers
                if has_interval_watchers {
                    evaluate_interval_watchers();
                }

                let sleep_ms = if has_interval_watchers { 500 } else { 800 };
                std::thread::sleep(Duration::from_millis(sleep_ms));
            }
        })
        .expect("Failed to spawn process watcher thread");
}

fn evaluate_interval_watchers() {
    let mut triggers: Vec<WatcherDefinition> = Vec::new();
    let now = Instant::now();

    {
        let mut map = get_watchers_map().lock();
        for record in map.values_mut() {
            if !record.definition.enabled {
                continue;
            }
            if let Some(every_ms) = record.definition.condition.get_min_interval_ms() {
                let interval_dur = Duration::from_millis(every_ms.max(500));
                let should_fire = match record.last_triggered {
                    None => true,
                    Some(last) => now.duration_since(last) >= interval_dur,
                };
                if should_fire {
                    record.last_triggered = Some(now);
                    triggers.push(record.definition.clone());
                }
            }
        }
    }

    for def in triggers {
        dispatch_watcher_trigger(
            def,
            "interval",
            serde_json::json!({ "triggered_by": "interval_timer" }),
        );
    }
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
            if record.definition.condition.matches_foreground_process(prev_proc, curr_proc) {
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
                            "transition": "process_match"
                        }),
                    ));
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
