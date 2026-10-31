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
const CMD_OC_ADJ_SUPPORT: u64 = 0x5_0000_0021;
const CMD_OC_ADJ_DEFAULT: u64 = 0x5_0000_0022;
const CMD_OC_ADJ_43: u64 = 0x5_0000_0023;
const CMD_OC_ADJ_46: u64 = 0x5_0000_0024;
const SUPPORT_MARKER: u8 = 0x55;

const REG_SUBKEY: &str = r"SOFTWARE\OEM\GamingCenter2\ItemSupport";
const REG_SUBKEY_WOW: &str = r"SOFTWARE\WOW6432Node\OEM\GamingCenter2\ItemSupport";
const REG_VALUE_SUPPORT: &str = "IsTurboSubModeSupport";
const REG_VALUE_SILENT_SWITCH: &str = "SilentPerformanceModeSwitch";

/// 否定结果只缓存一小段时间：WMI/COM 在启动早期可能还没就绪，之后要能重试。
const NEGATIVE_TTL: Duration = Duration::from_secs(5);

static SUPPORT_CACHE: Mutex<Option<(bool, Instant)>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// 预览模式：仅用于在**不支持**的机器上查看 UI 布局（绝不写硬件）。
// 启动进程时设置环境变量 OPENREVO_TURBO_PREVIEW=1 才会开启：
//   - 能力判定强制为 true（UI 显示入口）
//   - current_silent / apply_silent 改走内存里的模拟状态，不碰 EC / WMI / 注册表
// ---------------------------------------------------------------------------
static PREVIEW_MODE: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
    // 方式一：环境变量（直接启动时有效）
    if std::env::var("OPENREVO_TURBO_PREVIEW")
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
    {
        return true;
    }
    // 方式二：标志文件（提权重启会丢环境变量，这里用文件兜底）
    if let Ok(appdata) = std::env::var("APPDATA") {
        let flag = std::path::Path::new(&appdata)
            .join("OpenRevo")
            .join("turbo_preview.flag");
        if flag.exists() {
            return true;
        }
    }
    false
});
static PREVIEW_SILENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn is_preview() -> bool {
    *PREVIEW_MODE
}

fn probe_once() -> (bool, String) {
    let wmi_available = crate::driver::wmi_mulong::probe_mulong_available();
    let raw = crate::driver::wmi_mulong::mulong_query_raw(CMD_SUBMODE_SUPPORT);
    let wmi_ok = raw.map(|v| (v & 0xFF) as u8 == SUPPORT_MARKER).unwrap_or(false);

    let reg = read_registry_flag(REG_SUBKEY, REG_VALUE_SUPPORT)
        .or_else(|| read_registry_flag(REG_SUBKEY_WOW, REG_VALUE_SUPPORT));
    let reg_ok = reg == Some(true);

    // 第三条独立来源：平台本身的 Next-Gen（50 系）判定 —— 走 NVRAM 真值，
    // 由 wmi_ec::is_nextgen_oem_magic_ec() 复用宿主已有的驱动决策逻辑，不自己另写一套。
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
    if is_preview() {
        return true;
    }
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

/// 只读诊断：狂暴 OC 可调能力与出厂偏移值（不写任何东西）。
/// 返回值含义（低字节）：0x55 = 支持；其余值原样打印便于比对。
pub fn oc_adjust_diagnose() -> String {
    let read = |cmd: u64| -> String {
        match crate::driver::wmi_mulong::mulong_query_raw(cmd) {
            Some(v) => format!("0x{:016X}", v),
            None => "none".to_string(),
        }
    };
    format!(
        "oc_adj_support={} oc_adj_default={} silent_oc_default={} extreme_oc_default={}",
        read(CMD_OC_ADJ_SUPPORT),
        read(CMD_OC_ADJ_DEFAULT),
        read(CMD_OC_ADJ_43),
        read(CMD_OC_ADJ_46)
    )
}

/// 供启动日志使用的完整诊断串。
pub fn diagnose() -> String {
    let oc = oc_adjust_diagnose();
    if is_preview() {
        // 预览模式也报告一次真实探测结果，方便远程排查
        let (_val, detail) = probe_once();
        return format!(
            "supported=true (PREVIEW MODE: UI only, no hardware writes) real_detect: {} | {}",
            detail, oc
        );
    }
    let (val, detail) = probe_once();
    {
        let mut guard = match SUPPORT_CACHE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = Some((val, Instant::now()));
    }
    format!("supported={} {} | {}", val, detail, oc)
}

/// 读当前子模式：Some(true) = 静音狂暴，Some(false) = 高能狂暴。
pub fn current_silent() -> Option<bool> {
    if is_preview() {
        return Some(PREVIEW_SILENT.load(std::sync::atomic::Ordering::Relaxed));
    }
    if !is_supported() {
        return None;
    }
    let addr = crate::driver::wmi_ec::ADDR_TURBO_SUBMODE;
    let mask = crate::driver::wmi_ec::MASK_TURBO_SUBMODE_BIT as u8;
    crate::driver::wmi_ec::wmi_read_ec(addr).map(|v| (v & mask) != 0)
}

/// 切换子模式。成功返回 Ok(())，失败返回带原因的错误。
pub fn apply_silent(silent: bool) -> Result<(), String> {
    if is_preview() {
        // 预览模式：只改内存里的模拟状态，绝不写 EC / WMI / 注册表
        PREVIEW_SILENT.store(silent, std::sync::atomic::Ordering::Relaxed);
        crate::log_warn!(
            "POWER_MODE",
            "[preview] turbo submode UI only: silent={} (no hardware write)",
            silent
        );
        return Ok(());
    }
    if !is_supported() {
        return Err("本机不支持狂暴子模式（能力位未通过）".to_string());
    }

    let addr = crate::driver::wmi_ec::ADDR_TURBO_SUBMODE;
    let mask = crate::driver::wmi_ec::MASK_TURBO_SUBMODE_BIT as u8;

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
}

/// 注册表开关：0 = 静音狂暴，1 = 高能狂暴（与厂商控制台的读数语义一致）。
fn write_registry_switch(silent: bool) -> bool {
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
}
