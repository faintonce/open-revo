use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use parking_lot::RwLock;

pub use crate::system::power_mode::{FanTableData, ModelFanTables};

/// Raw JSON representation of built-in models embedded at compile time from resources/builtin_models.json
const BUILTIN_MODELS_JSON: &str = include_str!("../../resources/builtin_models.json");

/// Single power preset containing PL limits and thermal offset
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PowerPreset {
    pub pl1: u16,
    pub pl2: u16,
    pub pl4: u16,
    #[serde(default)]
    pub temp_offset: u8,
}

/// Dynamic active runtime power presets for 3 modes (customizable by user)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelPresets {
    pub office: PowerPreset,
    pub balanced: PowerPreset,
    pub turbo: PowerPreset,
}

/// Baseline hardware 出厂规格 (probed from EC / VBIOS, read-only reference & fallback)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FactorySpecs {
    pub office: PowerPreset,
    pub balanced: PowerPreset,
    pub turbo: PowerPreset,
    pub base_tgp: u16,
    pub max_db: u16,
}

/// Default sovereignty version string for Level 0 machine constitution
pub fn default_sovereignty_version() -> String {
    "2.0".to_string()
}

/// Level 0 Sovereign Contract: GPU MUX Switching Capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuMuxContract {
    pub modes_supported: Vec<String>,
    pub forbid_igpu_only: bool,
    pub is_safety_locked: bool,
    pub reason: Option<String>,
}

/// Level 0 Sovereign Contract: Lighting and Keyboard Peripherals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LightingContract {
    pub keyboard_type: String, // "4Zone", "TypeA", "SingleZone", "White", "None"
    pub backend: String,       // "USB_HID", "EC", "None"
    #[serde(default)]
    pub target_pid: Option<u16>,
    #[serde(default)]
    pub has_coastline: bool,
    #[serde(default)]
    pub has_logo: bool,
    #[serde(default)]
    pub has_air_vent: bool,
}

/// Cached active Lighting contract (see `get_active_lighting_contract`).
/// Populated on first call from the live HID topology, dropped by
/// `invalidate_lighting_contract_cache` whenever devices are re-enumerated.
/// Entries also expire after `LIGHTING_CONTRACT_TTL` so a device that appears
/// without a lifecycle event (hot-plug without sleep/wake) is still picked up.
static LIGHTING_CONTRACT_CACHE: LazyLock<RwLock<Option<(std::time::Instant, LightingContract)>>> =
    LazyLock::new(|| RwLock::new(None));

const LIGHTING_CONTRACT_TTL: std::time::Duration = std::time::Duration::from_secs(60);

/// Level 0 Sovereign Contract: Motherboard Communication Bus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunicationBusContract {
    pub bus_type: String, // "tongfang_acpi_kernel", "standard_wmi_direct", "ec_raw_io"
    pub wmi_device_id: String,
    pub has_acpi_driver: bool,
}

/// Level 0 Sovereign Contract: Official Control Center Takeover & Coexistence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TakeoverContract {
    pub strategy: String, // "FullTakeover", "Lightweight", "None"
    pub detected_centers: Vec<String>,
    pub detected_services: Vec<String>,
    pub has_official_center: bool,
}

/// Serializable Owned Model Profile for IPC & UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    #[serde(default)]
    pub mine: bool,
    pub model_id: String,
    pub display_name: String,
    #[serde(default)]
    pub alias: String,
    pub project_id: u8,
    #[serde(default)]
    pub bios_project_id: String,
    #[serde(default)]
    pub factory_specs: Option<FactorySpecs>,
    #[serde(default)]
    pub presets: Option<ModelPresets>,
    #[serde(default)]
    pub fan_tables: Option<ModelFanTables>,
    #[serde(default)]
    pub pl1_default: u16,
    #[serde(default)]
    pub pl2_default: u16,
    #[serde(default)]
    pub pl4_default: u16,
    #[serde(default)]
    pub base_tgp: u16,
    #[serde(default)]
    pub max_db: u16,
    #[serde(default)]
    pub max_fan_duty: u8,
    #[serde(default)]
    pub temp_levels: u8,
    #[serde(default)]
    pub fan_control_respective: bool,
    #[serde(default)]
    pub has_water_cooler: bool,
    #[serde(default)]
    pub keyboard_type: String,

    // Level 0 Machine Constitution & Sovereign Contracts
    #[serde(default)]
    pub gpu_mux: Option<GpuMuxContract>,
    #[serde(default)]
    pub lighting: Option<LightingContract>,
    #[serde(default)]
    pub bus: Option<CommunicationBusContract>,
    #[serde(default)]
    pub takeover: Option<TakeoverContract>,
    #[serde(default)]
    pub is_inferred: bool,
    #[serde(default = "default_sovereignty_version")]
    pub sovereignty_version: String,
}

/// Built-in model profile definitions deserialized from embedded builtin_models.json
///
/// The embedded JSON is ~168 KB and is parsed on every call, so the parse result is
/// cached once and cloned per caller. Cloning the parsed list is orders of magnitude
/// cheaper than re-running serde_json over the whole document.
pub fn get_builtin_profiles() -> Vec<ModelProfile> {
    static BUILTIN_CACHE: LazyLock<Vec<ModelProfile>> = LazyLock::new(|| {
        match serde_json::from_str::<Vec<ModelProfile>>(BUILTIN_MODELS_JSON) {
            Ok(profiles) => profiles,
            Err(e) => {
                crate::log_error!(
                    "MODEL_ENGINE",
                    "Fatal: Failed to deserialize embedded builtin_models.json: {}",
                    e
                );
                Vec::new()
            }
        }
    });
    BUILTIN_CACHE.clone()
}


/// Path to external %APPDATA%\OpenRevo\models.json
pub fn get_models_file_path() -> std::path::PathBuf {
    crate::system::config::get_config_dir().join("models.json")
}

/// Up to 30 Unicode characters (supports Chinese, English & Emoji)
pub fn sanitize_alias(alias: &str) -> String {
    let trimmed = alias.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.chars().take(30).collect::<String>()
}

