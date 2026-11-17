use crate::system::power_safety::{
    now_ms, CpuPowerLimits, PowerAssessment, PowerChangeRequest, PowerSafetySession, PowerSnapshot,
};
use std::sync::LazyLock;

static SESSION: LazyLock<parking_lot::Mutex<PowerSafetySession>> =
    LazyLock::new(|| parking_lot::Mutex::new(PowerSafetySession::default()));

#[tauri::command]
pub fn get_cpu_power_safety_snapshot(
    context: String,
    current: CpuPowerLimits,
    temperature_limit_c: f64,
) -> PowerSnapshot {
    let snapshot = super::system_cmd::get_cpu_monitor().power_snapshot();
    if temperature_limit_c.is_finite() && (50.0..=125.0).contains(&temperature_limit_c) {
        SESSION
            .lock()
            .observe(&context, current, &snapshot, temperature_limit_c, now_ms());
    }
    snapshot
}

#[tauri::command]
pub fn assess_cpu_power_change(request: PowerChangeRequest) -> Result<PowerAssessment, String> {
    let snapshot = super::system_cmd::get_cpu_monitor().power_snapshot();
    SESSION.lock().assess(&request, snapshot, now_ms())
}
