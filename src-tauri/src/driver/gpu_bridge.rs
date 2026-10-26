use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicI8, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use parking_lot::{Mutex, RwLock};

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Fast atomic cache for NVIDIA discrete GPU presence:
/// -1 = unprobed, 0 = no NVIDIA dGPU (pure iGPU / AMD / Intel Arc), 1 = NVIDIA dGPU present
static HAS_NVIDIA_DGPU: AtomicI8 = AtomicI8::new(-1);

/// Probes whether this laptop physically contains an NVIDIA discrete GPU via fast Win32 registry (<0.2ms).
/// On thin-and-light laptops (e.g. Mechrevo Wujie / Code series) or pure iGPU laptops, returns false.
/// Completely prevents any execution of 'nvidia-smi' to avoid 2.5s timeouts, hangs, and battery drain.
pub fn is_nvidia_dgpu_available() -> bool {
    let cached = HAS_NVIDIA_DGPU.load(Ordering::Relaxed);
    if cached != -1 {
        return cached == 1;
    }
    #[cfg(windows)]
    {
        let is_nv = probe_nvidia_presence_registry();
        HAS_NVIDIA_DGPU.store(if is_nv { 1 } else { 0 }, Ordering::SeqCst);
        if !is_nv {
            crate::log_info!(
                "GPU_BRIDGE",
                "Non-NVIDIA or pure iGPU laptop: permanently disabled nvidia-smi process queries and poller"
            );
        }
        is_nv
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Canonical Hardware Truth: Determines whether a GPU adapter name / driver description
/// represents a genuine discrete physical graphics card (NVIDIA, AMD Radeon RX, Intel Arc dGPU).
/// Strictly eliminates virtual display adapters (ToDesk, MuMu, GameViewer, RDP) and integrated SoCs.
pub fn is_discrete_gpu_name(name: &str) -> bool {
    let u = name.to_uppercase();
    if u.is_empty()
        || u.contains("BASIC")
        || u.contains("MIRAGE")
        || u.contains("REMOTE")
        || u.contains("VIRTUAL")
        || u.contains("ORAY")
        || u.contains("TODESK")
        || u.contains("MUMU")
        || u.contains("GAMEVIEWER")
        || u.contains("AUDIO")
    {
        return false;
    }

    if u.contains("UHD") || u.contains("IRIS") || u.contains("HD GRAPHICS") {
        return false;
    }

    let is_discrete_vendor_series = u.contains("NVIDIA")
        || u.contains("GEFORCE")
        || u.contains("RTX")
        || u.contains("GTX")
        || u.contains("QUADRO")
        || u.contains("TITAN")
        || u.contains("RADEON RX")
        || u.contains("RADEON PRO")
        || (u.contains("ARC") && (
            u.contains("A3") || u.contains("A5") || u.contains("A7")
            || u.contains("B5") || u.contains("B7") || u.contains("PRO A")
        ));

    is_discrete_vendor_series
}

pub fn is_nvidia_gpu_name(name: &str) -> bool {
    let u = name.to_uppercase();
    if !is_discrete_gpu_name(name) {
        return false;
    }
    u.contains("NVIDIA") || u.contains("GEFORCE") || u.contains("RTX") || u.contains("GTX") || u.contains("QUADRO") || u.contains("TITAN")
}

#[cfg(windows)]
fn probe_nvidia_presence_registry() -> bool {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    // 0. Quick OEM registry contract probe
    let oem_key_str: Vec<u16> = "SOFTWARE\\OEM\\GamingCenter2\\ItemSupport\0".encode_utf16().collect();
    let mut h_oem = HKEY::default();
    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(oem_key_str.as_ptr()), 0, KEY_READ, &mut h_oem).is_ok() {
            let mut val: u32 = 0;
            let mut val_size = std::mem::size_of::<u32>() as u32;
            let mut val_type = windows::Win32::System::Registry::REG_DWORD;
            let is_nv_val: Vec<u16> = "IsNvGpu\0".encode_utf16().collect();
            if RegQueryValueExW(
                h_oem,
                PCWSTR(is_nv_val.as_ptr()),
                None,
                Some(&mut val_type),
                Some(&mut val as *mut u32 as *mut _),
                Some(&mut val_size),
            ).is_ok() && val == 1 {
                let _ = RegCloseKey(h_oem);
                return true;
            }
            let _ = RegCloseKey(h_oem);
        }
    }

    // 1. Dynamic Win32 PnP Display Class Enumeration (handles unlimited driver reinstallations/reboots)
    let base_class_str: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\0".encode_utf16().collect();
    let mut h_base = HKEY::default();
    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(base_class_str.as_ptr()), 0, KEY_READ, &mut h_base).is_ok() {
            let mut index = 0;
            loop {
                let mut name_buf = [0u16; 64];
                let mut name_len = name_buf.len() as u32;
                let ret = RegEnumKeyExW(
                    h_base,
                    index,
                    PWSTR(name_buf.as_mut_ptr()),
                    &mut name_len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                );
                if ret.is_err() {
                    break;
                }

                let sub_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                if sub_name.len() == 4 && sub_name.chars().all(|c| c.is_ascii_digit()) {
                    let full_subkey = format!("SYSTEM\\CurrentControlSet\\Control\\Class\\{{4d36e968-e325-11ce-bfc1-08002be10318}}\\{}\0", sub_name);
                    let wide_full: Vec<u16> = full_subkey.encode_utf16().collect();
                    let mut h_sub = HKEY::default();
                    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_full.as_ptr()), 0, KEY_READ, &mut h_sub).is_ok() {
                        let mut buf = [0u16; 256];
                        let mut size = (buf.len() * 2) as u32;
                        let mut data_type = REG_SZ;
                        let val_name: Vec<u16> = "DriverDesc\0".encode_utf16().collect();

                        if RegQueryValueExW(
                            h_sub,
                            PCWSTR(val_name.as_ptr()),
                            None,
                            Some(&mut data_type),
                            Some(buf.as_mut_ptr() as *mut _),
                            Some(&mut size),
                        ).is_ok() {
                            let len = (size as usize / 2).saturating_sub(1);
                            let desc = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                            if is_nvidia_gpu_name(&desc) {
                                let _ = RegCloseKey(h_sub);
                                let _ = RegCloseKey(h_base);
                                return true;
                            }
                        }
                        let _ = RegCloseKey(h_sub);
                    }
                }
                index += 1;
            }
            let _ = RegCloseKey(h_base);
        }
    }
    false
}