/// Retrieve the full BIOS product device name from Windows Registry
pub fn get_device_full_name() -> String {
    #[cfg(windows)]
    unsafe {
        use windows::core::{w, PCWSTR};
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, HKEY,
        };
        const BIOS_KEY: PCWSTR = w!(r"HARDWARE\DESCRIPTION\System\BIOS");
        const VAL_BASEBOARD: PCWSTR = w!("BaseBoardProduct");
        const VAL_SYSNAME: PCWSTR = w!("SystemProductName");

        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, BIOS_KEY, 0, KEY_READ, &mut hkey).is_ok() {
            let read_val = |val_name: PCWSTR| -> Option<String> {
                let mut buf = [0u16; 128];
                let mut size = (buf.len() * 2) as u32;
                let mut data_type = REG_SZ;
                if RegQueryValueExW(
                    hkey,
                    val_name,
                    None,
                    Some(&mut data_type),
                    Some(buf.as_mut_ptr() as *mut _),
                    Some(&mut size),
                )
                .is_ok()
                {
                    let len = (size as usize / 2).saturating_sub(1);
                    let s = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                    if !s.is_empty() && s != "Standard" && !s.to_lowercase().starts_with("to be filled") && s != "System Product Name" {
                        return Some(s);
                    }
                }
                None
            };

            let sys_product = read_val(VAL_SYSNAME);
            let baseboard = read_val(VAL_BASEBOARD);
            let _ = RegCloseKey(hkey);

            return match (sys_product, baseboard) {
                (Some(sys), Some(bb)) => {
                    let s_low = sys.to_lowercase();
                    let b_low = bb.to_lowercase();
                    if s_low == b_low || b_low.contains(&s_low) {
                        bb
                    } else if s_low.contains(&b_low) {
                        sys
                    } else {
                        format!("{} {}", sys, bb)
                    }
                }
                (Some(sys), None) => sys,
                (None, Some(bb)) => bb,
                (None, None) => "MECHREVO Gaming Laptop".to_string(),
            };
        }
    }
    "MECHREVO Gaming Laptop".to_string()
}

pub const KNOWN_BRANDS: &[&str] = &[
    "JIAOLONG", "YAOSHI", "CANGLONG", "YILONG", "WUJIE", "KUANGSHI", "JIGUANG", "CODE"
];

pub fn extract_brand_token(s: &str) -> Option<&'static str> {
    let upper = s.to_uppercase();
    for &b in KNOWN_BRANDS {
        if upper.contains(b) {
            return Some(b);
        }
    }
    None
}

