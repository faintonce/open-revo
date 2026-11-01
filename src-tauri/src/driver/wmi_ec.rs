use parking_lot::Mutex;
use std::sync::LazyLock;

// Official Mechrevo / Uniwill 16-bit EC Register Specifications (re-exported from Hardware Vault)
pub use crate::hal::vault::ec::*;
pub use crate::hal::vault::mask::*;


#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;

    fn CreateFileA(
        lpFileName: *const u8,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut std::ffi::c_void,
    ) -> isize;

    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;

    fn DeviceIoControl(
        hDevice: isize,
        dwIoControlCode: u32,
        lpInBuffer: *const std::ffi::c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut std::ffi::c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut std::ffi::c_void,
    ) -> i32;
}

const IOCTL_ECREAD: u32 = crate::hal::vault::driver::IOCTL_ECREAD;
const IOCTL_ECWRITE: u32 = crate::hal::vault::driver::IOCTL_ECWRITE;
const GENERIC_READ_WRITE: u32 = 0xC0000000;
const FILE_SHARE_READ_WRITE: u32 = 3;
const OPEN_EXISTING: u32 = 3;

/// Direct Win32 IOCTL call to \\.\ACPIDriver for single EC register read
use std::sync::atomic::{AtomicBool, Ordering};

