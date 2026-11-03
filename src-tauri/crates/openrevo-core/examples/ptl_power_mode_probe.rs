//! PTL power-mode regression check. Default is read-only; --verify-switches exercises all presets.
use openrevo_core::hal::{
    declarative::{load_matched_manifest, DeclarativeHardwareDriver},
    HardwareDriver, HardwarePowerMode, MechrevoHardwareHAL,
};
use std::{thread, time::Duration};

struct Restore<'a>(&'a DeclarativeHardwareDriver, HardwarePowerMode);
impl Drop for Restore<'_> {
    fn drop(&mut self) { eprintln!("Restore original mode {:?}: {:?}",self.1,self.0.set_performance_mode(self.1)); }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = load_matched_manifest().ok_or("No matching contract")?;
    if !manifest.uses_ptl_pmc() { return Err("This probe requires P916F-PTL".into()); }
    let driver = DeclarativeHardwareDriver::new(manifest);
    let original = driver.get_performance_mode()?;
    println!("Initial hardware mode: {original:?}");
    if !std::env::args().any(|a| a == "--verify-switches") { return Ok(()); }
    let _restore = Restore(&driver,original);
    for mode in [HardwarePowerMode::Balance, HardwarePowerMode::Turbo, HardwarePowerMode::Office] {
        driver.set_performance_mode(mode)?;
        let actual = driver.get_performance_mode()?;
        println!("Set {mode:?}: OK; hardware readback {actual:?}");
        assert_eq!(mode,actual);
    }
    // Exceed the old frontend's five-second optimistic override.
    for second in 1..=10 {
        thread::sleep(Duration::from_secs(1));
        let actual = driver.get_performance_mode()?;
        println!("Office after {second}s: {actual:?}");
        assert_eq!(actual,HardwarePowerMode::Office);
    }
    Ok(())
}