#[derive(Default)]
struct GpuTelemetryInner {
    temp: AtomicU32,
    usage: AtomicU32,
    core_freq: AtomicU32,
    mem_freq: AtomicU32,
    vram_mb: AtomicU64,
    vram_used_mb: AtomicU64,
    core_offset: AtomicI32,
    mem_offset: AtomicI32,
    default_power_limit_w: AtomicU32,
    max_power_limit_w: AtomicU32,
    power_dw: AtomicU32,
}

pub struct GpuBridge {
    inner: Arc<GpuTelemetryInner>,
    gpu_name: Arc<parking_lot::Mutex<String>>,
    pub is_mock: bool,
}

#[cfg(windows)]
pub fn detect_dgpu_from_pci_enum() -> Option<(String, u64)> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    let pci_path: Vec<u16> = "SYSTEM\\CurrentControlSet\\Enum\\PCI\0".encode_utf16().collect();
    let mut h_pci = HKEY::default();
    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(pci_path.as_ptr()), 0, KEY_READ, &mut h_pci).is_ok() {
            let mut i = 0;
            loop {
                let mut name_buf = [0u16; 128];
                let mut name_len = name_buf.len() as u32;
                if RegEnumKeyExW(h_pci, i, PWSTR(name_buf.as_mut_ptr()), &mut name_len, None, PWSTR::null(), None, None).is_err() {
                    break;
                }
                let dev_key = String::from_utf16_lossy(&name_buf[..name_len as usize]).to_uppercase();
                if dev_key.starts_with("VEN_10DE") {
                    let sub_path = format!("SYSTEM\\CurrentControlSet\\Enum\\PCI\\{}\0", dev_key);
                    let sub_wide: Vec<u16> = sub_path.encode_utf16().collect();
                    let mut h_dev = HKEY::default();
                    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(sub_wide.as_ptr()), 0, KEY_READ, &mut h_dev).is_ok() {
                        let mut j = 0;
                        loop {
                            let mut inst_buf = [0u16; 128];
                            let mut inst_len = inst_buf.len() as u32;
                            if RegEnumKeyExW(h_dev, j, PWSTR(inst_buf.as_mut_ptr()), &mut inst_len, None, PWSTR::null(), None, None).is_err() {
                                break;
                            }
                            let inst_key = String::from_utf16_lossy(&inst_buf[..inst_len as usize]);
                            let inst_path = format!("SYSTEM\\CurrentControlSet\\Enum\\PCI\\{}\\{}\0", dev_key, inst_key);
                            let inst_wide: Vec<u16> = inst_path.encode_utf16().collect();
                            let mut h_inst = HKEY::default();
                            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(inst_wide.as_ptr()), 0, KEY_READ, &mut h_inst).is_ok() {
                                let mut desc_buf = [0u16; 256];
                                let mut desc_size = (desc_buf.len() * 2) as u32;
                                let mut desc_type = REG_SZ;
                                let val_name: Vec<u16> = "DeviceDesc\0".encode_utf16().collect();
                                if RegQueryValueExW(
                                    h_inst,
                                    PCWSTR(val_name.as_ptr()),
                                    None,
                                    Some(&mut desc_type),
                                    Some(desc_buf.as_mut_ptr() as *mut _),
                                    Some(&mut desc_size),
                                ).is_ok() {
                                    let len = (desc_size as usize / 2).saturating_sub(1);
                                    let raw_desc = String::from_utf16_lossy(&desc_buf[..len]).trim().to_string();
                                    let clean_desc = if let Some(pos) = raw_desc.rfind(';') {
                                        raw_desc[pos + 1..].trim().to_string()
                                    } else {
                                        raw_desc
                                    };
                                    if is_nvidia_gpu_name(&clean_desc) {
                                        let _ = RegCloseKey(h_inst);
                                        let _ = RegCloseKey(h_dev);
                                        let _ = RegCloseKey(h_pci);
                                        return Some((clean_desc, 16384));
                                    }
                                }
                                let _ = RegCloseKey(h_inst);
                            }
                            j += 1;
                        }
                        let _ = RegCloseKey(h_dev);
                    }
                }
                i += 1;
            }
            let _ = RegCloseKey(h_pci);
        }
    }
    None
}

