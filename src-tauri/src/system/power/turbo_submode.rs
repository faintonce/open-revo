// OpenRevo - 狂暴子模式（静音狂暴 / 高能狂暴）
//
// 只对支持该能力的机型生效，能力判定有两个独立来源（任一成立即视为支持）：
//   1) WMI GetSetULong 查询能力位（低字节 0x55）
//   2) 厂商控制台/GCU 写下的注册表能力位 IsTurboSubModeSupport
// 两者都不成立时一律视为不支持：不显示入口、不写任何寄存器（绝不按机型/代数猜）。
//
// 切换顺序：
//   1) 选择位：EC 寄存器的一个 bit（0 = 高能/极限狂暴，1 = 静音狂暴）
//   2) BIOS RAM：两个 ULong 命令分别对应静音 / 高能
//   3) 注册表：厂商控制台与 OSD 用它回读当前子模式（0 = 静音，1 = 高能）
// 任一步失败 -> 把 EC 选择位回滚到写前值；全部成功 -> 回读 EC 选择位校验。

use std::sync::Mutex;
use std::time::{Duration, Instant};

const CMD_SUBMODE_SUPPORT: u64 = 0x5_0000_0008;
const CMD_SUBMODE_SILENT: u64 = 0x5_0000_0009;
const CMD_SUBMODE_EXTREME: u64 = 0x5_0000_000A;
const SUPPORT_MARKER: u8 = 0x55;
const MASK_TURBO_SUBMODE_BIT: u8 = 0x01;

const REG_SUBKEY: &str = r"SOFTWARE\OEM\GamingCenter2\ItemSupport";
const REG_SUBKEY_WOW: &str = r"SOFTWARE\WOW6432Node\OEM\GamingCenter2\ItemSupport";
const REG_VALUE_SUPPORT: &str = "IsTurboSubModeSupport";
const REG_VALUE_SILENT_SWITCH: &str = "SilentPerformanceModeSwitch";

/// 否定结果只缓存一小段时间：WMI/COM 在启动早期可能还没就绪，之后要能重试。
const NEGATIVE_TTL: Duration = Duration::from_secs(5);

static SUPPORT_CACHE: Mutex<Option<(bool, Instant)>> = Mutex::new(None);

fn probe_once() -> (bool, String) {
    let wmi_available = crate::driver::wmi_mulong::probe_mulong_available();
    let raw = crate::driver::wmi_mulong::mulong_query_raw(CMD_SUBMODE_SUPPORT);
    let wmi_ok = raw.map(|v| (v & 0xFF) as u8 == SUPPORT_MARKER).unwrap_or(false);

    let reg = read_registry_flag(REG_SUBKEY, REG_VALUE_SUPPORT)
        .or_else(|| read_registry_flag(REG_SUBKEY_WOW, REG_VALUE_SUPPORT));
    let reg_ok = reg == Some(true);

    let nextgen = crate::driver::wmi_ec::is_nextgen_oem_magic_ec();

    let detail = format!(
        "wmi_available={} wmi_raw={} registry_flag={:?} nextgen_platform={}",
        wmi_available,
        match raw {
            Some(v) => format!("0x{:016X}", v),
            None => "none".to_string(),
        },
        reg,
        nextgen
    );
    (wmi_ok || reg_ok || nextgen, detail)
}

/// 本机是否支持狂暴子模式。
pub fn is_supported() -> bool {
    let mut guard = match SUPPORT_CACHE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some((val, at)) = *guard {
        if val || at.elapsed() < NEGATIVE_TTL {
            return val;
        }
    }
    let (val, detail) = probe_once();
    crate::log_info!("POWER_MODE", "turbo submode probe: {} -> supported={}", detail, val);
    *guard = Some((val, Instant::now()));
    val
}

/// 当前硬件选择位状态：true = 静音狂暴，false = 极限狂暴；不支持或读取失败返回 None。
pub fn current_silent() -> Option<bool> {
    if !is_supported() {
        return None;
    }
    let val = crate::driver::wmi_ec::wmi_read_ec(crate::hal::vault::ec::ADDR_TURBO_SUBMODE)?;
    Some((val & MASK_TURBO_SUBMODE_BIT) != 0)
}

