// OpenRevo Hardware Abstraction Layer (HAL) Module
pub mod power_options;

// pub mod baolongda_guard;
pub mod declarative;
pub mod cpu_power;
mod intel_msr;

pub mod cpu_temperature;
pub mod intel_gpu;
mod intel_pmt;
pub mod ptl_pmc;
pub mod fan_policy;
pub mod fan_smoothing;
pub mod driver_impl;
pub mod mock;
pub mod platform_probe;
pub mod traits;
pub mod vault;

pub use declarative::DeclarativeHardwareDriver;
pub use driver_impl::MechrevoHALImpl;
pub use mock::MockHAL;
pub use traits::*;

use std::sync::OnceLock;

static DRIVER_INSTANCE: OnceLock<Box<dyn HardwareDriver>> = OnceLock::new();

/// Global accessor for the active Hardware Driver backend
pub fn get_hardware_driver() -> &'static dyn HardwareDriver {
    DRIVER_INSTANCE
        .get_or_init(|| {
            // 1. Try declarative model driver first (e.g. Xingyao 14, thin & lights, external plugin drivers)
            if let Some(manifest) = declarative::load_matched_manifest() {
                crate::log_info!(
                    "HAL",
                    "Activated Declarative Hardware Driver backend: {} ({})",
                    manifest.name,
                    manifest.id
                );
                return Box::new(DeclarativeHardwareDriver::new(manifest));
            }

            // 2. Fall back to built-in Tongfang ACPIDriver for standard gaming laptops
            let real = MechrevoHALImpl::new();
            if real.probe() {
                crate::log_info!("HAL", "Activated Native Tongfang ACPI Hardware Driver backend");
                Box::new(real)
            } else {
                crate::log_warn!("HAL", "No physical hardware driver matched. Activating MockHAL backend.");
                Box::new(MockHAL::new())
            }
        })
        .as_ref()
}

/// Global accessor for the Hardware Abstraction Layer (backward compatibility)
pub fn get_hardware_hal() -> &'static dyn MechrevoHardwareHAL {
    get_hardware_driver()
}

