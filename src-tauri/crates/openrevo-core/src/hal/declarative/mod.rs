// OpenRevo Declarative Driver Architecture Module
// Manages JSON-driven model discovery, manifest compilation, and dynamic hardware dispatch.

pub mod driver;
pub mod model_manifest;
pub mod wmi_mailbox;

pub use driver::DeclarativeHardwareDriver;
pub use model_manifest::ModelManifest;

const BUILTIN_XINGYAO_14_PTL: &str = include_str!("../../../../../resources/drivers/xingyao_14_p916f_ptl.json");
const BUILTIN_XINGYAO_14: &str = include_str!("../../../../../resources/drivers/xingyao_14_p916f.json");
const BUILTIN_WUJIE_14_IP3: &str = include_str!("../../../../../resources/drivers/wujie_14_ip3.json");
const BUILTIN_WUJIE_14_BLD: &str = include_str!("../../../../../resources/drivers/wujie_14_bld.json");
const BUILTIN_WUJIE_X_SERIES: &str = include_str!("../../../../../resources/drivers/wujie_x_series.json");
const BUILTIN_JIAOLONG_16_BLD: &str = include_str!("../../../../../resources/drivers/jiaolong_16_bld.json");

static CACHED_MANIFEST: std::sync::OnceLock<Option<ModelManifest>> = std::sync::OnceLock::new();

/// Attempts to find and parse a matching declarative model manifest for the current computer.
/// Checks external `%APPDATA%\OpenRevo\plugins\drivers\` first, then falls back to built-in drivers.
/// The result is cached statically to prevent repetitive disk/registry queries and log spamming.
pub fn load_matched_manifest() -> Option<ModelManifest> {
    CACHED_MANIFEST
        .get_or_init(|| load_matched_manifest_internal())
        .clone()
}