/// Identifies the current machine's profile, ensures mine: true is set on it (and mine: false on all others),
/// populates display_name with the full device name, initializes alias if empty, and ensures mine: true is always at Index 0.
/// Returns true if any modification was made (meaning file should be saved).
pub fn sync_active_machine_profile(profiles: &mut Vec<ModelProfile>) -> bool {
    let board = get_baseboard_product().unwrap_or_else(|| "UNKNOWN".to_string());
    let clean_board = board.trim().to_uppercase();
    let ec_pid = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_PROJECT_ID).unwrap_or(0);
    let reg_bios_pid = read_registry_bios_project_id();
    let full_device_name = get_device_full_name();

    let current_brand = extract_brand_token(&clean_board)
        .or_else(|| extract_brand_token(&full_device_name));

    // Step 1: Find match
    let mut matched_idx = profiles.iter().position(|p| p.mine);

    // If an existing "mine: true" profile has a clear brand conflict with the current machine (e.g. board is JIAOLONG, profile is YAOSHI),
    // invalidate it so we re-match accurately!
    if let Some(idx) = matched_idx {
        if let Some(p) = profiles.get(idx) {
            let p_name = p.display_name.to_uppercase();
            let p_id = p.model_id.to_uppercase();
            let p_alias = p.alias.to_uppercase();
            if let Some(cb) = current_brand {
                if KNOWN_BRANDS.iter().copied().any(|b| b != cb && (p_name.contains(b) || p_id.contains(b) || p_alias.contains(b))) {
                    matched_idx = None;
                }
            }
        }
    }

    if matched_idx.is_none() {
        // Case A1: Exact match with Board Product or Full Device Name (brand-safe)
        for (i, p) in profiles.iter().enumerate() {
            let clean_model_id = p.model_id.trim().to_uppercase();
            if clean_model_id.is_empty() {
                continue;
            }
            if let Some(cb) = current_brand {
                let p_brand = extract_brand_token(&clean_model_id)
                    .or_else(|| extract_brand_token(&p.display_name));
                if let Some(pb) = p_brand {
                    if pb != cb {
                        continue;
                    }
                }
            }
            if clean_board == clean_model_id || full_device_name.eq_ignore_ascii_case(&p.model_id) {
                matched_idx = Some(i);
                break;
            }
        }

        // Case A2: Substring match with Board Product or Full Device Name (brand-safe)
        if matched_idx.is_none() {
            for (i, p) in profiles.iter().enumerate() {
                let clean_model_id = p.model_id.trim().to_uppercase();
                if clean_model_id.is_empty() {
                    continue;
                }
                if let Some(cb) = current_brand {
                    let p_brand = extract_brand_token(&clean_model_id)
                        .or_else(|| extract_brand_token(&p.display_name));
                    if let Some(pb) = p_brand {
                        if pb != cb {
                            continue;
                        }
                    }
                }
                if clean_board.contains(&clean_model_id) || full_device_name.to_uppercase().contains(&clean_model_id) {
                    matched_idx = Some(i);
                    break;
                }
            }
        }

        // Case B: Match by BIOS Project ID (prevent cross-brand collision)
        if matched_idx.is_none() {
            if let Some(ref bios_pid) = reg_bios_pid {
                let clean_bios_pid = bios_pid.trim().to_uppercase();
                for (i, p) in profiles.iter().enumerate() {
                    if !p.bios_project_id.is_empty() && clean_bios_pid == p.bios_project_id.trim().to_uppercase() {
                        if let Some(cb) = current_brand {
                            let p_name = p.display_name.to_uppercase();
                            let p_id = p.model_id.to_uppercase();
                            if KNOWN_BRANDS.iter().copied().any(|b| b != cb && (p_name.contains(b) || p_id.contains(b))) {
                                continue;
                            }
                        }
                        matched_idx = Some(i);
                        break;
                    }
                }
            }
        }

        // Case C: Match by EC ProjectID (strictly prevent cross-brand collision; never blindly match generic shared ID 26)
        if matched_idx.is_none() && ec_pid > 0 {
            for (i, p) in profiles.iter().enumerate() {
                if p.project_id == ec_pid {
                    if let Some(cb) = current_brand {
                        let p_name = p.display_name.to_uppercase();
                        let p_id = p.model_id.to_uppercase();
                        if !p_name.contains(cb) && !p_id.contains(cb) {
                            continue;
                        }
                    } else if ec_pid == 26 {
                        // ProjectID 26 is shared by almost all 50-series and newer laptops; do not blindly pick first profile
                        continue;
                    }
                    matched_idx = Some(i);
                    break;
                }
            }
        }
    }

    let mut changed = false;

    let target_idx = match matched_idx {
        Some(i) => i,
        None => {
            // Case D: Dynamic Hardware Truth & Declarative Driver Synthesized profile
            let cpu_brand = crate::system::cpu_monitor::get_cached_cpu_brand();
            let gpu_name = crate::driver::gpu_bridge::get_gpu_name_cached();
            let mut inferred = synthesize_hardware_profile(&cpu_brand, &gpu_name);
            let is_unadapted = crate::hal::platform_probe::get_platform_channel() == crate::hal::platform_probe::PlatformChannelKind::Unknown;
            inferred.mine = !is_unadapted;
            inferred.model_id = if !clean_board.is_empty() && clean_board != "UNKNOWN" {
                board.clone()
            } else {
                "MY_MACHINE".to_string()
            };
            inferred.display_name = full_device_name.clone();
            inferred.alias = String::new();
            if is_unadapted {
                inferred.factory_specs = None;
                inferred.presets = None;
            }
            profiles.insert(0, inferred);
            changed = true;
            0
        }
    };

    // Update target profile
    if let Some(target) = profiles.get_mut(target_idx) {
        if !target.mine {
            target.mine = true;
            changed = true;
        }
        if target.display_name != full_device_name {
            target.display_name = full_device_name.clone();
            changed = true;
        }

        let clean_alias = target.alias.trim().to_uppercase();
        let alias_conflict = if let Some(cb) = current_brand {
            KNOWN_BRANDS.iter().copied().any(|b| b != cb && clean_alias.contains(b))
        } else {
            false
        };

        if alias_conflict || clean_alias == target.model_id.to_uppercase() {
            target.alias = String::new();
            changed = true;
        }
        if target.fan_tables.is_none() {
            let builtin = get_builtin_profiles();
            if let Some(b) = builtin.iter().find(|b| b.model_id.eq_ignore_ascii_case(&target.model_id)) {
                if b.fan_tables.is_some() {
                    target.fan_tables = b.fan_tables.clone();
                    changed = true;
                }
            }
        }

        // Live hardware specs probing from motherboard EC registers
        let off_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL1).unwrap_or(0) as u16;
        let off_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL2).unwrap_or(0) as u16;
        let off_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL4).unwrap_or(0) as u16;
        let off_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_TCC).unwrap_or(15) as u8;

        let bal_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL1).unwrap_or(0) as u16;
        let bal_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL2).unwrap_or(0) as u16;
        let bal_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL4).unwrap_or(0) as u16;
        let bal_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_TCC).unwrap_or(5) as u8;

        let tur_pl1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL1).unwrap_or(0) as u16;
        let tur_pl2 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL2).unwrap_or(0) as u16;
        let tur_pl4 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL4).unwrap_or(0) as u16;
        let tur_tcc = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_TCC).unwrap_or(5) as u8;


        let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
        let drv_base = gpu_caps.base_tgp_max_watts as u16;
        let drv_db = gpu_caps.max_db_watts as u16;

        // Determine Base TGP purely from physical VBIOS reading or declared machine manifest:
        let base_tgp = if drv_base > 0 {
            drv_base
        } else if target.base_tgp > 0 && target.base_tgp <= 175 {
            target.base_tgp
        } else {
            0
        };

        let max_db = if drv_db > 0 {
            drv_db
        } else if target.max_db >= 5 && target.max_db <= 35 {
            target.max_db
        } else {
            25
        };

        let has_hw_ec = bal_pl1 > 0 || tur_pl1 > 0 || off_pl1 > 0;

        let probed_specs = if has_hw_ec {
            FactorySpecs {
                office: PowerPreset {
                    pl1: if off_pl1 > 0 { off_pl1 } else { 45 },
                    pl2: if off_pl2 > 0 { off_pl2 } else { 45 },
                    pl4: if off_pl4 > 0 { off_pl4 } else { 125 },
                    temp_offset: off_tcc,
                },
                balanced: PowerPreset {
                    pl1: if bal_pl1 > 0 { bal_pl1 } else { 75 },
                    pl2: if bal_pl2 > 0 { bal_pl2 } else { 75 },
                    pl4: if bal_pl4 > 0 { bal_pl4 } else { 125 },
                    temp_offset: bal_tcc,
                },
                turbo: PowerPreset {
                    pl1: if tur_pl1 > 0 { tur_pl1 } else if bal_pl1 > 0 { bal_pl1 + 25 } else { target.pl4_default.max(90) },
                    pl2: if tur_pl2 > 0 { tur_pl2 } else if bal_pl2 > 0 { bal_pl2 + 30 } else { target.pl4_default.max(105) },
                    pl4: if tur_pl4 > 0 { tur_pl4 } else if tur_pl2 > 0 { (tur_pl2 as f32 * 1.25) as u16 } else { 140 },
                    temp_offset: tur_tcc,
                },
                base_tgp,
                max_db,
            }
        } else {
            // Fallback from profile defaults
            FactorySpecs {
                office: PowerPreset {
                    pl1: target.pl1_default.min(45),
                    pl2: target.pl1_default.min(45),
                    pl4: 100,
                    temp_offset: 15,
                },
                balanced: PowerPreset {
                    pl1: target.pl1_default,
                    pl2: target.pl2_default,
                    pl4: target.pl2_default + 30,
                    temp_offset: 5,
                },
                turbo: PowerPreset {
                    pl1: target.pl4_default,
                    pl2: target.pl4_default,
                    pl4: target.pl4_default + 40,
                    temp_offset: 5,
                },
                base_tgp: target.base_tgp,
                max_db: target.max_db,
            }
        };

        if target.factory_specs.as_ref() != Some(&probed_specs) {
            target.factory_specs = Some(probed_specs.clone());
            changed = true;
        }

        // Two-Tier Architecture:
        // Do NOT automatically synthesize `target.presets = Some(...)` on unconfigured machines!
        // `target.presets` is strictly reserved for explicit User / Community DIY overrides (e.g. customized PL4).
        // When `target.presets` is None, the system adheres to the Zero-Override rule and yields to EC native factory table.
        // Auto-cleanse legacy mirrored presets: If `presets` was automatically cloned from `factory_specs`
        // by older versions of OpenRevo (meaning user never altered any values), clear `presets` to None.
        if let (Some(ref p), Some(ref s)) = (&target.presets, &target.factory_specs) {
            if p.office == s.office && p.balanced == s.balanced && p.turbo == s.turbo {
                target.presets = None;
                changed = true;
                crate::log_info!("MODEL_ENGINE", "Legacy auto-cloned presets cleansed to None. Zero-Override native EC scheduling activated.");
            }
        }

        // Enrich target profile with live Sovereign Contracts (Level 0)
        TruthReconstructor::enrich_contracts(target);
    }

    // Set mine = false on all other profiles
    for (i, p) in profiles.iter_mut().enumerate() {
        if i != target_idx && p.mine {
            p.mine = false;
            changed = true;
        }
    }

    // Ensure target_idx is always moved to index 0!
    if target_idx != 0 && target_idx < profiles.len() {
        let mine_prof = profiles.remove(target_idx);
        profiles.insert(0, mine_prof);
        changed = true;
    }

    changed
}

/// Retrieve the active machine profile (guaranteed to be mine: true at index 0)
pub fn get_active_profile() -> Option<ModelProfile> {
    let profiles = load_model_profiles();
    profiles.into_iter().find(|p| p.mine)
}

