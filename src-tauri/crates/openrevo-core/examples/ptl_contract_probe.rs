//! Read-only verification of the contract selected for this PTL machine.

use openrevo_core::hal::{
    declarative::{load_matched_manifest, DeclarativeHardwareDriver},
    HardwareDriver, MechrevoHardwareHAL,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = load_matched_manifest().ok_or("No declarative contract matched")?;
    if manifest.id != "mechrevo_xingyao_14_p916f_ptl" {
        return Err("This probe requires the exact P916F-PTL contract".into());
    }
    println!("Contract: {}", manifest.id);
    println!(
        "Battery charge limit: {}",
        manifest.capabilities.battery_charge_limit
    );
    println!(
        "Custom fan curve: {}",
        manifest.capabilities.fan_custom_curve
    );
    println!(
        "Baolongda fan controller: {}",
        manifest.capabilities.uses_bld_fan_controller()
    );
    println!("MIFS raw mailbox: {}", manifest.uses_mifs_mailbox());
    let driver = DeclarativeHardwareDriver::new(manifest);
    println!("Power mode: {:?}", driver.get_performance_mode()?);
    let (first, second) = driver.read_fan_rpm()?;
    println!("Fan readings: {}, {}", first, second);
    println!(
        "Battery state: {:?}",
        openrevo_core::hal::ptl_pmc::battery_state()?
    );
    Ok(())
}