#[cfg(windows)]
pub fn detect_discrete_gpu() -> Option<(String, u64)> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_QWORD, REG_SZ,
    };

    // Priority 1: Windows Display Class {4d36e968...}
    let base_class_str: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\0".encode_utf16().collect();
    let mut h_base = HKEY::default();

    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(base_class_str.as_ptr()), 0, KEY_READ, &mut h_base).is_ok() {
            let mut index = 0;
            loop {
                let mut name_buf = [0u16; 64];
                let mut name_len = name_buf.len() as u32;
                if RegEnumKeyExW(h_base, index, PWSTR(name_buf.as_mut_ptr()), &mut name_len, None, PWSTR::null(), None, None).is_err() {
                    break;
                }

                let sub_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                if sub_name.len() == 4 && sub_name.chars().all(|c| c.is_ascii_digit()) {
                    let full_subkey = format!("SYSTEM\\CurrentControlSet\\Control\\Class\\{{4d36e968-e325-11ce-bfc1-08002be10318}}\\{}\0", sub_name);
                    let wide_full: Vec<u16> = full_subkey.encode_utf16().collect();
                    let mut hkey = HKEY::default();

                    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_full.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
                        let mut buf = [0u16; 256];
                        let mut size = (buf.len() * 2) as u32;
                        let mut data_type = REG_SZ;
                        let val_name: Vec<u16> = "DriverDesc\0".encode_utf16().collect();

                        if RegQueryValueExW(
                            hkey,
                            PCWSTR(val_name.as_ptr()),
                            None,
                            Some(&mut data_type),
                            Some(buf.as_mut_ptr() as *mut _),
                            Some(&mut size),
                        ).is_ok() {
                            let len = (size as usize / 2).saturating_sub(1);
                            let desc = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                            let is_discrete = is_discrete_gpu_name(&desc);

                            if is_discrete {
                                let mut vram_bytes: u64 = 0;
                                let qw_name: Vec<u16> = "HardwareInformation.qwMemorySize\0".encode_utf16().collect();
                                let mut qw_val: u64 = 0;
                                let mut qw_size = std::mem::size_of::<u64>() as u32;
                                let mut qw_type = REG_QWORD;

                                if RegQueryValueExW(
                                    hkey,
                                    PCWSTR(qw_name.as_ptr()),
                                    None,
                                    Some(&mut qw_type),
                                    Some(&mut qw_val as *mut u64 as *mut _),
                                    Some(&mut qw_size),
                                ).is_ok() && qw_val > 0 {
                                    vram_bytes = qw_val;
                                } else {
                                    let dw_name: Vec<u16> = "HardwareInformation.MemorySize\0".encode_utf16().collect();
                                    let mut dw_val: u32 = 0;
                                    let mut dw_size = std::mem::size_of::<u32>() as u32;
                                    let mut dw_type = REG_DWORD;
                                    if RegQueryValueExW(
                                        hkey,
                                        PCWSTR(dw_name.as_ptr()),
                                        None,
                                        Some(&mut dw_type),
                                        Some(&mut dw_val as *mut u32 as *mut _),
                                        Some(&mut dw_size),
                                    ).is_ok() {
                                        vram_bytes = dw_val as u64;
                                    }
                                }

                                let vram_mb = if vram_bytes > 0 { vram_bytes / (1024 * 1024) } else { 8192 };
                                let _ = RegCloseKey(hkey);
                                let _ = RegCloseKey(h_base);
                                return Some((desc, vram_mb));
                            }
                        }
                        let _ = RegCloseKey(hkey);
                    }
                }
                index += 1;
            }
            let _ = RegCloseKey(h_base);
        }
    }

    // Priority 2: System PCI Bus Topology Registry Scan (bypasses virtual display driver clutter)
    if let Some(pci_dgpu) = detect_dgpu_from_pci_enum() {
        return Some(pci_dgpu);
    }

    // Priority 3: Fallback quick nvidia-smi probe (if NVIDIA driver is active)
    if is_nvidia_dgpu_available() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let out = Command::new("nvidia-smi")
                .creation_flags(CREATE_NO_WINDOW)
                .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
                .output();
            let _ = tx.send(out);
        });

        if let Ok(Ok(out)) = rx.recv_timeout(Duration::from_millis(600)) {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
                if !parts.is_empty() && !parts[0].is_empty() {
                    let name = parts[0].to_string();
                    let vram = parts.get(1).and_then(|v| v.parse::<u64>().ok()).unwrap_or(8192);
                    return Some((name, vram));
                }
            }
        }
    }

    None
}

#[cfg(windows)]
pub fn detect_igpu_from_pci_enum() -> Option<(String, u64)> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    let pci_path: Vec<u16> = "SYSTEM\\CurrentControlSet\\Enum\\PCI\0".encode_utf16().collect();
    let mut h_pci = HKEY::default();
    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(pci_path.as_ptr()), 0, KEY_READ, &mut h_pci).is_ok() {
            let mut i = 0;
            loop {
                let mut name_buf = [0u16; 128];
                let mut name_len = name_buf.len() as u32;
                if RegEnumKeyExW(h_pci, i, PWSTR(name_buf.as_mut_ptr()), &mut name_len, None, PWSTR::null(), None, None).is_err() {
                    break;
                }
                let dev_key = String::from_utf16_lossy(&name_buf[..name_len as usize]).to_uppercase();
                // VEN_1002 = AMD, VEN_8086 = Intel
                if dev_key.starts_with("VEN_1002") || dev_key.starts_with("VEN_8086") {
                    let sub_path = format!("SYSTEM\\CurrentControlSet\\Enum\\PCI\\{}\0", dev_key);
                    let sub_wide: Vec<u16> = sub_path.encode_utf16().collect();
                    let mut h_dev = HKEY::default();
                    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(sub_wide.as_ptr()), 0, KEY_READ, &mut h_dev).is_ok() {
                        let mut j = 0;
                        loop {
                            let mut inst_buf = [0u16; 128];
                            let mut inst_len = inst_buf.len() as u32;
                            if RegEnumKeyExW(h_dev, j, PWSTR(inst_buf.as_mut_ptr()), &mut inst_len, None, PWSTR::null(), None, None).is_err() {
                                break;
                            }
                            let inst_key = String::from_utf16_lossy(&inst_buf[..inst_len as usize]);
                            let inst_path = format!("SYSTEM\\CurrentControlSet\\Enum\\PCI\\{}\\{}\0", dev_key, inst_key);
                            let inst_wide: Vec<u16> = inst_path.encode_utf16().collect();
                            let mut h_inst = HKEY::default();
                            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(inst_wide.as_ptr()), 0, KEY_READ, &mut h_inst).is_ok() {
                                let mut desc_buf = [0u16; 256];
                                let mut desc_size = (desc_buf.len() * 2) as u32;
                                let mut desc_type = REG_SZ;
                                let val_name: Vec<u16> = "DeviceDesc\0".encode_utf16().collect();
                                if RegQueryValueExW(
                                    h_inst,
                                    PCWSTR(val_name.as_ptr()),
                                    None,
                                    Some(&mut desc_type),
                                    Some(desc_buf.as_mut_ptr() as *mut _),
                                    Some(&mut desc_size),
                                ).is_ok() {
                                    let len = (desc_size as usize / 2).saturating_sub(1);
                                    let raw_desc = String::from_utf16_lossy(&desc_buf[..len]).trim().to_string();
                                    let clean_desc = if let Some(pos) = raw_desc.rfind(';') {
                                        raw_desc[pos + 1..].trim().to_string()
                                    } else {
                                        raw_desc
                                    };
                                    let upper = clean_desc.to_uppercase();
                                    let is_igpu_match = (upper.contains("RADEON") || upper.contains("INTEL") || upper.contains("GRAPHICS") || upper.contains("UHD") || upper.contains("IRIS"))
                                        && !upper.contains("NVIDIA")
                                        && !upper.contains("GEFORCE")
                                        && !upper.contains("AUDIO")
                                        && !upper.contains("RADEON RX");
                                    if is_igpu_match {
                                        let _ = RegCloseKey(h_inst);
                                        let _ = RegCloseKey(h_dev);
                                        let _ = RegCloseKey(h_pci);
                                        return Some((clean_desc, 0));
                                    }
                                }
                                let _ = RegCloseKey(h_inst);
                            }
                            j += 1;
                        }
                        let _ = RegCloseKey(h_dev);
                    }
                }
                i += 1;
            }
            let _ = RegCloseKey(h_pci);
        }
    }
    None
}