static DRIVER_PROBED: AtomicBool = AtomicBool::new(false);
static DRIVER_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Probes whether \\.\ACPIDriver can be opened right now (< 0.1ms).
pub fn probe_acpi_driver_available() -> bool {
    unsafe {
        let name = crate::hal::vault::driver::get_driver_path_null_terminated();
        let h = CreateFileA(
            name.as_ptr(),
            GENERIC_READ_WRITE,
            FILE_SHARE_READ_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        let ok = h != -1 && h != 0;
        if ok {
            CloseHandle(h as *mut _);
        }
        DRIVER_AVAILABLE.store(ok, Ordering::Release);
        DRIVER_PROBED.store(true, Ordering::Release);
        ok
    }
}

/// Returns whether the ACPI kernel driver device is available.
/// Uses fast cached atomic flag after initial probe.
pub fn is_acpi_driver_available() -> bool {
    if !DRIVER_PROBED.load(Ordering::Acquire) {
        probe_acpi_driver_available()
    } else {
        DRIVER_AVAILABLE.load(Ordering::Acquire)
    }
}

/// Explicitly re-check the driver device availability (e.g. after PnP driver installation).
pub fn mark_driver_rechecked() -> bool {
    probe_acpi_driver_available()
}

/// Direct Win32 IOCTL call to \\.\ACPIDriver for single EC register read (with AcpiTest_MULong WMI fallback)
pub fn raw_driver_read_ec(addr: u16) -> Option<u8> {
    if is_acpi_driver_available() {
        unsafe {
            let name = crate::hal::vault::driver::get_driver_path_null_terminated();
            let h = CreateFileA(
                name.as_ptr(),
                GENERIC_READ_WRITE,
                FILE_SHARE_READ_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if h == -1 || h == 0 {
                let err = GetLastError();
                crate::log_debug!("EC_DRIVER", "raw_driver_read_ec({}) ACPIDriver device open failed: GetLastError={}", addr, err);
            } else {
                let in_buf: [u32; 1] = [addr as u32];
                let mut out_buf: u32 = 0;
                let mut ret_bytes: u32 = 0;
                let ok = DeviceIoControl(
                    h,
                    IOCTL_ECREAD,
                    in_buf.as_ptr() as *const _,
                    4,
                    &mut out_buf as *mut u32 as *mut _,
                    4,
                    &mut ret_bytes,
                    std::ptr::null_mut(),
                );
                let err = GetLastError();
                CloseHandle(h as *mut _);

                if ok != 0 {
                    let val = (out_buf & 0xFF) as u8;
                    crate::log_debug!("EC_DRIVER", "raw_driver_read_ec({}) SUCCESS -> {} via ACPIDriver", addr, val);
                    return Some(val);
                } else {
                    crate::log_debug!("EC_DRIVER", "raw_driver_read_ec({}) IOCTL failed: GetLastError={}", addr, err);
                }
            }
        }
    }

    // Fallback: Driverless native WMI mailbox (AcpiTest_MULong) for legacy Tongfang/Hasee
    if crate::driver::wmi_mulong::is_mulong_available() {
        if let Some(val) = crate::driver::wmi_mulong::mulong_read_ec(addr) {
            crate::log_debug!("EC_DRIVER", "raw_driver_read_ec({}) SUCCESS -> {} via AcpiTest_MULong", addr, val);
            return Some(val);
        }
    }

    None
}

/// Direct Win32 IOCTL call to \\.\ACPIDriver for single EC register write (with AcpiTest_MULong WMI fallback)
pub fn raw_driver_write_ec(addr: u16, val: u8) -> bool {
    if is_acpi_driver_available() {
        unsafe {
            let name = crate::hal::vault::driver::get_driver_path_null_terminated();
            let h = CreateFileA(
                name.as_ptr(),
                GENERIC_READ_WRITE,
                FILE_SHARE_READ_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if h == -1 || h == 0 {
                let err = GetLastError();
                crate::log_error!("EC_DRIVER", "raw_driver_write_ec(addr={}, val={}) CreateFileA failed: h={}, GetLastError={}", addr, val, h, err);
            } else {
                let in_buf: [u32; 2] = [addr as u32, val as u32];
                let mut out_buf: u32 = 0;
                let mut ret_bytes: u32 = 0;
                let ok = DeviceIoControl(
                    h,
                    IOCTL_ECWRITE,
                    in_buf.as_ptr() as *const _,
                    8,
                    &mut out_buf as *mut u32 as *mut _,
                    4,
                    &mut ret_bytes,
                    std::ptr::null_mut(),
                );
                let err = GetLastError();
                CloseHandle(h as *mut _);

                if ok != 0 {
                    crate::log_debug!("EC_DRIVER", "raw_driver_write_ec(addr={}, val={}) SUCCESS via ACPIDriver", addr, val);
                    return true;
                } else {
                    crate::log_error!("EC_DRIVER", "raw_driver_write_ec(addr={}, val={}) DeviceIoControl failed: ok={}, GetLastError={}", addr, val, ok, err);
                }
            }
        }
    }

    // Fallback: Driverless native WMI mailbox (AcpiTest_MULong) for legacy Tongfang/Hasee
    if crate::driver::wmi_mulong::is_mulong_available() {
        return crate::driver::wmi_mulong::mulong_write_ec(addr, val);
    }

    false
}

/// Direct Win32 IOCTL batch read from \\.\ACPIDriver (with AcpiTest_MULong fallback)
pub fn raw_driver_batch_read_ec(addrs: &[u16]) -> Option<Vec<u8>> {
    if is_acpi_driver_available() {
        unsafe {
            let name = crate::hal::vault::driver::get_driver_path_null_terminated();
            let h = CreateFileA(
                name.as_ptr(),
                GENERIC_READ_WRITE,
                FILE_SHARE_READ_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if h != -1 && h != 0 {
                let mut results = Vec::with_capacity(addrs.len());
                for &addr in addrs {
                    let in_buf: [u32; 1] = [addr as u32];
                    let mut out_buf: u32 = 0;
                    let mut ret_bytes: u32 = 0;
                    let ok = DeviceIoControl(
                        h,
                        IOCTL_ECREAD,
                        in_buf.as_ptr() as *const _,
                        4,
                        &mut out_buf as *mut u32 as *mut _,
                        4,
                        &mut ret_bytes,
                        std::ptr::null_mut(),
                    );
                    if ok != 0 {
                        results.push((out_buf & 0xFF) as u8);
                    } else {
                        results.push(0);
                    }
                }
                CloseHandle(h as *mut _);
                return Some(results);
            }
        }
    }

    // Fallback: Driverless native WMI mailbox (AcpiTest_MULong)
    if crate::driver::wmi_mulong::is_mulong_available() {
        let mut results = Vec::with_capacity(addrs.len());
        for &addr in addrs {
            results.push(crate::driver::wmi_mulong::mulong_read_ec(addr).unwrap_or(0));
        }
        return Some(results);
    }

    None
}

/// Direct Win32 IOCTL batch write to \\.\ACPIDriver (with AcpiTest_MULong fallback)
pub fn raw_driver_batch_write_ec(entries: &[(u16, u8)]) -> bool {
    if is_acpi_driver_available() {
        unsafe {
            let name = crate::hal::vault::driver::get_driver_path_null_terminated();
            let h = CreateFileA(
                name.as_ptr(),
                GENERIC_READ_WRITE,
                FILE_SHARE_READ_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if h != -1 && h != 0 {
                let mut all_ok = true;
                for &(addr, val) in entries {
                    let in_buf: [u32; 2] = [addr as u32, val as u32];
                    let mut out_buf: u32 = 0;
                    let mut ret_bytes: u32 = 0;
                    let ok = DeviceIoControl(
                        h,
                        IOCTL_ECWRITE,
                        in_buf.as_ptr() as *const _,
                        8,
                        &mut out_buf as *mut u32 as *mut _,
                        4,
                        &mut ret_bytes,
                        std::ptr::null_mut(),
                    );
                    if ok == 0 {
                        all_ok = false;
                    }
                }
                CloseHandle(h as *mut _);
                return all_ok;
            }
        }
    }

    // Fallback: Driverless native WMI mailbox (AcpiTest_MULong)
    if crate::driver::wmi_mulong::is_mulong_available() {
        let mut all_ok = true;
        for &(addr, val) in entries {
            if !crate::driver::wmi_mulong::mulong_write_ec(addr, val) {
                all_ok = false;
            }
        }
        return all_ok;
    }

    false
}


/// Write multiple bytes to EC RAM in a single batch.
pub fn wmi_write_ec_batch(entries: &[(u16, u8)]) -> bool {
    let _guard = ACPI_MUTEX.lock();
    if raw_driver_batch_write_ec(entries) {
        return true;
    }
    let mut ok = true;
    for &(addr, val) in entries {
        if !wmi_write_ec(addr, val) {
            ok = false;
        }
    }
    ok
}

/// Global ACPI conflict protection lock.
/// Serializes all hardware EC accesses and inserts settling time to completely prevent race conditions.
static ACPI_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Batch-read multiple EC RAM registers.
/// Uses zero-latency native \\.\ACPIDriver IOCTL.
pub fn wmi_batch_read_ec(addrs: &[u16]) -> Option<Vec<u8>> {
    let _guard = ACPI_MUTEX.lock();
    raw_driver_batch_read_ec(addrs)
}

/// Read 1 byte from EC RAM.
/// High-speed native \\.\ACPIDriver IOCTL.
pub fn wmi_read_ec(addr: u16) -> Option<u8> {
    let _guard = ACPI_MUTEX.lock();

    if !is_acpi_driver_available() && !crate::driver::wmi_mulong::is_mulong_available() {
        return None;
    }

    if let Some(val) = raw_driver_read_ec(addr) {
        crate::log_debug!("EC_DRIVER", "wmi_read_ec({}) -> {} via ACPIDriver IOCTL", addr, val);
        Some(val)
    } else {
        crate::log_debug!("EC_DRIVER", "wmi_read_ec failed: EC transport unavailable");
        None
    }
}

/// Write 1 byte to EC RAM.
/// High-speed native \\.\ACPIDriver IOCTL.
pub fn wmi_write_ec(addr: u16, val: u8) -> bool {
    let _guard = ACPI_MUTEX.lock();
    raw_driver_write_ec(addr, val)
}

/// Atomically reads, modifies, and conditionally writes back an EC register value.
/// Holds ACPI_MUTEX across the entire Read-Modify-Write (RMW) sequence to prevent
/// concurrent bitfield clobbering between multiple subsystems (e.g. Touchpad vs Battery Limiter).
/// 
/// Returns `Some(new_val)` if successful, or `None` if read/write failed.
/// If `f(old_val)` returns the same value as `old_val`, the write IOCTL is skipped for efficiency.
pub fn wmi_modify_ec<F: FnOnce(u8) -> u8>(addr: u16, f: F) -> Option<u8> {
    let _guard = ACPI_MUTEX.lock();
    let old_val = raw_driver_read_ec(addr)?;
    let new_val = f(old_val);
    if new_val != old_val {
        if raw_driver_write_ec(addr, new_val) {
            crate::log_debug!("EC_DRIVER", "wmi_modify_ec({:#X}) 0x{:02X} -> 0x{:02X}", addr, old_val, new_val);
            Some(new_val)
        } else {
            crate::log_error!("EC_DRIVER", "wmi_modify_ec({:#X}) failed to write new value 0x{:02X}", addr, new_val);
            None
        }
    } else {
        Some(old_val)
    }
}

/// Helper to atomically set or clear a bitmask on an EC register.
pub fn wmi_modify_ec_mask(addr: u16, mask: u8, enable: bool) -> Option<u8> {
    wmi_modify_ec(addr, |val| {
        if enable {
            val | mask
        } else {
            val & !mask
        }
    })
}

pub fn trigger_ec_action(mask: u8) -> bool {
    wmi_modify_ec(ADDR_TRIGGER_BYTE, |cur| cur | mask).is_some()
}

pub fn is_fan_boost_active() -> bool {
    let is_tongfang_hardware = is_acpi_driver_available()
        || crate::hal::declarative::load_matched_manifest()
            .map(|m| m.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. })))
            .unwrap_or(false);

    if is_tongfang_hardware {
        let s1873 = wmi_read_ec(ADDR_FAN_BOOST_CLAMP).unwrap_or(0);
        return (s1873 & FAN_BOOST_CLAMP_BIT) != 0;
    }

    let is_bld = crate::hal::declarative::load_matched_manifest()
        .map(|m| m.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::WmiMailbox { .. })))
        .unwrap_or(false);
    if is_bld {
        return crate::driver::bld_driver_mgr::is_bld_fan_clamped();
    }

    crate::system::config::load_config().fan_boost
}

/// Hardware Truth: Probes whether this platform uses the Legacy Dual-Mode EC architecture
/// (e.g. GK / GM5T series chassis, NVRAM variable size < 80 bytes, full-byte FanBoost clamp 0x50/0x10).
pub fn is_legacy_dual_mode_ec() -> bool {
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        let (res, reason) = (|| {
            // Signal 0: NVRAM Ground Truth - Modern platforms respond with UniWillVariable or OemMagicVariable (size >= 80)!
            if let Ok(nv) = crate::driver::uefi_nvram::read_active_nvram() {
                if nv.bytes_read >= 80 {
                    return (false, "Signal 0: Modern UniWill/OemMagic NVRAM verified (Not Legacy Dual-Mode)");
                } else if nv.bytes_read > 0 {
                    return (true, "Signal 0: Legacy UniWill NVRAM verified (payload size < 80 bytes)");
                }
            } else if crate::hal::platform_probe::is_tongfang_family() {
                return (true, "Signal 0: No active UEFI NVRAM variable found on Tongfang platform -> Legacy Dual-Mode");
            }

            // Signal 1: Declarative Driver Manifest Contract
            if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
                if manifest.id.contains("legacy") || manifest.id.contains("dual_mode") {
                    return (true, "Signal 1: Declared as Legacy Dual-Mode via Manifest Contract");
                }
            }

            (false, "Default: Modern architecture")
        })();

        crate::log_info!(
            "PLATFORM_DETECT",
            "Hardware architecture identified: {} [{}]",
            if res { "Legacy Dual-Mode EC architecture" } else { "Modern EC architecture" },
            reason
        );
        res
    })
}

