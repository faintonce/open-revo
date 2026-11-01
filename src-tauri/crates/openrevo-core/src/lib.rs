//! OpenRevo Core Engine (openrevo-core)
//! Ultra-lightweight, zero-UI-dependency hardware control, EC communications,
//! telemetry, and power orchestration engine for Mechrevo laptops.

pub mod engine;
pub mod event;
pub mod hal;
pub mod driver;
pub mod system;

pub use engine::OpenRevoEngine;
pub use event::{
    dispatch_event_global, emit_global, fan_boost_changed_global, get_global_event_sink,
    osd_notify_global, power_mode_changed_global, set_global_event_sink, CallbackEventSink,
    CoreEvent, CoreEventSink, NoopEventSink, SharedEventSink,
};
