// OpenRevo Declarative Driver - Model Manifest Serde Specification
// Defines identity, capabilities, communication channels, action primitives, and guardrails.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub author: Option<String>,
    pub identity: ModelIdentity,
    pub capabilities: ModelCapabilities,
    pub channels: HashMap<String, ChannelDef>,
    pub actions: HashMap<String, ActionDef>,
    pub guardrails: Option<GuardrailDef>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelIdentity {
    /// Form factor of the laptop: "thin_light" | "gaming" | "mini_pc" | "handheld"
    pub form_factor: String,
    /// High-level UI layout preset: "ultrabook_compact" | "gaming_full"
    #[serde(default = "default_ui_preset")]
    pub ui_preset: String,
    /// DMI Hardware matching signatures
    pub dmi_match: DmiMatchRule,
}

fn default_ui_preset() -> String {
    "ultrabook_compact".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmiMatchRule {
    pub vendor: Option<String>,
    #[serde(default)]
    pub board_products: Vec<String>,
    #[serde(default)]
    pub product_names: Vec<String>,
    #[serde(default)]
    pub bios_versions: Option<BiosVersionRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiosVersionRange {
    pub min: Option<String>,
    pub max: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    #[serde(default)]
    pub discrete_gpu: bool,
    #[serde(default)]
    pub gpu_mux_type: Option<String>,
    #[serde(default)]
    pub supported_gpu_modes: Vec<String>,
    #[serde(default)]
    pub water_cooling: bool,
    #[serde(default)]
    pub fan_custom_curve: bool,
    #[serde(default = "default_true")]
    pub fan_independent_channels: bool,
    #[serde(default = "default_true")]
    pub fan_ramp_rate: bool,
    #[serde(default)]
    pub fan_control_mode: Option<String>,
    #[serde(default)]
    pub fan_boost_mode: Option<String>,
    #[serde(default)]
    pub max_fan_rpm: Option<u32>,
    #[serde(default)]
    pub cpu_vendor: Option<String>,
    #[serde(default)]
    pub cpu_tuning_type: Option<String>,
    #[serde(default)]
    pub cpu_supports_fppt: Option<bool>,
    #[serde(default)]
    pub gpu_power_tuning_mode: Option<String>,
    #[serde(default)]
    pub battery_charge_limit: bool,
    #[serde(default)]
    pub battery_limit_mode: Option<String>,
    #[serde(default = "default_true")]
    pub fn_lock: bool,
    pub power_modes: Vec<String>,
    #[serde(default)]
    pub power_mode_watts: Option<HashMap<String, u32>>,
    pub rgb_keyboard: Option<KeyboardCapability>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardCapability {
    #[serde(default)]
    pub supported: bool,
    /// "monochrome_stepped" | "per_key_rgb" | "four_zone_rgb" | "single_zone_rgb" | "none"
    #[serde(rename = "type", default = "default_keyboard_type")]
    pub keyboard_type: String,
    #[serde(default)]
    pub steps: Vec<u8>,
    #[serde(default)]
    pub hardware_autonomous: bool,
    #[serde(default)]
    pub hotkey_osd_only: bool,
    #[serde(default)]
    pub sleep_timeout_supported: bool,
    #[serde(default)]
    pub timeout_options_seconds: Vec<u32>,
}

fn default_keyboard_type() -> String {
    "none".to_string()
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChannelDef {
    #[serde(rename = "wmi_mailbox")]
    WmiMailbox {
        namespace: String,
        class_name: String,
        method_name: String,
        #[serde(default = "default_mailbox_sizes")]
        buffer_sizes: MailboxSizes,
    },
    #[serde(rename = "acpi_ioctl")]
    AcpiIoctl {
        device_path: String,
        read_code: String,
        write_code: String,
    },
    #[serde(rename = "wmi_ip3")]
    WmiIp3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MailboxSizes {
    #[serde(rename = "in")]
    pub in_size: usize,
    #[serde(rename = "out")]
    pub out_size: usize,
}

fn default_mailbox_sizes() -> MailboxSizes {
    MailboxSizes {
        in_size: 64,
        out_size: 256,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractRule {
    pub offset: usize,
    /// "u8" | "u16_le" | "u16_be" | "u32_le" | "i8"
    #[serde(rename = "type")]
    pub value_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionDef {
    /// Map of sub-actions (e.g. power mode name -> input bytes)
    ModeMap(HashMap<String, Vec<serde_json::Value>>),
    /// Buffer-based Mailbox invocation
    BufferInvoke {
        channel: Option<String>,
        #[serde(default)]
        input: Vec<serde_json::Value>,
        parse: Option<HashMap<String, ExtractRule>>,
        mapping: Option<HashMap<String, String>>,
        timeout_ms: Option<u64>,
    },
    /// Raw byte sequence with optional parameter placeholders
    RawBytes(Vec<serde_json::Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailDef {
    #[serde(default)]
    pub safe_commands_only: bool,
    pub pl1_watts_limit: Option<RangeLimit>,
    pub pl2_watts_limit: Option<RangeLimit>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RangeLimit {
    pub min: u32,
    pub max: u32,
}

impl ModelManifest {
    pub fn matches_dmi(&self, board_product: &str, product_name: &str) -> bool {
        let match_wildcard = |target: &str, pattern: &str| -> bool {
            let t = target.trim().to_uppercase();
            let pat = pattern.trim().to_uppercase();
            if pat.is_empty() {
                return true;
            }
            let starts_wild = pat.starts_with('*');
            let ends_wild = pat.ends_with('*');
            let clean = pat.trim_matches('*');
            if clean.is_empty() {
                return true;
            }
            if starts_wild && ends_wild {
                t.contains(clean)
            } else if starts_wild {
                t.ends_with(clean)
            } else if ends_wild {
                t.starts_with(clean)
            } else {
                t == clean
            }
        };

        let board_matches = self.identity.dmi_match.board_products.is_empty()
            || self.identity.dmi_match.board_products.iter().any(|pattern| {
                match_wildcard(board_product, pattern)
            });

        let product_matches = self.identity.dmi_match.product_names.is_empty()
            || self.identity.dmi_match.product_names.iter().any(|pattern| {
                match_wildcard(product_name, pattern)
            });

        board_matches && product_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wujie_14_ip3_deserialization() {
        let json = include_str!("../../../../../resources/drivers/wujie_14_ip3.json");
        let manifest: ModelManifest = serde_json::from_str(json).expect("Failed to parse manifest");
        let action = manifest.actions.get("set_power_mode").expect("No set_power_mode");
        match action {
            ActionDef::ModeMap(map) => {
                assert_eq!(map.get("office").unwrap()[0], 2);
                assert_eq!(map.get("balance").unwrap()[0], 0);
                assert_eq!(map.get("turbo").unwrap()[0], 1);
            }
            _ => panic!("Expected ModeMap"),
        }
    }

    #[test]
    fn test_xingyao_14_p916f_deserialization() {
        let json = include_str!("../../../../../resources/drivers/xingyao_14_p916f.json");
        let manifest: ModelManifest = serde_json::from_str(json).expect("Failed to parse manifest");
        let action = manifest.actions.get("set_power_mode").expect("No set_power_mode");
        match action {
            ActionDef::ModeMap(map) => {
                // office: [3, 10, 1], balance: [3, 10, 2], turbo: [3, 10, 3]
                assert_eq!(map.get("office").unwrap()[2], 1);
                assert_eq!(map.get("balance").unwrap()[2], 2);
                assert_eq!(map.get("turbo").unwrap()[2], 3);
            }
            _ => panic!("Expected ModeMap"),
        }
    }

    #[test]
    fn test_wujie_14_bld_deserialization() {
        let json = include_str!("../../../../../resources/drivers/wujie_14_bld.json");
        let manifest: ModelManifest = serde_json::from_str(json).expect("Failed to parse manifest");
        let action = manifest.actions.get("set_power_mode").expect("No set_power_mode");
        match action {
            ActionDef::ModeMap(map) => {
                // office: [0, 251, 0, 8, 2], balance: [0, 251, 0, 8, 1], turbo: [0, 251, 0, 8, 0]
                assert_eq!(map.get("office").unwrap()[4], 2);
                assert_eq!(map.get("balance").unwrap()[4], 1);
                assert_eq!(map.get("turbo").unwrap()[4], 0);
            }
            _ => panic!("Expected ModeMap"),
        }
    }

    #[test]
    fn test_jiaolong_16_bld_matching() {
        let json = include_str!("../../../../../resources/drivers/jiaolong_16_bld.json");
        let manifest: ModelManifest = serde_json::from_str(json).expect("Failed to parse manifest");
        // Test real tester's DMI from report:
        // Board: MRID6-23, Product: Jiaolong Series MRID6
        assert!(manifest.matches_dmi("MRID6-23", "Jiaolong Series MRID6"));
        assert!(manifest.matches_dmi("MRID6", "JIAOLONG 16 PRO"));
    }

    #[test]
    fn test_wujie_x_series_deserialization_and_matching() {
        let json = include_str!("../../../../../resources/drivers/wujie_x_series.json");
        let manifest: ModelManifest = serde_json::from_str(json).expect("Failed to parse manifest");

        // Verify capabilities: thin & light, no dGPU, fan curve enabled
        assert_eq!(manifest.identity.form_factor, "thin_light");
        assert!(!manifest.capabilities.discrete_gpu);
        assert!(manifest.capabilities.fan_custom_curve);
        assert_eq!(manifest.capabilities.cpu_tuning_type.as_deref(), Some("none"));
        assert_eq!(manifest.capabilities.gpu_power_tuning_mode.as_deref(), Some("none"));

        // Verify power modes watts: 25W, 45W, 65W
        let watts = manifest.capabilities.power_mode_watts.as_ref().expect("power_mode_watts");
        assert_eq!(*watts.get("office").unwrap(), 25);
        assert_eq!(*watts.get("balance").unwrap(), 45);
        assert_eq!(*watts.get("turbo").unwrap(), 65);

        // Verify real tester 14X: Board=WUJIE14-GX4HRXL, Product=WUJIE14XA
        assert!(manifest.matches_dmi("WUJIE14-GX4HRXL", "WUJIE14XA"));

        // Verify real tester 15X: Board=WUJIE Series-X5SP4NAG, Product=WUJIE Series
        assert!(manifest.matches_dmi("WUJIE Series-X5SP4NAG", "WUJIE Series"));

        // Verify future 16X: Board=WUJIE Series-X6..., Product=WUJIE16X
        assert!(manifest.matches_dmi("WUJIE Series-X6SP4NAG", "WUJIE16XA"));
    }
}