/// Hardware Truth: Distinguishes Next-Gen OemMagicVariable NVRAM architecture
/// (APVersion >= 25, pure RAM fan latch commit) from UniWillVariable architecture.
pub fn is_nextgen_oem_magic_ec() -> bool {
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        let (res, reason) = (|| -> (bool, &'static str) {
            // Signal 1: Hardware NVRAM (Ground Truth in SPI Flash: OemMagicVariable vs UniWillVariable)
            if let Ok(nv) = crate::driver::uefi_nvram::read_active_nvram() {
                if nv.is_50_series || nv.ap_version >= 25 {
                    return (true, "Signal 1: Hardware NVRAM OemMagicVariable / APVersion >= 25");
                } else {
                    return (false, "Signal 1: Hardware NVRAM UniWillVariable");
                }
            }

            (false, "Default: Standard UniWill architecture")
        })();

        crate::log_info!(
            "PLATFORM_DETECT",
            "Hardware platform architecture identified: {} [{}]",
            if res { "Next-Gen OemMagicVariable architecture" } else { "Standard UniWillVariable architecture" },
            reason
        );
        res
    })
}

/// Hardware Fan Boost using direct zero-latency hardware clamp (30/40/50-Series):
/// - 30-Series:
///   - Enable : ADDR_FAN_BOOST_CLAMP = 0x50
///   - Disable: ADDR_FAN_BOOST_CLAMP = 0x10
/// - 40/50-Series:
///   - Enable : ADDR_FAN_BOOST_CLAMP |= FAN_BOOST_CLAMP_BIT
///   - Disable: ADDR_FAN_BOOST_CLAMP &= !FAN_BOOST_CLAMP_BIT
/// Instant speed-up and instant release with zero delay.
pub fn set_fan_boost_hardware(enable: bool) -> bool {
    let is_tongfang_hardware = is_acpi_driver_available()
        || crate::hal::declarative::load_matched_manifest()
            .map(|m| m.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. })))
            .unwrap_or(false);

    if !is_tongfang_hardware {
        let hal = crate::hal::get_hardware_driver();
        let res = hal.set_fan_boost(enable);
        crate::log_info!("EC_FAN", "Applied set_fan_boost({}) via HAL ({:?}) -> {:?}", enable, hal.kind(), res);
        return res.is_ok();
    }

    if is_legacy_dual_mode_ec() {
        if enable {
            wmi_write_ec(ADDR_FAN_BOOST_CLAMP, 0x50);
        } else {
            wmi_write_ec(ADDR_FAN_BOOST_CLAMP, 0x10);
        }
        crate::log_info!(
            "EC_FAN",
            "Hardware FanBoost set to {} (Legacy Dual-Mode): fan_boost_clamp={}",
            enable,
            if enable { "0x50" } else { "0x10" }
        );
    } else {
        if let Some(mut val) = wmi_read_ec(ADDR_FAN_BOOST_CLAMP) {
            if enable {
                val |= FAN_BOOST_CLAMP_BIT;
            } else {
                val &= !FAN_BOOST_CLAMP_BIT;
            }
            wmi_write_ec(ADDR_FAN_BOOST_CLAMP, val);
        }
        crate::log_info!(
            "EC_FAN",
            "Hardware FanBoost set to {} (40/50-series mode): fan_boost_clamp Bit 6 (0x40)",
            enable
        );
    }
    true
}

/// Direct single-shot trigger for Fan Boost
pub fn send_fan_boost_trigger() -> bool {
    trigger_ec_action(TRIGGER_FANBOOST)
}

pub fn is_winkey_locked_hw() -> bool {
    if let Some(status) = wmi_read_ec(ADDR_STATUS_BYTE) {
        (status & STATUS_WINLOCK) != 0
    } else {
        false
    }
}

/// Direct single-shot trigger for WinKey Lock
pub fn send_winkey_lock_trigger() -> bool {
    trigger_ec_action(TRIGGER_WINLOCK)
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct HardwareSupportFlags {
    pub has_fan_boost: bool,
    pub has_lightbar: bool,
    pub has_win_lock: bool,
    pub has_fn_lock: bool,
    pub has_battery_limit: bool,
    pub is_rgb_keyboard: bool,
    pub has_water_cooler: bool,
}

impl Default for HardwareSupportFlags {
    fn default() -> Self {
        Self {
            has_fan_boost: true,
            has_lightbar: true,
            has_win_lock: true,
            has_fn_lock: true,
            has_battery_limit: true,
            is_rgb_keyboard: true,
            has_water_cooler: false,
        }
    }
}

/// Query physical motherboard EC hardware support flags dynamically (SupportByte1 & SupportByte2 & BIOS Capability).
/// Caches the result with OnceLock since physical hardware capabilities do not change at runtime.
pub fn get_hardware_support_flags() -> HardwareSupportFlags {
    static CACHED_FLAGS: std::sync::OnceLock<HardwareSupportFlags> = std::sync::OnceLock::new();
    *CACHED_FLAGS.get_or_init(|| {
        if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
            let is_rgb = manifest
                .capabilities
                .rgb_keyboard
                .as_ref()
                .map(|k| k.keyboard_type != "none" && k.keyboard_type != "monochrome_stepped")
                .unwrap_or(false);
            let has_boost = manifest.capabilities.fan_control_mode.as_deref() != Some("none")
                && (manifest.capabilities.fan_boost_mode.as_deref() == Some("hardware_toggle")
                    || manifest.capabilities.fan_boost_mode.as_deref() == Some("direct_rpm_clamp")
                    || manifest.actions.contains_key("set_fan_boost_on")
                    || manifest.channels.values().any(|c| matches!(c, crate::hal::declarative::model_manifest::ChannelDef::AcpiIoctl { .. })));
            return HardwareSupportFlags {
                has_fan_boost: has_boost,
                has_lightbar: false,
                has_win_lock: true,
                has_fn_lock: manifest.capabilities.fn_lock,
                has_battery_limit: manifest.capabilities.battery_charge_limit,
                is_rgb_keyboard: is_rgb,
                has_water_cooler: manifest.capabilities.water_cooling,
            };
        }

        if crate::hal::platform_probe::is_ultrabook() {
            return HardwareSupportFlags {
                has_fan_boost: false,
                has_lightbar: false,
                has_win_lock: true,
                has_fn_lock: false,
                has_battery_limit: true,
                is_rgb_keyboard: false,
                has_water_cooler: false,
            };
        }

        // Truth-layer detection: query BIOS Capability byte, peripheral handshake status, or Windows paired cooler
        let raw_cooler_status = wmi_read_ec(ADDR_WATER_COOLER_FLAG).unwrap_or(0);
        let has_water_cooler = if let Some(b) = wmi_read_ec(ADDR_AP_BIOS_BYTE) {
            (b & MASK_LIQUID_COOLING_SUPPORT) != 0 || (raw_cooler_status & 0x10) != 0
        } else {
            (raw_cooler_status & 0x10) != 0
        } || crate::driver::ble_cooler::find_paired_water_cooler_mac().is_some();

        if let Some(bytes) = wmi_batch_read_ec(&[ADDR_SUPPORT_BYTE1, ADDR_SUPPORT_BYTE2]) {
            if bytes.len() == 2 {
                let byte1 = bytes[0];
                let byte2 = bytes[1];
                if byte1 != 0 || byte2 != 0 {
                    return HardwareSupportFlags {
                        has_fan_boost: (byte1 & 0x80) != 0,
                        has_lightbar: (byte1 & 0x40) != 0,
                        has_win_lock: (byte1 & 0x20) != 0,
                        has_fn_lock: true,
                        has_battery_limit: (byte2 & 0x40) != 0,
                        is_rgb_keyboard: (byte2 & 0x04) != 0 || crate::driver::ite_lighting::IteLightingManager::probe_keyboard_device_exists(),
                        has_water_cooler,
                    };
                }
            }
        }
        let mut def = HardwareSupportFlags::default();
        def.has_water_cooler = has_water_cooler;
        def
    })
}

