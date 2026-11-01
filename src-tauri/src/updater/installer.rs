// OpenRevo - Auto Updater Downloader and Safe Application Engine
// Pure ASCII compliant

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use sha2::{Digest, Sha256};
use tauri::Emitter;
use super::types::DownloadProgressPayload;

const PORTABLE_UPDATE_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$target = $env:OPENREVO_UPDATE_TARGET
$staged = $env:OPENREVO_UPDATE_STAGED
$backup = $staged + '.backup'
$replaced = $false
try {
    Wait-Process -Id ([int]$env:OPENREVO_UPDATE_PID) -ErrorAction SilentlyContinue
    [IO.File]::Replace($staged, $target, $backup)
    $replaced = $true
    Start-Process -FilePath $target
    [Console]::Out.WriteLine('Portable update completed')
} catch {
    [Console]::Error.WriteLine('Portable update failed: ' + $_.Exception.Message)
    if ($replaced -and [IO.File]::Exists($backup)) {
        try { [IO.File]::Replace($backup, $target, $staged) }
        catch { [Console]::Error.WriteLine('Rollback failed: ' + $_.Exception.Message) }
    }
    try { Start-Process -FilePath $target }
    catch { [Console]::Error.WriteLine('Recovery launch failed: ' + $_.Exception.Message) }
    exit 1
}
"#;

fn validate_update_asset(url: &str, sha256: &str, filename: &str) -> Result<(), String> {
    validate_file_name(filename)?;
    if !filename.to_ascii_lowercase().ends_with(".exe") {
        return Err("Only executable update assets are supported".into());
    }
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("A valid SHA256 checksum is required".into());
    }
    let local_debug = super::checker::is_local_debug_url(url);
    if (!url.starts_with("https://") && !local_debug) || url.chars().any(char::is_control) {
        return Err("Update downloads must use HTTPS".into());
    }
    Ok(())
}

/// Downloads update asset with real-time progress events and SHA256 integrity verification
pub async fn download_update_asset<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    download_url: &str,
    expected_sha256: &str,
    filename: &str,
) -> Result<PathBuf, String> {
    validate_update_asset(download_url, expected_sha256, filename)?;
    let app_handle = app.clone();
    let url = download_url.to_string();
    let expected_hash = expected_sha256.to_string();
    let fname = filename.to_string();

    tokio::task::spawn_blocking(move || {
        download_update_asset_sync(&app_handle, &url, &expected_hash, &fname)
    })
    .await
    .map_err(|e| format!("Worker thread error: {}", e))?
}

fn download_update_asset_sync<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    download_url: &str,
    expected_sha256: &str,
    filename: &str,
) -> Result<PathBuf, String> {
    let mut builder = ureq::builder()
        .https_only(download_url.starts_with("https://"))
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(60))
        .user_agent("OpenRevo-Updater")
        .redirects(5);

    match ureq::native_tls::TlsConnector::new() {
        Ok(tls) => {
            builder = builder.tls_connector(std::sync::Arc::new(tls));
        }
        Err(e) => {
            crate::log_error!("UPDATER", "Failed to initialize native TLS connector: {}", e);
        }
    }

    let agent = builder.build();

    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_nanos();
    let temp_update_dir = std::env::temp_dir().join(format!("openrevo_update_{}_{}", std::process::id(), nonce));
    std::fs::create_dir(&temp_update_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let partial_file_path = temp_update_dir.join(format!("{}.downloading", filename));
    let final_file_path = temp_update_dir.join(filename);

    crate::log_info!("UPDATER", "Downloading update via ureq from: {}", download_url);

    let resp = agent.get(download_url)
        .call()
        .map_err(|e| format!("Download request failed: {}", e))?;

    if resp.status() != 200 {
        return Err(format!("Download failed with HTTP {}", resp.status()));
    }

    let total_size: u64 = resp.header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = resp.into_reader();
    let mut file = File::options().write(true).create_new(true).open(&partial_file_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;

    let start_time = Instant::now();
    let mut last_emit = Instant::now();
    let mut buffer = [0u8; 32768]; // 32KB buffer

    loop {
        let n = reader.read(&mut buffer).map_err(|e| format!("Read chunk failed: {}", e))?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| format!("Write failed: {}", e))?;
        hasher.update(&buffer[..n]);
        downloaded += n as u64;

        if last_emit.elapsed().as_millis() >= 100 || (total_size > 0 && downloaded == total_size) {
            last_emit = Instant::now();
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let speed = if elapsed_secs > 0.0 {
                (downloaded as f64 / elapsed_secs) as u64
            } else {
                0
            };
            let percentage = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };

            let _ = app.emit("updater-progress", DownloadProgressPayload {
                downloaded,
                total: total_size,
                percentage,
                speed_bytes_per_sec: speed,
            });
        }
    }

    file.flush().map_err(|e| format!("Flush failed: {}", e))?;
    drop(file);

    // Verify SHA256 checksum
    let calculated_sha256 = format!("{:x}", hasher.finalize());
    crate::log_info!("UPDATER", "Download complete. Expected SHA: {}, Calculated: {}", expected_sha256, calculated_sha256);

    if !calculated_sha256.eq_ignore_ascii_case(expected_sha256) {
        let _ = std::fs::remove_file(&partial_file_path);
        return Err(format!("Integrity check failed! Expected hash: {}, got: {}", expected_sha256, calculated_sha256));
    }

    // Move to final path
    std::fs::rename(&partial_file_path, &final_file_path).map_err(|e| format!("Failed to finalize file: {}", e))?;

    Ok(final_file_path)
}

