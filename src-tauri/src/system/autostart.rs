use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::time::Duration;
use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY_CURRENT_USER, KEY_SET_VALUE, HKEY,
};

use crate::system::query_cache::ExpiringCache;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const TASK_NAME: &str = "OpenRevo";
const LEGACY_SUB_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const LEGACY_VALUE_NAME: PCWSTR = w!("OpenRevo");

/// Each call spawns a `schtasks.exe`; the telemetry heartbeat asks for this every
/// 1.5 s. The task itself only changes when this app (or the user) creates or deletes
/// it, and those paths invalidate the cache explicitly, so the TTL only bounds how
/// long an *external* change (manual edit in Task Scheduler) takes to show up.
/// Kept deliberately short: `schtasks /query` costs ~50 ms, so a 5 s window still
/// removes >99% of the spawns without a noticeable staleness window.
static AUTOSTART_CACHE: LazyLock<ExpiringCache<bool>> =
    LazyLock::new(|| ExpiringCache::new(Duration::from_secs(5)));

/// Drop the cached autostart state so the next query re-runs `schtasks`.
pub fn invalidate_autostart_cache() {
    AUTOSTART_CACHE.invalidate();
}

/// Checks whether the OpenRevo scheduled task exists in Windows Task Scheduler.
pub fn is_autostart_enabled() -> bool {
    let output = Command::new("schtasks")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/query", "/tn", TASK_NAME])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            return true;
        }
    }
    false
}

/// TTL-cached variant of [`is_autostart_enabled`] for hot polling paths.
/// The underlying task state is refreshed at most once per cache TTL, and always
/// immediately after this app creates or deletes the task.
pub fn is_autostart_enabled_cached() -> bool {
    AUTOSTART_CACHE.get_or_compute(is_autostart_enabled)
}

/// Extracts the registered executable path from the OpenRevo scheduled task XML, if it exists.
pub fn get_scheduled_task_exe_path() -> Option<PathBuf> {
    let output = Command::new("schtasks")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/query", "/tn", TASK_NAME, "/xml"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let xml_str = if output.stdout.starts_with(&[0xFF, 0xFE]) {
        let u16_slice: &[u16] = unsafe {
            std::slice::from_raw_parts(
                output.stdout[2..].as_ptr() as *const u16,
                (output.stdout.len() - 2) / 2,
            )
        };
        String::from_utf16_lossy(u16_slice)
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Extract content within <Command>...</Command>
    let start_tag = "<Command>";
    let end_tag = "</Command>";
    let start = xml_str.find(start_tag)? + start_tag.len();
    let end = xml_str[start..].find(end_tag)? + start;
    let raw_cmd = xml_str[start..end].trim();
    let unquoted = raw_cmd.trim_matches('"').trim();

    if unquoted.is_empty() {
        None
    } else {
        Some(PathBuf::from(unquoted))
    }
}

/// Normalizes path for case-insensitive and slash-agnostic Windows path comparison.
fn normalize_path_str(p: &Path) -> String {
    p.to_string_lossy()
        .replace('/', "\\")
        .trim_matches('"')
        .trim()
        .to_lowercase()
}

/// Automatically checks if the scheduled task exists and points to the current running executable.
/// If the task points to a stale, deleted, or moved previous version (common when updating portable single-file releases),
/// or if config has autostart=true but the task is missing, automatically updates the scheduled task to point to the current executable.
pub fn sync_autostart_target_path() {
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };

    let mut cfg = crate::system::config::load_config();
    let task_exists = is_autostart_enabled();

    if task_exists {
        if let Some(task_exe) = get_scheduled_task_exe_path() {
            let current_norm = normalize_path_str(&current_exe);
            let task_norm = normalize_path_str(&task_exe);

            if current_norm != task_norm || !task_exe.exists() {
                crate::log_info!(
                    "AUTOSTART",
                    "Scheduled task target mismatch (registered: {:?}, current: {:?}, exists={}). Auto-healing scheduled task...",
                    task_exe,
                    current_exe,
                    task_exe.exists()
                );
                let _ = set_autostart_enabled(true);
                if !cfg.autostart {
                    cfg.autostart = true;
                    crate::system::config::save_config(&cfg);
                }
            }
        } else {
            crate::log_warn!(
                "AUTOSTART",
                "Scheduled task exists but could not parse target command. Re-applying current executable path..."
            );
            let _ = set_autostart_enabled(true);
            if !cfg.autostart {
                cfg.autostart = true;
                crate::system::config::save_config(&cfg);
            }
        }
    } else if cfg.autostart {
        crate::log_info!(
            "AUTOSTART",
            "Config has autostart=true but scheduled task is missing. Restoring scheduled task for {:?}...",
            current_exe
        );
        let _ = set_autostart_enabled(true);
    }
}

pub fn set_autostart_enabled(enabled: bool) -> Result<bool, String> {
    clean_legacy_registry();

    if enabled {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_str = exe_path.to_string_lossy();

        let username = std::env::var("USERNAME").unwrap_or_default();
        let userdomain = std::env::var("USERDOMAIN").unwrap_or_default();
        let account = if !userdomain.is_empty() && !username.is_empty() {
            format!("{}\\{}", userdomain, username)
        } else if !username.is_empty() {
            username
        } else {
            "Users".to_string()
        };

        let temp_dir = std::env::temp_dir();
        let xml_path = temp_dir.join("openrevo_task.xml");

        let log_suffix = if crate::is_cli_log_active() && crate::system::config::load_config().log_enabled {
            " --log"
        } else {
            ""
        };
        let task_args = format!("--minimized{}", log_suffix);

        let xml_content = format!(
r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{}</UserId>
      <Delay>PT2S</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>false</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"{}"</Command>
      <Arguments>{}</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
            account, account, exe_str, task_args
        );

        let mut bytes: Vec<u8> = vec![0xFF, 0xFE]; // UTF-16LE BOM
        for u in xml_content.encode_utf16() {
            bytes.push((u & 0xFF) as u8);
            bytes.push(((u >> 8) & 0xFF) as u8);
        }

        let _ = std::fs::write(&xml_path, &bytes);

        let status = Command::new("schtasks")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "/create",
                "/f",
                "/tn",
                TASK_NAME,
                "/xml",
                &xml_path.to_string_lossy(),
            ])
            .status();

        let _ = std::fs::remove_file(&xml_path);

        let success = match status {
            Ok(s) if s.success() => true,
            _ => {
                let fallback = Command::new("schtasks")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args([
                        "/create",
                        "/f",
                        "/tn",
                        TASK_NAME,
                        "/tr",
                        &format!("\"{}\" {}", exe_str, task_args),
                        "/sc",
                        "onlogon",
                        "/delay",
                        "0000:02",
                        "/rl",
                        "highest",
                    ])
                    .status();
                fallback.map(|s| s.success()).unwrap_or(false)
            }
        };

        // Task state changed by us; keep the cached answer in sync instead of
        // waiting for the TTL to expire.
        invalidate_autostart_cache();
        if success {
            Ok(true)
        } else {
            Err("Failed to create OpenRevo startup scheduled task via schtasks".into())
        }
    } else {
        let _ = Command::new("schtasks")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/delete", "/f", "/tn", TASK_NAME])
            .status();
        invalidate_autostart_cache();
        Ok(false)
    }
}

fn clean_legacy_registry() {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, LEGACY_SUB_KEY, 0, KEY_SET_VALUE, &mut hkey).is_ok() {
            let _ = RegDeleteValueW(hkey, LEGACY_VALUE_NAME);
            let _ = RegCloseKey(hkey);
        }
    }
}
