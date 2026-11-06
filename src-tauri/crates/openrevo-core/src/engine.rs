//! OpenRevo Unified Engine Facade
//! High-level facade orchestrating hardware HAL, power arbiter, telemetry,
//! lighting effects, and event subscriptions. Provides a stable contract for
//! Slint UI, Tauri IPC, and external language FFI / SDK bindings.

use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::event::{
    dispatch_event_global, set_global_event_sink, CoreEvent, SharedEventSink,
};
use crate::system::hardware_probe::{
    collect_hardware_snapshot, get_hardware_snapshot_cached, HardwareSnapshot,
};
use crate::system::power::{
    apply_ec_hardware_mode, get_cached_factory_power_presets, FactoryPowerPresets,
};
use crate::driver::lighting::{get_global_lighting, SharedIteLightingManager};
use crate::system::telemetry::{is_telemetry_active, set_telemetry_active};

/// Main orchestration facade for OpenRevo Core.
pub struct OpenRevoEngine {
    is_running: AtomicBool,
}

static GLOBAL_ENGINE: OnceLock<Arc<OpenRevoEngine>> = OnceLock::new();

impl OpenRevoEngine {
    /// Initialize a new OpenRevo engine instance with an optional custom event sink.
    pub fn new(event_sink: Option<SharedEventSink>) -> Arc<Self> {
        if let Some(sink) = event_sink {
            set_global_event_sink(sink);
        }
        
        Arc::new(Self {
            is_running: AtomicBool::new(true),
        })
    }

    /// Retrieve or lazily initialize the process-wide global engine instance.
    pub fn global() -> Arc<Self> {
        GLOBAL_ENGINE
            .get_or_init(|| Self::new(None))
            .clone()
    }

    /// Return the semantic version of the core engine.
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Check if the engine runtime is currently active.
    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Attach or replace the event sink at runtime.
    pub fn set_event_sink(&self, sink: SharedEventSink) {
        set_global_event_sink(sink);
    }

    /// Dispatch a strongly typed event through the engine's event sink.
    pub fn dispatch_event(&self, event: CoreEvent) {
        dispatch_event_global(event);
    }

    // ------------------------------------------------------------------------
    // Hardware Probing & Snapshot Queries
    // ------------------------------------------------------------------------

    /// Query cached hardware snapshot (CPU, GPU, Board, Displays, EC registers).
    pub fn get_hardware_snapshot(&self) -> &'static HardwareSnapshot {
        get_hardware_snapshot_cached()
    }

    /// Refresh and collect a fresh hardware snapshot.
    pub fn collect_fresh_hardware_snapshot(&self) -> HardwareSnapshot {
        collect_hardware_snapshot()
    }

    /// Query hardware info as a serialized JSON string (ideal for FFI / SDK bindings).
    pub fn get_hardware_snapshot_json(&self) -> String {
        serde_json::to_string(self.get_hardware_snapshot()).unwrap_or_else(|_| "{}".to_string())
    }

    // ------------------------------------------------------------------------
    // Power & Performance Mode Orchestration
    // ------------------------------------------------------------------------

    /// Apply performance mode: 1 = Office, 2 = Balance, 3 = Turbo, 4 = Custom.
    pub fn set_power_mode(&self, mode: u8) -> Result<(), String> {
        if !self.is_active() {
            return Err("OpenRevo engine is not running or already shut down".to_string());
        }
        if crate::hal::declarative::load_matched_manifest().is_some_and(|m| m.capabilities.cpu_power_backend.as_deref() == Some("intel_rapl")) {
            let hw = match mode { 1 => crate::hal::HardwarePowerMode::Office, 2 => crate::hal::HardwarePowerMode::Balance, 3 => crate::hal::HardwarePowerMode::Turbo, 4 => crate::hal::HardwarePowerMode::Custom, _ => return Err("Invalid performance mode".into()) };
            crate::hal::get_hardware_driver().set_performance_mode(hw).map_err(|e| e.to_string())?;
            let mut cfg = crate::system::config::load_config();
            cfg.power_mode = mode;
            crate::system::config::save_config(&cfg);
        } else { apply_ec_hardware_mode(mode); }
        dispatch_event_global(CoreEvent::PhysicalModeSwitched(mode));
        Ok(())
    }

    /// Query cached factory power presets.
    pub fn get_power_presets(&self) -> Option<FactoryPowerPresets> {
        get_cached_factory_power_presets()
    }

    /// Query cached factory power presets as a JSON string (ideal for FFI / SDK bindings).
    pub fn get_power_presets_json(&self) -> String {
        match self.get_power_presets() {
            Some(presets) => serde_json::to_string(&presets).unwrap_or_else(|_| "{}".to_string()),
            None => "{}".to_string(),
        }
    }

    // ------------------------------------------------------------------------
    // Lighting & Streamer
    // ------------------------------------------------------------------------

    /// Obtain direct handle to the shared ITE lighting manager.
    pub fn lighting(&self) -> SharedIteLightingManager {
        get_global_lighting()
    }

    // ------------------------------------------------------------------------
    // Telemetry & Watchers
    // ------------------------------------------------------------------------

    /// Set telemetry heartbeat active state.
    pub fn set_telemetry_active(&self, active: bool) {
        set_telemetry_active(active);
    }

    /// Check if telemetry heartbeat is active.
    pub fn is_telemetry_active(&self) -> bool {
        is_telemetry_active()
    }

    // ------------------------------------------------------------------------
    // Graceful Shutdown & Lifecycle
    // ------------------------------------------------------------------------

    /// Gracefully shutdown the engine, signaling background monitors and releasing handles.
    pub fn shutdown(&self) {
        if self.is_running.swap(false, Ordering::SeqCst) {
            if let Err(e) = crate::hal::cpu_power::shutdown() { crate::log_error!("CPU_POWER", "CPU release on engine shutdown failed: {}", e); }
            crate::log_info!("ENGINE", "OpenRevo engine shutdown initiated.");
            set_telemetry_active(false);
        }
    }
}

impl Drop for OpenRevoEngine {
    fn drop(&mut self) {
        self.shutdown();
    }
}