#[cfg(windows)]
pub fn detect_integrated_gpu() -> Option<(String, u64)> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    let base_class_str: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\0".encode_utf16().collect();
    let mut h_base = HKEY::default();

    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(base_class_str.as_ptr()), 0, KEY_READ, &mut h_base).is_ok() {
            let mut index = 0;
            loop {
                let mut name_buf = [0u16; 64];
                let mut name_len = name_buf.len() as u32;
                if RegEnumKeyExW(h_base, index, PWSTR(name_buf.as_mut_ptr()), &mut name_len, None, PWSTR::null(), None, None).is_err() {
                    break;
                }

                let sub_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                if sub_name.len() == 4 && sub_name.chars().all(|c| c.is_ascii_digit()) {
                    let full_subkey = format!("SYSTEM\\CurrentControlSet\\Control\\Class\\{{4d36e968-e325-11ce-bfc1-08002be10318}}\\{}\0", sub_name);
                    let wide_full: Vec<u16> = full_subkey.encode_utf16().collect();
                    let mut hkey = HKEY::default();

                    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_full.as_ptr()), 0, KEY_READ, &mut hkey).is_ok() {
                        let mut buf = [0u16; 256];
                        let mut size = (buf.len() * 2) as u32;
                        let mut data_type = REG_SZ;
                        let val_name: Vec<u16> = "DriverDesc\0".encode_utf16().collect();

                        if RegQueryValueExW(
                            hkey,
                            PCWSTR(val_name.as_ptr()),
                            None,
                            Some(&mut data_type),
                            Some(buf.as_mut_ptr() as *mut _),
                            Some(&mut size),
                        ).is_ok() {
                            let len = (size as usize / 2).saturating_sub(1);
                            let desc = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                            let desc_upper = desc.to_uppercase();

                            let is_known_mobile_apu = desc_upper.contains("780M")
                                || desc_upper.contains("680M")
                                || desc_upper.contains("890M")
                                || desc_upper.contains("760M")
                                || desc_upper.contains("660M")
                                || desc_upper.contains("610M");

                            let is_workstation_discrete_pro = desc_upper.contains("RADEON PRO") && !is_known_mobile_apu;

                            if (desc_upper.contains("INTEL") || desc_upper.contains("RADEON") || desc_upper.contains("UHD") || desc_upper.contains("IRIS"))
                                && !desc_upper.contains("RX")
                                && !is_workstation_discrete_pro
                                && !desc_upper.contains("NVIDIA")
                            {
                                let _ = RegCloseKey(hkey);
                                let _ = RegCloseKey(h_base);
                                return Some((desc, 0));
                            }
                        }
                        let _ = RegCloseKey(hkey);
                    }
                }
                index += 1;
            }
            let _ = RegCloseKey(h_base);
        }
    }

    // Priority 2: System PCI Bus Topology Registry Scan for Integrated GPU (VEN_1002 / VEN_8086)
    if let Some(pci_igpu) = detect_igpu_from_pci_enum() {
        return Some(pci_igpu);
    }

    // Fallback: platform architecture fallback
    let (cpu, is_amd) = crate::driver::uefi_nvram::get_cpu_info();
    if is_amd {
        Some(("AMD Radeon Graphics".to_string(), 0))
    } else if !cpu.is_empty() {
        Some(("Intel(R) Graphics".to_string(), 0))
    } else {
        None
    }
}

#[cfg(windows)]
pub fn detect_gpu_from_registry() -> Option<(String, u64)> {
    // Crucial: Discrete GPU takes absolute precedence over integrated SoC
    detect_discrete_gpu().or_else(detect_integrated_gpu)
}

fn detect_initial_gpu() -> (String, u64, u16, u16) {
    // 0. 优先走纯 Win32 注册表探测（0 进程、0 锁、0.2ms 极速，不唤醒休眠独显，彻底根除冷启动卡死）
    #[cfg(windows)]
    if let Some((name, vram_mb)) = detect_gpu_from_registry() {
        let final_vram = if vram_mb > 0 { vram_mb } else { 8192 };
        crate::system::boot_trace::boot_log(
            "GPU_INIT",
            &format!("GPU detected via registry fast-path (0ms): {} ({} MB)", name, final_vram),
        );
        return (name, final_vram, 0, 0);
    }

    // 1. Fallback: Query nvidia-smi with strict timeout to prevent any boot hang
    // CRITICAL: On non-NVIDIA / pure iGPU laptops (e.g. Wujie/Code), completely bypass nvidia-smi!
    if !is_nvidia_dgpu_available() {
        return ("集成显卡/核显".to_string(), 4096, 0, 0);
    }

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let output = Command::new("nvidia-smi")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "--query-gpu=name,memory.total,power.default_limit,power.max_limit",
                "--format=csv,noheader,nounits",
            ])
            .output();
        let _ = tx.send(output);
    });

    if let Ok(Ok(out)) = rx.recv_timeout(Duration::from_millis(1500)) {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                let name = parts[0].to_string();
                let vram = parts.get(1).and_then(|v| v.parse::<u64>().ok()).unwrap_or(8192);
                let def_pwr = parts.get(2).and_then(|v| v.parse::<f32>().ok()).map(|f| f.round() as u16).unwrap_or(0);
                let max_pwr = parts.get(3).and_then(|v| v.parse::<f32>().ok()).map(|f| f.round() as u16).unwrap_or(0);
                return (name, vram, def_pwr, max_pwr);
            }
        }
    }

    ("集成显卡/核显".to_string(), 4096, 0, 0)
}

static CACHED_GPU_NAME: parking_lot::RwLock<Option<String>> = parking_lot::RwLock::new(None);

pub fn get_gpu_name_cached() -> String {
    if let Some(ref name) = *CACHED_GPU_NAME.read() {
        return name.clone();
    }
    #[cfg(windows)]
    if let Some((name, _)) = detect_gpu_from_registry() {
        *CACHED_GPU_NAME.write() = Some(name.clone());
        return name;
    }
    "集成显卡/核显".to_string()
}

