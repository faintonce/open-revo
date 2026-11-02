// OpenRevo Hardware Data Vault (hal::vault)
// Encapsulates and protects all proprietary hardware addresses, bitmasks, and vendor IOCTL codes.
//
// Dynamic Build Architecture:
// - Physical hardware addresses and keys are dynamically compiled from config/ec_registry.json.
// - In community open-source builds without registry, compiles with safe null stubs.
// - Zero plaintext EC physical addresses or constants exist in open-source repository files.

include!(concat!(env!("OUT_DIR"), "/generated_vault.rs"));

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

pub mod power {
    pub const AP_EXIST_BIT: u8 = 0x01;
    pub const AP_ACTIVE_BITS: u8 = 0x81;
    pub const MODE_BRIGHTNESS_MASK: u8 = 0xE0;
    pub const MODE_BITS_MASK: u8 = 0x1E;
    pub const LOGO_SUPPORT_BIT: u8 = 0x01;
    pub const MODE_BALANCED: u8 = 0x00;
    pub const MODE_TURBO: u8 = 0x10;
    pub const OFFICE_CLAMP: u8 = 0xA0;
    pub const TRIGGER_KEEP_MASK: u8 = 0x10;
    pub const TRIGGER_OFFICE: u8 = 0x28;
    pub const TRIGGER_PERFORMANCE: u8 = 0x20;
    pub const STANDARD_FAN_RAMP: u8 = 0x83;
    pub const CUSTOM_BACKLIGHT_CLEAR_BIT: u8 = 0x10;
}
