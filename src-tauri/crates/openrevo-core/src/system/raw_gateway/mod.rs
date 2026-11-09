// OpenRevo - Raw Hardware Driver Gateway (Protocol-Level Extensibility)
// Allows third-party plugins and geek developers to interact directly with motherboard ECs via 4 low-level protocols.

pub mod types;
pub mod shield;
pub mod executor;

pub use types::{RawEcCapabilities, RawEcProtocol, RawEcReadRequest, RawEcWriteRequest, RawMailboxRequest};
pub use executor::{execute_raw_mailbox, execute_raw_read, execute_raw_write};

/// Automatically detects and returns the preferred active Raw EC protocol on this machine
pub fn detect_auto_raw_protocol() -> RawEcProtocol {
    let caps = get_gateway_capabilities();
    if caps.is_direct_ec_available {
        RawEcProtocol::TongfangDirectEc
    } else if caps.is_generic_wmi_available {
        RawEcProtocol::GenericAcpiWmi
    } else if caps.is_bld_mailbox_available {
        RawEcProtocol::BldWmiMailbox
    } else {
        RawEcProtocol::TongfangDirectEc
    }
}

/// Probe and return the active machine's available low-level hardware protocols
pub fn get_gateway_capabilities() -> RawEcCapabilities {
    let mut available = Vec::new();

    let is_direct_ec = crate::driver::wmi_ec::is_acpi_driver_available();
    if is_direct_ec {
        available.push(RawEcProtocol::TongfangDirectEc);
    }

    let is_bld_mailbox = crate::hal::declarative::load_matched_manifest()
        .map(|m| m.channels.values().any(|ch| matches!(ch, crate::hal::declarative::model_manifest::ChannelDef::WmiMailbox { .. })))
        .unwrap_or(false)
        || crate::driver::bld_driver_mgr::BldDriverManager::get_or_init().is_ok();
    if is_bld_mailbox {
        available.push(RawEcProtocol::BldWmiMailbox);
    }

    let is_generic_wmi = crate::driver::wmi_mulong::is_mulong_available()
        || crate::driver::wmi_ip3::is_ip3_available();
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
