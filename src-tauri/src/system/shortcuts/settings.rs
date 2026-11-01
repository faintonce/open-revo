// Persistent on/off switches for the three built-in global shortcuts.
// These settings never execute shortcut actions or change hardware state.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{manager, ShortcutAction, ShortcutBinding};

// Serialize startup registration and IPC reads/updates so concurrent UI requests
// cannot overwrite one another's independently changed switches.
static SETTINGS_LOCK: Mutex<()> = Mutex::new(());
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Definition {
    id: &'static str,
    name: &'static str,
    combo: &'static str,
    action: ShortcutAction,
}

const DEFINITIONS: [Definition; 3] = [
    Definition {
        id: "sys_fan_boost",
        name: "一键全速强冷切换",
        combo: "Ctrl+Shift+F",
        action: ShortcutAction::ToggleFanBoost,
    },
    Definition {
        id: "sys_cycle_power_mode",
        name: "循环切换性能模式",
        combo: "Ctrl+Shift+M",
        action: ShortcutAction::CyclePowerMode,
    },
    Definition {
        id: "sys_toggle_refresh_rate",
        name: "屏幕刷新率瞬切 (高刷/省电)",
        combo: "Ctrl+Shift+R",
        action: ShortcutAction::ToggleRefreshRate,
    },
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SystemShortcutSetting {
    pub id: String,
    pub name: String,
    pub combo: String,
    pub enabled: bool,
}

// A fixed schema rejects unknown IDs and non-boolean values. Missing switches
// keep the original enabled defaults, including when upgrading an older install.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
struct Preferences {
    sys_fan_boost: bool,
    sys_cycle_power_mode: bool,
    sys_toggle_refresh_rate: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            sys_fan_boost: true,
            sys_cycle_power_mode: true,
            sys_toggle_refresh_rate: true,
        }
    }
}

impl Preferences {
    fn enabled(&self, id: &str) -> bool {
        match id {
            "sys_fan_boost" => self.sys_fan_boost,
            "sys_cycle_power_mode" => self.sys_cycle_power_mode,
            "sys_toggle_refresh_rate" => self.sys_toggle_refresh_rate,
            _ => unreachable!("only built-in definitions may be queried"),
        }
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        match id {
            "sys_fan_boost" => self.sys_fan_boost = enabled,
            "sys_cycle_power_mode" => self.sys_cycle_power_mode = enabled,
            "sys_toggle_refresh_rate" => self.sys_toggle_refresh_rate = enabled,
            _ => return Err(format!("未知的内置快捷键 ID：{}", id)),
        }
        Ok(())
    }

    fn settings(&self) -> Vec<SystemShortcutSetting> {
        DEFINITIONS
            .iter()
            .map(|definition| SystemShortcutSetting {
                id: definition.id.into(),
                name: definition.name.into(),
                combo: definition.combo.into(),
                enabled: self.enabled(definition.id),
            })
            .collect()
    }
}

fn settings_path() -> PathBuf {
    crate::system::config::get_config_dir().join("builtin_shortcuts.json")
}

fn read_preferences(path: &Path) -> Result<Preferences, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("内置快捷键配置格式错误，原文件未修改：{}", error)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Preferences::default()),
        Err(error) => Err(format!("读取内置快捷键配置失败：{}", error)),
    }
}