/// Retrieve the active machine alias (customized by user in models.json, or BIOS full name)
pub fn get_active_alias() -> String {
    let full_name = get_device_full_name();
    let current_brand = extract_brand_token(&full_name);
    let profiles = load_model_profiles();
    if let Some(first) = profiles.first() {
        if first.mine {
            let clean = sanitize_alias(&first.alias);
            if !clean.is_empty() {
                // Ensure the alias doesn't contradict the machine's hardware brand
                if let Some(cb) = current_brand {
                    if KNOWN_BRANDS.iter().copied().any(|b| b != cb && clean.to_uppercase().contains(b)) {
                        return full_name;
                    }
                }
                return clean;
            }
        }
    }
    full_name
}

/// In-memory cache of the fully synchronized profile list.
/// Every load re-reads and re-parses the ~174 KB models.json plus the embedded builtin
/// document, then re-probes EC registers for factory specs. Background pollers call this
/// several times per second, so the synchronized result is cached until the file changes.
static PROFILES_CACHE: LazyLock<RwLock<Option<(std::time::SystemTime, Vec<ModelProfile>)>>> =
    LazyLock::new(|| RwLock::new(None));

/// Drop the cached profile list so the next access re-reads models.json from disk.
pub fn invalidate_model_profiles_cache() {
    *PROFILES_CACHE.write() = None;
    // The active lighting contract is derived from the active profile's keyboard hint,
    // so it must be re-probed together with the profile list.
    invalidate_lighting_contract_cache();
}

/// Load model profiles from %APPDATA%\OpenRevo\models.json (creates file if missing, keeps mine: true at index 0)
pub fn load_model_profiles() -> Vec<ModelProfile> {
    let path = get_models_file_path();

    // Cache hit: reuse the synchronized list as long as models.json has not been touched.
    // A missing file is never cached, so an externally created file is still picked up.
    // The lock is never held across the disk load below, so a slow read cannot block the
    // background pollers that share this cache.
    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let Some((cached_mtime, cached)) = PROFILES_CACHE.read().as_ref() {
                if *cached_mtime == modified {
                    return cached.clone();
                }
            }

            let profiles = load_model_profiles_from_disk(&path);
            // The load itself may have rewritten models.json (first run / hardware re-sync),
            // so capture the post-write timestamp instead of the one read before the load.
            let cached_mtime = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(modified);
            *PROFILES_CACHE.write() = Some((cached_mtime, profiles.clone()));
            return profiles;
        }
    }

    load_model_profiles_from_disk(&path)
}

fn load_model_profiles_from_disk(path: &std::path::Path) -> Vec<ModelProfile> {
    let mut profiles = if !path.exists() {
        get_builtin_profiles()
    } else {
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Vec<ModelProfile>>(&content) {
                Ok(mut loaded) => {
                    if loaded.is_empty() {
                        get_builtin_profiles()
                    } else {
                        // Ensure builtins exist in the loaded profiles list, and backfill fan_tables if missing
                        let builtin = get_builtin_profiles();
                        for b in &builtin {
                            if let Some(loaded_p) = loaded.iter_mut().find(|p| p.model_id.eq_ignore_ascii_case(&b.model_id)) {
                                if loaded_p.fan_tables.is_none() && b.fan_tables.is_some() {
                                    loaded_p.fan_tables = b.fan_tables.clone();
                                }
                            }
                        }
                        for b in builtin {
                            if !loaded.iter().any(|p| p.model_id.eq_ignore_ascii_case(&b.model_id)) {
                                loaded.push(b);
                            }
                        }
                        loaded
                    }
                }
                Err(e) => {
                    crate::log_warn!("MODEL_ENGINE", "Failed to parse models.json: {}. Falling back to builtin profiles.", e);
                    get_builtin_profiles()
                }
            },
            Err(e) => {
                crate::log_warn!("MODEL_ENGINE", "Failed to read models.json: {}. Falling back to builtin profiles.", e);
                get_builtin_profiles()
            }
        }
    };

    let changed = sync_active_machine_profile(&mut profiles);
    if !path.exists() || changed {
        if let Ok(json) = serde_json::to_string_pretty(&profiles) {
            let _ = std::fs::write(&path, json);
        }
    }

    profiles
}

/// Save modified model profiles to %APPDATA%\OpenRevo\models.json
pub fn save_model_profiles(profiles: &[ModelProfile]) -> Result<(), String> {
    let path = get_models_file_path();
    let json = serde_json::to_string_pretty(profiles).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    invalidate_model_profiles_cache();
    Ok(())
}


/// Level 0 Truth Reconstructor:
/// Autonomous hardware truth assembly engine that probes hardware sensors,
/// chips, registers, and OS environments in real time.
/// Reconstructs 95%+ capabilities when DMI matches are missing,
/// while safely restricting only high-risk irreversible MUX iGPU switching.
pub struct TruthReconstructor;

impl TruthReconstructor {
    /// Probe GPU MUX switching capabilities dynamically from hardware
    pub fn probe_gpu_mux() -> GpuMuxContract {
        let caps = crate::driver::uefi_nvram::probe_gpu_capabilities();
        let mut modes = Vec::new();
        if caps.supports_hybrid {
            modes.push("dynamic".to_string());
        }
        if caps.supports_dgpu {
            modes.push("discrete".to_string());
        }
        if caps.supports_igpu_only {
            modes.push("integrated".to_string());
        }

        GpuMuxContract {
            modes_supported: modes,
            forbid_igpu_only: !caps.supports_igpu_only,
            is_safety_locked: caps.is_safety_locked,
            reason: caps.reason_igpu_unsupported,
        }
    }

    /// Probe physical lighting controllers and keyboard peripherals purely from hardware & manual overrides (No Model Name Guessing)
    pub fn probe_lighting(profile_kb_hint: &str) -> LightingContract {
        let (has_fourzone, has_perkey, mut has_coastline, mut has_air_vent) = (
            crate::driver::lighting::IteLightingManager::probe_four_zone_keyboard_exists(),
            crate::driver::ite_streamer::IteStreamerController::probe_exists(),
            crate::driver::lighting::IteLightingManager::probe_lightbar_device_exists(),
            crate::driver::lighting::IteLightingManager::probe_hinge_device_exists() || crate::driver::lighting::IteLightingManager::probe_ec_hinge_supported(),
        );

        let mut has_logo = crate::driver::lighting::IteLightingManager::probe_usb_logo_supported()
            || crate::driver::lighting::IteLightingManager::is_ec_logo_supported();

        let is_ec_single_zone = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_SUPPORT_BYTE2)
            .map(|b| (b & 0x04) != 0)
            .unwrap_or(false);

        let normalized_hint = match profile_kb_hint {
            "SingleZone_RGB" | "SingleZone" => "SingleZone",
            "MonochromeWhite" | "White" => "MonochromeWhite",
            "4Zone" => "4Zone",
            "PerKey" => "PerKey",
            "None" => "None",
            _ => "",
        };

