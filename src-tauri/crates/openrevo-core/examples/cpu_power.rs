//! Read-only Intel package PL1/PL2 probe. Requires an installed PawnIO driver
//! and administrator access. Does not start the engine or change power settings.

use openrevo_core::hal::cpu_power;

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let json = match args.as_slice() {
        [] => false,
        [arg] if arg == "--json" => true,
        _ => return Err("Usage: cpu_power [--json]".into()),
    };

    // Reserve stdout for the result; do not initialize app logging or config.
    openrevo_core::system::logger::set_log_level(openrevo_core::system::logger::LOG_LEVEL_OFF);
    openrevo_core::system::logger::set_log_filter(Some("CPU_POWER_PROBE"));
    let status = cpu_power::status();
    if !status.available {
        return Err(format!(
            "Cannot read Intel PL1/PL2: {}. Run as administrator with PawnIO installed.",
            status.error.as_deref().unwrap_or("Sensor unavailable")
        ));
    }
    let pl1 = status.pl1_watts.ok_or("PL1 reading unavailable")?;
    let pl2 = status.pl2_watts.ok_or("PL2 reading unavailable")?;
    if json {
        println!("{}", serde_json::json!({
            "pl1_watts": pl1,
            "pl2_watts": pl2,
            "pl1_enabled": status.pl1_enabled,
            "pl2_enabled": status.pl2_enabled,
            "locked": status.locked,
        }));
    } else {
        println!("PL1: {pl1} W (enabled: {})", status.pl1_enabled);
        println!("PL2: {pl2} W (enabled: {})", status.pl2_enabled);
        println!("Locked: {}", status.locked);
    }
    Ok(())
}
