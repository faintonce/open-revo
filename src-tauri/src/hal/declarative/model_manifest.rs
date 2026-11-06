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
    /// Independent of discrete-GPU controls; opts into read-only iGPU telemetry.
    #[serde(default)]
    pub gpu_telemetry_backend: Option<String>,
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
    pub fan_software_smoothing: bool,
    #[serde(default)]
    pub fan_control_mode: Option<String>,
    #[serde(default)]
    pub fan_boost_mode: Option<String>,
    #[serde(default)]
    pub max_fan_rpm: Option<u32>,
    #[serde(default)]
    pub min_fan_rpm: Option<[u16; 2]>,
    #[serde(default)]
    pub cpu_vendor: Option<String>,
    #[serde(default)]
    pub cpu_tuning_type: Option<String>,
    #[serde(default)]
    pub cpu_power_backend: Option<String>,
    #[serde(default)]
    pub cpu_power_max_watts: Option<u16>,
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

impl ModelCapabilities {
    pub fn supports_fan_smoothing(&self) -> bool {
        self.fan_ramp_rate || self.fan_software_smoothing
    }

    pub fn uses_bld_fan_controller(&self) -> bool {
        self.fan_control_mode.as_deref() == Some("direct_rpm_clamp")
            || self.fan_boost_mode.as_deref() == Some("direct_rpm_clamp")
    }
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
    #[serde(rename = "pmc_ec")]
    PmcEc { protocol: String },
    #[serde(rename = "wmi_mailbox")]
    WmiMailbox {
        namespace: String,
        class_name: String,
        method_name: String,
        #[serde(default = "default_mailbox_sizes")]
        buffer_sizes: MailboxSizes,
        #[serde(default)]
        response_status: Option<ResponseStatus>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResponseStatus {
    pub offset: usize,
    pub success: u8,
}

impl ResponseStatus {
    pub fn validate(&self, response: &[u8]) -> Result<(), String> {
        match response.get(self.offset) {
            Some(value) if *value == self.success => Ok(()),
            Some(value) => Err(format!("Firmware rejected command (status {})", value)),
            None => Err("Firmware response is missing its status byte".into()),
        }
    }
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
    /// Explicit transport and per-mode acknowledgements (some firmware echoes the EC command).
    ModeInvoke {
        channel: String,
        modes: HashMap<String, Vec<serde_json::Value>>,
        #[serde(default)]
        response_status: HashMap<String, ResponseStatus>,
    },
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

pub struct ModeInvocation<'a> {
    pub input: &'a [serde_json::Value],
    pub channel: Option<&'a str>,
    pub response_status: Option<&'a ResponseStatus>,
}

impl ActionDef {
    pub fn for_mode(&self, mode: &str) -> Option<ModeInvocation<'_>> {
        let (input, channel, response_status) = match self {
            Self::ModeMap(modes) => (modes.get(mode)?.as_slice(), None, None),
            Self::ModeInvoke { channel, modes, response_status } =>
                (modes.get(mode)?.as_slice(), Some(channel.as_str()), response_status.get(mode)),
            Self::BufferInvoke { channel, input, .. } => (input.as_slice(), channel.as_deref(), None),
            Self::RawBytes(input) => (input.as_slice(), None, None),
        };
        Some(ModeInvocation { input, channel, response_status })
    }
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
    pub fn matches_hardware(&self, board: &str, product: &str, vendor: &str, bios: &str) -> bool {
        self.matches_identity(board, product, vendor)
            && self.identity.dmi_match.bios_versions.as_ref().map_or(true, |range| range.matches(bios))
    }

    pub fn matches_identity(&self, board: &str, product: &str, vendor: &str) -> bool {
        self.matches_dmi(board, product)
            && self.identity.dmi_match.vendor.as_ref().map_or(true, |expected| {
                expected.trim().eq_ignore_ascii_case(vendor.trim())
            })
    }

    /// Keep a known chassis out of unrelated driver fallbacks while its
    /// firmware is unvalidated. Physical controls are all disabled.
    pub fn read_only(mut self) -> Self {
        self.name.push_str(" (unvalidated BIOS; monitoring only)");
        self.actions.clear();
        self.channels.clear();
        let caps = &mut self.capabilities;
        caps.power_modes.clear();
        caps.fan_custom_curve = false;
        caps.fan_control_mode = Some("none".into());
        caps.fan_boost_mode = Some("none".into());
        caps.fan_ramp_rate = false;
        caps.fan_software_smoothing = false;
        caps.battery_charge_limit = false;
        caps.battery_limit_mode = Some("none".into());
        caps.fn_lock = false;
        caps.cpu_power_backend = None;
        caps.cpu_power_max_watts = None;
        caps.cpu_tuning_type = Some("none".into());
        caps.gpu_power_tuning_mode = Some("none".into());
        self
    }

    pub fn uses_ptl_pmc(&self) -> bool {
        self.identity.dmi_match.board_products == ["XINGYAO Series-P916F-PTL"]
            && self.channels.values().any(|channel| matches!(channel,
                ChannelDef::PmcEc { protocol } if protocol == "xingyao_ptl_v1"))
    }

    pub fn uses_mifs_mailbox(&self) -> bool {
        self.channels.values().any(|channel| matches!(channel,
            ChannelDef::WmiMailbox { class_name, method_name, .. }
                if class_name.eq_ignore_ascii_case("MICommonInterface") && method_name.eq_ignore_ascii_case("MiInterface")))
    }

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

impl BiosVersionRange {
    pub fn matches(&self, actual: &str) -> bool {
        fn compare(actual: &str, expected: &str) -> Option<std::cmp::Ordering> {
            if actual.trim().eq_ignore_ascii_case(expected.trim()) {
                return Some(std::cmp::Ordering::Equal);
            }
            let parse = |s: &str| s.trim().split('.').map(str::parse::<u64>)
                .collect::<Result<Vec<_>, _>>().ok();
            let mut actual = parse(actual)?;
            let mut expected = parse(expected)?;
            let length = actual.len().max(expected.len());
            actual.resize(length, 0);
            expected.resize(length, 0);
            Some(actual.cmp(&expected))
        }
        !actual.trim().is_empty()
            && self.min.as_ref().map_or(true, |min| compare(actual, min).is_some_and(|order| !order.is_lt()))
            && self.max.as_ref().map_or(true, |max| compare(actual, max).is_some_and(|order| !order.is_gt()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_identity_is_enforced_and_unknown_bios_has_no_write_actions() {
        let manifest: ModelManifest = serde_json::from_str(include_str!("../../../resources/drivers/xingyao_14_p916f_ptl.json")).unwrap();
        assert!(manifest.matches_hardware("XINGYAO Series-P916F-PTL", "XINGYAO Series", "MECHREVO", "1.00"));
        for (vendor, bios) in [("OTHER", "1.00"), ("MECHREVO", "1.01"), ("MECHREVO", "")] {
            assert!(!manifest.matches_hardware("XINGYAO Series-P916F-PTL", "XINGYAO Series", vendor, bios));
        }
        let readonly = manifest.read_only();
        assert!(readonly.channels.is_empty() && readonly.actions.is_empty());
        assert!(!readonly.uses_ptl_pmc());
        assert!(readonly.capabilities.power_modes.is_empty());
        assert!(readonly.capabilities.cpu_power_backend.is_none());
        assert!(!readonly.capabilities.fan_custom_curve && !readonly.capabilities.battery_charge_limit);
    }

    #[test]
    fn bios_ranges_compare_numeric_components_and_reject_unknown_ordering() {
        let range = BiosVersionRange { min: Some("1.2".into()), max: Some("1.10".into()) };
        assert!(range.matches("1.9") && range.matches("1.10.0"));
        assert!(!range.matches("1.11") && !range.matches("1.1") && !range.matches("unknown"));
        let exact = BiosVersionRange { min: Some("N.1.10".into()), max: Some("N.1.10".into()) };
        assert!(exact.matches("n.1.10"));
        assert!(!exact.matches("N.1.11"));
    }

    #[test]
    fn ptl_contract_is_exact_and_uses_its_own_pmc_controller() {
        let manifest: ModelManifest = serde_json::from_str(include_str!("../../../resources/drivers/xingyao_14_p916f_ptl.json")).unwrap();
        assert!(manifest.matches_dmi("XINGYAO Series-P916F-PTL", "XINGYAO Series"));
        assert!(!manifest.matches_dmi("XINGYAO Series-P916F-STX", "XINGYAO Series"));
        assert!(!manifest.matches_dmi("XINGYAO Series-P916F-LNL", "XINGYAO Series"));
        assert!(manifest.capabilities.battery_charge_limit);
        assert!(manifest.capabilities.fan_custom_curve);
        assert!(manifest.capabilities.fan_independent_channels);
        assert!(!manifest.capabilities.fan_ramp_rate);
        assert!(manifest.capabilities.fan_software_smoothing);
        assert_eq!(manifest.capabilities.cpu_power_backend.as_deref(), Some("intel_rapl"));
        assert_eq!(manifest.capabilities.cpu_power_max_watts, Some(50));
        assert_eq!(manifest.capabilities.gpu_telemetry_backend.as_deref(), Some("intel_igcl_pmt"));
        assert!(manifest.capabilities.power_modes.iter().any(|mode| mode == "custom"));
        // Custom is implemented by the CPU backend, not an invented fourth WMI command.
        assert!(manifest.actions["set_power_mode"].for_mode("custom").is_none());
        assert_eq!(manifest.capabilities.min_fan_rpm, Some([1200, 1000]));
        assert_eq!(manifest.capabilities.fan_control_mode.as_deref(), Some("pmc_rpm_curve"));
        assert!(manifest.uses_ptl_pmc());
        assert!(!manifest.capabilities.uses_bld_fan_controller());
        assert!(!manifest.uses_mifs_mailbox());
        assert_eq!(manifest.capabilities.fan_boost_mode.as_deref(), Some("pmc_rpm"));
        let bld: ModelManifest = serde_json::from_str(include_str!("../../../resources/drivers/jiaolong_16_bld.json")).unwrap();
        assert!(bld.capabilities.uses_bld_fan_controller());
        assert!(bld.uses_mifs_mailbox());
    }

    #[test]
    fn ptl_fans_use_distinct_queries_and_the_same_response_offset() {
        let manifest: ModelManifest = serde_json::from_str(include_str!("../../../resources/drivers/xingyao_14_p916f_ptl.json")).unwrap();
        for (action, selector) in [("diagnostic_cpu_fan_rpm", 0), ("diagnostic_gpu_fan_rpm", 1)] {
            let ActionDef::BufferInvoke { channel, input, parse, .. } = &manifest.actions[action] else { panic!("Expected a mailbox query") };
            assert_eq!(channel.as_deref(), Some("wmi_mailbox"));
            assert_eq!(*input, vec![serde_json::json!(2), serde_json::json!(1), serde_json::json!(selector)]);
            let rule = &parse.as_ref().unwrap()["rpm"];
            assert_eq!(rule.offset, 1);
            assert_eq!(rule.value_type, "u16_le");
        }
        let ActionDef::BufferInvoke { channel, .. } = &manifest.actions["read_fan_rpm"] else { panic!("Expected PM2 action") };
        assert_eq!(channel.as_deref(), Some("pm2"));
    }

    #[test]
    fn firmware_status_requires_a_present_success_byte() {
        let status = ResponseStatus { offset: 0, success: 0 };
        assert!(status.validate(&[0, 42]).is_ok());
        assert!(status.validate(&[1, 42]).is_err());
        assert!(status.validate(&[]).is_err());
    }

    #[test]
    fn ptl_mode_acknowledgements_are_scoped_to_each_command() {
        let manifest: ModelManifest = serde_json::from_str(include_str!("../../../resources/drivers/xingyao_14_p916f_ptl.json")).unwrap();
        let action = &manifest.actions["set_power_mode"];
        assert!(matches!(action, ActionDef::ModeInvoke { .. }));
        for (mode, selector, acknowledgement) in [("office",1,148), ("balance",2,149), ("turbo",3,150)] {
            let invocation = action.for_mode(mode).unwrap();
            assert_eq!(invocation.channel, Some("wmi_mailbox"));
            assert_eq!(invocation.input, &[serde_json::json!(3),serde_json::json!(10),serde_json::json!(selector)]);
            let status = invocation.response_status.unwrap();
            assert!(status.validate(&[acknowledgement]).is_ok());
            for bad in [0,1,2,255,if acknowledgement == 148 {149} else {148}] {
                assert!(status.validate(&[bad]).is_err());
            }
            assert!(status.validate(&[]).is_err());
        }
        assert!(action.for_mode("custom").is_none());
        let ChannelDef::WmiMailbox { response_status: Some(query_status), .. } = &manifest.channels["wmi_mailbox"] else { panic!("Expected query status") };
        assert!(query_status.validate(&[0]).is_ok());
        assert!(query_status.validate(&[148]).is_err());
    }

    #[test]
    fn legacy_mode_maps_keep_their_default_transport_status() {
        let action: ActionDef = serde_json::from_str(r#"{"office":[1,2,3]}"#).unwrap();
        let invocation = action.for_mode("office").unwrap();
        assert_eq!(invocation.input, &[serde_json::json!(1),serde_json::json!(2),serde_json::json!(3)]);
        assert!(invocation.channel.is_none());
        assert!(invocation.response_status.is_none());
    }

    #[test]
    fn test_wujie_14_ip3_deserialization() {
        let json = include_str!("../../../resources/drivers/wujie_14_ip3.json");
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
        let json = include_str!("../../../resources/drivers/xingyao_14_p916f.json");
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
        let json = include_str!("../../../resources/drivers/wujie_14_bld.json");
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
        let json = include_str!("../../../resources/drivers/jiaolong_16_bld.json");
        let manifest: ModelManifest = serde_json::from_str(json).expect("Failed to parse manifest");
        // Test real tester's DMI from report:
        // Board: MRID6-23, Product: Jiaolong Series MRID6
        assert!(manifest.matches_dmi("MRID6-23", "Jiaolong Series MRID6"));
        assert!(manifest.matches_dmi("MRID6", "JIAOLONG 16 PRO"));
    }

    #[test]
    fn test_wujie_x_series_deserialization_and_matching() {
        let json = include_str!("../../../resources/drivers/wujie_x_series.json");
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