impl GpuBridge {
    pub fn new() -> Self {
        let inner = Arc::new(GpuTelemetryInner {
            temp: AtomicU32::new(35),
            usage: AtomicU32::new(0),
            core_freq: AtomicU32::new(210),
            mem_freq: AtomicU32::new(405),
            vram_mb: AtomicU64::new(8192),
            vram_used_mb: AtomicU64::new(1024),
            core_offset: AtomicI32::new(0),
            mem_offset: AtomicI32::new(0),
            default_power_limit_w: AtomicU32::new(0),
            max_power_limit_w: AtomicU32::new(0),
            power_dw: AtomicU32::new(0),
        });

        let initial_name = {
            #[cfg(windows)]
            {
                detect_gpu_from_registry().map(|(name, _)| name).unwrap_or_else(|| "高性能独立显卡".to_string())
            }
            #[cfg(not(windows))]
            {
                "高性能独立显卡".to_string()
            }
        };

        Self {
            inner,
            gpu_name: Arc::new(parking_lot::Mutex::new(initial_name)),
            is_mock: false,
        }
    }

    pub fn auto_load() -> Self {
        let bridge = Self::new();
        bridge.start_background_poller();
        bridge
    }

    fn start_background_poller(&self) {
        let inner = Arc::clone(&self.inner);
        let gpu_name = Arc::clone(&self.gpu_name);

        thread::spawn(move || {
            // Asynchronously detect GPU name, VRAM and VBIOS power limits on background thread
            let (detected_name, detected_vram, def_pwr, max_pwr) = detect_initial_gpu();
            *gpu_name.lock() = detected_name;
            inner.vram_mb.store(detected_vram, Ordering::SeqCst);
            if def_pwr > 0 {
                inner.default_power_limit_w.store(def_pwr as u32, Ordering::SeqCst);
            }
            if max_pwr > 0 {
                inner.max_power_limit_w.store(max_pwr as u32, Ordering::SeqCst);
            }

            // Non-NVIDIA / pure iGPU laptop: completely bypass background poller loop!
            // 0% CPU overhead, 0 background processes, prevents D3Cold wakeups
            if !is_nvidia_dgpu_available() {
                crate::log_info!("GPU_BRIDGE", "Non-NVIDIA or pure iGPU laptop: background poller terminated safely");
                return;
            }

            // Preferred path: NVML in-process sampling (no child process per tick).
            // Falls back to the legacy nvidia-smi subprocess loop only if NVML cannot
            // be loaded/initialised (missing or too-old driver).
            match crate::driver::nvml::NvmlContext::open() {
                Ok(ctx) => {
                    crate::log_info!(
                        "GPU_BRIDGE",
                        "GPU telemetry: NVML in-process sampling engaged (0 child processes per tick)"
                    );
                    run_nvml_poller(inner, gpu_name, ctx);
                }
                Err(e) => {
                    crate::log_warn!(
                        "GPU_BRIDGE",
                        "NVML unavailable ({}); falling back to nvidia-smi subprocess polling",
                        e
                    );
                    run_legacy_smi_poller(inner, gpu_name);
                }
            }
        });
    }

    pub fn get_gpu_count(&self) -> u32 {
        1
    }

    pub fn get_name(&self, _index: u32) -> String {
        self.gpu_name.lock().clone()
    }

    pub fn get_temp(&self, _index: u32) -> u32 {
        self.inner.temp.load(Ordering::SeqCst)
    }

    pub fn get_usage(&self, _index: u32) -> u32 {
        self.inner.usage.load(Ordering::SeqCst)
    }

    pub fn get_core_freq(&self, _index: u32) -> u32 {
        self.inner.core_freq.load(Ordering::SeqCst)
    }

    pub fn get_mem_freq(&self, _index: u32) -> u32 {
        self.inner.mem_freq.load(Ordering::SeqCst)
    }

    pub fn get_vram_mb(&self, _index: u32) -> u64 {
        self.inner.vram_mb.load(Ordering::SeqCst)
    }

    pub fn get_vram_used_mb(&self, _index: u32) -> u64 {
        self.inner.vram_used_mb.load(Ordering::SeqCst)
    }

    pub fn set_core_offset(&self, offset_mhz: i32) -> bool {
        self.inner.core_offset.store(offset_mhz, Ordering::SeqCst);
        let mem = self.inner.mem_offset.load(Ordering::SeqCst);
        if let Err(e) = crate::driver::nvapi::set_gpu_clock_offsets(offset_mhz, mem) {
            crate::log_warn!("GPU_BRIDGE", "Failed to apply core offset {}MHz to NVAPI: {}", offset_mhz, e);
            false
        } else {
            true
        }
    }

    pub fn set_mem_offset(&self, offset_mhz: i32) -> bool {
        self.inner.mem_offset.store(offset_mhz, Ordering::SeqCst);
        let core = self.inner.core_offset.load(Ordering::SeqCst);
        if let Err(e) = crate::driver::nvapi::set_gpu_clock_offsets(core, offset_mhz) {
            crate::log_warn!("GPU_BRIDGE", "Failed to apply mem offset {}MHz to NVAPI: {}", offset_mhz, e);
            false
        } else {
            true
        }
    }

    pub fn set_clock_offsets(&self, core_offset_mhz: i32, mem_offset_mhz: i32) -> bool {
        self.inner.core_offset.store(core_offset_mhz, Ordering::SeqCst);
        self.inner.mem_offset.store(mem_offset_mhz, Ordering::SeqCst);
        if let Err(e) = crate::driver::nvapi::set_gpu_clock_offsets(core_offset_mhz, mem_offset_mhz) {
            crate::log_warn!("GPU_BRIDGE", "Failed to apply clock offsets (Core={}, Mem={}) to NVAPI: {}", core_offset_mhz, mem_offset_mhz, e);
            false
        } else {
            true
        }
    }

    pub fn get_core_offset(&self) -> i32 {
        self.inner.core_offset.load(Ordering::SeqCst)
    }

    pub fn get_mem_offset(&self) -> i32 {
        self.inner.mem_offset.load(Ordering::SeqCst)
    }

    pub fn set_fan_policy(&self, _policy: u32) -> bool {
        true
    }

    pub fn get_power_limits(&self) -> (u16, u16) {
        let def = self.inner.default_power_limit_w.load(Ordering::SeqCst) as u16;
        let max = self.inner.max_power_limit_w.load(Ordering::SeqCst) as u16;
        (def, max)
    }

    pub fn get_power_w(&self, _index: u32) -> f32 {
        let dw = self.inner.power_dw.load(Ordering::SeqCst);
        dw as f32 / 10.0
    }
}

// ---------------------------------------------------------------------------
// GPU telemetry pollers
// ---------------------------------------------------------------------------

