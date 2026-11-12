use std::fs;
use std::path::{Path, PathBuf};

const MARKER: &str = ".openrevo-data";
const OWNER: &[u8] = b"OpenRevo user data v1\n";
const DATA_ENTRIES: &[&str] = &[
    "config.json", "models.json", "profiles", "plugins", "plugin_storage", "custom_effects",
    "color_profiles", "icc_profiles", "display_tuning.json", "color_calibration.json",
];

fn has_marker(dir: &Path) -> bool {
    let marker = dir.join(MARKER);
    fs::symlink_metadata(&marker).is_ok_and(|m| !m.file_type().is_symlink())
        && fs::read(marker).is_ok_and(|bytes| bytes == OWNER)
}

pub fn mark_owned_directory(dir: &Path) -> Result<(), String> {
    if has_marker(dir) { return Ok(()); }
    let mut marker = fs::OpenOptions::new().write(true).create_new(true)
        .open(dir.join(MARKER)).map_err(|e| e.to_string())?;
    use std::io::Write;
    marker.write_all(OWNER).and_then(|_| marker.sync_all()).map_err(|e| e.to_string())
}

pub fn prepare_migration_destination(dir: &Path) -> Result<(), String> {
    if fs::read_dir(dir).map_err(|e| e.to_string())?.any(|entry| entry.map_or(true, |entry| entry.file_name() != MARKER || !has_marker(dir))) {
        return Err("迁移目标必须为空目录，请使用专属子目录，避免覆盖已有数据。".into());
    }
    Ok(())
}

pub fn clearable_entries(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.is_absolute() || dir.parent().is_none() || !has_marker(dir) {
        return Err("拒绝清理未确认归属的目录，请先迁移到专属的 OpenRevo 空目录。".into());
    }
    DATA_ENTRIES.iter().filter(|name| dir.join(name).exists()).map(|name| {
        crate::system::path_safety::resolve_within(dir, name)
    }).collect()
}

pub fn is_standard_directory(dir: &Path) -> bool {
    let mut candidates = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        candidates.push(PathBuf::from(app_data).join("OpenRevo"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() { candidates.push(parent.join("data")); }
    }
    candidates.iter().any(|candidate| {
        candidate == dir && fs::symlink_metadata(dir).is_ok_and(|m| !m.file_type().is_symlink())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_clear_refuses_unowned_data_and_preserves_unknown_files() {
        let dir = std::env::temp_dir().join(format!("openrevo-clear-policy-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("personal.txt"), "KEEP").unwrap();
        fs::write(dir.join("config.json"), "{}").unwrap();
        assert!(clearable_entries(&dir).is_err());
        assert!(prepare_migration_destination(&dir).is_err());
        mark_owned_directory(&dir).unwrap();
        assert_eq!(clearable_entries(&dir).unwrap(), vec![dir.join("config.json")]);
        assert!(prepare_migration_destination(&dir).is_err());
        assert_eq!(fs::read(dir.join("personal.txt")).unwrap(), b"KEEP");
        for name in ["personal.txt", "config.json"] { fs::remove_file(dir.join(name)).unwrap(); }
        assert!(prepare_migration_destination(&dir).is_ok());
        fs::remove_file(dir.join(MARKER)).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