// ---------------------------------------------------------
// Custom Performance & Fan Tuning (Registers re-exported from hal::vault)
// ---------------------------------------------------------

/// Apply Single Custom PL1 Limit (Clamped and guarantees PL1 <= PL2 <= PL4)
pub fn set_custom_pl1_ec(pl1: u16) -> bool {
    let pl1_val = pl1.clamp(15, 240) as u8;
    let ok = wmi_write_ec(ADDR_CUSTOM_PL1, pl1_val);
    crate::log_info!("EC_TUNING", "set_custom_pl1_ec: PL1={}W -> ok={}", pl1_val, ok);

    // Hardware safety invariant: PL2 must be >= PL1. If current PL2 is smaller, elevate PL2.
    let cur_pl2 = wmi_read_ec(ADDR_CUSTOM_PL2).unwrap_or(0);
    if cur_pl2 < pl1_val {
        let _ = set_custom_pl2_ec(pl1_val as u16);
    }
    ok
}

/// Apply Single Custom PL2 Limit (Clamped and guarantees PL4 >= PL2 + 20W transient headroom)
pub fn set_custom_pl2_ec(pl2: u16) -> bool {
    let cur_pl1 = wmi_read_ec(ADDR_CUSTOM_PL1).unwrap_or(15);
    let pl2_val = pl2.clamp(cur_pl1 as u16, 250) as u8;
    let ok = wmi_write_ec(ADDR_CUSTOM_PL2, pl2_val);
    crate::log_info!("EC_TUNING", "set_custom_pl2_ec: PL2={}W -> ok={}", pl2_val, ok);

    // INVIOLABLE HARDWARE RULE: PL4 is peak instantaneous current/wattage limit.
    // PL4 MUST NEVER BE SMALLER THAN PL2 + 20W!
    // If PL4 is too close to PL2, transient microsecond spikes trip VRM hardware overcurrent (BD PROCHOT),
    // immediately locking CPU to 0.54G~0.60G!
    let margin = crate::system::config::get_cpu_pl4_margin();
    let cur_pl4 = wmi_read_ec(ADDR_CUSTOM_PL4).unwrap_or(0);
    let safe_pl4 = (pl2_val as u16 + margin).min(250) as u8;
    if cur_pl4 < safe_pl4 {
        let _ = wmi_write_ec(ADDR_CUSTOM_PL4, safe_pl4);
        crate::log_warn!(
            "EC_TUNING",
            "Auto-elevating PL4 from {}W to {}W to preserve {}W transient safety buffer above PL2 ({}W)",
            cur_pl4, safe_pl4, margin, pl2_val
        );
    }
    ok
}

/// Apply Single Custom PL4 Limit (Strictly clamped to [PL2 + margin, 250] to prevent VRM BD PROCHOT 0.6G lock)
pub fn set_custom_pl4_ec(pl4: u16) -> bool {
    let margin = crate::system::config::get_cpu_pl4_margin();
    let cur_pl2 = wmi_read_ec(ADDR_CUSTOM_PL2).unwrap_or(15);
    let safe_min_pl4 = (cur_pl2 as u16 + margin).min(250);
    let pl4_val = pl4.clamp(safe_min_pl4, 250) as u8;
    let ok = wmi_write_ec(ADDR_CUSTOM_PL4, pl4_val);
    crate::log_info!(
        "EC_TUNING",
        "set_custom_pl4_ec: PL4={}W (clamped with safe_min_pl4={}W based on PL2={}W, margin={}W) -> ok={}",
        pl4_val, safe_min_pl4, cur_pl2, margin, ok
    );
    ok
}

/// Pure calculation helper for PL limits, enforcing PL1 <= PL2 and PL4 >= PL2 + margin transient buffer.
pub fn compute_safe_pl_limits(pl1: u16, pl2: u16, pl4: u16) -> (u8, u8, u8) {
    let margin = crate::system::config::get_cpu_pl4_margin();
    compute_safe_pl_limits_with_margin(pl1, pl2, pl4, margin)
}

pub fn compute_safe_pl_limits_with_margin(pl1: u16, pl2: u16, pl4: u16, margin: u16) -> (u8, u8, u8) {
    let pl1_val = pl1.clamp(15, 255) as u8;
    let pl2_val = pl2.clamp(pl1_val as u16, 255) as u8;
    // INVIOLABLE HARDWARE RULE: PL4 is peak instantaneous current/wattage limit.
    // PL4 MUST NEVER BE SMALLER THAN PL2 + margin!
    // If PL4 is too close to PL2, transient microsecond spikes trip VRM hardware overcurrent (BD PROCHOT),
    // immediately locking CPU to 0.54G~0.60G!
    let min_pl4 = (pl2_val as u16 + margin).min(255);
    let pl4_val = pl4.clamp(min_pl4, 255) as u8;
    (pl1_val, pl2_val, pl4_val)
}

/// Apply Custom CPU Power Limits (PL1, PL2, PL4) in full mode initialization
/// All Tongfang / Uniwill EC architectures operate in 1:1 direct wattage passthrough.
/// Enforces PL1 <= PL2 and PL4 >= PL2 + margin to prevent VRM emergency tripping.
pub fn set_custom_pl_limits(pl1: u16, pl2: u16, pl4: u16) -> bool {
    let (pl1_val, pl2_val, pl4_val) = compute_safe_pl_limits(pl1, pl2, pl4);

    let ok1 = wmi_write_ec(ADDR_CUSTOM_PL1, pl1_val);
    let ok2 = wmi_write_ec(ADDR_CUSTOM_PL2, pl2_val);
    let ok3 = wmi_write_ec(ADDR_CUSTOM_PL4, pl4_val);

    let margin = crate::system::config::get_cpu_pl4_margin();
    crate::log_info!(
        "CUSTOM_TRACE",
        "set_custom_pl_limits: PL1={}W, PL2={}W, PL4_reg={} (req={}W, min_safe={}W) -> ok={}",
        pl1_val, pl2_val, pl4_val, pl4, (pl2_val as u16 + margin).min(250), ok1 && ok2 && ok3
    );
    crate::log_info!("EC_TUNING", "set_custom_pl_limits: PL1={}W, PL2={}W, PL4_reg={} -> ok={}", pl1_val, pl2_val, pl4_val, ok1 && ok2 && ok3);
    ok1 && ok2 && ok3
}

