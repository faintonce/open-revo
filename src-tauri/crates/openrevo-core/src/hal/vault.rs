// OpenRevo Hardware Data Vault (hal::vault)
// Encapsulates and protects all proprietary hardware addresses, bitmasks, and vendor IOCTL codes.
//
// Dynamic Build Architecture:
// - Physical hardware addresses and keys are dynamically compiled from config/ec_registry.json.
// - In community open-source builds without registry, compiles with safe null stubs.
// - Zero plaintext EC physical addresses or constants exist in open-source repository files.

include!(concat!(env!("OUT_DIR"), "/generated_vault.rs"));

/// P916F-PTL PM2 protocol. Never use these commands on STX/LNL or ACPI's primary EC port.
pub mod ptl {
    pub const DATA: u16 = 0x68;
    pub const STATUS_COMMAND: u16 = 0x6c;
    pub const IBF: u8 = 2;
    pub const OBF: u8 = 1;
    pub const READ_PORT: u32 = 0x9c402004;
    pub const WRITE_PORT: u32 = 0x9c402008;
    pub const FAN_QUERY: u8 = 0xd5;
    pub const FAN_AUTO: u8 = 0x10;
    pub const FAN_PWM: u8 = 0xd6;
    pub const FAN_RPM: [u8; 2] = [0xd7, 0xd8];
    pub const FAN_LOW: [u8; 2] = [0x18, 0x16];
    pub const FAN_HIGH: [u8; 2] = [0x19, 0x17];
    pub const MODE_QUERY: [u8; 2] = [0xe7, 0x11];
    pub const BATTERY: u8 = 0xf1;
    pub const BATTERY_DISABLE: u8 = 0x10;
    pub const BATTERY_ENABLE: u8 = 0x11;
    pub const BATTERY_ENABLED: u8 = 0x12;
    pub const BATTERY_LOWER: u8 = 0x13;
    pub const BATTERY_UPPER: u8 = 0x14;
    pub const BATTERY_SET_LOWER: u8 = 0xf2;
    pub const BATTERY_SET_UPPER: u8 = 0xf3;
    pub const INPOUT_SHA256: &str = "f8965fdce668692c3785afa3559159f9a18287bc0d53abb21902895a8ecf221b";
}

pub mod display {
    pub const IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS: u32 = 0x00230498;
    pub const IOCTL_VIDEO_SET_DISPLAY_BRIGHTNESS: u32 = 0x0023049C;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_driver_path_and_ioctls() {
        if IS_PROPRIETARY_VAULT_LOADED {
            let path = driver::get_driver_path_null_terminated();
            assert_eq!(path, b"\\\\.\\ACPIDriver\0");
            assert_eq!(driver::IOCTL_ECREAD, 0x9C40A488);
            assert_eq!(driver::IOCTL_ECWRITE, 0x9C40A48C);
            assert!(ec::ADDR_STATUS_BYTE > 1000 && ec::ADDR_STATUS_BYTE < 4096);
            assert!(ec::ADDR_PROJECT_ID > 1000 && ec::ADDR_PROJECT_ID < 4096);
            assert!(ec::ADDR_BATTERY_PROTECTION_MODE > 1000 && ec::ADDR_BATTERY_PROTECTION_MODE < 4096);
            assert_eq!(mask::TRIGGER_WINLOCK, 1);
            assert_eq!(mask::STATUS_WINLOCK, 1);
            assert_eq!(mask::STATUS_FANBOOST, 4);
            assert_eq!(ite::ITE_VID, 0x048D);
            assert_eq!(cooler::CMD_FILL_WATER, 25);
            assert_eq!(cooler::CMD_SET_FAN, 27);
            assert_eq!(cooler::CMD_SET_PUMP, 28);
        }
    }
}

/// Signed PawnIO Intel module ABI and package RAPL fields (Intel SDM).
pub mod intel {
    pub const IOCTL_LOAD_BINARY: u32 = (41394 << 16) | (0x821 << 2);
    pub const IOCTL_EXECUTE: u32 = (41394 << 16) | (0x841 << 2);
    pub const MSR_TEMPERATURE_TARGET: u64 = 0x1a2;
    pub const MSR_PACKAGE_THERM_STATUS: u64 = 0x1b1;
    pub const TEMPERATURE_VALID: u64 = 1 << 31;
    pub const MSR_RAPL_POWER_UNIT: u64 = 0x606;
    pub const MSR_PKG_POWER_LIMIT: u64 = 0x610;
    pub const POWER_VALUE_MASK: u64 = 0x7fff;
    pub const PL1_ENABLE: u64 = 1 << 15;
    pub const PL2_ENABLE: u64 = 1 << 47;
    pub const PACKAGE_LOCK: u64 = 1 << 63;
    pub const PL1_FIELDS: u64 = POWER_VALUE_MASK | PL1_ENABLE;
    pub const PL2_FIELDS: u64 = (POWER_VALUE_MASK << 32) | PL2_ENABLE;
    pub const OWNED_FIELDS: u64 = PL1_FIELDS | PL2_FIELDS;
}

/// Intel's read-only Windows PMT ABI and PTL normal telemetry layout.
/// Source: intel/compute-runtime sysman_pmt_helper.h and Intel-PMT/xml/PTL/0.
pub mod intel_pmt {
    pub const INTERFACE_GUID: u128 = 0x3dfb2563_5c44_4c59_8d80_baea7d06e6b8;
    pub const DISCOVERY_SIZE: u32 = (0x9998 << 16) | (1 << 14);
    pub const DISCOVERY: u32 = DISCOVERY_SIZE | (1 << 2);
    pub const READ_QWORD: u32 = DISCOVERY_SIZE | (3 << 2);
    pub const DISCOVERY_HEADER_BYTES: usize = 8;
    pub const DISCOVERY_ENTRY_BYTES: usize = 16;
    pub const PTL_NORMAL_GUID: u32 = 0x03086000;
    pub const PTL_TEMPERATURE_QWORD: u32 = 15;
    pub const PTL_GT_MAX_SHIFT: u32 = 56;
}
