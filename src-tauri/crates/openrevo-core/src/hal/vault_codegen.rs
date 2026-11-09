use serde_json::Value;
use std::{fs, path::Path};

/// Aliases emitted into `ec` after the dictionary entries.
///
/// The left name must not appear in the dictionary (it would be defined twice) and the
/// right name must exist there; previously both mistakes only surfaced as an `E0428` /
/// `E0425` inside the generated file, where the cause is hard to see.
const EC_ALIASES: [(&str, &str); 9] = [
    ("ADDR_SUPPORT_BYTE3", "ADDR_SUPPORT_BYTE6"),
    ("ADDR_DC_ROUND_JACK", "ADDR_POWER_STATUS"),
    ("ADDR_SUPER_PERFORMANCE", "ADDR_FAN_BOOST_CLAMP"),
    ("ADDR_GPU_DB_MAX", "ADDR_DB_WATTS1"),
    ("ADDR_RGB_CONTROL", "ADDR_FAN_ISOLATED"),
    ("ADDR_MYFAN2_L1_PWM", "ADDR_DB_CTRL1"),
    ("ADDR_MYFAN2_L2_PWM", "ADDR_TGP_OFFSET"),
    ("ADDR_MYFAN2_L3_PWM", "ADDR_DB_FLAG1"),
    ("ADDR_MYFAN2_L4_PWM", "ADDR_DB_WATTS1"),
];

fn validate_registry(value: &Value) -> Result<(), String> {
    for (section, maximum) in [
        ("ec", u16::MAX as u64),
        ("mask", u8::MAX as u64),
        ("cooler", u8::MAX as u64),
        ("ite", u16::MAX as u64),
    ] {
        let entries = value[section]
            .as_object()
            .ok_or_else(|| format!("{} must be an object", section))?;
        if entries.is_empty() {
            return Err(format!("{} must not be empty", section));
        }
        for (name, number) in entries {
            if !name.starts_with(|c: char| c.is_ascii_uppercase())
                || !name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                return Err(format!("Invalid hardware constant name: {}", name));
            }
            checked(number, maximum).map_err(|e| format!("{}.{}: {}", section, name, e))?;
        }
    }
    let ec = value["ec"]
        .as_object()
        .ok_or_else(|| "ec must be an object".to_string())?;
    for (alias, target) in EC_ALIASES {
        if ec.contains_key(alias) {
            return Err(format!(
                "ec.{} collides with the built-in alias for {}",
                alias, target
            ));
        }
        if !ec.contains_key(target) {
            return Err(format!("ec.{} is required by the alias {}", target, alias));
        }
    }
    for (section, names, maximum) in [
        ("keys", &["key_u16"][..], u16::MAX as u64),
        ("keys", &["key_u8", "driver_obf_key"][..], u8::MAX as u64),
        ("keys", &["key_u32"][..], u32::MAX as u64),
        (
            "driver",
            &["IOCTL_ECREAD", "IOCTL_ECWRITE"][..],
            u32::MAX as u64,
        ),
        (
            "driver",
            &[
                "PORT_BLD_EC_ADDR",
                "PORT_BLD_EC_DATA",
                "PORT_ACPI_EC_CMD",
                "PORT_ACPI_EC_DATA",
            ][..],
            u16::MAX as u64,
        ),
        (
            "ite",
            &[
                "ITE_VID",
                "ITE_PID_KEYBOARD_PERKEY",
                "ITE_PID_KEYBOARD_FOURZONE",
                "ITE_PID_LIGHTBAR_BASE",
                "ITE_PID_LIGHTBAR_HINGE",
                "ITE_PID_AMBIENT_STRIP",
                "USAGE_PAGE_ITE",
                "USAGE_PAGE_FOURZONE",
                "USAGE_PAGE_ME_1ST",
            ][..],
            u16::MAX as u64,
        ),
    ] {
        for name in names {
            checked(&value[section][name], maximum)
                .map_err(|e| format!("{}.{}: {}", section, name, e))?;
        }
    }
    Ok(())
}

fn checked(value: &Value, maximum: u64) -> Result<(), String> {
    match value.as_u64() {
        Some(number) if number <= maximum => Ok(()),
        _ => Err(format!("expected unsigned integer <= {}", maximum)),
    }
}

