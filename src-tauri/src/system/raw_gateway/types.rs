// OpenRevo - Raw EC & Hardware Driver Gateway Protocol Types
// Provides unified abstractions over Tongfang IOCTL, Baolongda Mailbox, Generic WMI, and ITE HID buses.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawEcProtocol {
    /// 1. Tongfang DirectEC IOCTL (High-speed Ring0 native transport)
    TongfangDirectEc,
    /// 2. Baolongda / JiaoLong WMI Mailbox Protocol (Multi-byte buffer handshake)
    BldWmiMailbox,
    /// 3. Generic ACPI WMI MULONG Transport
    GenericAcpiWmi,
    /// 4. ITE Dedicated RGB Lighting HID Direct Bus
    IteLightingBus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum RawEcProtocolKind {
    TongfangDirectEc,
    BldWmiMailbox,
    GenericAcpiWmi,
    IteLightingBus,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawEcProtocolHelper {
    Direct(RawEcProtocolKind),
    StringFallback(String),
    Tagged {
        #[serde(rename = "type")]
        protocol_type: Option<String>,
        protocol: Option<String>,
    },
}

impl Serialize for RawEcProtocol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        let name = match self {
            RawEcProtocol::TongfangDirectEc => "TongfangDirectEc",
            RawEcProtocol::BldWmiMailbox => "BldWmiMailbox",
            RawEcProtocol::GenericAcpiWmi => "GenericAcpiWmi",
            RawEcProtocol::IteLightingBus => "IteLightingBus",
        };
        map.serialize_entry("type", name)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for RawEcProtocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = RawEcProtocolHelper::deserialize(deserializer)?;
        let parse_str = |s: &str| -> Option<RawEcProtocol> {
            let lower = s.to_ascii_lowercase();
            if lower.contains("tongfang") || lower == "direct" || lower == "ioctl" {
                Some(RawEcProtocol::TongfangDirectEc)
            } else if lower.contains("bld") || lower.contains("baolongda") || lower.contains("jiaolong") {
                Some(RawEcProtocol::BldWmiMailbox)
            } else if lower.contains("wmi") || lower.contains("mulong") || lower.contains("generic") || lower.contains("ip3") {
                Some(RawEcProtocol::GenericAcpiWmi)
            } else if lower.contains("ite") || lower.contains("lighting") || lower.contains("hid") {
                Some(RawEcProtocol::IteLightingBus)
            } else {
                None
            }
        };

        match helper {
            RawEcProtocolHelper::Direct(k) => match k {
                RawEcProtocolKind::TongfangDirectEc => Ok(RawEcProtocol::TongfangDirectEc),
                RawEcProtocolKind::BldWmiMailbox => Ok(RawEcProtocol::BldWmiMailbox),
                RawEcProtocolKind::GenericAcpiWmi => Ok(RawEcProtocol::GenericAcpiWmi),
                RawEcProtocolKind::IteLightingBus => Ok(RawEcProtocol::IteLightingBus),
            },
            RawEcProtocolHelper::StringFallback(s) => {
                parse_str(&s).ok_or_else(|| serde::de::Error::custom(format!("Unknown RawEcProtocol string: '{}'", s)))
            }
            RawEcProtocolHelper::Tagged { protocol_type, protocol } => {
                let s = protocol_type.or(protocol).unwrap_or_default();
                parse_str(&s).ok_or_else(|| serde::de::Error::custom(format!("Unknown RawEcProtocol tagged type: '{}'", s)))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEcReadRequest {
    pub protocol: RawEcProtocol,
    pub address: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEcWriteRequest {
    pub protocol: RawEcProtocol,
    pub address: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMailboxRequest {
    pub in_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEcCapabilities {
    pub available_protocols: Vec<RawEcProtocol>,
    pub is_direct_ec_available: bool,
    pub is_bld_mailbox_available: bool,
    pub is_generic_wmi_available: bool,
    pub is_ite_hid_available: bool,
}
