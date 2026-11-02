#[path = "../../../crates/openrevo-core/src/system/power/fan_table_io.rs"]
mod fan_table_io;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanTableData {
    pub cpu_upt: [u8; 16],
    pub cpu_downt: [u8; 16],
    pub cpu_duty: [u8; 16],
    pub gpu_upt: [u8; 16],
    pub gpu_downt: [u8; 16],
    pub gpu_duty: [u8; 16],
    pub max_duty: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ModelFanTables {
    #[serde(default)]
    pub office: Option<FanTableData>,
    #[serde(default)]
    pub balanced: Option<FanTableData>,
    #[serde(default)]
    pub turbo: Option<FanTableData>,
}

pub const FAN_TABLE_OFFICE: FanTableData = FanTableData {
    cpu_upt:   [0, 55, 57, 59, 61, 63, 65, 67, 69, 255, 255, 255, 255, 255, 255, 255],
    cpu_downt: [48, 58, 60, 62, 64, 66, 68, 70, 255, 255, 255, 255, 255, 255, 255, 255],
    cpu_duty:  [0, 60, 60, 60, 60, 70, 70, 90, 110, 110, 110, 110, 110, 110, 110, 110], // 55% max duty
    gpu_upt:   [0, 55, 55, 57, 57, 59, 62, 64, 66, 255, 255, 255, 255, 255, 255, 255],
    gpu_downt: [48, 56, 58, 58, 60, 63, 65, 67, 255, 255, 255, 255, 255, 255, 255, 255],
    gpu_duty:  [0, 60, 60, 60, 60, 70, 70, 90, 110, 110, 110, 110, 110, 110, 110, 110],
    max_duty:  110,
};

pub const FAN_TABLE_GAMING: FanTableData = FanTableData {
    cpu_upt:   [0, 55, 58, 61, 65, 69, 72, 75, 78, 81, 255, 255, 255, 255, 255, 255],
    cpu_downt: [48, 58, 61, 65, 69, 73, 76, 79, 82, 255, 255, 255, 255, 255, 255, 255],
    cpu_duty:  [0, 60, 70, 80, 100, 110, 120, 130, 150, 160, 160, 160, 160, 160, 160, 160], // 80% max duty
    gpu_upt:   [0, 55, 58, 61, 65, 69, 72, 75, 78, 81, 255, 255, 255, 255, 255, 255],
    gpu_downt: [48, 58, 61, 65, 69, 73, 76, 79, 82, 255, 255, 255, 255, 255, 255, 255],
    gpu_duty:  [0, 60, 70, 80, 100, 110, 120, 130, 150, 160, 160, 160, 160, 160, 160, 160],
    max_duty:  160,
};

pub const FAN_TABLE_TURBO: FanTableData = FanTableData {
    cpu_upt:   [0, 55, 58, 61, 65, 69, 72, 75, 78, 81, 84, 255, 255, 255, 255, 255],
    cpu_downt: [48, 58, 61, 65, 69, 73, 76, 79, 82, 85, 255, 255, 255, 255, 255, 255],
    cpu_duty:  [0, 60, 70, 80, 100, 110, 120, 130, 150, 160, 180, 180, 180, 180, 180, 180], // 90% max duty
    gpu_upt:   [0, 55, 58, 61, 65, 69, 72, 75, 78, 81, 84, 255, 255, 255, 255, 255],
    gpu_downt: [48, 58, 61, 65, 69, 73, 76, 79, 82, 85, 255, 255, 255, 255, 255, 255],
    gpu_duty:  [0, 60, 70, 80, 100, 110, 120, 130, 150, 160, 180, 180, 180, 180, 180, 180],
    max_duty:  180,
};

pub fn get_mode_fan_table(ui_mode: u8) -> FanTableData {
    let active_profile = crate::system::model_engine::get_active_profile();
    let fan_tables = active_profile.as_ref().and_then(|p| p.fan_tables.as_ref());
    match ui_mode {
        1 => fan_tables.and_then(|ft| ft.office.clone()).unwrap_or_else(|| FAN_TABLE_OFFICE.clone()),
        2 => fan_tables.and_then(|ft| ft.balanced.clone()).unwrap_or_else(|| FAN_TABLE_GAMING.clone()),
        3 => fan_tables.and_then(|ft| ft.turbo.clone()).unwrap_or_else(|| FAN_TABLE_TURBO.clone()),
        _ => fan_tables.and_then(|ft| ft.balanced.clone()).unwrap_or_else(|| FAN_TABLE_GAMING.clone()),
    }
}

pub fn apply_mode_fan_table(ui_mode: u8) -> Result<(), String> {
    let cfg = crate::system::config::load_config();
    if cfg.fan_boost || crate::driver::wmi_ec::is_fan_boost_active() {
        crate::log_info!("POWER_MODE", "apply_mode_fan_table: fan boost is active, skipping fan table write to preserve full fan speed");
        return Ok(());
    }
    // Ground Truth: Legacy dual-mode chassis (e.g. GK5 / 10-series / 20-series) strictly uses
    // internal EC ROM fan curves. Injecting RAM fan tables corrupts the legacy fan state machine!
    if crate::driver::wmi_ec::is_legacy_dual_mode_ec() {
        crate::log_info!("POWER_MODE", "apply_mode_fan_table: Legacy Dual-Mode chassis detected, yielding to native EC firmware fan curve");
        return Ok(());
    }
    let table = get_mode_fan_table(ui_mode);
    apply_fan_table_to_ec(&table)
}

pub fn build_custom_fan_table(
    cpu_curve: &[crate::system::config::FanCurvePoint],
    gpu_curve: &[crate::system::config::FanCurvePoint],
) -> FanTableData {
    let turbo_table = get_mode_fan_table(3);

    // Standardized safe linear temperature progression (30°C ~ 100°C in 5°C increments)
    // Eliminates factory mute freeze (all 65s) and hysteresis deadlocks (DownT >= UpT)
    let mut custom_upt = [0u8; 16];
    let mut custom_downt = [0u8; 16];
    custom_upt[0] = 0;
    custom_downt[0] = 0;
    for i in 1..16 {
        let t = (30 + (i - 1) * 5) as u8; // Step 1: 30°C ... Step 15: 100°C
        custom_upt[i] = t;
        // Anti-Hunting Hysteresis: Widen gap to 8°C for idle/low-load (<= 55°C) to prevent start-stop oscillation!
        let hyst = if t <= 55 { 8 } else { 5 };
        custom_downt[i] = t.saturating_sub(hyst).max(custom_downt[i - 1]);
    }

    let (effective_cpu_upt, effective_cpu_downt) = if !cpu_curve.is_empty() {
        (custom_upt, custom_downt)
    } else {
        (turbo_table.cpu_upt, turbo_table.cpu_downt)
    };

    let (effective_gpu_upt, effective_gpu_downt) = if !gpu_curve.is_empty() {
        (custom_upt, custom_downt)
    } else {
        let mut upt = turbo_table.gpu_upt;
        let mut downt = turbo_table.gpu_downt;
        if upt[1] == 65 && upt[5] == 65 {
            upt = custom_upt;
            downt = custom_downt;
        } else {
            for i in 1..16 {
                if upt[i] != 255 && downt[i] >= upt[i] {
                    downt[i] = upt[i].saturating_sub(4);
                }
            }
        }
        (upt, downt)
    };

    let interpolate_duty = |curve: &[crate::system::config::FanCurvePoint], base_upt: &[u8; 16], fallback_duty: &[u8; 16]| -> [u8; 16] {
        let mut duty = [0u8; 16];
        if curve.is_empty() {
            return *fallback_duty;
        }

        for step in 0..16 {
            let t = base_upt[step];
            if t == 255 {
                duty[step] = duty[step.saturating_sub(1)];
                continue;
            }
            if t <= curve[0].temp {
                duty[step] = ((curve[0].duty as u16 * 200) / 100).min(200) as u8;
                continue;
            }
            let last_idx = curve.len() - 1;
            if t >= curve[last_idx].temp {
                duty[step] = ((curve[last_idx].duty as u16 * 200) / 100).min(200) as u8;
                continue;
            }
            // Linear interpolation between adjacent curve points
            let mut found = false;
            for i in 0..last_idx {
                let p1 = &curve[i];
                let p2 = &curve[i + 1];
                if t >= p1.temp && t <= p2.temp {
                    let span_t = (p2.temp - p1.temp) as f32;
                    let factor = if span_t > 0.0 { (t - p1.temp) as f32 / span_t } else { 0.0 };
                    let d = p1.duty as f32 + factor * (p2.duty as f32 - p1.duty as f32);
                    duty[step] = ((d * 2.0).round() as u16).min(200) as u8;
                    found = true;
                    break;
                }
            }
            if !found {
                duty[step] = ((curve[last_idx].duty as u16 * 200) / 100).min(200) as u8;
            }
        }

        // Hardware Safety Guard: Enforce strict monotonic non-decreasing progression
        // EC firmware rejects fan tables with inverted/decreasing duty steps!
        let mut running_max = 0u8;
        for step in 0..16 {
            if duty[step] < running_max {
                duty[step] = running_max;
            } else {
                running_max = duty[step];
            }
        }

        // CRITICAL MECHREVO / UNIWILL EC FIRMWARE RULE:
        // Step 0 (EC 3872 / 3920) is the Zero-RPM standby threshold (0~50°C idle).
        // It MUST ALWAYS be 0x00! If written with non-zero, the EC fan FSM state machine
        // treats it as invalid standby configuration and completely locks up fan control!
        duty[0] = 0;

        duty
    };

    let cpu_duty = interpolate_duty(cpu_curve, &effective_cpu_upt, &turbo_table.cpu_duty);
    let mut gpu_duty = interpolate_duty(gpu_curve, &effective_gpu_upt, &turbo_table.gpu_duty);

    // Live hardware sniffer rule: GPU tail steps 13..15 (3933..3935 / RamFan1p5) must sync to max GPU duty
    let max_gpu = gpu_duty[12].max(gpu_duty[15]);
    gpu_duty[13] = gpu_duty[13].max(max_gpu);
    gpu_duty[14] = gpu_duty[14].max(max_gpu);
    gpu_duty[15] = max_gpu;

    let max_duty = cpu_duty[15].max(gpu_duty[15]).max(160);

    FanTableData {
        cpu_upt: effective_cpu_upt,
        cpu_downt: effective_cpu_downt,
        cpu_duty,
        gpu_upt: effective_gpu_upt,
        gpu_downt: effective_gpu_downt,
        gpu_duty,
        max_duty,
    }
}

pub fn apply_fan_table_to_ec(table: &FanTableData) -> Result<(), String> {
    use crate::driver::wmi_ec;
    if wmi_ec::is_legacy_dual_mode_ec() {
        return Err("RAM fan table injection is not supported on legacy chassis".into());
    }
    fan_table_io::apply(
        table,
        wmi_ec::is_nextgen_oem_magic_ec(),
        |address, preserve, value| {
            wmi_ec::wmi_modify_ec(address, |current| (current & preserve) | value)
        },
        |pre| {
            if pre {
                wmi_ec::latch_fan_table_ec(true)
            } else {
                wmi_ec::commit_fan_table_latch_ec()
            }
        },
        wmi_ec::wmi_write_ec_batch,
    )?;
    crate::log_info!("POWER_MODE", "Fan table control acknowledged");
    Ok(())
}
