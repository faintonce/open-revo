// OpenRevo Platform & Hardware Channel Probe
// Dynamically identifies ODM architecture family to determine the correct communication channel.

use std::sync::OnceLock;

/// Centrally signed Chassis Architecture Tier based on Hardware Truth
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisArchitectureTier {
    /// Declarative manifest or thin & light ultrabook (Wujie, Xingyao, Baolongda, IP3) -> WMI Mailbox / Ip3Wmi
    DeclarativeOrUltrabook,
    /// 10/16/20-Series Pascal / Turing Legacy Tongfang (GK5CN, GK5CP, Z2 Air, etc.)
    /// Pre-APVersion era (NVRAM <= 64 bytes or absent), driverless WMI AcpiTest_MULong, Optimus MSHybrid, no DB/cTGP
    LegacyPascalTuring,
    /// 30-Series Ampere Dual-Mode Tongfang (GM5T, GK5T, etc.)
    /// Modern NVRAM payload (>= 128 bytes) with APVersion < 23, \\.\ACPIDriver, Dual-Mode MUX (no iGPU-only), full-byte FanBoost clamp
    DualModeAmpere,
    /// 40-Series Ada Lovelace Modern Tongfang (Kuangshi 16, Jiaolong 16, etc.)
    /// Modern NVRAM payload (>= 128 bytes) with APVersion >= 23, \\.\ACPIDriver, Tri-Mode MUX, Bitmask FanBoost, full DB/cTGP
    ModernAda,
    /// 50-Series Blackwell Next-Gen Tongfang (Yilong, etc.)
    /// OemMagicVariable NVRAM (>= 128 bytes), \\.\ACPIDriver, OemMagicDoor, autonomous scheduling
    NextGenBlackwell,
    /// Unknown fallback platform
    Unknown,
}

static DETECTED_TIER: OnceLock<ChassisArchitectureTier> = OnceLock::new();