fn write_preferences(path: &Path, preferences: &Preferences) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "内置快捷键配置路径无效".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("创建快捷键配置目录失败：{}", error))?;
    let bytes = serde_json::to_vec_pretty(preferences)
        .map_err(|error| format!("序列化快捷键配置失败：{}", error))?;

    // Write and sync a new sibling file before replacing the previous complete
    // settings file. Never truncate the live settings on a failed/partial write.
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_path = parent.join(format!(
        ".builtin_shortcuts.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let mut temporary_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary_path)
        .map_err(|error| format!("创建快捷键配置临时文件失败：{}", error))?;
    let result = (|| {
        temporary_file.write_all(&bytes)?;
        temporary_file.write_all(b"\n")?;
        temporary_file.sync_all()?;
        drop(temporary_file);
        fs::rename(&temporary_path, path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!("保存内置快捷键配置失败，运行状态未改变：{}", error));
    }
    Ok(())
}

fn binding(definition: &Definition, enabled: bool) -> ShortcutBinding {
    ShortcutBinding {
        id: definition.id.into(),
        name: definition.name.into(),
        combo: definition.combo.into(),
        action: definition.action.clone(),
        osd: true,
        enabled,
        swallow: false,
        plugin_id: None,
    }
}

pub(super) fn init_system_shortcuts() {
    let _guard = SETTINGS_LOCK.lock();
    let preferences = read_preferences(&settings_path()).unwrap_or_else(|error| {
        crate::log_error!("SHORTCUTS", "{}；使用原有的默认快捷键开关", error);
        Preferences::default()
    });
    for definition in &DEFINITIONS {
        manager::register_shortcut(binding(definition, preferences.enabled(definition.id)));
    }
}

pub fn get_system_shortcut_settings() -> Result<Vec<SystemShortcutSetting>, String> {
    let _guard = SETTINGS_LOCK.lock();
    Ok(read_preferences(&settings_path())?.settings())
}

fn persist_then_apply<F>(
    path: &Path,
    preferences: &Preferences,
    apply: F,
) -> Result<(), String>
where
    F: FnOnce(),
{
    write_preferences(path, preferences)?;
    apply();
    Ok(())
}

fn updated_preferences(path: &Path, id: &str, enabled: bool) -> Result<Preferences, String> {
    if !DEFINITIONS.iter().any(|definition| definition.id == id) {
        return Err(format!("未知的内置快捷键 ID：{}", id));
    }
    let mut preferences = read_preferences(path)?;
    preferences.set_enabled(id, enabled)?;
    Ok(preferences)
}

pub fn set_system_shortcut_enabled(
    id: String,
    enabled: bool,
) -> Result<Vec<SystemShortcutSetting>, String> {
    let _guard = SETTINGS_LOCK.lock();
    // Validate the whitelist before resolving/creating the user's config dir.
    let definition = DEFINITIONS
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| format!("未知的内置快捷键 ID：{}", id))?;
    let path = settings_path();
    let preferences = updated_preferences(&path, &id, enabled)?;
    persist_then_apply(&path, &preferences, || {
        if !manager::set_shortcut_enabled(&id, enabled) {
            // A custom plugin may have unregistered this built-in binding.
            // Reinstating the binding only changes registration, never dispatches.
            manager::register_shortcut(binding(definition, enabled));
        }
    })?;
    Ok(preferences.settings())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "openrevo-shortcuts-test-{}-{}-{}",
                std::process::id(), stamp, sequence
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn settings_path(&self) -> PathBuf {
            self.0.join("builtin_shortcuts.json")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_keeps_defaults_in_fixed_order() {
        let directory = TestDirectory::new();
        let settings = read_preferences(&directory.settings_path()).unwrap().settings();
        let actual: Vec<_> = settings
            .iter()
            .map(|setting| (setting.id.as_str(), setting.combo.as_str(), setting.enabled))
            .collect();
        assert_eq!(actual, vec![
            ("sys_fan_boost", "Ctrl+Shift+F", true),
            ("sys_cycle_power_mode", "Ctrl+Shift+M", true),
            ("sys_toggle_refresh_rate", "Ctrl+Shift+R", true),
        ]);
        assert!(!directory.settings_path().exists());
    }

    #[test]
    fn saved_false_survives_reload_and_independent_updates() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        fs::write(&path, br#"{"sys_fan_boost":false}"#).unwrap();
        let preferences = read_preferences(&path).unwrap();
        assert!(!preferences.sys_fan_boost);
        assert!(preferences.sys_cycle_power_mode);
        assert!(preferences.sys_toggle_refresh_rate);
        let updated = updated_preferences(&path, "sys_cycle_power_mode", false).unwrap();
        write_preferences(&path, &updated).unwrap();
        let reloaded = read_preferences(&path).unwrap();
        assert!(!reloaded.sys_fan_boost);
        assert!(!reloaded.sys_cycle_power_mode);
        assert!(reloaded.sys_toggle_refresh_rate);
        // Also verify replacement of an already existing settings file.
        let updated = updated_preferences(&path, "sys_fan_boost", true).unwrap();
        write_preferences(&path, &updated).unwrap();
        assert_eq!(read_preferences(&path).unwrap(), updated);
    }

    #[test]
    fn unknown_id_cannot_create_or_modify_settings() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        assert!(updated_preferences(&path, "plugin_shortcut", false).is_err());
        assert!(!path.exists());
        let original = br#"{"sys_fan_boost":false}"#;
        fs::write(&path, original).unwrap();
        assert!(updated_preferences(&path, "sys_fan_boost/../config", true).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn invalid_configuration_is_reported_and_preserved() {
        let directory = TestDirectory::new();
        let path = directory.settings_path();
        for original in [
            "{broken json",
            r#"{"sys_fan_boost":"false"}"#,
            r#"{"unknown_shortcut":false}"#,
            r#"{"sys_fan_boost":null}"#,
        ] {
            fs::write(&path, original).unwrap();
            assert!(read_preferences(&path).is_err());
            assert!(updated_preferences(&path, "sys_fan_boost", false).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
        }
    }

    #[test]
    fn failed_save_does_not_apply_runtime_change() {
        let directory = TestDirectory::new();
        let blocked_parent = directory.0.join("not_a_directory");
        fs::write(&blocked_parent, b"unchanged").unwrap();
        let applied = Cell::new(false);
        let result = persist_then_apply(
            &blocked_parent.join("builtin_shortcuts.json"),
            &Preferences::default(),
            || applied.set(true),
        );
        assert!(result.is_err());
        assert!(!applied.get());
        assert_eq!(fs::read(&blocked_parent).unwrap(), b"unchanged");
    }
}