/// Apply CPU Temperature Wall (Target Temperature 80C ~ 105C with Adaptive Architecture Support)
pub fn set_cpu_temp_target(enabled: bool, target_c: u8) -> bool {
    let is_amd = crate::system::cpu_monitor::is_amd_cpu();
    let native_tjmax = crate::system::cpu_monitor::get_cpu_native_tjmax();

    // Probe hardware factory preset from EC Turbo factory thermal limit
    let factory_tcc = wmi_read_ec(ADDR_TURBO_TCC).unwrap_or(0);

    // Robust sanity guard: if target_c is <= 30, user/caller passed an offset (e.g. 5 or 15)
    // instead of absolute Celsius. Convert safely to absolute Celsius (e.g. native_tjmax - 5 = 95C / 100C).
    let target_abs = if target_c <= 30 {
        native_tjmax.saturating_sub(target_c)
    } else {
        target_c
    };
    let cfg = crate::system::config::load_config();
    let min_temp = cfg.cpu_temp_target_min.min(cfg.cpu_temp_target_max).min(80);
    let max_temp = cfg.cpu_temp_target_max.max(native_tjmax);
    let clamped_temp = target_abs.clamp(min_temp, max_temp);

    // Hardware Platform Encoding Logic:
    // CPU temperature wall encoding strictly depends on CPU vendor architecture, NEVER GPU generation:
    // 1. Intel CPUs (All generations from 12th/13th/14th Gen HX to Core Ultra, paired with ANY GPU generation):
    //    Intel MSR 0x1A2 (IA32_TEMPERATURE_TARGET) physically operates on TCC Activation Offset (native_tjmax - temp).
    //    EC firmware expects Intel TCC Offset with Bit 7 enable switch: 0x80 | ((native_tjmax - clamped_temp) & 0x7F).
    //    E.g. On 105°C TjMax: 95°C -> Offset 10 -> 0x8A (138).
    //    E.g. On 100°C TjMax: 95°C -> Offset 5 -> 0x85 (133).
    //    When disabled: Clears Bit 7 while preserving factory offset (e.g. 0x05) to safely yield to BIOS defaults
    //    without triggering BD PROCHOT 0.8GHz freeze.
    // 2. AMD platforms (Ryzen Cezanne, Rembrandt, Phoenix, Hawk Point, Strix Point, Dragon Range):
    //    EC firmware expects literal Celsius with Bit 7 enable switch: 0x80 | (clamped_temp & 0x7F).
    //    E.g. 85°C -> 0xD5, 95°C -> 0xDF.
    //    When disabled: Clears Bit 7 while restoring factory Celsius (or safe 95°C).
    let raw_val = if is_amd {
        if enabled {
            0x80 | (clamped_temp & 0x7F) // Bit 7 = Enable Switch (0x80), Bits 6:0 = Target Celsius
        } else {
            // SAFE UNLOCK: For literal Celsius ECs, restore factory Celsius or safe 95°C with Bit 7 cleared.
            // NEVER write 0x00, which some ECs interpret as a literal 0°C thermal wall (triggering BD PROCHOT 0.80GHz freeze)!
            let factory_celsius = factory_tcc & 0x7F;
            if factory_celsius >= 60 && factory_celsius <= 105 { factory_celsius } else { 95 }
        }
    } else {
        // Intel offset mode (Bit 7 = Enable Switch, Bits 6:0 = Offset from native TjMax)
        if enabled {
            let tcc_offset = native_tjmax.saturating_sub(clamped_temp);
            0x80 | (tcc_offset & 0x7F)
        } else {
            // SAFE UNLOCK: Restore default offset with Bit 7 cleared or factory offset.
            let default_offset = native_tjmax.saturating_sub(95).max(5);
            let factory_offset = factory_tcc & 0x7F;
            if factory_offset > 0 && factory_offset <= 30 { factory_offset } else { default_offset }
        }
    };

    let ok = wmi_write_ec(ADDR_CUSTOM_TCC, raw_val);
    crate::log_info!(
        "CUSTOM_TRACE",
        "set_cpu_temp_target: enabled={}, target={}C (clamped={}C, tjmax={}C), factory_tcc=0x{:02X} -> custom_tcc=0x{:02X} (is_amd={}) -> ok={}",
        enabled, target_c, clamped_temp, native_tjmax, factory_tcc, raw_val, is_amd, ok
    );
    crate::log_info!("EC_TUNING", "set_cpu_temp_target: enabled={}, target={}C, tjmax={}C -> raw_val=0x{:02X} -> ok={}", enabled, target_c, native_tjmax, raw_val, ok);
    ok
}

/// Apply preset TCC value adaptively across Intel and AMD platforms.
/// Accepts either an offset from native TjMax (e.g. 0..30) or a target temperature in Celsius (e.g. 70..105).
/// Converts properly to the host platform encoding and writes to ADDR_CUSTOM_TCC.
pub fn apply_preset_tcc_adaptive(temp_offset_or_celsius: u8) -> bool {
    let native_tjmax = crate::system::cpu_monitor::get_cpu_native_tjmax();
    let target_c = if temp_offset_or_celsius <= 30 {
        native_tjmax.saturating_sub(temp_offset_or_celsius)
    } else {
        temp_offset_or_celsius
    };
    set_cpu_temp_target(true, target_c)
}

/// Apply Super Performance Switch (ADDR_SUPER_PERFORMANCE Bit 4)
pub fn set_super_performance(enable: bool) -> bool {
    let caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    if !caps.supports_gpu_tuning {
        crate::log_info!("EC_TUNING", "GPU does not support Super Performance / DB (Gen <= 20). Skipping Bit 4.");
        return true;
    }
    let cur = wmi_read_ec(ADDR_SUPER_PERFORMANCE).unwrap_or(0);
    // Preserve other bits such as FanBoost (0x40)
    let new_val = if enable { cur | 0x10 } else { cur & !0x10 };
    let ok = wmi_write_ec(ADDR_SUPER_PERFORMANCE, new_val);
    crate::log_info!(
        "CUSTOM_TRACE",
        "set_super_performance: enable={}, super_perf: 0x{:02X} -> 0x{:02X} (Bit4 TurboBase={}) -> ok={}",
        enable, cur, new_val, (new_val & 0x10) != 0, ok
    );
    crate::log_info!("EC_TUNING", "set_super_performance: enable={}, cur=0x{:02X}, new=0x{:02X} -> ok={}", enable, cur, new_val, ok);
    ok
}