        let (mut kb_type, mut backend, mut pid) = if has_perkey {
            ("PerKey".to_string(), "USB_HID".to_string(), Some(crate::hal::vault::ite::ITE_PID_KEYBOARD_PERKEY))
        } else if has_fourzone {
            ("4Zone".to_string(), "USB_HID".to_string(), Some(crate::hal::vault::ite::ITE_PID_KEYBOARD_FOURZONE))
        } else if is_ec_single_zone {
            ("SingleZone".to_string(), "EC".to_string(), None)
        } else if normalized_hint == "None" {
            ("None".to_string(), "None".to_string(), None)
        } else if !normalized_hint.is_empty() {
            let backend = if normalized_hint == "4Zone" || normalized_hint == "PerKey" { "USB_HID" } else { "EC" };
            (normalized_hint.to_string(), backend.to_string(), None)
        } else {
            ("MonochromeWhite".to_string(), "EC".to_string(), None)
        };

        // Apply Manual Hardware Topology Override if configured
        let cfg = crate::system::config::load_config();
        let hw_override = &cfg.lighting.hardware_override;
        if hw_override.enabled {
            match hw_override.keyboard.as_str() {
                "per_key" => {
                    kb_type = "PerKey".to_string();
                    backend = "USB_HID".to_string();
                    pid = Some(crate::hal::vault::ite::ITE_PID_KEYBOARD_PERKEY);
                }
                "four_zone" => {
                    kb_type = "4Zone".to_string();
                    backend = "USB_HID".to_string();
                    pid = Some(crate::hal::vault::ite::ITE_PID_KEYBOARD_FOURZONE);
                }
                "single_zone" => {
                    kb_type = "SingleZone".to_string();
                    backend = "EC".to_string();
                    pid = None;
                }
                "single_white" => {
                    kb_type = "MonochromeWhite".to_string();
                    backend = "EC".to_string();
                    pid = None;
                }
                "none" => {
                    kb_type = "None".to_string();
                    backend = "None".to_string();
                    pid = None;
                }
                _ => {} // "auto"
            }

            match hw_override.front_bar.as_str() {
                "present" => has_coastline = true,
                "none" => has_coastline = false,
                _ => {}
            }

            match hw_override.ambient.as_str() {
                "vent" | "shoulder" => has_air_vent = true,
                "none" => has_air_vent = false,
                _ => {}
            }

            match hw_override.logo.as_str() {
                "letter" | "shield" => has_logo = true,
                "none" => has_logo = false,
                _ => {}
            }
        }

        LightingContract {
            keyboard_type: kb_type,
            backend,
            target_pid: pid,
            has_coastline,
            has_logo,
            has_air_vent,
        }
    }

    /// Probe motherboard communication bus and kernel driver bridge
    pub fn probe_bus() -> CommunicationBusContract {
        let has_driver = crate::driver::wmi_ec::is_acpi_driver_available();
        let bus_type = if has_driver {
            "tongfang_acpi_kernel".to_string()
        } else {
            "standard_wmi_direct".to_string()
        };

        CommunicationBusContract {
            bus_type,
            wmi_device_id: "root\\WMI:AcpiTest_Mulong".to_string(),
            has_acpi_driver: has_driver,
        }
    }

    /// Probe official vendor control center presence and takeover status
    pub fn probe_takeover() -> TakeoverContract {
        let is_installed = crate::system::takeover::is_oem_installed();
        let is_running = crate::system::takeover::is_oem_running();
        let center_loc = crate::system::takeover::find_official_control_center();

        let strategy = if is_running {
            "FullTakeover".to_string()
        } else if is_installed {
            "Lightweight".to_string()
        } else {
            "None".to_string()
        };

        let detected_centers = if let Some(loc) = center_loc {
            match loc {
                crate::system::takeover::ControlCenterLocation::Win32Exe(p) => {
                    vec![p.file_name().unwrap_or_default().to_string_lossy().to_string()]
                }
                crate::system::takeover::ControlCenterLocation::UwpApp(u) => vec![u],
            }
        } else {
            Vec::new()
        };

        let detected_services = crate::system::takeover::OEM_SERVICES
            .iter()
            .map(|s| s.to_string())
            .collect();

        TakeoverContract {
            strategy,
            detected_centers,
            detected_services,
            has_official_center: is_installed || is_running,
        }
    }

    /// Dynamically enrich an existing profile with real-time sovereign contracts
    pub fn enrich_contracts(profile: &mut ModelProfile) {
        profile.gpu_mux = Some(Self::probe_gpu_mux());
        let kb_hint = profile.keyboard_type.as_str();
        let lighting = Self::probe_lighting(kb_hint);
        if profile.keyboard_type.is_empty()
            || profile.keyboard_type == "None"
            || profile.keyboard_type == "TypeA"
            || profile.keyboard_type == "TypeB"
            || profile.keyboard_type == "SingleZone_RGB"
        {
            if lighting.keyboard_type != "TypeA" && lighting.keyboard_type != "TypeB" {
                profile.keyboard_type = lighting.keyboard_type.clone();
            }
        }
        profile.lighting = Some(lighting);
        profile.bus = Some(Self::probe_bus());
        profile.takeover = Some(Self::probe_takeover());
        if profile.sovereignty_version.is_empty() {
            profile.sovereignty_version = default_sovereignty_version();
        }
    }

    /// Self-heal and reconstruct a fully qualified ModelProfile from live hardware truth
    pub fn reconstruct(cpu_name: &str, gpu_name: &str, baseboard: &str) -> ModelProfile {
        let mut profile = synthesize_hardware_profile(cpu_name, gpu_name);
        if !baseboard.is_empty() && baseboard != "UNKNOWN" {
            profile.model_id = baseboard.to_string();
        }
        let is_unadapted = crate::hal::platform_probe::get_platform_channel() == crate::hal::platform_probe::PlatformChannelKind::Unknown;
        if is_unadapted {
            profile.mine = false;
            profile.factory_specs = None;
            profile.presets = None;
        }
        Self::enrich_contracts(&mut profile);
        profile
    }
}

