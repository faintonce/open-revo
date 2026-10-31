use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FactoryPowerPresets {
    pub office_pl1: u16,
    pub office_pl2: u16,
    #[serde(default)]
    pub office_pl4: u16,
    pub balance_pl1: u16,
    pub balance_pl2: u16,
    #[serde(default)]
    pub balance_pl4: u16,
    pub turbo_pl1: u16,
    pub turbo_pl2: u16,
    #[serde(default)]
    pub turbo_pl4: u16,
    pub base_tgp: u16,
    pub max_db: u16,
    pub hardware_read: bool,
}

static PRESETS_CACHE: parking_lot::RwLock<Option<FactoryPowerPresets>> = parking_lot::RwLock::new(None);

pub fn get_cached_factory_power_presets() -> Option<FactoryPowerPresets> {
    PRESETS_CACHE.read().clone()
}

pub fn get_factory_power_presets(has_gpu: bool) -> FactoryPowerPresets {
    if let Some(ref cached) = *PRESETS_CACHE.read() {
        return cached.clone();
    }
    refresh_factory_power_presets(has_gpu)
}

pub fn refresh_factory_power_presets(_has_gpu: bool) -> FactoryPowerPresets {
    // 0. Declarative Hardware Manifest Ground Truth:
    // If the active machine matches a declarative driver with explicit power_mode_watts,
    // adopt its authoritative vendor calibration directly.
    if let Some(manifest) = crate::hal::declarative::load_matched_manifest() {
        if let Some(ref pmw) = manifest.capabilities.power_mode_watts {
            let off = pmw.get("office").copied().unwrap_or(45) as u16;
            let bal = pmw.get("balance").copied().unwrap_or(80) as u16;
            let tur = pmw.get("turbo").copied().unwrap_or(115) as u16;
            let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
            let base_tgp = gpu_caps.base_tgp_max_watts as u16;
            let max_db = gpu_caps.max_db_watts as u16;

            let presets = FactoryPowerPresets {
                office_pl1: off,
                office_pl2: off,
                office_pl4: off,
                balance_pl1: bal,
                balance_pl2: bal,
                balance_pl4: bal,
                turbo_pl1: tur,
                turbo_pl2: tur,
                turbo_pl4: tur,
                base_tgp,
                max_db,
                hardware_read: true,
            };
            *PRESETS_CACHE.write() = Some(presets.clone());
            return presets;
        }
    }

    let model_match = crate::system::model_engine::detect_model_profile();
    let profile = &model_match.matched_profile;

    let off_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL1).unwrap_or(0) as u16;
    let off_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL2).unwrap_or(0) as u16;
    let off_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL4).unwrap_or(0) as u16;
    let bal_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL1).unwrap_or(0) as u16;
    let bal_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL2).unwrap_or(0) as u16;
    let bal_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL4).unwrap_or(0) as u16;
    let tur_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL1).unwrap_or(0) as u16;
    let tur_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL2).unwrap_or(0) as u16;
    let tur_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL4).unwrap_or(0) as u16;

    let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    let drv_base = gpu_caps.base_tgp_max_watts as u16;
    let drv_db = gpu_caps.max_db_watts as u16;

    let is_hw = off_pl1 > 0 || bal_pl1 > 0 || tur_pl1 > 0;

    if !is_hw {
        crate::log_warn!(
            "HARDWARE",
            "Power presets: WMI EC read returned 0s (bal={}, off={}, tur={}). Automatically applying adapted profile [{}] (reason: {}).",
            bal_pl1, off_pl1, tur_pl1, profile.display_name, model_match.confidence_reason
        );
    }

    // Determine Base TGP & Max DB purely from hardware truth and declared specs:
    let base_tgp = if drv_base > 0 {
        drv_base
    } else if let Some(ref s) = profile.factory_specs {
        if s.base_tgp > 0 && s.base_tgp <= 175 { s.base_tgp } else { 0 }
    } else {
        0
    };

    let max_db = if drv_db > 0 {
        drv_db
    } else if let Some(ref s) = profile.factory_specs {
        s.max_db.clamp(5, 35)
    } else {
        25
    };

    let off_pl2_val = if off_pl2 > 0 { off_pl2 } else { profile.pl1_default.min(45) };
    let bal_pl2_val = if bal_pl2 > 0 { bal_pl2 } else { profile.pl2_default };
    let tur_pl2_val = if tur_pl2 > 0 { tur_pl2 } else { profile.pl4_default };

    let calc_pl4 = |raw_pl4: u16, pl2_val: u16, fallback: u16| -> u16 {
        let val = if raw_pl4 > 0 { raw_pl4 } else { fallback };
        val.max(pl2_val.saturating_add(20))
    };

    let presets = FactoryPowerPresets {
        office_pl1: if off_pl1 > 0 { off_pl1 } else { profile.pl1_default.min(45) },
        office_pl2: off_pl2_val,
        office_pl4: calc_pl4(off_pl4, off_pl2_val, 100),
        balance_pl1: if bal_pl1 > 0 { bal_pl1 } else { profile.pl1_default },
        balance_pl2: bal_pl2_val,
        balance_pl4: calc_pl4(bal_pl4, bal_pl2_val, profile.pl2_default + 30),
        turbo_pl1: if tur_pl1 > 0 { tur_pl1 } else { profile.pl4_default },
        turbo_pl2: tur_pl2_val,
        turbo_pl4: calc_pl4(tur_pl4, tur_pl2_val, profile.pl4_default + 40),
        base_tgp,
        max_db,
        hardware_read: is_hw || drv_base > 0 || model_match.is_trusted,
    };

    *PRESETS_CACHE.write() = Some(presets.clone());
    presets
}

/// 读取当前档位**实时生效**的 CPU PL1 / PL2（W，EC 寄存器真值）。
/// - 1/2/3 档：读对应档位的工厂表字节（固件/外部工具改写后会变）
/// - 4 档（自定义）：读自定义 PL 寄存器（用户下发后的实际值）
/// 读不到的档位返回 0，调用方回落到预设值。
pub fn read_live_cpu_pl(ui_mode: u8) -> (u16, u16) {
    use crate::driver::wmi_ec as ec;
    let (a1, a2) = match ui_mode {
        1 => (ec::ADDR_OFFICE_PL1, ec::ADDR_OFFICE_PL2),
        2 => (ec::ADDR_BALANCE_PL1, ec::ADDR_BALANCE_PL2),
        3 => (ec::ADDR_TURBO_PL1, ec::ADDR_TURBO_PL2),
        4 => (ec::ADDR_CUSTOM_PL1, ec::ADDR_CUSTOM_PL2),
        _ => (ec::ADDR_BALANCE_PL1, ec::ADDR_BALANCE_PL2),
    };
    let pl1 = ec::wmi_read_ec(a1).unwrap_or(0) as u16;
    let pl2 = ec::wmi_read_ec(a2).unwrap_or(0) as u16;
    (pl1, pl2)
}