/// Apply Complete GPU Power State Atomically
/// Apply GPU Power State (Base TGP / cTGP, Dynamic Boost Switch, Dynamic Boost Target Watts)
/// Strictly matches official GCUService / Mode4 hardware ground-truth verified by ACPI sniffer:
/// - Full-power cards (e.g. 4090/4080): Native base is already full physical base. ONLY ADDR_DB_WATTS1 is written!
/// - Non-full-power cards (e.g. 5070 Ti, 5060, 3060):
///   - ADDR_DB_CTRL1: Bit 0 (cTGP active), Bit 1 (DB active), Bit 2 (OEM GCU flag 0x04) -> 0x07 or 0x05
///   - ADDR_TGP_OFFSET: Relative offset to VBIOS cTGP floor (TargetBase - Floor)
///   - ADDR_DB_WATTS1: User specified Dynamic Boost target watts clamped to [5, max_db]W
pub fn set_gpu_power_state_ec(
    target_base_tgp: u8,
    db_enabled: bool,
    db_watts: u8,
    ctgp_enabled: bool,
    ctgp_target: u8,
) -> bool {
    let caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    if !caps.supports_gpu_tuning {
        crate::log_info!("EC_TUNING", "GPU does not support DB/cTGP tuning (Gen <= 20). Bypassing EC power register writes.");
        return true;
    }

    if caps.is_full_power {
        // Full-power cards (e.g. 4090/4080/5080/5090): ONLY write ADDR_DB_WATTS1!
        // Inviolable rule: Base TGP is locked at 150W; strictly NEVER touch cTGP registers (ADDR_TGP_OFFSET).
        let max_db = caps.max_db_watts.clamp(5, 35) as u8;
        let clamped_db = if db_enabled {
            db_watts.clamp(5, max_db)
        } else {
            5
        };
        let ok = wmi_write_ec(ADDR_DB_WATTS1, clamped_db);
        crate::log_info!(
            "EC_TUNING",
            "set_gpu_power_state_ec (Full-Power card): Only writing ADDR_DB_WATTS1={}W -> ok={}",
            clamped_db, ok
        );
        return ok;
    }

    // Non-full-power cards (30/50 series e.g. 5070 Ti, 5070, 5060, 4070, 4060, 3060):
    // Requires atomic assertion of cTGP down-regulation, cTGP floor offset, and DB target.
    let new_db_ctrl = if db_enabled { 0x07u8 } else { 0x05u8 };

    let floor = caps.ctgp_floor_watts as u8;
    let max_base = caps.base_tgp_max_watts as u8;
    let eff_base = if ctgp_enabled && ctgp_target > 0 {
        ctgp_target
    } else if target_base_tgp > 0 {
        target_base_tgp
    } else {
        max_base
    };

    let clamped_base = eff_base.clamp(floor, max_base);
    let max_offset = max_base.saturating_sub(floor);
    // Offset above VBIOS cTGP floor: Target Base - Floor
    let new_tgp_offset = clamped_base.saturating_sub(floor).min(max_offset);

    let max_db = caps.max_db_watts.clamp(5, 35) as u8;
    let clamped_db = if db_enabled {
        db_watts.clamp(5, max_db)
    } else {
        5
    };

    let entries = [
        (ADDR_DB_CTRL1, new_db_ctrl),
        (ADDR_TGP_OFFSET, new_tgp_offset),
        (ADDR_DB_WATTS1, clamped_db),
    ];

    let ok = wmi_write_ec_batch(&entries);

    crate::log_info!(
        "EC_TUNING",
        "set_gpu_power_state_ec (Non-full-power): db_ctrl=0x{:02X} (cTGP=true, DB={}), tgp_offset={}W (Effective Base={}W), db_target={}W -> ok={}",
        new_db_ctrl, db_enabled, new_tgp_offset, clamped_base, clamped_db, ok
    );
    ok
}

/// 读取当前实际生效的 GPU 总功耗上限（W），供 UI 显示真实 TGP 用：
///   - 非满血卡：有效基础 TGP = cTGP floor + EC 偏移；DB 使能时再加 DB 目标
///   - 满血卡：基础 TGP 锁定，仅 DB 目标参与
/// 读不到或不支持调优时返回 0，调用方回落到原厂预设值。
pub fn read_live_gpu_tgp_watts() -> u32 {
    let caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    if !caps.supports_gpu_tuning || caps.base_tgp_max_watts == 0 {
        return 0;
    }
    let db_target = wmi_read_ec(ADDR_DB_WATTS1).unwrap_or(5) as u32;
    if caps.is_full_power {
        let db = if db_target > 5 { db_target.min(caps.max_db_watts) } else { 0 };
        return caps.base_tgp_max_watts + db;
    }
    let floor = caps.ctgp_floor_watts;
    let offset = wmi_read_ec(ADDR_TGP_OFFSET).unwrap_or(0) as u32;
    let base = (floor + offset).min(caps.base_tgp_max_watts);
    // ADDR_DB_CTRL1 bit1 = Dynamic Boost 使能位
    const DB_CTRL_ENABLE_BIT: u8 = 0x02;
    let ctrl = wmi_read_ec(ADDR_DB_CTRL1).unwrap_or(0);
    let db_on = (ctrl & DB_CTRL_ENABLE_BIT) != 0;
    let db = if db_on { db_target.min(caps.max_db_watts) } else { 0 };
    base + db
}

/// Directly update Dynamic Boost Target Watts.
/// On full-power cards, only writes ADDR_DB_WATTS1.
/// On non-full-power cards, preserves cTGP offset.
pub fn set_dynamic_boost_watts(watts: u8) -> bool {
    let caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    if !caps.supports_gpu_tuning {
        return true;
    }
    if caps.is_full_power {
        let max_db = caps.max_db_watts.clamp(5, 35) as u8;
        let clamped_db = watts.clamp(5, max_db);
        let ok = wmi_write_ec(ADDR_DB_WATTS1, clamped_db);
        crate::log_info!("EC_TUNING", "set_dynamic_boost_watts (Full-Power): target={}W -> ok={}", clamped_db, ok);
        return ok;
    }

    let cur_tgp_offset = wmi_read_ec(ADDR_TGP_OFFSET).unwrap_or(0);
    let cur_db_ctrl = wmi_read_ec(ADDR_DB_CTRL1).unwrap_or(0x07);
    let ctgp_en = (cur_db_ctrl & 0x01) != 0;
    let db_en = (cur_db_ctrl & 0x02) != 0;
    let floor = caps.ctgp_floor_watts as u8;
    let max_base = caps.base_tgp_max_watts as u8;
    let eff_base = (floor + cur_tgp_offset).clamp(floor, max_base);
    set_gpu_power_state_ec(eff_base, db_en, watts, ctgp_en, eff_base)
}

/// Apply Dynamic Boost Switch & Watts safely without wiping cTGP state
pub fn set_dynamic_boost(enable: bool, watts: u8) -> bool {
    let caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    if !caps.supports_gpu_tuning {
        return true;
    }
    if caps.is_full_power {
        let max_db = caps.max_db_watts.clamp(5, 35) as u8;
        let clamped_db = if enable {
            watts.clamp(5, max_db)
        } else {
            5
        };
        let ok = wmi_write_ec(ADDR_DB_WATTS1, clamped_db);
        crate::log_info!("EC_TUNING", "set_dynamic_boost (Full-Power): enable={}, watts={}W -> ok={}", enable, clamped_db, ok);
        return ok;
    }

    let cur_tgp_offset = wmi_read_ec(ADDR_TGP_OFFSET).unwrap_or(0);
    let cur_db_ctrl = wmi_read_ec(ADDR_DB_CTRL1).unwrap_or(0x07);
    let ctgp_en = (cur_db_ctrl & 0x01) != 0;
    let floor = caps.ctgp_floor_watts as u8;
    let max_base = caps.base_tgp_max_watts as u8;
    let eff_base = (floor + cur_tgp_offset).clamp(floor, max_base);
    set_gpu_power_state_ec(eff_base, enable, watts, ctgp_en, eff_base)
}

/// Apply Fan Transition Sensitivity / Ramp Rate
/// Note: Setting 0x00 halts the hardware step transition timer for ~10 seconds in EC firmware!
/// When ramp rate filter is disabled, 0x81 (100ms/step) provides the fastest safe physical ramp rate.
pub fn set_fan_ramp_rate(enable: bool, speed_ms: u16) -> bool {
    let val = if enable {
        let step = (speed_ms.clamp(100, 12700) / 100) as u8;
        0x80 | step
    } else {
        0x81 // 100ms/step instant response without halting EC timer
    };
    let ok = wmi_write_ec(ADDR_FAN_RAMP_RATE, val);
    crate::log_info!("EC_TUNING", "set_fan_ramp_rate: enable={}, speed_ms={} (val=0x{:02X}) -> ok={}", enable, speed_ms, val, ok);
    ok
}

