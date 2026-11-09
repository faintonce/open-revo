// OpenRevo - Raw Driver Gateway Execution Engine
// Bridges high-level plugin requests directly into Tongfang IOCTL, Baolongda Mailbox, Generic WMI, or ITE buses.

use super::types::{RawEcProtocol, RawEcReadRequest, RawEcWriteRequest, RawMailboxRequest};
use super::shield::{validate_read_address, validate_write_operation};

pub fn execute_raw_read(req: RawEcReadRequest) -> Result<u8, String> {
    validate_read_address(req.address)?;

    match req.protocol {
        RawEcProtocol::TongfangDirectEc => {
            crate::driver::wmi_ec::wmi_read_ec(req.address)
                .ok_or_else(|| format!("Tongfang DirectEC failed to read address 0x{:04X}", req.address))
        }
        RawEcProtocol::GenericAcpiWmi => {
            // 1. Try legacy Tongfang AcpiTest_MULong first
            if crate::driver::wmi_mulong::is_mulong_available() {
                if let Some(val) = crate::driver::wmi_mulong::mulong_read_ec(req.address) {
                    return Ok(val);
                }
            }

            // 2. Fallback to Shenzhen IP3 ACPI-WMI SuRwECRegInterface (e.g. Wujie 14/15)
            if crate::driver::wmi_ip3::is_ip3_available() {
                let group = (req.address >> 8) as u8;
                let offset = (req.address & 0xFF) as u8;
                if let Some(val) = crate::driver::wmi_ip3::ip3_read_ec_reg(group, offset) {
                    return Ok(val);
                }
            }

            Err(format!("Generic WMI failed to read address 0x{:04X} (neither AcpiTest_MULong nor SuRwECRegInterface responded)", req.address))
        }
        RawEcProtocol::BldWmiMailbox => {
            // 1. Try Baolongda kernel driver IO ports first
            if let Ok(driver) = crate::driver::bld_driver_mgr::BldDriverManager::get_or_init() {
                if let Some(val) = driver.read_ec_ram(req.address) {
                    return Ok(val);
                }
            }

            Err(format!("Baolongda driver failed to read address 0x{:04X}. If this machine uses packet mailbox only, use 'raw_mailbox_invoke'.", req.address))
        }
        RawEcProtocol::IteLightingBus => {
            Err("ITE Lighting Bus is a dedicated packet HID bus for RGB LEDs. Use dedicated lighting APIs or raw HID streamer.".to_string())
        }
    }
}

pub fn execute_raw_write(req: RawEcWriteRequest) -> Result<bool, String> {
    validate_write_operation(req.address, req.value)?;

    match req.protocol {
        RawEcProtocol::TongfangDirectEc => {
            if crate::driver::wmi_ec::wmi_write_ec(req.address, req.value) {
                crate::log_info!("RAW_GATEWAY", "Wrote 0x{:02X} to 0x{:04X} via Tongfang DirectEC", req.value, req.address);
                Ok(true)
            } else {
                Err(format!("Tongfang DirectEC failed to write 0x{:02X} to 0x{:04X}", req.value, req.address))
            }
        }
        RawEcProtocol::GenericAcpiWmi => {
            // 1. Try legacy Tongfang AcpiTest_MULong first
            if crate::driver::wmi_mulong::is_mulong_available() {
                if crate::driver::wmi_mulong::mulong_write_ec(req.address, req.value) {
                    crate::log_info!("RAW_GATEWAY", "Wrote 0x{:02X} to 0x{:04X} via AcpiTest_MULong WMI", req.value, req.address);
                    return Ok(true);
                }
            }

            // 2. Fallback to Shenzhen IP3 ACPI-WMI SuRwECRegInterface (e.g. Wujie 14/15)
            if crate::driver::wmi_ip3::is_ip3_available() {
                let group = (req.address >> 8) as u8;
                let offset = (req.address & 0xFF) as u8;
                if crate::driver::wmi_ip3::ip3_write_ec_reg(group, offset, req.value) {
                    crate::log_info!("RAW_GATEWAY", "Wrote 0x{:02X} to 0x{:04X} via IP3 SuRwECReg WMI", req.value, req.address);
                    return Ok(true);
                }
            }

            Err(format!("Generic WMI failed to write 0x{:02X} to 0x{:04X} (neither AcpiTest_MULong nor SuRwECRegInterface responded)", req.value, req.address))
        }
        RawEcProtocol::BldWmiMailbox => {
            // 1. Try Baolongda kernel driver IO ports first
            if let Ok(driver) = crate::driver::bld_driver_mgr::BldDriverManager::get_or_init() {
                if driver.write_ec_ram(req.address, req.value) {
                    crate::log_info!("RAW_GATEWAY", "Wrote 0x{:02X} to 0x{:04X} via Baolongda EC Driver", req.value, req.address);
                    return Ok(true);
                }
            }

            Err(format!("Baolongda driver failed to write 0x{:02X} to 0x{:04X}. If this machine uses packet mailbox only, use 'raw_mailbox_invoke'.", req.value, req.address))
        }
        RawEcProtocol::IteLightingBus => {
            Err("ITE Lighting Bus is a dedicated packet HID bus for RGB LEDs. Use dedicated lighting APIs or raw HID streamer.".to_string())
        }
    }
}

pub fn execute_raw_mailbox(req: RawMailboxRequest) -> Result<Vec<u8>, String> {
    if req.in_bytes.is_empty() {
        return Err("Mailbox input buffer cannot be empty".to_string());
    }
    if req.in_bytes.len() > 256 {
        return Err("Mailbox buffer exceeds maximum 256-byte security boundary".to_string());
    }

    crate::hal::declarative::invoke_raw_mailbox(&req.in_bytes)
}