/// Hardware Truth & Declarative Driver Engine:
/// Synthesizes profile limits strictly based on:
/// 1. Declarative Model Manifest (if matched, authoritative contract)
/// 2. Live Hardware Truth Telemetry (VBIOS power limits, EC registers)
/// NEVER infers hardware parameters from motherboard, CPU, or GPU model names!
pub fn synthesize_hardware_profile(cpu_name: &str, gpu_name: &str) -> ModelProfile {
    // Principle 1: Declarative Driver Center First
    let manifest = crate::hal::declarative::load_matched_manifest();
    let manifest_pmw = manifest.as_ref().and_then(|m| m.capabilities.power_mode_watts.clone());

    // Principle 2: Truth Telemetry for GPU
    // Directly query the authoritative hardware truth source: probe_gpu_tuning_capabilities
    let gpu_caps = crate::driver::gpu_bridge::probe_gpu_tuning_capabilities();
    let base_tgp = gpu_caps.base_tgp_max_watts as u16;
    let max_db = gpu_caps.max_db_watts as u16;

    // Principle 2: Truth Telemetry for CPU Power Envelope
    // 1. Declarative manifest contract
    // 2. Physical EC registers (ADDR_BALANCE_PL1 / ADDR_OFFICE_PL1 / ADDR_TURBO_PL1)
    // 3. Fallback safe standard mobile baseline (45, 65, 95)
    // NEVER infer based on whether CPU name contains "HX", "H", "U", etc.!
    let (office_pl1, bal_pl1, tur_pl1) = if let Some(ref pmw) = manifest_pmw {
        let off = pmw.get("office").copied().unwrap_or(45) as u16;
        let bal = pmw.get("balance").copied().unwrap_or(65) as u16;
        let tur = pmw.get("turbo").copied().unwrap_or(95) as u16;
        (off, bal, tur)
    } else {
        let off_ec = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_OFFICE_PL1).unwrap_or(0) as u16;
        let bal_ec = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_BALANCE_PL1).unwrap_or(0) as u16;
        let tur_ec = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TURBO_PL1).unwrap_or(0) as u16;
        if off_ec > 0 || bal_ec > 0 || tur_ec > 0 {
            (
                if off_ec > 0 { off_ec } else { 45 },
                if bal_ec > 0 { bal_ec } else { 65 },
                if tur_ec > 0 { tur_ec } else { 95 },
            )
        } else {
            (45, 65, 95)
        }
    };

    let title = format!("{} + {}", cpu_name.trim(), gpu_name.trim());
    let specs = FactorySpecs {
        office: PowerPreset {
            pl1: office_pl1,
            pl2: office_pl1,
            pl4: office_pl1.saturating_add(30),
            temp_offset: 15,
        },
        balanced: PowerPreset {
            pl1: bal_pl1,
            pl2: bal_pl1.saturating_add(25),
            pl4: bal_pl1.saturating_add(45),
            temp_offset: 5,
        },
        turbo: PowerPreset {
            pl1: tur_pl1,
            pl2: tur_pl1.saturating_add(30),
            pl4: tur_pl1.saturating_add(50),
            temp_offset: 5,
        },
        base_tgp,
        max_db,
    };

    let mut prof = ModelProfile {
        mine: true,
        model_id: "DECLARATIVE_HARDWARE_PROFILE".to_string(),
        display_name: title.clone(),
        alias: title,
        project_id: 0,
        bios_project_id: "TRUTH_SYNTHESIZED".to_string(),
        factory_specs: Some(specs),
        presets: None,
        fan_tables: None,
        pl1_default: bal_pl1,
        pl2_default: bal_pl1.saturating_add(25),
        pl4_default: bal_pl1.saturating_add(45),
        base_tgp,
        max_db,
        max_fan_duty: 100,
        temp_levels: 10,
        fan_control_respective: false,
        has_water_cooler: false,
        keyboard_type: String::new(),
        gpu_mux: None,
        lighting: None,
        bus: None,
        takeover: None,
        is_inferred: true,
        sovereignty_version: default_sovereignty_version(),
    };

    TruthReconstructor::enrich_contracts(&mut prof);
    prof
}

/// Deprecated: Renamed to synthesize_hardware_profile under Hardware Truth principles
#[deprecated(note = "Use synthesize_hardware_profile which strictly adheres to Hardware Truth Telemetry")]
#[inline]
pub fn infer_from_hardware(cpu_name: &str, gpu_name: &str) -> ModelProfile {
    synthesize_hardware_profile(cpu_name, gpu_name)
}

/// Retrieve the full active machine Sovereign Profile
pub fn get_sovereign_profile() -> ModelProfile {
    let match_res = detect_model_profile();
    match_res.matched_profile
}

/// Retrieve the active GPU MUX contract
pub fn get_active_gpu_mux_contract() -> GpuMuxContract {
    TruthReconstructor::probe_gpu_mux()
}

/// Retrieve the active Lighting contract
///
/// Profiling the idle background loop showed this being reached twice per 500 ms tick,
/// and each call previously re-loaded models.json and ran five full SetupAPI/HID
/// enumerations. The probe result is stable for a given machine, so it is cached and
/// invalidated whenever the profile list or the HID topology is invalidated.
pub fn get_active_lighting_contract() -> LightingContract {
    // The guard must be dropped before probing: probe_lighting -> get_active_profile ->
    // load_model_profiles takes the PROFILES_CACHE lock, and invalidate_model_profiles_cache
    // takes these two locks in the opposite order, so holding this one here would deadlock.
    let cached = {
        let guard = LIGHTING_CONTRACT_CACHE.read();
        guard.as_ref()
            .filter(|(stamped, _)| stamped.elapsed() < LIGHTING_CONTRACT_TTL)
            .map(|(_, contract)| contract.clone())
    };
    if let Some(contract) = cached {
        return contract;
    }

    let contract = if let Some(prof) = get_active_profile() {
        TruthReconstructor::probe_lighting(&prof.keyboard_type)
    } else {
        TruthReconstructor::probe_lighting("TypeA")
    };
    *LIGHTING_CONTRACT_CACHE.write() = Some((std::time::Instant::now(), contract.clone()));
    contract
}

/// Drop the cached active Lighting contract so the next call re-probes the HID topology.
/// Must be invoked whenever devices are re-enumerated or the hardware override changes.
pub fn invalidate_lighting_contract_cache() {
    *LIGHTING_CONTRACT_CACHE.write() = None;
}

/// Retrieve the active Takeover contract
pub fn get_active_takeover_contract() -> TakeoverContract {
    TruthReconstructor::probe_takeover()
}

/// Hardware Model Detection & Adaptation Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMatchResult {
    pub matched_profile: ModelProfile,
    pub board_product: String,
    pub ec_project_id: u8,
    pub bios_project_id: Option<String>,
    pub is_trusted: bool,
    pub confidence_reason: String,
}

