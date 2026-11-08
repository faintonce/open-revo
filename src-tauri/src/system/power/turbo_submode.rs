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


/// 否定结果只缓存一小段时间：WMI/COM 在启动早期可能还没就绪，之后要能重试。
const NEGATIVE_TTL: Duration = Duration::from_secs(5);

static SUPPORT_CACHE: Mutex<Option<(bool, Instant)>> = Mutex::new(None);

/// Serializes the full read-modify-write sequence on the sub-mode register.
///
/// Every writer (UI command, mode-3 replay, physical Q-key replay) reaches the
/// register through `apply_silent`, so one process-wide guard is enough to keep
/// concurrent applies ordered and stop a failure rollback from restoring a value
/// that a later apply already replaced.
///
/// Lock order: this guard is always the innermost hardware lock (`POWER_MODE_LOCK`
/// -> `APPLY_LOCK`); `current_silent` only touches the short-lived support cache
/// mutex, so taking it after this guard cannot deadlock.
static APPLY_LOCK: Mutex<()> = Mutex::new(());

fn lock_apply() -> std::sync::MutexGuard<'static, ()> {
    match APPLY_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn probe_once() -> (bool, String) {
    let wmi_available = crate::driver::wmi_mulong::probe_mulong_available();
    let raw = crate::driver::wmi_mulong::mulong_query_raw(CMD_SUBMODE_SUPPORT);
    let wmi_ok = raw.map(|v| (v & 0xFF) as u8 == SUPPORT_MARKER).unwrap_or(false);

    let nextgen = crate::driver::wmi_ec::is_nextgen_oem_magic_ec();

    let detail = format!(
        "wmi_available={} wmi_raw={} nextgen_platform={}",
        wmi_available,
        match raw {
            Some(v) => format!("0x{:016X}", v),
            None => "none".to_string(),
        },
        nextgen
    );
    (wmi_ok || nextgen, detail)
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

    // Hold the process-wide guard for the whole read -> write -> WMI -> readback
    // -> rollback sequence. A rollback must never write back a `prev` that
    // another apply already superseded.
    let _apply = lock_apply();

    let prev = crate::driver::wmi_ec::wmi_read_ec(addr)
        .ok_or_else(|| "读取子模式寄存器失败".to_string())?;
    let target = if silent { prev | mask } else { prev & !mask };

    if target != prev && !crate::driver::wmi_ec::wmi_write_ec(addr, target) {
        return Err("写子模式寄存器失败".to_string());
    }

    let cmd = if silent { CMD_SUBMODE_SILENT } else { CMD_SUBMODE_EXTREME };
    let wmi_ok = crate::driver::wmi_mulong::mulong_exec_raw(cmd);

    if !wmi_ok {
        // 回滚选择位，保证硬件状态与软件认知一致
        let _ = crate::driver::wmi_ec::wmi_write_ec(addr, prev);
        crate::log_error!(
            "POWER_MODE",
            "turbo submode write failed (silent={}): WMI channel unavailable -> rolled back EC bit",
            silent
        );
        return Err("写入失败（WMI 通道不可用），已回滚寄存器".to_string());
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
