use serde_json::Value;
use std::path::Path;

fn update_file(
    path: &Path,
    update: impl FnOnce(&mut Value) -> Result<(), String>,
) -> Result<(), String> {
    let _io = crate::system::config_io::DATA_IO.lock();
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Cannot read OEM settings {}: {error}",
                path.display()
            ))
        }
    };
    let mut value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid OEM settings {}: {error}", path.display()))?;
    if !value.is_object() {
        return Err(format!(
            "OEM settings must be an object: {}",
            path.display()
        ));
    }
    update(&mut value)?;
    let bytes = serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?;
    crate::system::config_io::atomic_write(path, &bytes)
}

fn update_mode(value: &mut Value, ui_mode: u8) -> Result<(), String> {
    let operating = value
        .get_mut("OperatingMode")
        .ok_or("OEM OperatingMode is missing")?;
    *operating = serde_json::json!(ui_mode - 1);
    Ok(())
}

pub fn sync_oem_main_option(ui_mode: u8) -> Result<(), String> {
    if !(1..=4).contains(&ui_mode) {
        return Err("Invalid OEM power mode".into());
    }
    for path in [
        r"C:\Program Files\OEM\智控中心\AiStoneService\MyControlCenter\UserPofiles\MainOption.json",
        r"C:\Program Files\OEM\机械革命电竞控制台\AiStoneService\MyControlCenter\UserPofiles\MainOption.json",
        r"C:\Program Files\OEM\机械革命控制中心\AiStoneService\MyControlCenter\UserPofiles\MainOption.json",
    ] {
        update_file(Path::new(path), |value| update_mode(value, ui_mode))?;
    }
    Ok(())
}

fn update_profile(
    value: &mut Value,
    profile: &crate::system::config::CustomProfileConfig,
) -> Result<(), String> {
    if value.get("CPU").is_none() && value.get("GPU").is_none() {
        return Err("OEM profile has no CPU or GPU section".into());
    }
    if let Some(cpu) = value.get_mut("CPU") {
        let cpu = cpu
            .as_object_mut()
            .ok_or("OEM CPU settings must be an object")?;
        for (key, number) in [
            ("PL1", profile.cpu_pl1_watts),
            ("PL2", profile.cpu_pl2_watts),
            ("PL4", profile.cpu_pl4_watts),
            ("AmdSPL", profile.cpu_pl1_watts),
            ("AmdSPPT", profile.cpu_pl2_watts),
            ("AmdFPPT", profile.cpu_pl4_watts),
        ] {
            cpu.insert(key.into(), serde_json::json!(number));
        }
    }
    if let Some(gpu) = value.get_mut("GPU") {
        let gpu = gpu
            .as_object_mut()
            .ok_or("OEM GPU settings must be an object")?;
        if profile.gpu_ctgp_enabled && profile.gpu_ctgp_watts > 0 {
            gpu.insert(
                "ConfigurableTGPTarget".into(),
                serde_json::json!(profile.gpu_ctgp_watts),
            );
        }
        gpu.insert(
            "DynamicBoostSwitch".into(),
            serde_json::json!(if profile.gpu_db_enabled { 1 } else { 0 }),
        );
        gpu.insert(
            "DynamicBoost".into(),
            serde_json::json!(profile.gpu_db_watts),
        );
        gpu.insert(
            "CoreClockOffset".into(),
            serde_json::json!(profile.gpu_core_offset_mhz),
        );
        gpu.insert(
            "MemoryClockOffset".into(),
            serde_json::json!(profile.gpu_mem_offset_mhz),
        );
    }
    Ok(())
}

pub fn sync_oem_mode4_profile(
    profile: &crate::system::config::CustomProfileConfig,
) -> Result<(), String> {
    for directory in [
        r"C:\Program Files\OEM\智控中心\AiStoneService\MyControlCenter\UserPofiles",
        r"C:\Program Files\OEM\机械革命电竞控制台\AiStoneService\MyControlCenter\UserPofiles",
        r"C:\Program Files\OEM\机械革命控制中心\AiStoneService\MyControlCenter\UserPofiles",
        r"C:\Program Files\OEM\Control Center\AiStoneService\MyControlCenter\UserPofiles",
        r"C:\Program Files\OEM\Control Center\UniwillService\MyControlCenter\UserPofiles",
    ] {
        for name in [
            "Mode4_Profile1.json",
            "Mode4_Profile2.json",
            "Mode4_Profile3.json",
            "Mode4_Profile4.json",
            "Mode4_Profile5.json",
        ] {
            update_file(&Path::new(directory).join(name), |value| {
                update_profile(value, profile)
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn fixture(content: &[u8]) -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/oem-mode-tests");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!(
            "{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn regression_oem_mode_update_preserves_minified_json_and_unrelated_fields() {
        for original in [
            br#"{"OperatingMode":1,"keep":{"enabled":true}}"#.as_slice(),
            b"{\n  \"OperatingMode\": 1\n}",
        ] {
            let path = fixture(original);
            let mut expected: Value = serde_json::from_slice(original).unwrap();
            expected["OperatingMode"] = serde_json::json!(3);
            update_file(&path, |value| update_mode(value, 4)).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn regression_oem_invalid_schema_and_locked_files_keep_the_original_contents() {
        for original in [
            b"broken".as_slice(),
            b"[]",
            br#"{"other":1}"#,
            br#"{"CPU":[],"GPU":{}}"#,
        ] {
            let path = fixture(original);
            let result = if original.starts_with(b"{\"CPU") {
                update_file(&path, |value| {
                    update_profile(
                        value,
                        &crate::system::config::CustomProfileConfig::default(),
                    )
                })
            } else {
                update_file(&path, |value| update_mode(value, 2))
            };
            assert!(result.is_err());
            assert_eq!(std::fs::read(path).unwrap(), original);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            for share in [0, 1] {
                let original = br#"{"OperatingMode":1}"#;
                let path = fixture(original);
                let locked = std::fs::OpenOptions::new()
                    .read(true)
                    .share_mode(share)
                    .open(&path)
                    .unwrap();
                assert!(update_file(&path, |value| update_mode(value, 3)).is_err());
                drop(locked);
                assert_eq!(std::fs::read(path).unwrap(), original);
            }
        }
    }

    #[test]
    fn regression_oem_profile_update_preserves_other_fields_and_disabled_ctgp() {
        let mut profile = crate::system::config::CustomProfileConfig::default();
        profile.gpu_ctgp_enabled = false;
        let mut value = serde_json::json!({"CPU":{"keep":true},"GPU":{"ConfigurableTGPTarget":77,"keep":"gpu"},"other":[1,2]});
        update_profile(&mut value, &profile).unwrap();
        assert_eq!(
            value["CPU"]["PL1"],
            serde_json::json!(profile.cpu_pl1_watts)
        );
        assert_eq!(value["CPU"]["keep"], true);
        assert_eq!(value["GPU"]["keep"], "gpu");
        assert_eq!(value["other"], serde_json::json!([1, 2]));
        assert_eq!(value["GPU"]["ConfigurableTGPTarget"], 77);
    }
}
