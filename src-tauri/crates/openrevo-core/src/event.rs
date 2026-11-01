use std::sync::{Arc, OnceLock};

/// Strongly typed event representation emitted by OpenRevo Core Engine.
/// Designed for zero-overhead internal dispatching, easy FFI translation,
/// and future C#/WinUI3/Slint native bindings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum CoreEvent {
    /// Physical performance mode changed (e.g., via hardware Q-key or AC plug/unplug)
    PhysicalModeSwitched(u8),
    /// Fan boost state toggled
    FanBoostChanged(bool),
    /// On-Screen Display (OSD) notification trigger
    OsdNotify { feature: String, value: String },
    /// Telemetry data tick (temps, powers, fan RPMs, etc.)
    TelemetryUpdated(serde_json::Value),
    /// Magnetic BLE water cooler status change
    BleCoolerStatusChanged { connected: bool, temp: f32, pump_rpm: u16 },
    /// Generic or custom event extension
    Custom { event: String, payload: serde_json::Value },
}

/// Universal Event Sink for OpenRevo Core Engine.
/// Decouples hardware event notifications (Q-key switches, fan alerts, OSD triggers)
/// from any specific UI framework (Tauri, Direct2D native OSD, WinUI 3, Slint, CLI, etc.).
pub trait CoreEventSink: Send + Sync {
    /// Emit a named event with a JSON payload to the UI or subscriber.
    fn emit(&self, event: &str, payload: serde_json::Value);

    /// High-level strongly-typed event handler. Defaults to mapping into `emit` and specific handlers.
    fn on_core_event(&self, event: &CoreEvent) {
        match event {
            CoreEvent::PhysicalModeSwitched(mode) => {
                self.on_power_mode_changed(*mode);
            }
            CoreEvent::FanBoostChanged(enabled) => {
                self.on_fan_boost_changed(*enabled);
            }
            CoreEvent::OsdNotify { feature, value } => {
                self.on_osd_notify(feature, value);
            }
            CoreEvent::TelemetryUpdated(payload) => {
                self.emit("telemetry-tick", payload.clone());
            }
            CoreEvent::BleCoolerStatusChanged { connected, temp, pump_rpm } => {
                let payload = serde_json::json!({
                    "connected": connected,
                    "temp": temp,
                    "pump_rpm": pump_rpm,
                });
                self.emit("ble-cooler-status", payload);
            }
            CoreEvent::Custom { event, payload } => {
                self.emit(event, payload.clone());
            }
        }
    }

    /// Notify UI to display or refresh an on-screen display (OSD) notification.
    fn on_osd_notify(&self, feature: &str, value: &str) {
        let payload = serde_json::json!({
            "feature": feature,
            "value": value
        });
        self.emit("osd-notify", payload);
    }

    /// Notify UI when the physical performance mode is changed (via Q-key or AC plug).
    fn on_power_mode_changed(&self, mode: u8) {
        self.emit("physical-mode-switched", serde_json::json!(mode));
        self.on_osd_notify("power_mode", &mode.to_string());
    }

    /// Notify UI when physical fan boost is toggled.
    fn on_fan_boost_changed(&self, enabled: bool) {
        self.emit("fan-boost-changed", serde_json::json!(enabled));
        self.emit("physical-fan-boost-switched", serde_json::json!(enabled));
    }
}

/// A functional closure event sink for easy runtime event interception.
pub struct CallbackEventSink<F>
where
    F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
{
    callback: F,
}

impl<F> CallbackEventSink<F>
where
    F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> CoreEventSink for CallbackEventSink<F>
where
    F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
{
    fn emit(&self, event: &str, payload: serde_json::Value) {
        (self.callback)(event, payload);
    }
}

/// A no-op event sink used for headless CLI tests, background daemons, or unattached mode.
#[derive(Debug, Default, Clone)]
pub struct NoopEventSink;

impl CoreEventSink for NoopEventSink {
    fn emit(&self, _event: &str, _payload: serde_json::Value) {}
}

/// Helper type alias for shared event sink instances.
pub type SharedEventSink = Arc<dyn CoreEventSink>;

static GLOBAL_EVENT_SINK: OnceLock<SharedEventSink> = OnceLock::new();

/// Set the process-wide global event sink (e.g. called on application bootstrap).
pub fn set_global_event_sink(sink: SharedEventSink) {
    let _ = GLOBAL_EVENT_SINK.set(sink);
}

/// Retrieve the global event sink, or fallback to NoopEventSink if uninitialized.
pub fn get_global_event_sink() -> SharedEventSink {
    GLOBAL_EVENT_SINK
        .get()
        .cloned()
        .unwrap_or_else(|| Arc::new(NoopEventSink))
}

/// Convenient helper to emit a strongly-typed event via the global event sink.
pub fn dispatch_event_global(event: CoreEvent) {
    get_global_event_sink().on_core_event(&event);
}

/// Convenient helper to emit an event via the global event sink.
pub fn emit_global(event: &str, payload: serde_json::Value) {
    get_global_event_sink().emit(event, payload);
}

/// Convenient helper to trigger an OSD notification via the global event sink.
pub fn osd_notify_global(feature: &str, value: &str) {
    get_global_event_sink().on_osd_notify(feature, value);
}

/// Convenient helper to trigger a power mode change event via the global event sink.
pub fn power_mode_changed_global(mode: u8) {
    get_global_event_sink().on_power_mode_changed(mode);
}

/// Convenient helper to trigger a fan boost change event via the global event sink.
pub fn fan_boost_changed_global(enabled: bool) {
    get_global_event_sink().on_fan_boost_changed(enabled);
}