pub fn registry_code(content: &str) -> Result<String, String> {
    // Windows editors happily save the registry with a UTF-8 BOM; serde_json rejects it.
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    // Note: serde_json keeps the last value of a duplicated key, so duplicates are not rejected
    // here. The shipped registry has none; detecting them would need a custom visitor.
    let val: Value = serde_json::from_str(content)
        .map_err(|e| format!("Invalid hardware registry JSON: {}", e))?;
    validate_registry(&val)?;
    let keys = &val["keys"];
    let ec = &val["ec"];
    let mask = &val["mask"];
    let driver = &val["driver"];
    let cooler = &val["cooler"];
    let ite = &val["ite"];

    let k_u16 = u16::try_from(keys["key_u16"].as_u64().expect("validated integer"))
        .expect("validated range");
    let k_u8 =
        u8::try_from(keys["key_u8"].as_u64().expect("validated integer")).expect("validated range");
    let k_u32 = u32::try_from(keys["key_u32"].as_u64().expect("validated integer"))
        .expect("validated range");
    let drv_obf_key = u8::try_from(keys["driver_obf_key"].as_u64().expect("validated integer"))
        .expect("validated range");

    let mut code = String::new();
    code.push_str("// Auto-generated Hardware Vault (Proprietary Physical Hardware Keys Loaded)\n");
    code.push_str(
        "// Build-time obfuscation active. Zero plaintext addresses in binary string sections.\n\n",
    );
    code.push_str(&format!("pub const KEY_U16: u16 = 0x{:04X};\n", k_u16));
    code.push_str(&format!("pub const KEY_U8: u8 = 0x{:02X};\n", k_u8));
    code.push_str(&format!("pub const KEY_U32: u32 = 0x{:08X};\n", k_u32));
    code.push_str("pub const IS_PROPRIETARY_VAULT_LOADED: bool = true;\n\n");

    code.push_str(
        r#"#[inline(always)]
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

    "#,
    );

    // ec module
    code.push_str("pub mod ec {\n    use super::*;\n\n");
    if let Some(ec_map) = ec.as_object() {
        for (k, v) in ec_map {
            let num =
                u16::try_from(v.as_u64().expect("validated integer")).expect("validated range");
            let obf = num ^ k_u16;
            code.push_str(&format!(
                "    pub const {}: u16 = deobf_u16(0x{:04X});\n",
                k, obf
            ));
        }
    }
    // Aliases
    for (alias, target) in EC_ALIASES {
        code.push_str(&format!(
            "    pub const {}: u16 = {};\n",
            alias, target
        ));
    }
    code.push_str("}\n\n");

    // mask module
    code.push_str("pub mod mask {\n    use super::*;\n\n");
    if let Some(mask_map) = mask.as_object() {
        for (k, v) in mask_map {
            let num =
                u8::try_from(v.as_u64().expect("validated integer")).expect("validated range");
            let obf = num ^ k_u8;
            code.push_str(&format!(
                "    pub const {}: u8 = deobf_u8(0x{:02X});\n",
                k, obf
            ));
        }
    }
    code.push_str("}\n\n");

    // driver module
    code.push_str("pub mod driver {\n    use super::*;\n\n");
    let drv_path = "\\\\.\\ACPIDriver\0";
    let obf_bytes: Vec<String> = drv_path
        .bytes()
        .map(|b| format!("0x{:02X}", b ^ drv_obf_key))
        .collect();
    code.push_str(&format!(
        "    const OBF_DRIVER_PATH: &[u8] = &[{}];\n",
        obf_bytes.join(", ")
    ));
    code.push_str(&format!("    pub fn get_driver_path_null_terminated() -> Vec<u8> {{\n        deobf_bytes(OBF_DRIVER_PATH, 0x{:02X})\n    }}\n\n", drv_obf_key));
    let ioctl_read = u32::try_from(driver["IOCTL_ECREAD"].as_u64().expect("validated integer"))
        .expect("validated range");
    let ioctl_write = u32::try_from(driver["IOCTL_ECWRITE"].as_u64().expect("validated integer"))
        .expect("validated range");
    code.push_str(&format!(
        "    pub const IOCTL_ECREAD: u32 = deobf_u32(0x{:08X});\n",
        ioctl_read ^ k_u32
    ));
    code.push_str(&format!(
        "    pub const IOCTL_ECWRITE: u32 = deobf_u32(0x{:08X});\n",
        ioctl_write ^ k_u32
    ));
    let port_addr = u16::try_from(
        driver["PORT_BLD_EC_ADDR"]
            .as_u64()
            .expect("validated integer"),
    )
    .expect("validated range");
    let port_data = u16::try_from(
        driver["PORT_BLD_EC_DATA"]
            .as_u64()
            .expect("validated integer"),
    )
    .expect("validated range");
    code.push_str(&format!(
        "    pub const PORT_BLD_EC_ADDR: u16 = deobf_u16(0x{:04X});\n",
        port_addr ^ k_u16
    ));
    code.push_str(&format!(
        "    pub const PORT_BLD_EC_DATA: u16 = deobf_u16(0x{:04X});\n",
        port_data ^ k_u16
    ));
    let port_acpi_cmd = u16::try_from(
        driver["PORT_ACPI_EC_CMD"]
            .as_u64()
            .expect("validated integer"),
    )
    .expect("validated range");
    let port_acpi_data = u16::try_from(
        driver["PORT_ACPI_EC_DATA"]
            .as_u64()
            .expect("validated integer"),
    )
    .expect("validated range");
    code.push_str(&format!(
        "    pub const PORT_ACPI_EC_CMD: u16 = deobf_u16(0x{:04X});\n",
        port_acpi_cmd ^ k_u16
    ));
    code.push_str(&format!(
        "    pub const PORT_ACPI_EC_DATA: u16 = deobf_u16(0x{:04X});\n",
        port_acpi_data ^ k_u16
    ));
    code.push_str("    pub const WMI_INPUT_LEN: usize = 64;\n");
    code.push_str("    pub const WMI_OUTPUT_LEN: usize = 256;\n");
    code.push_str("}\n\n");

    // cooler module
    code.push_str("pub mod cooler {\n    use super::*;\n\n");
    code.push_str(
        "    pub const NUS_SERVICE_UUID: &str = \"6e400001-b5a3-f393-e0a9-e50e24dcca9e\";\n",
    );
    code.push_str(
        "    pub const NUS_TX_WRITE_UUID: &str = \"6e400002-b5a3-f393-e0a9-e50e24dcca9e\";\n",
    );
    code.push_str(
        "    pub const NUS_RX_NOTIFY_UUID: &str = \"6e400003-b5a3-f393-e0a9-e50e24dcca9e\";\n",
    );
    code.push_str("    pub const FRAME_HEADER: u8 = 0xFE;\n");
    code.push_str("    pub const FRAME_TAIL: u8 = 0xEF;\n");
    if let Some(cooler_map) = cooler.as_object() {
        for (k, v) in cooler_map {
            let num =
                u8::try_from(v.as_u64().expect("validated integer")).expect("validated range");
            let obf = num ^ k_u8;
            code.push_str(&format!(
                "    pub const {}: u8 = deobf_u8(0x{:02X});\n",
                k, obf
            ));
        }
    }
    code.push_str("    pub const VOLT_QUIET_7V: u8 = 7;\n");
    code.push_str("    pub const VOLT_BALANCE_8V: u8 = 8;\n");
    code.push_str("    pub const VOLT_TURBO_11V: u8 = 11;\n");
    code.push_str("}\n\n");

    // ite module
    code.push_str("pub mod ite {\n    use super::*;\n\n");
    let ite_vid = u16::try_from(ite["ITE_VID"].as_u64().expect("validated integer"))
        .expect("validated range");
    code.push_str(&format!(
        "    pub const ITE_VID: u16 = deobf_u16(0x{:04X});\n",
        ite_vid ^ k_u16
    ));
    let ite_pids = [
        "ITE_PID_KEYBOARD_PERKEY",
        "ITE_PID_KEYBOARD_FOURZONE",
        "ITE_PID_LIGHTBAR_BASE",
        "ITE_PID_LIGHTBAR_HINGE",
        "ITE_PID_AMBIENT_STRIP",
    ];
    for pid in &ite_pids {
        let num =
            u16::try_from(ite[pid].as_u64().expect("validated integer")).expect("validated range");
        code.push_str(&format!(
            "    pub const {}: u16 = deobf_u16(0x{:04X});\n",
            pid,
            num ^ k_u16
        ));
    }
    let ite_usages = ["USAGE_PAGE_ITE", "USAGE_PAGE_FOURZONE", "USAGE_PAGE_ME_1ST"];
    for usage in &ite_usages {
        let num = u16::try_from(ite[usage].as_u64().expect("validated integer"))
            .expect("validated range");
        code.push_str(&format!(
            "    pub const {}: u16 = deobf_u16(0x{:04X});\n",
            usage,
            num ^ k_u16
        ));
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

    Ok(code)
}

fn fallback_code() -> String {
    let mut fallback = String::new();
    fallback.push_str("// Auto-generated Hardware Vault (Community Open-Source Mock Mode)\n");
    fallback.push_str("// Notice: config/ec_registry.json absent. Physical registers replaced with safe null-stubs.\n\n");
    fallback.push_str("pub const KEY_U16: u16 = 0x0000;\n");
    fallback.push_str("pub const KEY_U8: u8 = 0x00;\n");
    fallback.push_str("pub const KEY_U32: u32 = 0x00000000;\n");
    fallback.push_str("pub const IS_PROPRIETARY_VAULT_LOADED: bool = false;\n\n");

    fallback.push_str(
        r#"#[inline(always)]
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
    pub const ADDR_TURBO_TRIGGER: u16 = 0;
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
    pub const MASK_QKEY_WHITE_AC_RECOVERY: u8 = 0x80;
    pub const MASK_QKEY_WHITE_CAMERA_CUTOFF: u8 = 0x40;
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
"#,
    );

    fallback
}

/// Emits the vault module for `registry` into `OUT_DIR`.
///
/// A missing registry is the documented community-fork case and produces the mock
/// vault. An existing file that cannot be read, parsed or validated stops the build:
/// silently falling back to the mock would hide a broken hardware configuration.
/// Reads the registry, or reports that it is absent.
///
/// `Ok(None)` is the documented community-fork case (mock vault). An existing path that cannot
/// be read is an error: silently falling back to the mock would hide a broken configuration.
fn vault_source(registry: &Path) -> Result<Option<String>, String> {
    if registry.is_dir() {
        return Err(format!(
            "Hardware registry {} is a directory, expected a JSON file",
            registry.display()
        ));
    }
    match fs::read_to_string(registry) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Cannot read hardware registry {}: {}",
            registry.display(),
            error
        )),
    }
}