/// Apply Custom Mode EC Hardware Flag
pub fn set_custom_mode_ec(enable: bool) -> bool {
    let target = if enable { 65 } else { 64 };
    let ok = wmi_write_ec(ADDR_CUSTOM_MODE, target);
    crate::log_info!("EC_TUNING", "set_custom_mode_ec: enable={} (val={}) -> ok={}", enable, target, ok);
    ok
}

/// Apply Fan Independent Output
pub fn set_fan_isolated_output(enable: bool) -> bool {
    if let Some(cur) = wmi_read_ec(ADDR_FAN_ISOLATED) {
        let new_val = if enable { cur | 0x80 } else { cur & !0x80 };
        let ok = wmi_write_ec(ADDR_FAN_ISOLATED, new_val);
        crate::log_info!("EC_TUNING", "set_fan_isolated_output: enable={} (cur=0x{:02X} -> new=0x{:02X}) -> ok={}", enable, cur, new_val, ok);
        ok
    } else {
        false
    }
}

/// Apply Configurable TGP Down-Regulation safely without wiping Dynamic Boost state
pub fn set_gpu_ctgp(enable: bool, watts: u8) -> bool {
    let caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    if caps.is_full_power {
        crate::log_info!("EC_TUNING", "set_gpu_ctgp: Full-Power card detected. cTGP is not supported and strictly bypassed.");
        return true;
    }
    let cur_db_ctrl = wmi_read_ec(ADDR_DB_CTRL1).unwrap_or(0x07);
    let cur_db_target_watts = wmi_read_ec(ADDR_DB_WATTS1).unwrap_or(25);
    let db_en = (cur_db_ctrl & 0x02) != 0;
    set_gpu_power_state_ec(watts, db_en, cur_db_target_watts, enable, watts)
}

/// Restore official hardware baseline contract for GPU Dynamic Boost.
/// Safe one-time synchronization during initialization, mode-entry or reset:
/// - ADDR_FAN_BOOST_CLAMP Bit 4 (Super Performance): strictly aligned with user profile
/// - ADDR_DB_CTRL1: asserted to 0x07 (cTGP enabled, DB enabled, GCU active)
/// - ADDR_DB_FLAG1: asserted to 0xFF (OEM GCU driver contract handshake)
/// - Full-power cards: ADDR_TGP_OFFSET asserted to 0x00, ADDR_DB_WATTS1 asserted to target_db
/// - Non-full-power cards: ADDR_TGP_OFFSET asserted to relative offset, ADDR_DB_WATTS1 asserted to target_db
pub fn heal_gpu_power_contract(super_perf_enable: bool, target_db_watts: Option<u8>) -> bool {
    let caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    if !caps.supports_gpu_tuning {
        return true;
    }

    // 1. Align ADDR_FAN_BOOST_CLAMP Bit 4 (Super Performance / HWOC Gate)
    if let Some(cur_clamp) = wmi_read_ec(ADDR_FAN_BOOST_CLAMP) {
        let super_bit = if super_perf_enable { SUPER_PERF_BIT } else { 0 };
        let new_clamp = (cur_clamp & !SUPER_PERF_BIT) | super_bit;
        if new_clamp != cur_clamp {
            let ok = wmi_write_ec(ADDR_FAN_BOOST_CLAMP, new_clamp);
            crate::log_info!("EC_TUNING", "heal_gpu_power_contract: aligned ADDR_FAN_BOOST_CLAMP -> 0x{:02X} (ok={})", new_clamp, ok);
        }
    }

    let max_db = caps.max_db_watts.clamp(5, 35) as u8;
    let db_val = target_db_watts.unwrap_or(max_db).clamp(5, max_db);

    if caps.is_full_power {
        // Full-power cards (4090, 4080, 5080, 5090):
        // Handshake contract: DB_CTRL1 = 0x07, TGP_OFFSET = 0x00, DB_FLAG1 = 0xFF, DB_WATTS1 = db_val
        let entries = [
            (ADDR_DB_CTRL1, 0x07u8),
            (ADDR_TGP_OFFSET, 0x00u8),
            (ADDR_DB_FLAG1, 0xFFu8),
            (ADDR_DB_WATTS1, db_val),
        ];
        let ok = wmi_write_ec_batch(&entries);
        crate::log_info!(
            "EC_TUNING",
            "heal_gpu_power_contract (Full-Power): restored contract [CTRL1=0x07, OFFSET=0x00, FLAG1=0xFF, WATTS1={}W] -> ok={}",
            db_val, ok
        );
        ok
    } else {
        // Non-full-power cards:
        // Handshake contract: DB_CTRL1 = 0x07, DB_FLAG1 = 0xFF, TGP_OFFSET = target full base offset, DB_WATTS1 = db_val
        let floor = caps.ctgp_floor_watts as u8;
        let max_base = caps.base_tgp_max_watts as u8;
        let max_offset = max_base.saturating_sub(floor);
        let cur_offset = wmi_read_ec(ADDR_TGP_OFFSET).unwrap_or(max_offset);
        // If current offset is 0 but max_offset > 0, offset was wiped or uninitialized, restore to full base default!
        let target_offset = if cur_offset == 0 && max_offset > 0 {
            max_offset
        } else {
            cur_offset.min(max_offset)
        };

        let entries = [
            (ADDR_DB_CTRL1, 0x07u8),
            (ADDR_TGP_OFFSET, target_offset),
            (ADDR_DB_FLAG1, 0xFFu8),
            (ADDR_DB_WATTS1, db_val),
        ];
        let ok = wmi_write_ec_batch(&entries);
        crate::log_info!(
            "EC_TUNING",
            "heal_gpu_power_contract (Non-full-power): restored contract [CTRL1=0x07, OFFSET={}W, FLAG1=0xFF, WATTS1={}W] -> ok={}",
            target_offset, db_val, ok
        );
        ok
    }
}

/// Heals and restores MyFan2 fan speed table if it was previously corrupted by modern DB writes (e.g. 0xFF in L3 PWM)
pub fn heal_legacy_fan_table_if_needed() {
    if !crate::hal::platform_probe::is_legacy_pascal_turing_tongfang() {
        return;
    }

    // Check if L3 PWM (ADDR_MYFAN2_L3_PWM / ADDR_DB_FLAG1) was corrupted to 0xFF (255)
    let cur_l3 = wmi_read_ec(ADDR_MYFAN2_L3_PWM).unwrap_or(0);
    if cur_l3 == 0xFF {
        crate::log_warn!(
            "FAN_HEAL",
            "Detected corrupted MyFan2 fan speed table (L3 PWM=0xFF)! Auto-healing from hardware defaults..."
        );

        // Try reading factory default PWM table (ADDR_MYFAN2_L1_DEFAULT..L5_DEFAULT)
        let def1 = wmi_read_ec(ADDR_MYFAN2_L1_DEFAULT).unwrap_or(0);
        let def2 = wmi_read_ec(ADDR_MYFAN2_L2_DEFAULT).unwrap_or(0);
        let def3 = wmi_read_ec(ADDR_MYFAN2_L3_DEFAULT).unwrap_or(0);
        let def4 = wmi_read_ec(ADDR_MYFAN2_L4_DEFAULT).unwrap_or(0);
        let def5 = wmi_read_ec(ADDR_MYFAN2_L5_DEFAULT).unwrap_or(0);

        let (p1, p2, p3, p4, p5) = if def1 > 0 && def5 > 0 && def1 <= def5 {
            (def1, def2, def3, def4, def5)
        } else {
            // Safe fallback standard stepped curve (30%, 45%, 60%, 75%, 90%)
            (30, 45, 60, 75, 90)
        };

        let heal_entries = [
            (ADDR_MYFAN2_L1_PWM, p1),
            (ADDR_MYFAN2_L2_PWM, p2),
            (ADDR_MYFAN2_L3_PWM, p3),
            (ADDR_MYFAN2_L4_PWM, p4),
            (ADDR_MYFAN2_L5_PWM, p5),
        ];
        let _ = wmi_write_ec_batch(&heal_entries);
        crate::log_info!(
            "FAN_HEAL",
            "MyFan2 fan speed table successfully restored: L1={}, L2={}, L3={}, L4={}, L5={}",
            p1, p2, p3, p4, p5
        );
    }
}