/// Applies update by either launching installer silently or hot-swapping portable binary
pub fn apply_update(downloaded_file: &Path, is_installer: bool) -> Result<(), String> {
    if !downloaded_file.exists() {
        return Err("Update file not found".to_string());
    }

    let mut header = [0; 2];
    File::open(downloaded_file).and_then(|mut file| file.read_exact(&mut header))
        .map_err(|e| format!("Cannot read update executable: {}", e))?;
    if header != *b"MZ" {
        return Err("Update asset is not a Windows executable".into());
    }

    if is_installer {
        crate::log_info!("UPDATER", "Launching NSIS installer in silent mode: {:?}", downloaded_file);
        // Launch installer with /S for silent upgrade
        std::process::Command::new(downloaded_file)
            .arg("/S")
            .spawn()
            .map_err(|e| format!("Failed to spawn installer: {}", e))?;

        Ok(())
    } else {
        // Stage beside the current executable, then replace atomically after exit.
        let current_exe = std::env::current_exe().map_err(|e| format!("Failed to locate current exe: {}", e))?;
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?.as_nanos();
        let staged_path = current_exe.with_extension(format!("update.{}", nonce));
        let mut staged = File::options().write(true).create_new(true).open(&staged_path)
            .map_err(|e| format!("Failed to stage update beside executable: {}", e))?;
        std::io::copy(&mut File::open(downloaded_file).map_err(|e| e.to_string())?, &mut staged)
            .and_then(|_| staged.sync_all()).map_err(|e| format!("Failed to stage update: {}", e))?;
        drop(staged);
        let log_path = crate::system::config::get_config_dir().join("update.log");
        let mut log = File::options().create(true).append(true).open(&log_path)
            .map_err(|e| format!("Cannot open update log {}: {}", log_path.display(), e))?;
        writeln!(log, "Starting portable update at {:?}", std::time::SystemTime::now()).map_err(|e| e.to_string())?;

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", PORTABLE_UPDATE_SCRIPT])
                .env("OPENREVO_UPDATE_TARGET", &current_exe)
                .env("OPENREVO_UPDATE_STAGED", &staged_path)
                .env("OPENREVO_UPDATE_PID", std::process::id().to_string())
                .stdin(std::process::Stdio::null())
                .stderr(std::process::Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
                .stdout(std::process::Stdio::from(log))
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| format!("Failed to execute update script: {}", e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_update_requires_safe_executable_and_checksum() {
        let hash = "a".repeat(64);
        assert!(validate_update_asset("https://example.com/update.exe", &hash, "update.exe").is_ok());
        for filename in ["../update.exe", "C:\\update.exe", "\\update.exe", "update.exe:stream", "NUL.exe", "update.7z"] {
            assert!(validate_update_asset("https://example.com/update.exe", &hash, filename).is_err(), "{filename}");
        }
        for invalid_hash in ["", "1234", &"z".repeat(64)] {
            assert!(validate_update_asset("https://example.com/update.exe", invalid_hash, "update.exe").is_err());
        }
        for url in ["http://example.com/update.exe", "file:///update.exe", "https://example.com/\r\n", "http://localhost:80@evil.example/update.exe"] {
            assert!(validate_update_asset(url, &hash, "update.exe").is_err(), "{url}");
        }
    }

    #[test]
    fn regression_rejects_archive_renamed_as_executable() {
        let file = std::env::temp_dir().join(format!("openrevo-invalid-update-{}.exe", std::process::id()));
        std::fs::write(&file, b"PK\x03\x04archive").unwrap();
        assert!(apply_update(&file, false).unwrap_err().contains("not a Windows executable"));
        std::fs::remove_file(file).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn regression_portable_replacement_preserves_backup_and_literal_paths() {
        use std::os::windows::process::CommandExt;
        let dir = std::env::temp_dir().join(format!("openrevo-update-测试-%PATH%-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let target = dir.join("old.exe");
        let staged = dir.join("new.exe");
        std::fs::write(&target, b"old executable").unwrap();
        std::fs::write(&staged, b"new executable").unwrap();
        let script = PORTABLE_UPDATE_SCRIPT.replace("Start-Process -FilePath $target", "");
        let result = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .env("OPENREVO_UPDATE_TARGET", &target)
            .env("OPENREVO_UPDATE_STAGED", &staged)
            .env("OPENREVO_UPDATE_PID", "2147483647")
            .creation_flags(0x08000000)
            .output().unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        assert_eq!(std::fs::read(&target).unwrap(), b"new executable");
        let backup = dir.join("new.exe.backup");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old executable");
        std::fs::remove_file(target).unwrap();
        std::fs::remove_file(backup).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn regression_failed_launch_restores_previous_executable() {
        use std::os::windows::process::CommandExt;
        let dir = std::env::temp_dir().join(format!("openrevo-update-rollback-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let target = dir.join("old.exe");
        let staged = dir.join("new.exe");
        std::fs::write(&target, b"old executable").unwrap();
        std::fs::write(&staged, b"new executable").unwrap();
        let script = PORTABLE_UPDATE_SCRIPT.replacen("Start-Process -FilePath $target", "throw 'Simulated launch failure'", 1)
            .replace("Start-Process -FilePath $target", "");
        let result = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .env("OPENREVO_UPDATE_TARGET", &target).env("OPENREVO_UPDATE_STAGED", &staged)
            .env("OPENREVO_UPDATE_PID", "2147483647")
            .creation_flags(0x08000000).output().unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("Simulated launch failure"));
        assert_eq!(std::fs::read(&target).unwrap(), b"old executable");
        assert_eq!(std::fs::read(&staged).unwrap(), b"new executable");
        std::fs::remove_file(target).unwrap();
        std::fs::remove_file(staged).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn regression_locked_executable_leaves_original_and_download_intact() {
        use std::os::windows::{fs::OpenOptionsExt, process::CommandExt};
        let dir = std::env::temp_dir().join(format!("openrevo-update-locked-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let target = dir.join("old.exe");
        let staged = dir.join("new.exe");
        std::fs::write(&target, b"old executable").unwrap();
        std::fs::write(&staged, b"new executable").unwrap();
        let lock = File::options().read(true).share_mode(0).open(&target).unwrap();
        let script = PORTABLE_UPDATE_SCRIPT.replace("Start-Process -FilePath $target", "");
        let result = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .env("OPENREVO_UPDATE_TARGET", &target).env("OPENREVO_UPDATE_STAGED", &staged)
            .env("OPENREVO_UPDATE_PID", "2147483647")
            .creation_flags(0x08000000).output().unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("Portable update failed"));
        drop(lock);
        assert_eq!(std::fs::read(&target).unwrap(), b"old executable");
        assert_eq!(std::fs::read(&staged).unwrap(), b"new executable");
        std::fs::remove_file(target).unwrap();
        std::fs::remove_file(staged).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}


fn validate_file_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("Invalid file name".into());
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    }) {
        return Err("Reserved Windows device name".into());
    }
    Ok(())
}