pub fn generate_hardware_vault(registry: &Path) {
    // Cargo treats a registered path that does not exist as permanently stale (re-running this
    // script on every build), so the file itself cannot be watched before it appears. Watch its
    // directory instead; while that directory is missing too (a fork that removed `config/`),
    // fall back to the nearest existing ancestor, which must exist and therefore does not spin.
    // The fallback only stays in effect until the real directory is created.
    let directory = registry
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let watched = if directory.is_dir() {
        directory
    } else {
        directory
            .ancestors()
            .find(|ancestor| !ancestor.as_os_str().is_empty() && ancestor.is_dir())
            .unwrap_or_else(|| Path::new("."))
    };
    println!("cargo:rerun-if-changed={}", watched.display());

    let code = match vault_source(registry) {
        Ok(Some(content)) => registry_code(&content).unwrap_or_else(|error| {
            panic!(
                "Hardware registry {} rejected: {}",
                registry.display(),
                error
            )
        }),
        Ok(None) => fallback_code(),
        Err(error) => panic!("{}", error),
    };
    let output = std::env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo");
    fs::write(Path::new(&output).join("generated_vault.rs"), code)
        .expect("Failed to write generated hardware vault");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_vault_generation_rejects_wrong_types_overflow_and_missing_keys() {
        let source = include_str!("../../../../config/ec_registry.json");
        let valid: Value = serde_json::from_str(source).unwrap();
        assert!(registry_code(source)
            .unwrap()
            .contains("IS_PROPRIETARY_VAULT_LOADED: bool = true"));
        assert!(registry_code(source).unwrap().contains("pub const ITE_PID_AMBIENT_STRIP: u16"));
        assert!(fallback_code().contains("pub const ITE_PID_AMBIENT_STRIP: u16"));
        for (section, name, bad) in [
            ("keys", "key_u16", serde_json::json!(65536)),
            ("keys", "key_u8", serde_json::json!(-1)),
            ("keys", "key_u32", serde_json::json!(4294967296u64)),
            ("ec", "ADDR_PROJECT_ID", serde_json::json!("1856")),
            ("mask", "TRIGGER_WINLOCK", serde_json::json!(256)),
            ("driver", "IOCTL_ECREAD", Value::Null),
            ("ite", "ITE_VID", serde_json::json!(65536)),
            ("ite", "ITE_PID_AMBIENT_STRIP", Value::Null),
            ("cooler", "CMD_SET_FAN", serde_json::json!(false)),
        ] {
            let mut invalid = valid.clone();
            invalid[section][name] = bad;
            assert!(
                registry_code(&invalid.to_string()).is_err(),
                "{}.{}",
                section,
                name
            );
        }
        let mut invalid = valid;
        invalid.as_object_mut().unwrap().remove("ec");
        assert!(registry_code(&invalid.to_string()).is_err());
        assert!(registry_code("not JSON").is_err());
        assert!(fallback_code().contains("IS_PROPRIETARY_VAULT_LOADED: bool = false"));
    }

    #[test]
    fn regression_vault_rejects_alias_collisions_and_missing_alias_targets() {
        let source = include_str!("../../../../config/ec_registry.json");
        let valid: Value = serde_json::from_str(source).unwrap();

        // `ec` must not redefine a built-in alias, otherwise the generated module has two
        // `ADDR_RGB_CONTROL` constants (E0428 in OUT_DIR instead of a clear registry error).
        for (alias, _) in EC_ALIASES {
            let mut invalid = valid.clone();
            invalid["ec"][alias] = serde_json::json!(0x42);
            let error = registry_code(&invalid.to_string()).unwrap_err();
            assert!(error.contains(alias), "{}: {}", alias, error);
        }

        // Every alias target has to exist, otherwise the generated module fails with E0425.
        for (alias, target) in EC_ALIASES {
            let mut invalid = valid.clone();
            invalid["ec"].as_object_mut().unwrap().remove(target);
            let error = registry_code(&invalid.to_string()).unwrap_err();
            assert!(error.contains(target), "{}: {}", alias, error);
        }
    }

    #[test]
    fn regression_vault_accepts_a_utf8_bom() {
        let source = include_str!("../../../../config/ec_registry.json");
        let with_bom = format!("\u{feff}{}", source);
        assert_eq!(registry_code(&with_bom).unwrap(), registry_code(source).unwrap());
    }

    #[test]
    fn regression_vault_source_distinguishes_missing_broken_and_present() {
        // Same convention as the OEM-sync tests: a scratch directory under the crate's target
        // dir, which is always writable during a build.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/vault-source-tests")
            .join(std::process::id().to_string());
        std::fs::create_dir_all(&root).unwrap();

        // No file at all is the community-fork case: mock vault, not an error.
        assert!(vault_source(&root.join("missing.json")).unwrap().is_none());

        // A directory is a configuration mistake and must stop the build.
        let directory = root.join("directory.json");
        std::fs::create_dir_all(&directory).unwrap();
        assert!(vault_source(&directory)
            .unwrap_err()
            .contains("is a directory"));

        // A UTF-8 BOM has to work end to end (read from disk, then parsed).
        let bom = root.join("bom.json");
        std::fs::write(
            &bom,
            format!(
                "\u{feff}{}",
                include_str!("../../../../config/ec_registry.json")
            ),
        )
        .unwrap();
        let content = vault_source(&bom).unwrap().unwrap();
        assert!(registry_code(&content)
            .unwrap()
            .contains("IS_PROPRIETARY_VAULT_LOADED: bool = true"));

        // Broken content is reported by the parser instead of degrading to the mock vault.
        let broken = root.join("broken.json");
        std::fs::write(&broken, "not JSON").unwrap();
        assert!(registry_code(&vault_source(&broken).unwrap().unwrap()).is_err());

        std::fs::remove_dir_all(&root).ok();
    }
}
