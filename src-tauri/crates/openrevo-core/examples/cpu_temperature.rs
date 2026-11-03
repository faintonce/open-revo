//! Read-only CPU Package temperature probe. Run as administrator with PawnIO installed.

use openrevo_core::hal::cpu_temperature::IntelPackageTemperature;

fn main() -> std::io::Result<()> {
    let sensor = IntelPackageTemperature::open()?;
    for sample in 0..5 {
        if sample > 0 {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        println!("CPU Package: {} C", sensor.read_celsius()?);
    }
    Ok(())
}
