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
    apply_ec_hardware_mode, ensure_power_mode_allowed, get_cached_factory_power_presets, FactoryPowerPresets,
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

    /// Apply SDK mode: 0 = Office, 1 = Balance, 2 = Turbo (hardware modes 1 through 3).
    pub fn set_power_mode(&self, mode: u8) -> Result<(), String> {
        let _operation = crate::hal::power_control::MODE_OPERATION.lock();
        self.set_power_mode_with(mode, |ui_mode| {
            ensure_power_mode_allowed(ui_mode)?;
            apply_ec_hardware_mode(ui_mode)
        })
    }

    fn set_power_mode_with(&self, mode: u8, apply: impl FnOnce(u8) -> Result<(), String>) -> Result<(), String> {
        if !self.is_active() {
            return Err("OpenRevo engine is not running or already shut down".to_string());
        }
        let ui_mode = match mode {
            0..=2 => mode + 1,
            _ => return Err("Invalid SDK power mode".into()),
        };
        apply(ui_mode)?;
        dispatch_event_global(CoreEvent::PhysicalModeSwitched(ui_mode));
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_sdk_power_modes_keep_the_documented_numbering_and_report_failure() {
        let engine = OpenRevoEngine::new(None);
        for mode in 0..=2 {
            engine.set_power_mode_with(mode, |actual| {
                assert_eq!(actual, mode + 1);
                Ok(())
            }).unwrap();
            assert_eq!(engine.set_power_mode_with(mode, |_| Err("write failed".into())).unwrap_err(), "write failed");
        }
        for mode in 3..=u8::MAX {
            assert!(engine.set_power_mode_with(mode, |_| panic!("Invalid SDK mode reached hardware")).is_err());
        }
        engine.is_running.store(false, Ordering::SeqCst);
        assert!(engine.set_power_mode_with(0, |_| panic!("Stopped engine reached hardware")).is_err());
    }
}
