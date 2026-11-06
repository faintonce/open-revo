// OpenRevo Platform & Hardware Channel Probe
// Dynamically identifies ODM architecture family to determine the correct communication channel.

use std::sync::OnceLock;

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
                    crate::hal::declarative::model_manifest::ChannelDef::PmcEc { .. } => continue,
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
            use windows::core::{w, PCWSTR};
            use windows::Win32::System::Registry::{
                RegCloseKey, RegOpenKeyExW, HKEY_LOCAL_MACHINE, KEY_READ, HKEY,
            };

            // 1. Fast path: Check if standard Tongfang ACPIDriver is already active & responsive
            let acpi_driver_active = crate::driver::wmi_ec::is_acpi_driver_available();
            if acpi_driver_active {
                crate::log_info!("PLATFORM_PROBE", "Platform identified: Tongfang ACPI Kernel Driver (active device link)");
                return PlatformChannelKind::TongfangAcpiKernel;
            }

            // 2. Hardware Ground Truth: Physical UEFI NVRAM (UniWillVariable / OemMagicVariable)
            // Modern Tongfang platforms (Canglong, Yaoshi, Kuangshi, etc.) physically respond in SPI Flash
            let has_tongfang_nvram = crate::driver::uefi_nvram::read_active_nvram()
                .map(|nv| nv.bytes_read > 0)
                .unwrap_or(false);

            // 3. System Service Ground Truth: UWACPIDriver registered in system services
            let service_key: PCWSTR = w!(r"SYSTEM\CurrentControlSet\Services\UWACPIDriver");
            let mut h_svc = HKEY::default();
            let has_driver_service = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, service_key, 0, KEY_READ, &mut h_svc).is_ok() };
            if has_driver_service {
                unsafe { let _ = RegCloseKey(h_svc); }
            }

            // 4. Hardware Topology Ground Truth: Discrete NVIDIA GPU presence
            let has_nvidia_dgpu = crate::driver::gpu_bridge::is_nvidia_dgpu_available();

            // 5. WMI Mulong Check: Does the platform have AcpiTest_MULong AND does it actually communicate?
            let has_mulong_wmi = crate::driver::wmi_mulong::is_mulong_available();
            let is_mulong_functional = if has_mulong_wmi {
                crate::driver::wmi_mulong::verify_mulong_functional()
            } else {
                false
            };

            crate::log_info!(
                "PLATFORM_PROBE",
                "Probe synthesis: driver_active={}, nvram={}, service={}, dgpu={}, mulong_wmi={}, mulong_functional={}",
                acpi_driver_active,
                has_tongfang_nvram,
                has_driver_service,
                has_nvidia_dgpu,
                has_mulong_wmi,
                is_mulong_functional
            );

            // Comprehensive Multi-Protocol Arbitration:
            // Rule A: Modern Tongfang Gaming Laptop Indicators
            // If the machine has Tongfang UEFI NVRAM, or driver service registered, or discrete NVIDIA gaming GPU topology,
            // it is definitively a modern architecture laptop requiring the ACPI kernel driver.
            // Even if WMI AcpiTest_MULong is present in BIOS, it is merely legacy firmware residue (Ghost WMI).
            if has_tongfang_nvram || has_driver_service || has_nvidia_dgpu {
                crate::log_info!("PLATFORM_PROBE", "Platform identified: Modern Tongfang Gaming (TongfangAcpiKernel, self-healing ready)");
                return PlatformChannelKind::TongfangAcpiKernel;
            }

            // Rule B: Genuine Legacy Tongfang Gaming Laptop
            // Only if there are NO modern indicators AND Mulong successfully performs active hardware EC communication,
            // do we classify it as driverless Legacy MULong.
            if has_mulong_wmi && is_mulong_functional {
                crate::log_info!("PLATFORM_PROBE", "Platform identified: Genuine Legacy Tongfang/Hasee Gaming (Driverless AcpiTest_MULong)");
                return PlatformChannelKind::LegacyTongfangMulong;
            }

            // Rule C: Ghost MULong rejection
            if has_mulong_wmi && !is_mulong_functional {
                crate::log_warn!("PLATFORM_PROBE", "AcpiTest_MULong detected in WMI but failed active hardware handshake (ghost interface). Rejecting as legacy channel.");
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
/// - Has active UWACPIDriver (Tongfang ACPI Kernel interface)
/// - Not a thin & light ultrabook (has discrete gaming GPU)
/// - SPI Flash lacks modern UniWillVariable and OemMagicVariable NVRAM variables
pub fn is_legacy_pascal_turing_tongfang() -> bool {
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        if is_ultrabook() {
            return false;
        }
        if !is_tongfang_family() {
            return false;
        }
        // Check if modern Tongfang NVRAM variables exist
        let has_modern_nvram = crate::driver::uefi_nvram::read_active_nvram()
            .map(|nv| nv.bytes_read > 0)
            .unwrap_or(false);
        let is_legacy = !has_modern_nvram;
        if is_legacy {
            crate::log_info!(
                "PLATFORM_PROBE",
                "Chassis generation confirmed: Legacy Pascal/Turing Tongfang (UWACPIDriver active, no modern NVRAM)"
            );
        }
        is_legacy
    })
}