/// Centrally determines and signs the authoritative Chassis Architecture Tier based on SPI Flash NVRAM & Channel Truth.
pub fn get_chassis_architecture_tier() -> ChassisArchitectureTier {
    *DETECTED_TIER.get_or_init(|| {
        // Step 1: Declarative Manifest Contract Priority (Level 0 Architecture)
        if crate::hal::declarative::load_matched_manifest().is_some() {
            crate::log_info!("PLATFORM_PROBE", "Chassis Tier signed: Declarative Manifest Matched");
            return ChassisArchitectureTier::DeclarativeOrUltrabook;
        }

        #[cfg(windows)]
        {
            // Step 2: SPI Flash UEFI NVRAM & APVersion Ground Truth Arbitration
            if let Ok(nv) = crate::driver::uefi_nvram::read_active_nvram() {
                // Priority A: 50-Series (OemMagicVariable or ap_version >= 25)
                if nv.var_name.starts_with("OemMagicVariable") || nv.is_50_series || nv.ap_version >= 25 {
                    crate::log_info!("PLATFORM_PROBE", "Chassis Tier signed: Next-Gen Blackwell (50-Series, ap_version={})", nv.ap_version);
                    return ChassisArchitectureTier::NextGenBlackwell;
                }

                // Priority B: 40-Series (Modern payload >= 128 bytes and ap_version >= 23)
                if nv.bytes_read >= 128 && nv.ap_version >= 23 {
                    crate::log_info!("PLATFORM_PROBE", "Chassis Tier signed: Modern Ada Lovelace (40-Series, ap_version={})", nv.ap_version);
                    return ChassisArchitectureTier::ModernAda;
                }

                // Priority C: 30-Series (Modern payload >= 128 bytes and ap_version < 23)
                if nv.bytes_read >= 128 && nv.ap_version < 23 {
                    crate::log_info!("PLATFORM_PROBE", "Chassis Tier signed: Dual-Mode Ampere (30-Series, payload={}B, ap_version={})", nv.bytes_read, nv.ap_version);
                    return ChassisArchitectureTier::DualModeAmpere;
                }

                // Priority D: Pre-APVersion 10/16/20-Series (Legacy payload <= 64 bytes)
                if nv.bytes_read <= 64 {
                    crate::log_info!("PLATFORM_PROBE", "Chassis Tier signed: Legacy Pascal/Turing (10/16/20-Series, payload={}B <= 64B)", nv.bytes_read);
                    return ChassisArchitectureTier::LegacyPascalTuring;
                }
            }

            // Priority E: Completely missing NVRAM on discrete GPU platform
            if crate::driver::wmi_mulong::is_mulong_available() {
                crate::log_info!("PLATFORM_PROBE", "Chassis Tier signed: Legacy Pascal/Turing via MULong (No NVRAM)");
                return ChassisArchitectureTier::LegacyPascalTuring;
            }
        }

        ChassisArchitectureTier::Unknown
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformChannelKind {
    /// Traditional Tongfang Gaming Laptops requiring UWACPIDriver.sys (\\.\ACPIDriver)
    TongfangAcpiKernel,
    /// Legacy Tongfang / Hasee Gaming Laptops using Driverless WMI (AcpiTest_MULong)
    LegacyTongfangMulong,
    /// Modern Thin & Light Laptops using WMI Mailbox (root\wmi: AcpiTest_WMI.OemWMIfun)
    WmiMailbox,
    /// Shenzhen IP3 Thin & Light Laptops (Wujie 14 IP3 xN39, PowerSwitchInterface & SuRwECRegInterface)
    Ip3Wmi,
    /// Driverless WMI Laptops (Bitland/Baolongda MRID6, etc.)
    DriverlessWmi,
    /// Fallback / Virtual / Mock Environment
    Unknown,
}

static DETECTED_CHANNEL: OnceLock<PlatformChannelKind> = OnceLock::new();

/// Detects the active hardware communication channel required for the current machine.
pub fn get_platform_channel() -> PlatformChannelKind {
    *DETECTED_CHANNEL.get_or_init(|| {
        // 0. Declarative Driver Contract First (Level 0 Architecture)
        // If an explicit manifest matches DMI, the manifest declares the physical channel!
        if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
            for channel in manifest.channels.values() {
                match channel {
                    crate::hal::declarative::model_manifest::ChannelDef::WmiIp3 => {
                        crate::log_info!("PLATFORM_PROBE", "Platform identified via Manifest: Shenzhen IP3 WMI ({})", manifest.id);
                        return PlatformChannelKind::Ip3Wmi;
                    }
                    crate::hal::declarative::model_manifest::ChannelDef::WmiMailbox { .. } => {
                        crate::log_info!("PLATFORM_PROBE", "Platform identified via Manifest: WMI Mailbox ({})", manifest.id);
                        return PlatformChannelKind::WmiMailbox;
                    }
                    crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. } => {
                        crate::log_info!("PLATFORM_PROBE", "Platform identified via Manifest: ACPI IOCTL ({})", manifest.id);
                        return PlatformChannelKind::TongfangAcpiKernel;
                    }
                }
            }
        }

        #[cfg(windows)]
        {
            // Route via Authoritative Signed Chassis Tier (No small-condition guessing)
            match get_chassis_architecture_tier() {
                ChassisArchitectureTier::LegacyPascalTuring => {
                    crate::log_info!("PLATFORM_PROBE", "Platform identified: Legacy Tongfang Gaming (LegacyTongfangMulong driverless channel)");
                    return PlatformChannelKind::LegacyTongfangMulong;
                }
                ChassisArchitectureTier::DualModeAmpere
                | ChassisArchitectureTier::ModernAda
                | ChassisArchitectureTier::NextGenBlackwell => {
                    crate::log_info!("PLATFORM_PROBE", "Platform identified: Modern Tongfang Gaming (TongfangAcpiKernel)");
                    return PlatformChannelKind::TongfangAcpiKernel;
                }
                ChassisArchitectureTier::DeclarativeOrUltrabook => {
                    return PlatformChannelKind::WmiMailbox;
                }
                ChassisArchitectureTier::Unknown => {}
            }

            // Fallback: Check if standard Tongfang ACPIDriver is already active & responsive
            if crate::driver::wmi_ec::is_acpi_driver_available() {
                crate::log_info!("PLATFORM_PROBE", "Platform identified via active ACPIDriver link");
                return PlatformChannelKind::TongfangAcpiKernel;
            }

            // Fallback: WMI Mulong Check
            if crate::driver::wmi_mulong::is_mulong_available() && crate::driver::wmi_mulong::verify_mulong_functional() {
                crate::log_info!("PLATFORM_PROBE", "Platform identified: Genuine Legacy Tongfang/Hasee Gaming (Driverless AcpiTest_MULong fallback)");
                return PlatformChannelKind::LegacyTongfangMulong;
            }
        }

        PlatformChannelKind::Unknown
    })
}

/// Returns true if this platform is a Tongfang architecture (either kernel driver or driverless WMI)
pub fn is_tongfang_family() -> bool {
    matches!(
        get_platform_channel(),
        PlatformChannelKind::TongfangAcpiKernel | PlatformChannelKind::LegacyTongfangMulong
    )
}

/// Returns true if this platform strictly requires the kernel driver \\.\ACPIDriver (UWACPIDriver.sys)
pub fn needs_acpi_kernel_driver() -> bool {
    matches!(get_platform_channel(), PlatformChannelKind::TongfangAcpiKernel)
}

/// Returns true if this platform supports physical chassis key event broadcasts (AcpiTest_EventULong)
pub fn has_acpi_physical_key_events() -> bool {
    is_tongfang_family()
}

/// Returns true if this platform is a modern thin & light / ultrabook (e.g. Wujie, Xingyao series)
pub fn is_ultrabook() -> bool {
    if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        if manifest.identity.form_factor == "thin_light" || !manifest.capabilities.discrete_gpu {
            return true;
        } else {
            return false;
        }
    }
    matches!(
        get_platform_channel(),
        PlatformChannelKind::WmiMailbox | PlatformChannelKind::Ip3Wmi | PlatformChannelKind::DriverlessWmi
    )
}

/// Returns true if this platform is a Legacy Tongfang Gaming Laptop (GTX 10-series / RTX 20-series):
/// Centrally signed by the Chassis Architecture Tier.
pub fn is_legacy_pascal_turing_tongfang() -> bool {
    matches!(get_chassis_architecture_tier(), ChassisArchitectureTier::LegacyPascalTuring)
}