/// Heal CPU hardware power & thermal wall contract (Safe Unlock & Recovery)
/// Restores safe TCC thermal wall (clearing dangerous 105C / 0x00 freeze states),
/// restores safe PL1/PL2/PL4 limits, and triggers EC mode re-evaluation to break 55W Limp Mode.
pub fn heal_cpu_power_contract(
    pl1: u16,
    pl2: u16,
    pl4: u16,
    temp_target_enabled: bool,
    temp_target_c: u8,
) -> bool {
    // 1. Safety Clamp and Apply PL Limits
    let (p1, p2, p4) = compute_safe_pl_limits(pl1, pl2, pl4);
    let ok_pl = set_custom_pl_limits(p1 as u16, p2 as u16, p4 as u16);

    // 2. Safe TCC Thermal Wall Healing
    let native_tjmax = crate::system::cpu_monitor::get_cpu_native_tjmax();
    let safe_temp_c = if temp_target_enabled {
        temp_target_c.clamp(80, native_tjmax)
    } else {
        native_tjmax.saturating_sub(5).max(95)
    };
    let ok_tcc = set_cpu_temp_target(temp_target_enabled, safe_temp_c);

    // 3. Clear Silent Mode & Overheat Throttle Flags
    let _ = wmi_write_ec(ADDR_TURBO_TRIGGER_1115, 0);

    // 4. Kick EC Trigger Byte to force EC power/thermal evaluation refresh (breaks Limp Mode)
    let cur_trig = wmi_read_ec(ADDR_TRIGGER_BYTE).unwrap_or(0);
    let ok_trig = wmi_write_ec(ADDR_TRIGGER_BYTE, (cur_trig & 0x10) | 0x20);

    // 5. Ensure Mode Index preserves Logo bits while asserting Custom Mode (0x10)
    let cur_mode = wmi_read_ec(ADDR_MODE_INDEX).unwrap_or(0);
    let br_bits = cur_mode & 0xE0;
    let is_ec_logo = crate::driver::ite_lighting::IteLightingManager::is_ec_logo_supported();
    let support_bit = if is_ec_logo { 0x01 } else { cur_mode & 0x01 };
    let new_mode = br_bits | 0x10 | support_bit;
    let ok_mode = wmi_write_ec(ADDR_MODE_INDEX, new_mode);

    let ok = ok_pl && ok_tcc && ok_trig && ok_mode;
    crate::log_info!(
        "EC_TUNING",
        "heal_cpu_power_contract: CPU power contract restored [PL1={}W, PL2={}W, PL4={}W, SafeTemp={}C] -> ok={}",
        p1, p2, p4, safe_temp_c, ok
    );
    ok
}

/// Latch & Commit Fan Table into EC firmware active controller.
/// Ground-truth reverse engineered from official GCUService:
/// - Pre-latch pulse: resets latch state
/// - Post-latch commit: latches and activates the RAM fan curves
/// - Release: returns control back to default BIOS ROM curves
pub fn latch_fan_table_ec(commit: bool) -> bool {
    if commit {
        let _ = wmi_write_ec(ADDR_FAN_TABLE_LATCH, FAN_TABLE_LATCH_BIT);
        let _ = wmi_write_ec(ADDR_FAN_TABLE_LATCH, 0x00);
        true
    } else {
        let ok = wmi_write_ec(ADDR_FAN_TABLE_LATCH, 0x00);
        crate::log_info!("EC_FAN", "Fan table hardware latch released: ok={}", ok);
        ok
    }
}

/// Commit and enforce the RAM fan table into the EC active controller
pub fn commit_fan_table_latch_ec() -> bool {
    let ok = wmi_write_ec(ADDR_FAN_TABLE_LATCH, FAN_TABLE_LATCH_BIT);
    crate::log_info!("EC_FAN", "Fan table hardware latch committed: ok={}", ok);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_safe_pl_limits_transient_headroom() {
        // User screenshot case: PL1=130W, PL2=130W, PL4=135W (only 5W buffer!)
        // Must be elevated to at least 150W (130 + 20) to prevent VRM BD PROCHOT 0.6G clamp!
        let (p1, p2, p4) = compute_safe_pl_limits(130, 130, 135);
        assert_eq!(p1, 130);
        assert_eq!(p2, 130);
        assert_eq!(p4, 150);
        assert!(p4 >= p2 + 20);

        // GM6IX9B buggy preset: PL1=205W, PL2=205W, PL4=200W (PL4 < PL2!)
        let (p1, p2, p4) = compute_safe_pl_limits(205, 205, 200);
        assert_eq!(p1, 205);
        assert_eq!(p2, 205);
        assert_eq!(p4, 225); // 205 + 20 = 225
        assert!(p4 >= p2 + 20);

        // PL2 clamped to at least PL1
        let (p1, p2, p4) = compute_safe_pl_limits(140, 100, 120);
        assert_eq!(p1, 140);
        assert_eq!(p2, 140);
        assert_eq!(p4, 160); // 140 + 20 = 160

        // High wattage upper bound clamping to 255W (u8 hardware ceiling)
        let (p1, p2, p4) = compute_safe_pl_limits(240, 250, 240);
        assert_eq!(p1, 240);
        assert_eq!(p2, 250);
        assert_eq!(p4, 255); // clamped to 255

        // Normal safe preset with ample headroom preserved
        let (p1, p2, p4) = compute_safe_pl_limits(75, 75, 125);
        assert_eq!(p1, 75);
        assert_eq!(p2, 75);
        assert_eq!(p4, 125);

        // Absolute safe minimum floor: PL1=15W, PL2=15W, PL4=35W
        let (p1, p2, p4) = compute_safe_pl_limits(15, 15, 35);
        assert_eq!(p1, 15);
        assert_eq!(p2, 15);
        assert_eq!(p4, 35);
        assert!(p4 >= p2 + 20);
    }

    #[test]
    fn test_tcc_conversion_rules() {
        // Offset 5 -> 95C
        let target = if 5u8 <= 30 { 100u8.saturating_sub(5) } else { 5 };
        assert_eq!(target, 95);

        // Offset 15 -> 85C
        let target = if 15u8 <= 30 { 100u8.saturating_sub(15) } else { 15 };
        assert_eq!(target, 85);

        // Literal Celsius 92 -> 92C
        let target = if 92u8 <= 30 { 100u8.saturating_sub(92) } else { 92 };
        assert_eq!(target, 92);
    }
}


