//! OpenRevo Core Engine - Native C-ABI / FFI Export Layer
//! Compiles into `openrevo_core.dll` (Windows) and `libopenrevo_core.so` (Linux).
//! Enables seamless interoperability with C#, WinUI 3, C++, Python, Go, and GUI frontends.

use openrevo_core::{CoreEventSink, OpenRevoEngine, SharedEventSink};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::panic::catch_unwind;
use std::sync::Arc;

/// C-compatible function pointer for asynchronous hardware event notifications.
/// - `event_type`: C-string representing the event name (e.g., "physical-mode-switched", "fan-boost-changed")
/// - `json_payload`: C-string containing the event JSON payload
/// - `user_data`: User-defined context pointer passed during callback registration
pub type OpenRevoEventCallback = unsafe extern "C" fn(
    event_type: *const c_char,
    json_payload: *const c_char,
    user_data: *mut c_void,
);

struct FfiEventSink {
    callback: OpenRevoEventCallback,
    user_data: usize, // store pointer as integer for Send + Sync safety
}

unsafe impl Send for FfiEventSink {}
unsafe impl Sync for FfiEventSink {}

impl CoreEventSink for FfiEventSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let event_c = match CString::new(event) {
            Ok(c) => c,
            Err(_) => return,
        };
        let payload_str = payload.to_string();
        let payload_c = match CString::new(payload_str) {
            Ok(c) => c,
            Err(_) => return,
        };

        unsafe {
            (self.callback)(
                event_c.as_ptr(),
                payload_c.as_ptr(),
                self.user_data as *mut c_void,
            );
        }
    }
}

// ----------------------------------------------------------------------------
// Lifecycle & General Information
// ----------------------------------------------------------------------------

/// Initialize the global OpenRevo Core engine runtime.
/// Returns 0 on success, or -1 on initialization failure / panic.
#[no_mangle]
pub extern "C" fn openrevo_init() -> i32 {
    let result = catch_unwind(|| {
        let _ = OpenRevoEngine::global();
        0
    });
    result.unwrap_or(-1)
}

/// Query the semantic version of the core engine.
/// The returned pointer is a static string and MUST NOT be freed.
#[no_mangle]
pub extern "C" fn openrevo_version() -> *const c_char {
    static VERSION_CSTR: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION_CSTR.as_ptr() as *const c_char
}

/// Check if the engine runtime is currently active.
/// Returns 1 if active, 0 if stopped.
#[no_mangle]
pub extern "C" fn openrevo_is_active() -> i32 {
    let result = catch_unwind(|| {
        if OpenRevoEngine::global().is_active() {
            1
        } else {
            0
        }
    });
    result.unwrap_or(0)
}

/// Gracefully shutdown the OpenRevo engine, stopping background monitors and releasing handles.
#[no_mangle]
pub extern "C" fn openrevo_shutdown() {
    let _ = catch_unwind(|| {
        OpenRevoEngine::global().shutdown();
    });
}

// ----------------------------------------------------------------------------
// Event Subscription
// ----------------------------------------------------------------------------

/// Register a process-wide asynchronous event callback for hardware triggers (Q-key, fan alerts, OSD).
/// Returns 0 on success, or -1 on error.
#[no_mangle]
pub extern "C" fn openrevo_register_event_callback(
    callback: OpenRevoEventCallback,
    user_data: *mut c_void,
) -> i32 {
    let result = catch_unwind(move || {
        let sink: SharedEventSink = Arc::new(FfiEventSink {
            callback,
            user_data: user_data as usize,
        });
        OpenRevoEngine::global().set_event_sink(sink);
        0
    });
    result.unwrap_or(-1)
}

// ----------------------------------------------------------------------------
// Hardware Status & Preset Queries (JSON String Outputs)
// ----------------------------------------------------------------------------

/// Query the cached hardware snapshot as a newly allocated JSON C-string.
///
/// # Safety
/// The caller MUST free the returned pointer using `openrevo_free_string`.
#[no_mangle]
pub extern "C" fn openrevo_get_hardware_snapshot_json() -> *mut c_char {
    let result = catch_unwind(|| {
        let json = OpenRevoEngine::global().get_hardware_snapshot_json();
        match CString::new(json) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    });
    result.unwrap_or(std::ptr::null_mut())
}

/// Query cached factory power presets as a newly allocated JSON C-string.
///
/// # Safety
/// The caller MUST free the returned pointer using `openrevo_free_string`.
#[no_mangle]
pub extern "C" fn openrevo_get_power_presets_json() -> *mut c_char {
    let result = catch_unwind(|| {
        let json = OpenRevoEngine::global().get_power_presets_json();
        match CString::new(json) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    });
    result.unwrap_or(std::ptr::null_mut())
}

// ----------------------------------------------------------------------------
// Performance Mode & Telemetry Control
// ----------------------------------------------------------------------------

/// Set physical performance mode (0 = Office, 1 = Game/Balanced, 2 = Beast/Turbo).
/// Returns 0 on success, or -1 on hardware communication failure.
#[no_mangle]
pub extern "C" fn openrevo_set_power_mode(mode: u8) -> i32 {
    let result = catch_unwind(|| {
        match OpenRevoEngine::global().set_power_mode(mode) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    });
    result.unwrap_or(-1)
}

/// Enable or pause the telemetry polling heartbeat.
#[no_mangle]
pub extern "C" fn openrevo_set_telemetry_active(active: i32) {
    let _ = catch_unwind(|| {
        OpenRevoEngine::global().set_telemetry_active(active != 0);
    });
}

/// Check if the telemetry heartbeat is active.
/// Returns 1 if active, 0 otherwise.
#[no_mangle]
pub extern "C" fn openrevo_is_telemetry_active() -> i32 {
    let result = catch_unwind(|| {
        if OpenRevoEngine::global().is_telemetry_active() {
            1
        } else {
            0
        }
    });
    result.unwrap_or(0)
}

// ----------------------------------------------------------------------------
// Memory Deallocation Gatekeeper
// ----------------------------------------------------------------------------

/// Free a dynamically allocated C-string returned by OpenRevo Core functions.
///
/// # Safety
/// Pass only pointers returned by `openrevo_*` functions that allocate strings.
#[no_mangle]
pub extern "C" fn openrevo_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = catch_unwind(|| unsafe {
            let _ = CString::from_raw(ptr);
        });
    }
}
