// OpenRevo - Raw Hardware Driver Gateway (Protocol-Level Extensibility)
// Allows third-party plugins and geek developers to interact directly with motherboard ECs via 4 low-level protocols.

pub mod types;
pub mod shield;
pub mod executor;

pub use types::{RawEcCapabilities, RawEcProtocol, RawEcReadRequest, RawEcWriteRequest, RawMailboxRequest};
pub use executor::{execute_raw_mailbox, execute_raw_read, execute_raw_write};

/// Probe and return the active machine's available low-level hardware protocols
pub fn get_gateway_capabilities() -> RawEcCapabilities {
    let mut available = Vec::new();

    let is_direct_ec = crate::hal::platform_probe::is_tongfang_family()
        && crate::driver::wmi_ec::is_acpi_driver_available();
    if is_direct_ec {
        available.push(RawEcProtocol::TongfangDirectEc);
    }

    let is_bld_mailbox = crate::hal::declarative::load_matched_manifest()
        .map(|m| m.uses_mifs_mailbox())
        .unwrap_or(false);
    if is_bld_mailbox {
        available.push(RawEcProtocol::BldWmiMailbox);
    }

    let is_generic_wmi = crate::driver::wmi_mulong::is_mulong_available();
    if is_generic_wmi {
        available.push(RawEcProtocol::GenericAcpiWmi);
    }

    let is_ite_hid = !crate::hal::platform_probe::is_ultrabook();
    if is_ite_hid {
        available.push(RawEcProtocol::IteLightingBus);
    }

    RawEcCapabilities {
        available_protocols: available,
        is_direct_ec_available: is_direct_ec,
        is_bld_mailbox_available: is_bld_mailbox,
        is_generic_wmi_available: is_generic_wmi,
        is_ite_hid_available: is_ite_hid,
    }
}
