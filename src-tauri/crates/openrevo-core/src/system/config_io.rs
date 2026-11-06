use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub static DATA_IO: parking_lot::ReentrantMutex<()> = parking_lot::ReentrantMutex::new(());

pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);
    let mut name = path
        .file_name()
        .ok_or("Invalid storage file name")?
        .to_os_string();
    name.push(format!(
        ".tmp.{}.{}",
        std::process::id(),
        NEXT_WRITE.fetch_add(1, Ordering::Relaxed)
    ));
    let temporary = path.with_file_name(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("Cannot create temporary file: {e}"))?;
    let result = file.write_all(content).and_then(|_| file.sync_all());
    drop(file);
    result
        .and_then(|_| fs::rename(&temporary, path))
        .map_err(|e| {
            let _ = fs::remove_file(&temporary);
            format!("Cannot save {}: {e}", path.display())
        })
}

/// Commit an active profile and config together; restore the profile if the
/// config cannot be replaced. Callers serialize writers with DATA_IO.
pub fn atomic_write_pair(
    first: &Path,
    first_content: &[u8],
    second: &Path,
    second_content: &[u8],
) -> Result<(), String> {
    let original = match fs::read(first) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("Cannot back up {}: {error}", first.display())),
    };
    atomic_write(first, first_content)?;
    if let Err(error) = atomic_write(second, second_content) {
        let restored = match original {
            Some(bytes) => atomic_write(first, &bytes),
            None => fs::remove_file(first).map_err(|restore| restore.to_string()),
        };
        return Err(match restored {
            Ok(()) => error,
            Err(restore) => format!("{error}; profile rollback failed: {restore}"),
        });
    }
    Ok(())
}

pub fn merge_changes(current: &mut Value, baseline: &Value, updated: &Value) {
    if baseline == updated {
        return;
    }
    if let (Some(current), Some(baseline), Some(updated)) = (
        current.as_object_mut(),
        baseline.as_object(),
        updated.as_object(),
    ) {
        for key in baseline.keys().filter(|key| !updated.contains_key(*key)) {
            current.remove(key);
        }
        for (key, value) in updated {
            if let (Some(before), Some(latest)) = (baseline.get(key), current.get_mut(key)) {
                merge_changes(latest, before, value);
            } else if baseline.get(key) != Some(value) {
                current.insert(key.clone(), value.clone());
            }
        }
    } else {
        *current = updated.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn failed_config_commit_restores_previous_profile_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "openrevo-profile-rollback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let profile = dir.join("profile.json");
        let config = dir.join("config.json");
        fs::write(&profile, b"original profile").unwrap();
        fs::create_dir(&config).unwrap();
        assert!(
            atomic_write_pair(&profile, b"updated profile", &config, b"updated config").is_err()
        );
        assert_eq!(fs::read(&profile).unwrap(), b"original profile");
        fs::remove_file(&profile).unwrap();
        fs::remove_dir(&config).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn failed_config_commit_removes_only_the_newly_created_profile() {
        let dir = std::env::temp_dir().join(format!(
            "openrevo-new-profile-rollback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let profile = dir.join("profile.json");
        let config = dir.join("config.json");
        fs::create_dir(&config).unwrap();
        assert!(atomic_write_pair(&profile, b"new profile", &config, b"updated config").is_err());
        assert!(!profile.exists());
        fs::remove_dir(&config).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn regression_merges_independent_config_and_map_edits() {
        let baseline =
            json!({"mode": 2, "theme": "mono", "widgets": {"a": [1, 2], "removed": [0, 0]}});
        let mut current = baseline.clone();
        current["mode"] = json!(3);
        current["widgets"]["b"] = json!([3, 4]);
        let updated = json!({"mode": 2, "theme": "cyber", "widgets": {"a": [5, 6]}});
        merge_changes(&mut current, &baseline, &updated);
        assert_eq!(
            current,
            json!({"mode": 3, "theme": "cyber", "widgets": {"a": [5, 6], "b": [3, 4]}})
        );
        merge_changes(&mut current, &baseline, &baseline);
        assert_eq!(current["mode"], 3);
    }

    #[test]
    fn regression_unchanged_fields_do_not_resurrect_deleted_entries() {
        let baseline = json!({"widgets": {"a": [1, 2]}, "mode": 2});
        let mut current = json!({"widgets": {}, "mode": 2});
        let updated = json!({"widgets": {"a": [1, 2]}, "mode": 3});
        merge_changes(&mut current, &baseline, &updated);
        assert_eq!(current, json!({"widgets": {}, "mode": 3}));
    }
}