fn load_matched_manifest_internal() -> Option<ModelManifest> {
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, HKEY,
        };

        let bios_key: PCWSTR = w!(r"HARDWARE\DESCRIPTION\System\BIOS");
        let mut hkey = HKEY::default();
        if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, bios_key, 0, KEY_READ, &mut hkey).is_ok() } {
            let read_reg = |val_name: PCWSTR| -> String {
                let mut buf = [0u16; 128];
                let mut size = (buf.len() * 2) as u32;
                let mut data_type = REG_SZ;
                let res = unsafe {
                    RegQueryValueExW(
                        hkey,
                        val_name,
                        None,
                        Some(&mut data_type),
                        Some(buf.as_mut_ptr() as *mut _),
                        Some(&mut size),
                    )
                };
                if res.is_ok() {
                    let len = (size as usize / 2).saturating_sub(1);
                    String::from_utf16_lossy(&buf[..len]).trim().to_uppercase()
                } else {
                    String::new()
                }
            };

            let board = read_reg(w!("BaseBoardProduct"));
            let sys_prod = read_reg(w!("SystemProductName"));
            let vendor = read_reg(w!("SystemManufacturer"));
            let bios = read_reg(w!("BIOSVersion"));
            unsafe { let _ = RegCloseKey(hkey); }

            // 1. Check external driver plugins in %APPDATA%\OpenRevo\plugins\drivers\
            if let Ok(appdata) = std::env::var("APPDATA") {
                let drivers_dir = std::path::PathBuf::from(appdata)
                    .join("OpenRevo")
                    .join("plugins")
                    .join("drivers");

                if drivers_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(drivers_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(manifest) = serde_json::from_str::<ModelManifest>(&content) {
                                        if manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                                            crate::log_info!(
                                                "DECLARATIVE_LOADER",
                                                "Matched external driver plugin: {} ({}) for board '{}'",
                                                manifest.name,
                                                manifest.id,
                                                board
                                            );
                                            return Some(manifest);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Exact PTL contract takes priority over the older P916F wildcard.
            if let Ok(manifest) = serde_json::from_str::<ModelManifest>(BUILTIN_XINGYAO_14_PTL) {
                if manifest.matches_identity(&board, &sys_prod, &vendor) && !manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                    crate::log_warn!("DECLARATIVE_LOADER", "Unvalidated BIOS {}; disabling physical controls for {}", bios, board);
                    return Some(manifest.read_only());
                }
                if manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                    crate::log_info!("DECLARATIVE_LOADER", "Matched PTL driver manifest: {} for board '{}'", manifest.id, board);
                    return Some(manifest);
                }
            }

            // 2. Check built-in manifest: Xingyao 14 P916F
            if let Ok(manifest) = serde_json::from_str::<ModelManifest>(BUILTIN_XINGYAO_14) {
                if manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                    crate::log_info!(
                        "DECLARATIVE_LOADER",
                        "Matched built-in driver manifest: {} ({}) for board '{}'",
                        manifest.name,
                        manifest.id,
                        board
                    );
                    return Some(manifest);
                }
            }

            // 3. Check built-in manifest: Wujie 14 (Shenzhen IP3)
            if let Ok(manifest) = serde_json::from_str::<ModelManifest>(BUILTIN_WUJIE_14_IP3) {
                // Use pure static DMI matching only. Do NOT gate on is_ip3_available(),
                // which is populated by an async WMI worker thread and may return false
                // during early startup (race condition), causing MockHAL fallback.
                if manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                    crate::log_info!(
                        "DECLARATIVE_LOADER",
                        "Matched built-in driver manifest: {} ({}) for board '{}'",
                        manifest.name,
                        manifest.id,
                        board
                    );
                    return Some(manifest);
                }
            }

            // 4. Check built-in manifest: Wujie 14 Pro (Baolongda T140)
            if let Ok(manifest) = serde_json::from_str::<ModelManifest>(BUILTIN_WUJIE_14_BLD) {
                if manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                    crate::log_info!(
                        "DECLARATIVE_LOADER",
                        "Matched built-in driver manifest: {} ({}) for board '{}'",
                        manifest.name,
                        manifest.id,
                        board
                    );
                    return Some(manifest);
                }
            }

            // 5. Check built-in manifest: Wujie 14X / 15X / 16X (Tongfang X Series)
            if let Ok(manifest) = serde_json::from_str::<ModelManifest>(BUILTIN_WUJIE_X_SERIES) {
                if manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                    crate::log_info!(
                        "DECLARATIVE_LOADER",
                        "Matched built-in driver manifest: {} ({}) for board '{}'",
                        manifest.name,
                        manifest.id,
                        board
                    );
                    return Some(manifest);
                }
            }

            // 6. Check built-in manifest: Jiaolong 16 Pro (Baolongda MRID6)
            if let Ok(manifest) = serde_json::from_str::<ModelManifest>(BUILTIN_JIAOLONG_16_BLD) {
                if manifest.matches_hardware(&board, &sys_prod, &vendor, &bios) {
                    crate::log_info!(
                        "DECLARATIVE_LOADER",
                        "Matched built-in driver manifest: {} ({}) for board '{}'",
                        manifest.name,
                        manifest.id,
                        board
                    );
                    return Some(manifest);
                }
            }
        }
    }

    None
}

/// Helper to invoke a raw buffer command on the active declarative driver's mailbox
pub fn invoke_raw_mailbox(in_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let manifest = load_matched_manifest()
        .ok_or_else(|| "No matched declarative model manifest found".to_string())?;

    let channel = manifest
        .channels
        .values()
        .find_map(|c| match c {
            model_manifest::ChannelDef::WmiMailbox {
                namespace,
                class_name,
                method_name,
                buffer_sizes,
                ..
            } => Some((namespace, class_name, method_name, buffer_sizes)),
            _ => None,
        })
        .ok_or_else(|| "No WmiMailbox channel configured in manifest".to_string())?;

    wmi_mailbox::invoke_wmi_mailbox(
        channel.0,
        channel.1,
        channel.2,
        in_bytes,
        channel.3.in_size,
        channel.3.out_size,
    )
}