/// Perform multi-signal cross validation to determine machine profile:
/// Loads models.json (which guarantees Index 0 is mine: true) and returns the active profile.
pub fn detect_model_profile() -> ModelMatchResult {
    let board = get_baseboard_product().unwrap_or_else(|| "UNKNOWN".to_string());
    let ec_pid = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_PROJECT_ID).unwrap_or(0);
    let reg_bios_pid = read_registry_bios_project_id();

    // load_model_profiles ensures that mine: true is at index 0 and synced!
    let profiles = load_model_profiles();

    if let Some(mine) = profiles.first() {
        if mine.mine {
            return ModelMatchResult {
                matched_profile: mine.clone(),
                board_product: board,
                ec_project_id: ec_pid,
                bios_project_id: reg_bios_pid,
                is_trusted: true,
                confidence_reason: format!("匹配本机预设 [{}] (代号: {})", mine.alias, mine.model_id),
            };
        }
    }

    // Declarative Driver & Hardware Truth Synthesis
    let cpu_brand = crate::system::cpu_monitor::get_cached_cpu_brand();
    let gpu_name = crate::driver::gpu_bridge::get_gpu_name_cached();
    let mut inferred = synthesize_hardware_profile(&cpu_brand, &gpu_name);
    let is_unadapted = crate::hal::platform_probe::get_platform_channel() == crate::hal::platform_probe::PlatformChannelKind::Unknown;
    if is_unadapted {
        inferred.mine = false;
        inferred.factory_specs = None;
        inferred.presets = None;
    }

    ModelMatchResult {
        matched_profile: inferred,
        board_product: board,
        ec_project_id: ec_pid,
        bios_project_id: reg_bios_pid,
        is_trusted: !is_unadapted,
        confidence_reason: if is_unadapted {
            "当前机型尚未适配声明式驱动契约，已启用安全只读保护，禁止自定义调优".to_string()
        } else {
            let channel = crate::hal::platform_probe::get_platform_channel();
            format!("底层通信通道就绪 [{:?}]，基于物理真理传感器与 VBIOS 遥测驱动硬件", channel)
        },
    }
}


/// Helper: Get BaseBoard Product from Windows Registry
fn get_baseboard_product() -> Option<String> {
    #[cfg(windows)]
    unsafe {
        use windows::core::w;
        use windows::Win32::System::Registry::{
            RegOpenKeyExW, RegQueryValueExW, RegCloseKey, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
        };

        let mut hkey: HKEY = HKEY::default();
        let subkey = w!("HARDWARE\\DESCRIPTION\\System\\BIOS");
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey, 0, KEY_READ, &mut hkey).is_ok() {
            let mut buf = [0u16; 256];
            let mut size = (buf.len() * 2) as u32;
            let mut data_type = REG_SZ;
            let val_name = w!("BaseBoardProduct");

            let res = if RegQueryValueExW(
                hkey,
                val_name,
                None,
                Some(&mut data_type),
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut size),
            )
            .is_ok()
            {
                let len = (size as usize / 2).saturating_sub(1);
                let s = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                if !s.is_empty() && s != "Standard" {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            };
            let _ = RegCloseKey(hkey);
            if res.is_some() {
                return res;
            }
        }
    }
    None
}

/// Helper: Read OEM ItemSupport BIOS_PROJECT_ID from Windows Registry
fn read_registry_bios_project_id() -> Option<String> {
    #[cfg(windows)]
    unsafe {
        use windows::core::w;
        use windows::Win32::System::Registry::{
            RegOpenKeyExW, RegQueryValueExW, RegCloseKey, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
        };

        let mut hkey: HKEY = HKEY::default();
        let subkey = w!("SOFTWARE\\OEM\\GamingCenter2\\ItemSupport");
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey, 0, KEY_READ, &mut hkey).is_ok() {
            let mut buf = [0u16; 128];
            let mut size = (buf.len() * 2) as u32;
            let mut data_type = REG_SZ;
            let val_name = w!("BIOS_PROJECT_ID");

            let res = if RegQueryValueExW(
                hkey,
                val_name,
                None,
                Some(&mut data_type),
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut size),
            )
            .is_ok()
            {
                let len = (size as usize / 2).saturating_sub(1);
                let s = String::from_utf16_lossy(&buf[..len]).trim().to_string();
                if !s.is_empty() {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            };
            let _ = RegCloseKey(hkey);
            return res;
        }
    }
    None
}

/// Generate model-specific fan curve points based on machine temp_levels
pub fn generate_model_fan_curve(_temp_levels: u8) -> Vec<crate::system::config::FanCurvePoint> {
    use crate::system::config::FanCurvePoint;
    vec![
        FanCurvePoint { temp: 45, duty: 25 },
        FanCurvePoint { temp: 60, duty: 45 },
        FanCurvePoint { temp: 70, duty: 60 },
        FanCurvePoint { temp: 80, duty: 80 },
        FanCurvePoint { temp: 90, duty: 100 },
    ]
}

/// Build a fully populated CustomProfileConfig adapted to a specific ModelProfile
pub fn build_model_default_custom_profile(profile: &ModelProfile) -> crate::system::config::CustomProfileConfig {
    let manifest = crate::hal::declarative::load_matched_manifest();
    let ramp_rate = manifest.as_ref().map(|m| m.capabilities.fan_ramp_rate).unwrap_or(true);
    let isolated = manifest.as_ref().map(|m| m.capabilities.fan_independent_channels).unwrap_or(profile.fan_control_respective);
    let super_perf = true;

    crate::system::config::CustomProfileConfig {
        cpu_temp_target_enabled: true,
        cpu_temp_target: 95,
        cpu_pl1_watts: profile.pl1_default,
        cpu_pl2_watts: profile.pl2_default,
        cpu_pl4_watts: profile.pl4_default,
        bios_oc_enabled: false,
        gpu_super_perf: super_perf,
        gpu_ctgp_enabled: false,
        gpu_ctgp_watts: (profile.base_tgp.max(45).min(175)) as u8,
        gpu_db_enabled: profile.max_db > 0,
        gpu_db_watts: profile.max_db.min(25) as u8,
        gpu_core_offset_mhz: 0,
        gpu_mem_offset_mhz: 0,
        fan_ramp_rate: ramp_rate,
        fan_ramp_speed_ms: 300,
        fan_isolated_output: isolated,
        fan_curve_enabled: false,
        cpu_fan_curve: crate::system::config::default_fan_curve(),
        gpu_fan_curve: crate::system::config::default_fan_curve(),
        fan_curve_linked: !isolated,
    }
}