/// 活跃态采样间隔（毫秒）。与原实现保持一致，不改变 UI 刷新手感。
const NVML_ACTIVE_INTERVAL_MS: u64 = 1_500;
/// GPU 空闲（P8 且利用率为 0）时的采样间隔。
///
/// NVML 单次采样约 8μs，拉长间隔的目的不是省 CPU，而是**减少对驱动的访问频次**，
/// 避免长期维持设备活跃引用而影响 RTD3 / D3Cold 进入。
const NVML_IDLE_INTERVAL_MS: u64 = 5_000;
/// 设备不可达（被固件切电 / D3Cold）后的重试间隔。
const NVML_UNREACHABLE_RETRY_MS: u64 = 30_000;
/// 连续多少次空闲采样后切换到低频模式。
const IDLE_STREAK_THRESHOLD: u32 = 2;

/// NVML 直读轮询：进程内采样，每次约 8μs，不创建任何子进程。
fn run_nvml_poller(
    inner: Arc<GpuTelemetryInner>,
    gpu_name: Arc<Mutex<String>>,
    first_ctx: crate::driver::nvml::NvmlContext,
) {
    use crate::driver::nvml::{NvmlContext, SampleError};

    let mut ctx: Option<NvmlContext> = Some(first_ctx);
    let mut idle_streak: u32 = 0;
    let mut unreachable_logged = false;

    loop {
        let sleep_ms = if ctx.is_some() {
            if idle_streak >= IDLE_STREAK_THRESHOLD {
                NVML_IDLE_INTERVAL_MS
            } else {
                NVML_ACTIVE_INTERVAL_MS
            }
        } else {
            NVML_UNREACHABLE_RETRY_MS
        };
        thread::sleep(Duration::from_millis(sleep_ms));

        // 设备不可达时上下文已被释放，这里按低频节奏尝试重建
        if ctx.is_none() {
            match NvmlContext::open() {
                Ok(c) => {
                    if let Some(n) = c.name() {
                        let mut guard = gpu_name.lock();
                        if *guard != n {
                            *guard = n;
                        }
                    }
                    crate::log_info!("GPU_BRIDGE", "NVML context rebuilt after device became reachable");
                    ctx = Some(c);
                    idle_streak = 0;
                    unreachable_logged = false;
                }
                Err(e) => {
                    if !unreachable_logged {
                        crate::log_warn!(
                            "GPU_BRIDGE",
                            "NVML still unavailable ({}); next retry in {}ms",
                            e,
                            NVML_UNREACHABLE_RETRY_MS
                        );
                        unreachable_logged = true;
                    }
                    continue;
                }
            }
        }

        // 先取出 owned 结果，避免后续 `ctx = None` 与不可变借用冲突
        let sampled = match ctx.as_ref() {
            Some(c) => c.sample(),
            None => continue,
        };

        match sampled {
            Ok(s) => {
                inner.core_freq.store(s.core_clock_mhz, Ordering::SeqCst);
                inner.mem_freq.store(s.mem_clock_mhz, Ordering::SeqCst);
                inner.temp.store(s.temp_c, Ordering::SeqCst);
                inner.usage.store(s.usage_pct, Ordering::SeqCst);
                if s.vram_total_mb > 0 {
                    inner.vram_mb.store(s.vram_total_mb, Ordering::SeqCst);
                }
                inner.vram_used_mb.store(s.vram_used_mb, Ordering::SeqCst);
                // 功耗为 0 通常表示该型号不提供功耗传感器，保留上一次读数而不是清零
                if s.power_w > 0.0 {
                    inner
                        .power_dw
                        .store((s.power_w * 10.0).round() as u32, Ordering::SeqCst);
                }

                idle_streak = if s.is_idle() {
                    idle_streak.saturating_add(1)
                } else {
                    0
                };
            }
            Err(SampleError::Unreachable(code)) => {
                if !unreachable_logged {
                    crate::log_info!(
                        "GPU_BRIDGE",
                        "NVML device unreachable (code {}): releasing handle to allow RTD3, retry in {}ms",
                        code,
                        NVML_UNREACHABLE_RETRY_MS
                    );
                    unreachable_logged = true;
                }
                // 主动释放：Drop 会调用 nvmlShutdown，解除驱动侧的活动引用
                ctx = None;
                idle_streak = 0;
            }
            Err(SampleError::Transient(code)) => {
                // 瞬时错误：保留上一次读数，下个周期再试，不重置空闲计数
                crate::log_debug!(
                    "GPU_BRIDGE",
                    "NVML transient sampling error (code {}), keeping last values",
                    code
                );
            }
        }
    }
}

/// 兼容性回退：`nvidia-smi` 子进程轮询（NVML 不可用时使用）。
///
/// 每次调用都会创建一个子进程（实测 450~550ms），因此仅在 NVML 加载失败时启用。
fn run_legacy_smi_poller(inner: Arc<GpuTelemetryInner>, gpu_name: Arc<Mutex<String>>) {
    let mut consecutive_failures: u32 = 0;
    loop {
        // 自适应休眠避让：若连续查询失败或独显休眠，自适应拉长休眠时间至 4000ms，避免冲击 D3Cold 驱动
        let sleep_ms = if consecutive_failures > 1 { 4000 } else { 1500 };
        thread::sleep(Duration::from_millis(sleep_ms));

        // Execute nvidia-smi with CREATE_NO_WINDOW for real-time telemetry
        let output = Command::new("nvidia-smi")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "--query-gpu=name,clocks.current.graphics,clocks.current.memory,temperature.gpu,utilization.gpu,memory.total,memory.used,power.draw",
                "--format=csv,noheader,nounits",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                consecutive_failures = 0;
                let text = String::from_utf8_lossy(&out.stdout);
                let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
                if parts.len() >= 7 {
                    let name = parts[0];
                    if !name.is_empty() {
                        let mut n = gpu_name.lock();
                        if *n != name {
                            *n = name.to_string();
                        }
                    }
                    if let Ok(c) = parts[1].parse::<u32>() {
                        inner.core_freq.store(c, Ordering::SeqCst);
                    }
                    if let Ok(m) = parts[2].parse::<u32>() {
                        inner.mem_freq.store(m, Ordering::SeqCst);
                    }
                    if let Ok(t) = parts[3].parse::<u32>() {
                        inner.temp.store(t, Ordering::SeqCst);
                    }
                    if let Ok(u) = parts[4].parse::<u32>() {
                        inner.usage.store(u, Ordering::SeqCst);
                    }
                    if let Ok(v_tot) = parts[5].parse::<u64>() {
                        inner.vram_mb.store(v_tot, Ordering::SeqCst);
                    }
                    if let Ok(v_use) = parts[6].parse::<u64>() {
                        inner.vram_used_mb.store(v_use, Ordering::SeqCst);
                    }
                    if parts.len() >= 8 {
                        if let Ok(p) = parts[7].parse::<f32>() {
                            let dw = (p * 10.0).round() as u32;
                            inner.power_dw.store(dw, Ordering::SeqCst);
                        }
                    }
                }
            } else {
                consecutive_failures = consecutive_failures.saturating_add(1);
            }
        } else {
            consecutive_failures = consecutive_failures.saturating_add(1);
        }
    }
}

