//! Read the Intel package digital thermal sensor through an installed PawnIO driver.
//! ACPI thermal zones are intentionally excluded: they can describe the chassis.

use super::intel_msr::{IntelMsr, ThreadAffinity};
use super::vault::intel::{MSR_PACKAGE_THERM_STATUS, MSR_TEMPERATURE_TARGET, TEMPERATURE_VALID};
use std::io;
use std::time::{Duration, Instant};
const PROBE_INTERVAL: Duration = Duration::from_secs(30);

fn supports_package_sensor() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::__cpuid;
        let vendor = __cpuid(0);
        return vendor.eax >= 6
            && vendor.ebx == u32::from_le_bytes(*b"Genu")
            && vendor.edx == u32::from_le_bytes(*b"ineI")
            && vendor.ecx == u32::from_le_bytes(*b"ntel")
            && (__cpuid(6).eax & (1 << 6)) != 0;
    }
    #[cfg(not(target_arch = "x86_64"))]
    false
}

fn decode_package_temperature(target: u64, status: u64) -> Option<i32> {
    if status & TEMPERATURE_VALID == 0 {
        return None;
    }
    // Read the hardware TjMax, independently of configurable temperature walls.
    let tj_max = ((target >> 16) & 0xFF) as i32;
    if !(70..=125).contains(&tj_max) {
        return None;
    }
    let delta = ((status >> 16) & 0x7F) as i32;
    let celsius = tj_max - delta;
    (celsius > 0 && celsius <= tj_max).then_some(celsius)
}

/// Read-only view of the shared signed Intel MSR transport.
pub struct IntelPackageTemperature {
    device: IntelMsr,
}
impl IntelPackageTemperature {
    pub fn open() -> io::Result<Self> {
        if !supports_package_sensor() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Intel package thermal sensor unavailable",
            ));
        }
        Ok(Self {
            device: IntelMsr::open()?,
        })
    }
    pub fn read_celsius(&self) -> io::Result<i32> {
        let _affinity = ThreadAffinity::pin()?;
        let target = self.device.read_msr(MSR_TEMPERATURE_TARGET)?;
        let status = self.device.read_msr(MSR_PACKAGE_THERM_STATUS)?;
        decode_package_temperature(target, status).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid CPU package temperature sample",
            )
        })
    }
}

/// Reconnect periodically so installing PawnIO does not require restarting OpenRevo.
pub struct PackageTemperatureReader {
    sensor: Option<IntelPackageTemperature>,
    next_probe: Instant,
    supported: bool,
    reported_unavailable: bool,
}

impl PackageTemperatureReader {
    pub fn new() -> Self {
        Self {
            sensor: None,
            next_probe: Instant::now(),
            supported: supports_package_sensor(),
            reported_unavailable: false,
        }
    }

    pub fn read_celsius(&mut self) -> Option<i32> {
        if !self.supported {
            return None;
        }
        if self.sensor.is_none() {
            if Instant::now() < self.next_probe {
                return None;
            }
            self.next_probe = Instant::now() + PROBE_INTERVAL;
            match IntelPackageTemperature::open() {
                Ok(sensor) => {
                    crate::log_info!("CPU_TEMP", "CPU Package sensor connected through PawnIO");
                    self.sensor = Some(sensor);
                    self.reported_unavailable = false;
                }
                Err(error) => {
                    if !self.reported_unavailable {
                        crate::log_warn!("CPU_TEMP", "CPU Package sensor unavailable: {}. Signed PawnIO driver: https://pawnio.eu", error);
                        self.reported_unavailable = true;
                    }
                    return None;
                }
            }
        }
        match self.sensor.as_ref()?.read_celsius() {
            Ok(celsius) => Some(celsius),
            Err(error) => {
                crate::log_warn!("CPU_TEMP", "CPU Package sample unavailable: {}", error);
                self.sensor = None;
                self.next_probe = Instant::now() + PROBE_INTERVAL;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_hardware_tjmax_for_package_temperature() {
        for (tj_max, delta) in [(100, 57), (105, 62), (110, 67)] {
            assert_eq!(
                decode_package_temperature(tj_max << 16, TEMPERATURE_VALID | (delta << 16)),
                Some(43)
            );
        }
    }

    #[test]
    fn accepts_tjmax_without_clamping_overheating_readings() {
        assert_eq!(
            decode_package_temperature(110 << 16, TEMPERATURE_VALID),
            Some(110)
        );
    }

    #[test]
    fn rejects_invalid_samples_instead_of_manufacturing_a_temperature() {
        assert_eq!(decode_package_temperature(100 << 16, 57 << 16), None);
        assert_eq!(decode_package_temperature(0, TEMPERATURE_VALID), None);
        assert_eq!(
            decode_package_temperature(255 << 16, TEMPERATURE_VALID),
            None
        );
        assert_eq!(
            decode_package_temperature(100 << 16, TEMPERATURE_VALID | (110 << 16)),
            None
        );
    }
}