/// Detect the hardware model and return its dedicated default custom profile
pub fn get_model_default_custom_profile() -> crate::system::config::CustomProfileConfig {
    let match_res = detect_model_profile();
    build_model_default_custom_profile(&match_res.matched_profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_profiles_has_ptl() {
        let profiles = get_builtin_profiles();
        let ptl = profiles.iter().find(|p| p.model_id.contains("XxPTL"));
        assert!(ptl.is_some(), "XxPTL should exist in built-in profiles");
        let ptl = ptl.unwrap();
        assert_eq!(ptl.base_tgp, 115);
        assert_eq!(ptl.max_db, 25);
    }

    #[test]
    fn test_synthesize_hardware_profile_truth_baseline() {
        let profile_5060 = synthesize_hardware_profile("Intel(R) Core(TM) Ultra 7 356H", "NVIDIA GeForce RTX 5060 Laptop GPU");
        // Offline / mock environment without live VBIOS telemetry returns safe baseline, never fabricates parameters from model string!
        assert!(profile_5060.pl1_default >= 45);
        assert_eq!(profile_5060.model_id, "DECLARATIVE_HARDWARE_PROFILE");

        let profile_hx = synthesize_hardware_profile("Intel Core i9-14900HX", "NVIDIA GeForce RTX 4090 Laptop GPU");
        // Strict Hardware Truth: base_tgp is governed strictly by live VBIOS/EC telemetry (or 0 if mock offline), never string guessing!
        assert!(profile_hx.base_tgp == 0 || profile_hx.base_tgp >= 50);
        assert!(profile_hx.max_db == 0 || profile_hx.max_db <= 35);
    }

    #[test]
    fn test_json_roundtrip() {
        let profiles = get_builtin_profiles();
        let json = serde_json::to_string_pretty(&profiles).expect("Serialize profiles");
        let parsed: Vec<ModelProfile> = serde_json::from_str(&json).expect("Deserialize profiles");
        assert_eq!(parsed.len(), profiles.len());
        assert_eq!(parsed[0].model_id, profiles[0].model_id);
    }

    #[test]
    fn test_factory_specs_and_presets() {
        let profile_hx = synthesize_hardware_profile("Intel Core i9-14900HX", "NVIDIA GeForce RTX 4090 Laptop GPU");
        assert!(profile_hx.factory_specs.is_some());
        // User presets must be None by default, strictly honoring the Zero-Override rule!
        assert!(profile_hx.presets.is_none());
        let specs = profile_hx.factory_specs.unwrap();
        assert!(specs.office.pl1 >= 30);
        assert!(specs.balanced.pl1 >= 45);
        assert!(specs.turbo.pl1 >= 65);
        assert!(specs.base_tgp == 0 || specs.base_tgp >= 50);
        assert!(specs.max_db == 0 || specs.max_db <= 35);
    }

    #[test]
    fn test_sync_preserves_user_alias_and_presets() {
        let mut profiles = get_builtin_profiles();
        profiles[0].alias = "🎮耀妹16熟了，就喊你来🚀".to_string();
        profiles[0].presets = Some(ModelPresets {
            office: PowerPreset { pl1: 30, pl2: 35, pl4: 60, temp_offset: 20 },
            balanced: PowerPreset { pl1: 65, pl2: 70, pl4: 90, temp_offset: 10 },
            turbo: PowerPreset { pl1: 150, pl2: 160, pl4: 180, temp_offset: 5 },
        });

        sync_active_machine_profile(&mut profiles);

        assert_eq!(profiles[0].alias, "🎮耀妹16熟了，就喊你来🚀");
        assert!(profiles[0].mine);
        let presets = profiles[0].presets.as_ref().unwrap();
        assert_eq!(presets.office.pl1, 30);
        assert_eq!(presets.balanced.pl1, 65);
        assert_eq!(presets.turbo.pl1, 150);
        assert!(profiles[0].factory_specs.is_some());
    }

    #[test]
    fn test_fan_tables_loaded() {
        let profiles = get_builtin_profiles();
        assert_eq!(profiles.len(), 29);

        // Verify GM6IX9B has valid fan tables
        let gm6 = profiles.iter().find(|p| p.model_id == "GM6IX9B").expect("GM6IX9B exists");
        assert!(gm6.fan_tables.is_some());
        let ft = gm6.fan_tables.as_ref().unwrap();
        assert!(ft.office.is_some());
        assert!(ft.balanced.is_some());
        assert!(ft.turbo.is_some());
        assert_eq!(ft.office.as_ref().unwrap().max_duty, 110);
        assert_eq!(ft.balanced.as_ref().unwrap().max_duty, 160);
        assert_eq!(ft.turbo.as_ref().unwrap().max_duty, 180);

        // Verify all 29 models have fan_tables populated
        for p in &profiles {
            assert!(
                p.fan_tables.is_some(),
                "Model {} ({}) should have fan_tables populated",
                p.model_id,
                p.display_name
            );
            let ft = p.fan_tables.as_ref().unwrap();
            assert!(ft.office.is_some(), "Model {} missing office fan table", p.model_id);
            assert!(ft.balanced.is_some(), "Model {} missing balanced fan table", p.model_id);
            assert!(ft.turbo.is_some(), "Model {} missing turbo fan table", p.model_id);
        }
    }

    #[test]
    fn test_brand_token_extraction() {
        assert_eq!(extract_brand_token("JIAOLONG Series-X6xR55xK-B2"), Some("JIAOLONG"));
        assert_eq!(extract_brand_token("YAOSHI Series-X6AR55xY"), Some("YAOSHI"));
        assert_eq!(extract_brand_token("CANGLONG Series-X6xR55xY"), Some("CANGLONG"));
        assert_eq!(extract_brand_token("YILONG 15 Pro"), Some("YILONG"));
        assert_eq!(extract_brand_token("WUJIE 14X Extreme"), Some("WUJIE"));
        assert_eq!(extract_brand_token("UNKNOWN Generic Board"), None);
    }

    #[test]
    fn test_builtin_models_aliases_are_clean() {
        let profiles = get_builtin_profiles();
        for p in &profiles {
            assert!(
                p.alias.is_empty(),
                "Builtin template {} should have empty alias to avoid overriding real hardware name",
                p.model_id
            );
        }
    }

    #[test]
    fn test_sovereign_contracts_assembly() {
        let profile = synthesize_hardware_profile("AMD Ryzen 7 8845H", "NVIDIA GeForce RTX 4070 Laptop GPU");
        assert_eq!(profile.sovereignty_version, "2.0");
        assert!(profile.is_inferred);

        // Verify Level 0 Sovereign Contracts are all assembled
        assert!(profile.gpu_mux.is_some(), "GpuMuxContract must be assembled");
        assert!(profile.lighting.is_some(), "LightingContract must be assembled");
        assert!(profile.bus.is_some(), "CommunicationBusContract must be assembled");
        assert!(profile.takeover.is_some(), "TakeoverContract must be assembled");

        let mux = profile.gpu_mux.as_ref().unwrap();
        assert!(!mux.modes_supported.is_empty());

        let bus = profile.bus.as_ref().unwrap();
        assert!(!bus.bus_type.is_empty());
        assert!(!bus.wmi_device_id.is_empty());

        let takeover = profile.takeover.as_ref().unwrap();
        assert!(!takeover.strategy.is_empty());
    }

    #[test]
    fn test_truth_reconstructor_reconstruct() {
        let reconstructed = TruthReconstructor::reconstruct(
            "Intel Core i9-14900HX",
            "NVIDIA GeForce RTX 4090 Laptop GPU",
            "UNKNOWN_CUSTOM_CHASSIS",
        );

        assert_eq!(reconstructed.model_id, "UNKNOWN_CUSTOM_CHASSIS");
        assert!(reconstructed.pl1_default >= 45);
        assert!(reconstructed.factory_specs.is_some());
        assert!(reconstructed.is_inferred);
        assert_eq!(reconstructed.sovereignty_version, "2.0");

        // Contracts assembled
        assert!(reconstructed.gpu_mux.is_some());
        assert!(reconstructed.lighting.is_some());
        assert!(reconstructed.bus.is_some());
        assert!(reconstructed.takeover.is_some());
    }
}