static NVIDIA_POWER_LIMITS_CACHE: RwLock<Option<(u16, u16)>> = RwLock::new(None);
static NVIDIA_PROBE_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Locate nvidia-smi executable across system PATH and standard NVIDIA directories
fn find_nvidia_smi_command() -> Command {
    let standard_paths = [
        r"C:\Windows\System32\nvidia-smi.exe",
        r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
    ];
    for p in standard_paths {
        if std::path::Path::new(p).exists() {
            let mut cmd = Command::new(p);
            cmd.creation_flags(CREATE_NO_WINDOW);
            return cmd;
        }
    }
    let mut cmd = Command::new("nvidia-smi");
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Internal direct execution of nvidia-smi power query with raw process execution.
fn execute_nvidia_smi_power_query_raw() -> Option<(u16, u16)> {
    if !is_nvidia_dgpu_available() {
        return None;
    }
    let mut cmd = find_nvidia_smi_command();
    let output = cmd
        .args([
            "--query-gpu=power.default_limit,power.max_limit",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?;
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() >= 2 {
        let def_w = parts[0].parse::<f32>().ok()?.round() as u16;
        let max_w = parts[1].parse::<f32>().ok()?.round() as u16;
        if def_w > 0 && max_w >= def_w {
            crate::log_info!(
                "GPU_BRIDGE",
                "VBIOS Power Limits verified: Base TGP={}W, Max TGP={}W, DB Capacity=+{}W",
                def_w, max_w, max_w - def_w
            );
            return Some((def_w, max_w));
        }
    }
    None
}

/// Execute nvidia-smi power query with a strict 2500ms timeout barrier to prevent D3Cold wake deadlocks.
pub fn execute_nvidia_smi_power_query() -> Option<(u16, u16)> {
    if !is_nvidia_dgpu_available() {
        return None;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let res = execute_nvidia_smi_power_query_raw();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(Duration::from_millis(2500)) {
        Ok(res) => res,
        Err(_) => {
            crate::log_warn!(
                "GPU_BRIDGE",
                "nvidia-smi VBIOS limits query timed out (>2500ms)! GPU may be in D3Cold sleep. Bypassing to protect UI message loop."
            );
            None
        }
    }
}

/// Standalone non-blocking fast query for NVIDIA GPU VBIOS power limits (Base TGP & Max TGP).
/// Returns cached value immediately (0ms). Spawns background asynchronous probe if not yet cached,
/// guaranteeing that IPC callers (e.g. get_hardware_status) NEVER freeze or block WebView2.
pub fn query_nvidia_power_limits_static() -> Option<(u16, u16)> {
    if !is_nvidia_dgpu_available() {
        return None;
    }
    if let Some(cached) = *NVIDIA_POWER_LIMITS_CACHE.read() {
        return Some(cached);
    }
    // Spawn background probe if not already running
    spawn_async_nvidia_power_probe::<fn((u16, u16))>(None);
    None
}

/// Get currently cached NVIDIA power limits without executing any child process (0ms).
pub fn get_cached_nvidia_power_limits() -> Option<(u16, u16)> {
    *NVIDIA_POWER_LIMITS_CACHE.read()
}

/// Asynchronously probes NVIDIA VBIOS power limits in a background thread to prevent any UI/boot block.
/// Once ready, executes the optional callback on completion.
pub fn spawn_async_nvidia_power_probe<F>(callback: Option<F>)
where
    F: FnOnce((u16, u16)) + Send + 'static,
{
    if !is_nvidia_dgpu_available() {
        return;
    }
    if NVIDIA_PROBE_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        if let Some(limits) = execute_nvidia_smi_power_query() {
            *NVIDIA_POWER_LIMITS_CACHE.write() = Some(limits);
            let presets = crate::system::power_mode::refresh_factory_power_presets(true);
            let caps = probe_gpu_tuning_capabilities();
            if let Some(app) = crate::system::webview_orchestrator::WebviewOrchestrator::global().app() {
                use tauri::Emitter;
                let _ = app.emit("gpu-power-limits-ready", &caps);
                let _ = app.emit("factory-power-presets-ready", &presets);
            }
            if let Some(cb) = callback {
                cb(limits);
            }
        }
        NVIDIA_PROBE_RUNNING.store(false, Ordering::SeqCst);
    });
}

pub type SharedGpuBridge = Arc<Mutex<GpuBridge>>;

static GLOBAL_GPU: std::sync::OnceLock<SharedGpuBridge> = std::sync::OnceLock::new();

pub fn get_global_gpu() -> SharedGpuBridge {
    GLOBAL_GPU.get_or_init(|| Arc::new(Mutex::new(GpuBridge::auto_load()))).clone()
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GpuTuningCapabilities {
    pub gpu_generation: u32,
    pub gpu_name: String,
    pub supports_gpu_tuning: bool,
    pub supports_db: bool,
    pub supports_ctgp: bool,
    pub is_full_power: bool,
    pub ctgp_floor_watts: u32,
    pub base_tgp_max_watts: u32,
    pub max_db_watts: u32,
    pub max_total_tgp: u32,
    #[serde(default)]
    pub is_pending: bool,
}

/// Helper: Autonomous GPU generation extraction purely for display metadata.
/// Under Hardware Truth principles, generation strings NEVER govern or throttle tuning capabilities.
pub fn extract_gpu_generation(gpu_name: &str) -> u32 {
    let name_upper = gpu_name.to_uppercase();
    if name_upper.contains("RTX 60") || name_upper.contains("RTX60") {
        60
    } else if name_upper.contains("RTX 50") || name_upper.contains("RTX50") {
        50
    } else if name_upper.contains("RTX 40") || name_upper.contains("RTX40") {
        40
    } else if name_upper.contains("RTX 30") || name_upper.contains("RTX30") {
        30
    } else if name_upper.contains("RTX 20") || name_upper.contains("RTX20") {
        20
    } else if name_upper.contains("GTX 16") || name_upper.contains("GTX16") {
        16
    } else if name_upper.contains("GTX 10") || name_upper.contains("GTX10") {
        10
    } else {
        0
    }
}

pub fn probe_gpu_tuning_capabilities() -> GpuTuningCapabilities {
    let gpu = get_global_gpu();
    let mut gpu_name = gpu.lock().get_name(0);
    if gpu_name == "高性能独立显卡" || gpu_name == "集成显卡/核显" || gpu_name.is_empty() {
        #[cfg(windows)]
        if let Some((reg_name, _)) = detect_gpu_from_registry() {
            gpu_name = reg_name;
        }
    }
    let gen = extract_gpu_generation(&gpu_name);
    let is_dgpu = is_nvidia_dgpu_available();

    // 1. Hardware Truth: Verify physical discrete GPU silicon presence first
    if !is_dgpu {
        return GpuTuningCapabilities {
            gpu_generation: gen,
            gpu_name,
            supports_gpu_tuning: false,
            supports_db: false,
            supports_ctgp: false,
            is_full_power: false,
            ctgp_floor_watts: 0,
            base_tgp_max_watts: 0,
            max_db_watts: 0,
            max_total_tgp: 0,
            is_pending: false,
        };
    }

    // 2. Chassis & Generation Gatekeeper:
    // Pascal/Turing GPUs (GTX 10/16-series, RTX 20-series) or Legacy Tongfang chassis
    // physically lack Dynamic Boost / cTGP EC protocol support!
    // In legacy chassis, EC registers 1859..1863 are MyFan2 fan speed PWM tables.
    // Writing DB commands to them corrupts the fan curve and causes violent RPM surging!
    if crate::hal::platform_probe::is_legacy_pascal_turing_tongfang() || (gen <= 20 && gen > 0) {
        crate::log_info!(
            "GPU_TUNING",
            "Legacy architecture detected (chassis_legacy={}, gpu_gen={}): DB/cTGP EC tuning permanently bypassed to protect fan table.",
            crate::hal::platform_probe::is_legacy_pascal_turing_tongfang(),
            gen
        );
        return GpuTuningCapabilities {
            gpu_generation: gen,
            gpu_name,
            supports_gpu_tuning: false,
            supports_db: false,
            supports_ctgp: false,
            is_full_power: false,
            ctgp_floor_watts: 0,
            base_tgp_max_watts: 0,
            max_db_watts: 0,
            max_total_tgp: 0,
            is_pending: false,
        };
    }

    let (def_w, max_w) = query_nvidia_power_limits_static().unwrap_or((0, 0));

    // Authoritative Hardware Physical DB Capacity:
    // When total capacity max_w <= 115W (standard 115W tier, e.g. RTX 5060/4060 115W), hardware physical envelope is strictly 100W Base + 15W DB.
    // When total capacity max_w > 115W (e.g. 140W full-power tier), hardware physical envelope is 115W Base + 25W DB.
    // NEVER read dynamic runtime setpoint register ADDR_DB_WATTS1 (which holds the active user/system slider value, e.g. 5W when DB is OFF)!
    let hardware_db_cap: u32 = if max_w <= 115 && max_w > 0 {
        15
    } else {
        25
    };

    if def_w > 0 && max_w > 0 {
        // Universal Zero-Hardcoding Mathematical Algorithm:
        let delta = max_w.saturating_sub(def_w) as u32;

        if delta > 0 && delta <= 25 {
            // Standard discrete envelope where default_limit == Base TGP (e.g. RTX 4080/4090 or fixed-power cards)
            let full_base = def_w as u32;
            let full_total = max_w as u32;
            let full_db = delta;
            GpuTuningCapabilities {
                gpu_generation: gen,
                gpu_name,
                supports_gpu_tuning: true,
                supports_db: true,
                supports_ctgp: false,
                is_full_power: true,
                ctgp_floor_watts: full_base,
                base_tgp_max_watts: full_base,
                max_db_watts: full_db,
                max_total_tgp: full_total,
                is_pending: false,
            }
        } else if delta > 25 {
            // Wide cTGP envelope: default_limit is the down-regulation floor! (e.g. RTX 4060/4070/5060/5070)
            // True Base TGP = Max Limit - Physical DB Capacity (e.g. 115W - 25W = 90W Base, or 140W - 25W = 115W Base)
            let true_max_db = hardware_db_cap.min(delta);
            let true_base_tgp = (max_w as u32).saturating_sub(true_max_db);
            let floor_w = def_w as u32;
            GpuTuningCapabilities {
                gpu_generation: gen,
                gpu_name,
                supports_gpu_tuning: true,
                supports_db: true,
                supports_ctgp: true,
                is_full_power: false,
                ctgp_floor_watts: floor_w,
                base_tgp_max_watts: true_base_tgp,
                max_db_watts: true_max_db,
                max_total_tgp: max_w as u32,
                is_pending: false,
            }
        } else {
            // Special / fallback boundary where delta == 0
            let true_max_db = hardware_db_cap;
            let true_base_tgp = (max_w as u32).saturating_sub(true_max_db);
            GpuTuningCapabilities {
                gpu_generation: gen,
                gpu_name,
                supports_gpu_tuning: true,
                supports_db: true,
                supports_ctgp: false,
                is_full_power: true,
                ctgp_floor_watts: true_base_tgp,
                base_tgp_max_watts: true_base_tgp,
                max_db_watts: true_max_db,
                max_total_tgp: max_w as u32,
                is_pending: false,
            }
        }
    } else {
        // Fallback when VBIOS power limits query returned (0, 0) (e.g. GPU in D3Cold sleep / background query pending)
        // Under Hardware Truth: We NEVER invent phantom 115W or 25W numbers!
        // Return genuine pending placeholder state (0W, is_pending: true) with supports_gpu_tuning: true.
        GpuTuningCapabilities {
            gpu_generation: gen,
            gpu_name,
            supports_gpu_tuning: true,
            supports_db: true,
            supports_ctgp: true,
            is_full_power: false,
            ctgp_floor_watts: 0,
            base_tgp_max_watts: 0,
            max_db_watts: 0,
            max_total_tgp: 0,
            is_pending: true,
        }
    }
}

