use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn find_ec_registry() -> Option<PathBuf> {
    let candidate1 = Path::new("config/ec_registry.json");
    if candidate1.exists() {
        return Some(candidate1.to_path_buf());
    }
    let candidate2 = Path::new("../../config/ec_registry.json");
    if candidate2.exists() {
        return Some(candidate2.to_path_buf());
    }
    None
}

fn generate_hardware_vault() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo");
    let dest_path = Path::new(&out_dir).join("generated_vault.rs");

    let registry_opt = find_ec_registry();
    if let Some(registry_path) = registry_opt {
        println!("cargo:rerun-if-changed={}", registry_path.display());
        if let Ok(content) = fs::read_to_string(&registry_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let keys = &val["keys"];
                let ec = &val["ec"];
                let mask = &val["mask"];
                let driver = &val["driver"];
                let cooler = &val["cooler"];
                let ite = &val["ite"];

                let k_u16 = keys["key_u16"].as_u64().unwrap_or(0x5A3C) as u16;
                let k_u8 = keys["key_u8"].as_u64().unwrap_or(0x7E) as u8;
                let k_u32 = keys["key_u32"].as_u64().unwrap_or(0xA5C3F012) as u32;
                let drv_obf_key = keys["driver_obf_key"].as_u64().unwrap_or(0x4B) as u8;

                let mut code = String::new();
                code.push_str("// Auto-generated Hardware Vault (Proprietary Physical Hardware Keys Loaded)\n");
                code.push_str("// Build-time obfuscation active. Zero plaintext addresses in binary string sections.\n\n");
                code.push_str(&format!("pub const KEY_U16: u16 = 0x{:04X};\n", k_u16));
                code.push_str(&format!("pub const KEY_U8: u8 = 0x{:02X};\n", k_u8));
                code.push_str(&format!("pub const KEY_U32: u32 = 0x{:08X};\n", k_u32));
                code.push_str("pub const IS_PROPRIETARY_VAULT_LOADED: bool = true;\n\n");

                code.push_str(r#"#[inline(always)]
pub const fn deobf_u16(val: u16) -> u16 {
    val ^ KEY_U16
}

#[inline(always)]
pub const fn deobf_u8(val: u8) -> u8 {
    val ^ KEY_U8
}

#[inline(always)]
pub const fn deobf_u32(val: u32) -> u32 {
    val ^ KEY_U32
}

#[inline(always)]
pub fn deobf_bytes(bytes: &[u8], key: u8) -> Vec<u8> {
    bytes.iter().map(|b| b ^ key).collect()
}

"#);

                // ec module
                code.push_str("pub mod ec {\n    use super::*;\n\n");
                if let Some(ec_map) = ec.as_object() {
                    for (k, v) in ec_map {
                        let num = v.as_u64().unwrap_or(0) as u16;
                        let obf = num ^ k_u16;
                        code.push_str(&format!("    pub const {}: u16 = deobf_u16(0x{:04X});\n", k, obf));
                    }
                }
                // Aliases
                code.push_str("    pub const ADDR_SUPPORT_BYTE3: u16 = ADDR_SUPPORT_BYTE6;\n");
                code.push_str("    pub const ADDR_DC_ROUND_JACK: u16 = ADDR_POWER_STATUS;\n");
                code.push_str("    pub const ADDR_SUPER_PERFORMANCE: u16 = ADDR_FAN_BOOST_CLAMP;\n");
                code.push_str("    pub const ADDR_GPU_DB_MAX: u16 = ADDR_DB_WATTS1;\n");
                code.push_str("    pub const ADDR_RGB_CONTROL: u16 = ADDR_FAN_ISOLATED;\n");
                code.push_str("    pub const ADDR_MYFAN2_L1_PWM: u16 = ADDR_DB_CTRL1;\n");
                code.push_str("    pub const ADDR_MYFAN2_L2_PWM: u16 = ADDR_TGP_OFFSET;\n");
                code.push_str("    pub const ADDR_MYFAN2_L3_PWM: u16 = ADDR_DB_FLAG1;\n");
                code.push_str("    pub const ADDR_MYFAN2_L4_PWM: u16 = ADDR_DB_WATTS1;\n");
                code.push_str("}\n\n");

                // mask module
                code.push_str("pub mod mask {\n    use super::*;\n\n");
                if let Some(mask_map) = mask.as_object() {
                    for (k, v) in mask_map {
                        let num = v.as_u64().unwrap_or(0) as u8;
                        let obf = num ^ k_u8;
                        code.push_str(&format!("    pub const {}: u8 = deobf_u8(0x{:02X});\n", k, obf));
                    }
                }
                code.push_str("}\n\n");

                // driver module
                code.push_str("pub mod driver {\n    use super::*;\n\n");
                let drv_path = "\\\\.\\ACPIDriver\0";
                let obf_bytes: Vec<String> = drv_path.bytes().map(|b| format!("0x{:02X}", b ^ drv_obf_key)).collect();
                code.push_str(&format!("    const OBF_DRIVER_PATH: &[u8] = &[{}];\n", obf_bytes.join(", ")));
                code.push_str(&format!("    pub fn get_driver_path_null_terminated() -> Vec<u8> {{\n        deobf_bytes(OBF_DRIVER_PATH, 0x{:02X})\n    }}\n\n", drv_obf_key));
                let ioctl_read = driver["IOCTL_ECREAD"].as_u64().unwrap_or(0x9c40a488) as u32;
                let ioctl_write = driver["IOCTL_ECWRITE"].as_u64().unwrap_or(0x9c40a48c) as u32;
                code.push_str(&format!("    pub const IOCTL_ECREAD: u32 = deobf_u32(0x{:08X});\n", ioctl_read ^ k_u32));
                code.push_str(&format!("    pub const IOCTL_ECWRITE: u32 = deobf_u32(0x{:08X});\n", ioctl_write ^ k_u32));
                let port_addr = driver["PORT_BLD_EC_ADDR"].as_u64().unwrap_or(0x4E) as u16;
                let port_data = driver["PORT_BLD_EC_DATA"].as_u64().unwrap_or(0x4F) as u16;
                code.push_str(&format!("    pub const PORT_BLD_EC_ADDR: u16 = deobf_u16(0x{:04X});\n", port_addr ^ k_u16));
                code.push_str(&format!("    pub const PORT_BLD_EC_DATA: u16 = deobf_u16(0x{:04X});\n", port_data ^ k_u16));
                let port_acpi_cmd = driver["PORT_ACPI_EC_CMD"].as_u64().unwrap_or(0x66) as u16;
                let port_acpi_data = driver["PORT_ACPI_EC_DATA"].as_u64().unwrap_or(0x62) as u16;
                code.push_str(&format!("    pub const PORT_ACPI_EC_CMD: u16 = deobf_u16(0x{:04X});\n", port_acpi_cmd ^ k_u16));
                code.push_str(&format!("    pub const PORT_ACPI_EC_DATA: u16 = deobf_u16(0x{:04X});\n", port_acpi_data ^ k_u16));
                code.push_str("    pub const WMI_INPUT_LEN: usize = 64;\n");
                code.push_str("    pub const WMI_OUTPUT_LEN: usize = 256;\n");
                code.push_str("}\n\n");

                // cooler module
                code.push_str("pub mod cooler {\n    use super::*;\n\n");
                code.push_str("    pub const NUS_SERVICE_UUID: &str = \"6e400001-b5a3-f393-e0a9-e50e24dcca9e\";\n");
                code.push_str("    pub const NUS_TX_WRITE_UUID: &str = \"6e400002-b5a3-f393-e0a9-e50e24dcca9e\";\n");
                code.push_str("    pub const NUS_RX_NOTIFY_UUID: &str = \"6e400003-b5a3-f393-e0a9-e50e24dcca9e\";\n");
                code.push_str("    pub const FRAME_HEADER: u8 = 0xFE;\n");
                code.push_str("    pub const FRAME_TAIL: u8 = 0xEF;\n");
                if let Some(cooler_map) = cooler.as_object() {
                    for (k, v) in cooler_map {
                        let num = v.as_u64().unwrap_or(0) as u8;
                        let obf = num ^ k_u8;
                        code.push_str(&format!("    pub const {}: u8 = deobf_u8(0x{:02X});\n", k, obf));
                    }
                }
                code.push_str("    pub const VOLT_QUIET_7V: u8 = 7;\n");
                code.push_str("    pub const VOLT_BALANCE_8V: u8 = 8;\n");
                code.push_str("    pub const VOLT_TURBO_11V: u8 = 11;\n");
                code.push_str("}\n\n");

                // ite module
                code.push_str("pub mod ite {\n    use super::*;\n\n");
                let ite_vid = ite["ITE_VID"].as_u64().unwrap_or(0x048D) as u16;
                code.push_str(&format!("    pub const ITE_VID: u16 = deobf_u16(0x{:04X});\n", ite_vid ^ k_u16));
                let ite_pids = ["ITE_PID_KEYBOARD_PERKEY", "ITE_PID_KEYBOARD_FOURZONE", "ITE_PID_LIGHTBAR_BASE", "ITE_PID_LIGHTBAR_HINGE", "ITE_PID_AMBIENT_STRIP"];
                for pid in &ite_pids {
                    let num = ite[pid].as_u64().unwrap_or(0) as u16;
                    code.push_str(&format!("    pub const {}: u16 = deobf_u16(0x{:04X});\n", pid, num ^ k_u16));
                }
                let ite_usages = ["USAGE_PAGE_ITE", "USAGE_PAGE_FOURZONE", "USAGE_PAGE_ME_1ST"];
                for usage in &ite_usages {
                    let num = ite[usage].as_u64().unwrap_or(0) as u16;
                    code.push_str(&format!("    pub const {}: u16 = deobf_u16(0x{:04X});\n", usage, num ^ k_u16));
                }
                code.push_str("    pub const USAGE_KEYBOARD: u16 = 1;\n");
                code.push_str("    pub const USAGE_LIGHTBAR: u16 = 2;\n");
                code.push_str("    pub const CONTROL_KEYBOARD: u8 = 2;\n");
                code.push_str("    pub const CONTROL_LIGHTBAR: u8 = 34;\n");
                code.push_str("    pub const CONTROL_LOGO: u8 = 35;\n");
                code.push_str("    pub const CONTROL_HINGE: u8 = 33;\n");
                code.push_str("    pub const CONTROL_STOP: u8 = 5;\n");
                code.push_str("    pub const BLANK_KEYBOARD: u8 = 0;\n");
                code.push_str("    pub const BLANK_HINGE: u8 = 1;\n");
                code.push_str("    pub const BLANK_LIGHTBAR: u8 = 2;\n");
                code.push_str("    pub const BLANK_LOGO: u8 = 3;\n");
                code.push_str("}\n\n");

                code.push_str("pub mod miniled {\n");
                code.push_str("    pub const CMD_MINILED_ON: u64 = 0x000005000000000E;\n");
                code.push_str("    pub const CMD_MINILED_OFF: u64 = 0x000005000000000D;\n");
                code.push_str("}\n");

                fs::write(dest_path, code).expect("Failed to write generated_vault.rs");
                return;
            }
        }
    }

    // =========================================================================
    // FALLBACK MODE: Public Open Source Safe Mock Vault
    // =========================================================================
    let mut fallback = String::new();
    fallback.push_str("// Auto-generated Hardware Vault (Community Open-Source Mock Mode)\n");
    fallback.push_str("// Notice: config/ec_registry.json absent. Physical registers replaced with safe null-stubs.\n\n");
    fallback.push_str("pub const KEY_U16: u16 = 0x0000;\n");
    fallback.push_str("pub const KEY_U8: u8 = 0x00;\n");
    fallback.push_str("pub const KEY_U32: u32 = 0x00000000;\n");
    fallback.push_str("pub const IS_PROPRIETARY_VAULT_LOADED: bool = false;\n\n");

    fallback.push_str(r#"#[inline(always)]
pub const fn deobf_u16(val: u16) -> u16 { val }
#[inline(always)]
pub const fn deobf_u8(val: u8) -> u8 { val }
#[inline(always)]
pub const fn deobf_u32(val: u32) -> u32 { val }
#[inline(always)]
pub fn deobf_bytes(bytes: &[u8], _key: u8) -> Vec<u8> { bytes.to_vec() }

pub mod ec {
    pub const ADDR_PROJECT_ID: u16 = 0;
    pub const ADDR_AP_OEM_BYTE: u16 = 0;
    pub const ADDR_SUPPORT_BYTE1: u16 = 0;
    pub const ADDR_SUPPORT_BYTE2: u16 = 0;
    pub const ADDR_SUPPORT_BYTE6: u16 = 0;
    pub const ADDR_SUPPORT_BYTE3: u16 = 0;
    pub const ADDR_TRIGGER_BYTE: u16 = 0;
    pub const ADDR_STATUS_BYTE: u16 = 0;
    pub const ADDR_RGB_RED: u16 = 0;
    pub const ADDR_RGB_GREEN: u16 = 0;
    pub const ADDR_RGB_BLUE: u16 = 0;
    pub const ADDR_LIGHTBAR_CONTROL_BYTE: u16 = 0;
    pub const ADDR_REDBAR: u16 = 0;
    pub const ADDR_GREENBAR: u16 = 0;
    pub const ADDR_BLUEBAR: u16 = 0;
    pub const ADDR_MODE_INDEX: u16 = 0;
    pub const ADDR_BATTERY_PROTECTION_MODE: u16 = 0;
    pub const ADDR_BATTERY_LIMIT_UP: u16 = 0;
    pub const ADDR_BATTERY_LIMIT_DOWN: u16 = 0;
    pub const ADDR_BATTERY_PLATFORM_OVERRIDE: u16 = 0;
    pub const ADDR_BATTERY_GATE_STATUS: u16 = 0;
    pub const ADDR_PLATFORM_REAL_ID: u16 = 0;
    pub const ADDR_MAIN_FAN_RPM_B1: u16 = 0;
    pub const ADDR_MAIN_FAN_RPM_B2: u16 = 0;
    pub const ADDR_SECOND_FAN_RPM_B1: u16 = 0;
    pub const ADDR_SECOND_FAN_RPM_B2: u16 = 0;
    pub const ADDR_AC_RECOVERY: u16 = 0;
    pub const ADDR_CAMERA_CUTOFF: u16 = 0;
    pub const ADDR_FN_LOCK: u16 = 0;
    pub const ADDR_BACKLIGHT_WHITE: u16 = 0;
    pub const ADDR_AP_BIOS_BYTE: u16 = 0;
    pub const ADDR_WATER_COOLER_FLAG: u16 = 0;
    pub const ADDR_PL1_LOW: u16 = 0;
    pub const ADDR_PL1_HIGH: u16 = 0;
    pub const ADDR_PL2_LOW: u16 = 0;
    pub const ADDR_PL2_HIGH: u16 = 0;
    pub const ADDR_TGP_OFFSET: u16 = 0;
    pub const ADDR_OFFICE_PL1: u16 = 0;
    pub const ADDR_OFFICE_PL2: u16 = 0;
    pub const ADDR_OFFICE_PL4: u16 = 0;
    pub const ADDR_OFFICE_TCC: u16 = 0;
    pub const ADDR_BALANCE_PL1: u16 = 0;
    pub const ADDR_BALANCE_PL2: u16 = 0;
    pub const ADDR_BALANCE_PL4: u16 = 0;
    pub const ADDR_BALANCE_TCC: u16 = 0;
    pub const ADDR_TURBO_PL1: u16 = 0;
    pub const ADDR_TURBO_PL2: u16 = 0;
    pub const ADDR_TURBO_PL4: u16 = 0;
    pub const ADDR_TURBO_TCC: u16 = 0;
    pub const ADDR_CUSTOM_MODE: u16 = 0;
    pub const ADDR_FAN_RAMP_RATE: u16 = 0;
    pub const ADDR_FAN_ISOLATED: u16 = 0;
    pub const ADDR_RGB_CONTROL: u16 = 0;
    pub const ADDR_FAN_TABLE_LATCH: u16 = 0;
    pub const ADDR_POWER_STATUS: u16 = 0;
    pub const ADDR_DC_ROUND_JACK: u16 = 0;
    pub const ADDR_FAN_BOOST_CLAMP: u16 = 0;
    pub const ADDR_SUPER_PERFORMANCE: u16 = 0;
    pub const ADDR_TURBO_TRIGGER_1115: u16 = 0;
    pub const ADDR_CUSTOM_PL1: u16 = 0;
    pub const ADDR_CUSTOM_PL2: u16 = 0;
    pub const ADDR_CUSTOM_PL4: u16 = 0;
    pub const ADDR_CUSTOM_TCC: u16 = 0;
    pub const ADDR_DB_CTRL1: u16 = 0;
    pub const ADDR_DB_FLAG1: u16 = 0;
    pub const ADDR_DB_WATTS1: u16 = 0;
    pub const ADDR_DB_CTRL2: u16 = 0;
    pub const ADDR_DB_FLAG2: u16 = 0;
    pub const ADDR_DB_WATTS2: u16 = 0;
    pub const ADDR_GPU_DB_MAX: u16 = 0;
    pub const ADDR_MAIN_FAN_L_DUTY_BYTE: u16 = 0;
    pub const ADDR_MAIN_FAN_R_DUTY_BYTE: u16 = 0;
    pub const ADDR_LC_FAN_VALUE: u16 = 0;
    pub const ADDR_RAMFAN_CPU_UPT_BASE: u16 = 0;
    pub const ADDR_RAMFAN_CPU_DOWNT_BASE: u16 = 0;
    pub const ADDR_RAMFAN_CPU_DUTY_BASE: u16 = 0;
    pub const ADDR_RAMFAN_GPU_UPT_BASE: u16 = 0;
    pub const ADDR_RAMFAN_GPU_DOWNT_BASE: u16 = 0;
    pub const ADDR_RAMFAN_GPU_DUTY_BASE: u16 = 0;
    pub const ADDR_RAMFAN_GPU_MAX_THRESHOLD1: u16 = 0;
    pub const ADDR_RAMFAN_GPU_MAX_THRESHOLD2: u16 = 0;
    pub const ADDR_RAMFAN_TABLE_CTRL: u16 = 0;
    pub const ADDR_BLD_FAN1_RPM_SET: u16 = 0;
    pub const ADDR_BLD_FAN2_RPM_SET: u16 = 0;
    pub const ADDR_BLD_FAN_CTRL_MODE: u16 = 0;
    pub const ADDR_BLD_FAN1_RPM_READ: u16 = 0;
    pub const ADDR_BLD_FAN2_RPM_READ: u16 = 0;
    pub const ADDR_BLD_CHIP_ID: u16 = 0;
    pub const ADDR_BLD_EC_INIT: u16 = 0;
    pub const ADDR_BLD_BAT_CMD: u16 = 0;
    pub const ADDR_BLD_BAT_SUBCMD: u16 = 0;
    pub const ADDR_BLD_BAT_RESULT: u16 = 0;
    pub const ADDR_BLD_BAT_HIGH: u16 = 0;
    pub const ADDR_BLD_BAT_LOW: u16 = 0;
    pub const ADDR_BLD_BAT_MODE_STATUS: u16 = 0;
    pub const ADDR_LOCAL_DIMMING_SUPPORT: u16 = 0;
    pub const ADDR_MYFAN2_L1_PWM: u16 = 0;
    pub const ADDR_MYFAN2_L2_PWM: u16 = 0;
    pub const ADDR_MYFAN2_L3_PWM: u16 = 0;
    pub const ADDR_MYFAN2_L4_PWM: u16 = 0;
    pub const ADDR_MYFAN2_L5_PWM: u16 = 0;
    pub const ADDR_MYFAN2_L1_DEFAULT: u16 = 0;
    pub const ADDR_MYFAN2_L2_DEFAULT: u16 = 0;
    pub const ADDR_MYFAN2_L3_DEFAULT: u16 = 0;
    pub const ADDR_MYFAN2_L4_DEFAULT: u16 = 0;
    pub const ADDR_MYFAN2_L5_DEFAULT: u16 = 0;
}

pub mod mask {
    pub const TRIGGER_WINLOCK: u8 = 0x01;
    pub const TRIGGER_LIGHTBAR: u8 = 0x02;
    pub const TRIGGER_FANBOOST: u8 = 0x04;
    pub const TRIGGER_SILENT_MODE: u8 = 0x08;
    pub const TRIGGER_USB_CHARGE: u8 = 0x10;
    pub const TRIGGER_RGB_KB: u8 = 0x20;
    pub const TRIGGER_RGB_RAINBOW: u8 = 0x80;
    pub const STATUS_WINLOCK: u8 = 0x01;
    pub const STATUS_FANBOOST: u8 = 0x04;
    pub const BATTERY_MODE_LONGLIFE: u8 = 0x01;
    pub const BATTERY_MODE_BALANCE: u8 = 0x11;
    pub const BATTERY_MODE_STATION: u8 = 0x21;
    pub const BATTERY_OVERRIDE_PIN_VALUE: u8 = 0x05;
    pub const BATTERY_OVERRIDE_RESET_VALUE: u8 = 0xFF;
    pub const BATTERY_GATE_ACTIVE_BIT: u8 = 0x04;
    pub const TOUCHPAD_BITMASK: u8 = 0x08;
    pub const AC_RECOVERY_BITMASK: u8 = 0x08;
    pub const CAMERA_CUTOFF_BITMASK: u8 = 0x08;
    pub const MASK_QKEY_WHITE_1830: u8 = 0x80;
    pub const MASK_QKEY_WHITE_1831: u8 = 0x40;
    pub const MASK_LIQUID_COOLING_SUPPORT: u8 = 0x80;
    pub const FAN_TABLE_LATCH_BIT: u8 = 0x04;
    pub const FAN_BOOST_CLAMP_BIT: u8 = 0x40;
    pub const SUPER_PERF_BIT: u8 = 0x10;
    pub const POWER_STATUS_DC_JACK: u8 = 0x01;
    pub const POWER_STATUS_TYPE_C_MASK: u8 = 0x3E;
    pub const MASK_LIGHTBAR_SUPPORT: u8 = 0x40;
    pub const MASK_FOURZONE_DECLARED: u8 = 0x02;
    pub const MASK_FOURZONE_READY: u8 = 0x04;
    pub const MASK_LIGHTBAR_EC_SUPPORT: u8 = 0x10;
    pub const BLD_FAN_CPU_MANUAL_MASK: u8 = 0x02;
    pub const BLD_FAN_GPU_MANUAL_MASK: u8 = 0x08;
    pub const BLD_FAN_RELEASE_MASK: u8 = 0x00;
    pub const BLD_BAT_SUBCMD_DUAL_THRESHOLD: u8 = 0x0D;
    pub const BLD_BAT_SUBCMD_RESTORE_DEFAULT: u8 = 0x0F;
    pub const BLD_BAT_CMD_BATTERY: u8 = 0x10;
    pub const BLD_BAT_RESULT_SUCCESS: u8 = 0x55;
    pub const MASK_LOCAL_DIMMING_SUPPORT: u8 = 0x01;
}

pub mod driver {
    pub fn get_driver_path_null_terminated() -> Vec<u8> { b"\\\\.\\ACPIDriver\0".to_vec() }
    pub const IOCTL_ECREAD: u32 = 0;
    pub const IOCTL_ECWRITE: u32 = 0;
    pub const PORT_BLD_EC_ADDR: u16 = 0x4E;
    pub const PORT_BLD_EC_DATA: u16 = 0x4F;
    pub const PORT_ACPI_EC_CMD: u16 = 0x66;
    pub const PORT_ACPI_EC_DATA: u16 = 0x62;
    pub const WMI_INPUT_LEN: usize = 64;
    pub const WMI_OUTPUT_LEN: usize = 256;
}

pub mod cooler {
    pub const NUS_SERVICE_UUID: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
    pub const NUS_TX_WRITE_UUID: &str = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
    pub const NUS_RX_NOTIFY_UUID: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";
    pub const FRAME_HEADER: u8 = 0xFE;
    pub const FRAME_TAIL: u8 = 0xEF;
    pub const CMD_FILL_WATER: u8 = 0;
    pub const CMD_SET_FAN: u8 = 0;
    pub const CMD_SET_PUMP: u8 = 0;
    pub const CMD_SET_RESERVOIR_LED: u8 = 0;
    pub const CMD_POLL_FLOW: u8 = 0;
    pub const CMD_SET_FAN_LED: u8 = 0;
    pub const VOLT_QUIET_7V: u8 = 7;
    pub const VOLT_BALANCE_8V: u8 = 8;
    pub const VOLT_TURBO_11V: u8 = 11;
}

pub mod ite {
    pub const ITE_VID: u16 = 0;
    pub const ITE_PID_KEYBOARD_PERKEY: u16 = 0;
    pub const ITE_PID_KEYBOARD_FOURZONE: u16 = 0;
    pub const ITE_PID_LIGHTBAR_BASE: u16 = 0;
    pub const ITE_PID_LIGHTBAR_HINGE: u16 = 0;
    pub const ITE_PID_AMBIENT_STRIP: u16 = 0;
    pub const USAGE_PAGE_ITE: u16 = 0;
    pub const USAGE_PAGE_FOURZONE: u16 = 0;
    pub const USAGE_PAGE_ME_1ST: u16 = 0;
    pub const USAGE_KEYBOARD: u16 = 1;
    pub const USAGE_LIGHTBAR: u16 = 2;
    pub const CONTROL_KEYBOARD: u8 = 2;
    pub const CONTROL_LIGHTBAR: u8 = 34;
    pub const CONTROL_LOGO: u8 = 35;
    pub const CONTROL_HINGE: u8 = 33;
    pub const CONTROL_STOP: u8 = 5;
    pub const BLANK_KEYBOARD: u8 = 0;
    pub const BLANK_HINGE: u8 = 1;
    pub const BLANK_LIGHTBAR: u8 = 2;
    pub const BLANK_LOGO: u8 = 3;
}

pub mod miniled {
    pub const CMD_MINILED_ON: u64 = 0x000005000000000E;
    pub const CMD_MINILED_OFF: u64 = 0x000005000000000D;
}
"#);

    fs::write(dest_path, fallback).expect("Failed to write fallback generated_vault.rs");
}

fn main() {
    generate_hardware_vault();
}