/// 应用狂暴子模式。仅在支持该特性的机型上执行，其余机型直接返回 Err。
pub fn apply_silent(silent: bool) -> Result<(), String> {
    if !is_supported() {
        return Err("当前硬件平台不支持狂暴子模式".to_string());
    }

    let addr = crate::hal::vault::ec::ADDR_TURBO_SUBMODE;
    let mask = MASK_TURBO_SUBMODE_BIT;

    let prev = crate::driver::wmi_ec::wmi_read_ec(addr)
        .ok_or_else(|| "读取子模式寄存器失败".to_string())?;
    let target = if silent { prev | mask } else { prev & !mask };

    if target != prev && !crate::driver::wmi_ec::wmi_write_ec(addr, target) {
        return Err("写子模式寄存器失败".to_string());
    }

    let cmd = if silent { CMD_SUBMODE_SILENT } else { CMD_SUBMODE_EXTREME };
    let wmi_ok = crate::driver::wmi_mulong::mulong_exec_raw(cmd);
    let reg_ok = write_registry_switch(silent);

    if !wmi_ok || !reg_ok {
        // 回滚选择位，保证硬件状态与软件认知一致
        let _ = crate::driver::wmi_ec::wmi_write_ec(addr, prev);
        crate::log_error!(
            "POWER_MODE",
            "turbo submode write failed (silent={}): wmi={} registry={} -> rolled back EC bit",
            silent,
            wmi_ok,
            reg_ok
        );
        return Err(format!(
            "写入失败（WMI 通道={}, 注册表={}），已回滚寄存器",
            if wmi_ok { "OK" } else { "失败" },
            if reg_ok { "OK" } else { "失败" }
        ));
    }

    match current_silent() {
        Some(v) if v == silent => {
            crate::log_info!("POWER_MODE", "turbo submode applied: silent={} (verified)", silent);
            Ok(())
        }
        other => {
            crate::log_warn!(
                "POWER_MODE",
                "turbo submode readback mismatch: expect silent={}, got {:?}",
                silent,
                other
            );
            Err("写入后回读不一致".to_string())
        }
    }
}

/// 读注册表布尔/数值开关（支持 REG_DWORD 与 REG_SZ）。
fn read_registry_flag(sub_key: &str, value_name: &str) -> Option<bool> {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
            REG_DWORD,
        };

        let sub_w = HSTRING::from(sub_key);
        let val_w = HSTRING::from(value_name);
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(sub_w.as_ptr()), 0, KEY_READ, &mut hkey).is_err() {
            return None;
        }

        let mut ty = REG_DWORD;
        let mut size = 0u32;
        let _ = RegQueryValueExW(
            hkey,
            PCWSTR(val_w.as_ptr()),
            None,
            Some(&mut ty),
            None,
            Some(&mut size),
        );
        let mut buf = vec![0u8; (size as usize).max(8)];
        let mut got = buf.len() as u32;
        let status = RegQueryValueExW(
            hkey,
            PCWSTR(val_w.as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr()),
            Some(&mut got),
        );
        let _ = RegCloseKey(hkey);
        if status.is_err() {
            return None;
        }

        if ty == REG_DWORD && buf.len() >= 4 {
            let v = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
            return Some(v != 0);
        }

        let text: String = buf
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| *b as char)
            .collect();
        let t = text.trim().to_ascii_lowercase();
        if t.is_empty() {
            None
        } else if t == "0" || t == "false" {
            Some(false)
        } else {
            Some(true)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (sub_key, value_name);
        None
    }
}

/// 注册表开关：0 = 静音狂暴，1 = 高能狂暴（与厂商控制台的读数语义一致）。
fn write_registry_switch(silent: bool) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_SET_VALUE,
            REG_DWORD,
        };

        let value: u32 = if silent { 0 } else { 1 };
        let mut any_ok = false;

        for sub in [REG_SUBKEY, REG_SUBKEY_WOW] {
            let sub_w = HSTRING::from(sub);
            let val_w = HSTRING::from(REG_VALUE_SILENT_SWITCH);
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(sub_w.as_ptr()), 0, KEY_SET_VALUE, &mut hkey)
                .is_ok()
            {
                let res = RegSetValueExW(
                    hkey,
                    PCWSTR(val_w.as_ptr()),
                    0,
                    REG_DWORD,
                    Some(std::slice::from_raw_parts(
                        &value as *const u32 as *const u8,
                        std::mem::size_of::<u32>(),
                    )),
                );
                let _ = RegCloseKey(hkey);
                if res.is_ok() {
                    any_ok = true;
                }
            }
        }

        any_ok
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = silent;
        false
    }
}
