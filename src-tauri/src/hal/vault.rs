// OpenRevo Hardware Data Vault (hal::vault)
// Encapsulates and protects all proprietary hardware addresses, bitmasks, and vendor IOCTL codes.
//
// Dynamic Build Architecture:
// - Physical hardware addresses and keys are dynamically compiled from config/ec_registry.json.
// - In community open-source builds without registry, compiles with safe null stubs.
// - Zero plaintext EC physical addresses or constants exist in open-source repository files.

include!(concat!(env!("OUT_DIR"), "/generated_vault.rs"));
pub use openrevo_core::hal::vault::{display, ptl};

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
