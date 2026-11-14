use windows::core::{PCWSTR, PWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
};

#[repr(C)]
struct SYSTEM_POWER_STATUS {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

extern "system" {
    fn GetSystemPowerStatus(lpSystemPowerStatus: *mut SYSTEM_POWER_STATUS) -> i32;
}

pub fn get_live_power_status() -> (u8, bool, bool) {
    let mut status = SYSTEM_POWER_STATUS {
        ac_line_status: 1,
        battery_flag: 0,
        battery_life_percent: 100,
        system_status_flag: 0,
        battery_life_time: 0,
        battery_full_life_time: 0,
    };
    let success = unsafe { GetSystemPowerStatus(&mut status) };
    if success != 0 {
        let percent = if status.battery_life_percent <= 100 {
            status.battery_life_percent
        } else {
            100
        };
        let is_ac = status.ac_line_status == 1;
        let is_charging = (status.battery_flag & 0x08) != 0;
        (percent, is_ac, is_charging)
    } else {
        (100, true, false)
    }
}

static DGPU_CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub fn is_discrete_gpu_name(name: &str) -> bool {
    crate::driver::gpu_bridge::is_discrete_gpu_name(name)
}

pub fn check_dgpu_present() -> bool {
    *DGPU_CACHE.get_or_init(|| {
        if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
            if !manifest.capabilities.discrete_gpu {
                return false;
            }
        }

        #[cfg(windows)]
        unsafe {
            // 0. Quick OEM registry contract probe
            let oem_key_str: Vec<u16> = "SOFTWARE\\OEM\\GamingCenter2\\ItemSupport\0".encode_utf16().collect();
            let mut h_oem = HKEY::default();
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

            // 1. Dynamic Display Class registry enumeration
            let base_class_str: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\0".encode_utf16().collect();
            let mut h_base = HKEY::default();
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
                                let desc = String::from_utf16_lossy(&buf[..len]);
                                if is_discrete_gpu_name(&desc) {
                                    let _ = RegCloseKey(hkey);
                                    let _ = RegCloseKey(h_base);
                                    return true;
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

        let gpu_bridge = crate::driver::gpu_bridge::get_global_gpu();
        let bridge_name = gpu_bridge.lock().get_name(0);
        if is_discrete_gpu_name(&bridge_name) {
            return true;
        }

        false
    })
}

/// Determines whether the machine is a thin-and-light laptop or pure iGPU laptop
pub fn is_thin_and_light_or_pure_igpu() -> bool {
    // 1. Unified ultrabook / thin-and-light platform probe check
    if crate::hal::platform_probe::is_ultrabook() {
        return true;
    }

    // 2. Declarative model manifest form factor check
    if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        if manifest.identity.form_factor == "thin_light" || manifest.identity.form_factor == "ultrabook" {
            return true;
        }
        if manifest.identity.form_factor == "gaming" {
            return false;
        }
    }

    // 3. Physical discrete GPU presence check: pure iGPU without dGPU is always treated as thin-and-light power topology
    if !check_dgpu_present() {
        return true;
    }

    false
}

/// Pure decision function for power online status across laptop topologies.
pub fn decide_power_online_status(is_thin_and_light: bool, ec_pwr: Option<u8>, is_ac: bool) -> bool {
    if let Some(pwr) = ec_pwr {
        if (pwr & crate::driver::wmi_ec::POWER_STATUS_DC_JACK) != 0 {
            return true;
        }

        if (pwr & crate::driver::wmi_ec::POWER_STATUS_TYPE_C_MASK) != 0 {
            if is_thin_and_light {
                return true;
            } else {
                crate::log_info!("POWER", "Gating high power modes on gaming laptop: Type-C PD active (PowerStatus=0x{:02X})", pwr);
                return false;
            }
        }
    }

    if !is_ac {
        return false;
    }

    if is_thin_and_light {
        return true;
    }

    true
}

/// Check whether the laptop is powered by the full-wattage power supply.
pub fn is_dc_round_plugged_in() -> bool {
    let is_thin_and_light = is_thin_and_light_or_pure_igpu();
    let ec_pwr = if crate::hal::platform_probe::is_tongfang_family() {
        crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_POWER_STATUS)
    } else {
        None
    };
    let (_, is_ac, _) = get_live_power_status();
    decide_power_online_status(is_thin_and_light, ec_pwr, is_ac)
}
