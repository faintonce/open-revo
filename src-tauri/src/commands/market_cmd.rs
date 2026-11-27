// OpenRevo - Plugin Market Backend Commands
// Provides full-lifecycle capabilities required by the in-app plugin market (plugin-market):
// installation, uninstallation, reload, external browsing, clipboard bridge, and host capabilities probing.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::commands::system_cmd::CustomPluginItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHostCapabilities {
    pub host_version: String,
    pub can_install: bool,
    pub can_uninstall: bool,
    pub can_reload: bool,
    pub can_open_url: bool,
    pub can_write_clipboard: bool,
    pub can_read_manifest: bool,
    pub can_manage_service: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReport {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: String,
}

/// Query host version and capability flags for runtime market adaptation
#[tauri::command]
pub fn get_plugin_host_capabilities() -> PluginHostCapabilities {
    let host_version = env!("CARGO_PKG_VERSION").to_string();
    PluginHostCapabilities {
        host_version,
        can_install: true,
        can_uninstall: true,
        can_reload: true,
        can_open_url: true,
        can_write_clipboard: true,
        can_read_manifest: true,
        can_manage_service: true,
    }
}

/// Safely open an external HTTPS link in the system default web browser
#[tauri::command]
pub fn open_external_url(url: String) -> Result<bool, String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") && !trimmed.starts_with("http://") {
        return Err("Security violation: Only http:// and https:// URLs are permitted".into());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let _ = std::process::Command::new("cmd")
            .args(&["/c", "start", "", trimmed])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to open system browser: {}", e))?;
    }

    Ok(true)
}

/// Write text safely into the Windows clipboard
#[tauri::command]
pub fn write_clipboard(text: String) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut child = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-Command", "$input | Set-Clipboard"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to spawn clipboard process: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }

    Ok(true)
}

/// Re-scan plugins directory and return the freshest list without page switching
#[tauri::command]
pub fn reload_plugins() -> Result<Vec<CustomPluginItem>, String> {
    crate::commands::system_cmd::get_custom_plugins()
}

/// Read manifest.json raw JSON object for a specific plugin ID
#[tauri::command]
pub fn get_plugin_manifest(id: String) -> Result<serde_json::Value, String> {
    if !is_safe_plugin_id(&id) {
        return Err("Invalid plugin ID format".into());
    }

    let plugins_dir = crate::system::config::get_config_dir().join("plugins");
    let manifest_path = plugins_dir.join(&id).join("manifest.json");
    if !manifest_path.exists() {
        return Err(format!("manifest.json not found for plugin '{}'", id));
    }

    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
    let val: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse manifest.json: {}", e))?;
    Ok(val)
}

/// Unified enable/disable control handling skin, background, and extension toggles automatically
#[tauri::command]
pub fn set_plugin_enabled_unified(id: String, enabled: bool) -> Result<bool, String> {
    if !is_safe_plugin_id(&id) {
        return Err("Invalid plugin ID format".into());
    }

    let plugins_dir = crate::system::config::get_config_dir().join("plugins");
    let manifest_path = plugins_dir.join(&id).join("manifest.json");

    let plugin_type = if manifest_path.exists() {
        if let Ok(c) = std::fs::read_to_string(&manifest_path) {
            if let Ok(m) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&c) {
                if enabled {
                    let (is_compat, notice) = crate::commands::system_cmd::check_version_compatibility(m.min_host_version.as_deref());
                    if !is_compat {
                        return Err(notice.unwrap_or_else(|| "Версия хоста ниже требуемой плагином".into()));
                    }
                }
                m.plugin_type.unwrap_or_else(|| "extension".into())
            } else {
                "extension".into()
            }
        } else {
            "extension".into()
        }
    } else {
        "extension".into()
    };

    let mut cfg = crate::system::config::load_config();
    if plugin_type == "skin" {
        cfg.active_skin = if enabled { Some(id.clone()) } else { None };
    } else if plugin_type == "background" {
        cfg.active_background = if enabled { Some(id.clone()) } else { None };
    } else {
        cfg.custom_plugin_toggles.insert(id.clone(), enabled);
        if !enabled {
            let _ = crate::system::plugin_service::stop_service(&id);
        }
    }
    crate::system::config::save_config(&cfg);
    Ok(true)
}

/// Safely uninstall and move plugin folder to system Recycle Bin
#[tauri::command]
pub fn uninstall_plugin(id: String) -> Result<bool, String> {
    if !is_safe_plugin_id(&id) {
        return Err("Security violation: Plugin ID contains illegal characters or path traversal elements".into());
    }

    let plugins_dir = crate::system::config::get_config_dir().join("plugins");
    let target_dir = plugins_dir.join(&id);

    if !target_dir.exists() {
        return Err(format!("Plugin '{}' does not exist in plugins directory", id));
    }

    // Safety check: ensure target is strictly inside plugins_dir
    let canon_plugins = canonicalize_safe(&plugins_dir).map_err(|e| format!("Failed to canonicalize plugins dir: {}", e))?;
    let canon_target = canonicalize_safe(&target_dir).map_err(|e| format!("Failed to canonicalize target dir: {}", e))?;
    if !canon_target.starts_with(&canon_plugins) {
        return Err("Security violation: Target is outside of plugins directory".into());
    }

    // 1. Stop companion service if running & unload declared shortcuts
    let _ = crate::system::plugin_service::stop_service(&id);
    crate::system::shortcuts::unload_plugin_shortcuts(&id);

    // 2. Recycle via Windows Shell API (Inviolable Rule 2: Preserve recoverability)
    move_dir_to_recycle_bin(&canon_target)?;

    // 3. Clear config references
    let mut cfg = crate::system::config::load_config();
    cfg.custom_plugin_toggles.remove(&id);
    if cfg.active_skin.as_deref() == Some(&id) {
        cfg.active_skin = None;
    }
    if cfg.active_background.as_deref() == Some(&id) {
        cfg.active_background = None;
    }
    crate::system::config::save_config(&cfg);

    Ok(true)
}

/// Download, verify, and unpack plugin archive directly into plugins directory
#[tauri::command]
pub fn install_plugin_from_url(
    url: String,
    id: String,
    subpath: Option<String>,
    sha256: Option<String>,
) -> Result<InstallReport, String> {
    let trimmed_url = url.trim();
    if !trimmed_url.starts_with("https://") {
        return Err("Security violation: Plugin download URL must strictly use HTTPS".into());
    }

    if !is_safe_plugin_id(&id) {
        return Err("Security violation: Plugin ID contains illegal characters or path separators".into());
    }

    let plugins_dir = crate::system::config::get_config_dir().join("plugins");
    let _ = std::fs::create_dir_all(&plugins_dir);

    let temp_root = plugins_dir.join(format!(".tmp_install_{}", id));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).map_err(|e| format!("Failed to create temporary folder: {}", e))?;

    let archive_path = temp_root.join("package.zip");
    let extract_dir = temp_root.join("extracted");
    let _ = std::fs::create_dir_all(&extract_dir);

    // Step 1: Download archive via system curl or powershell
    download_file_native(trimmed_url, &archive_path)?;

    // Step 2: Check SHA-256 integrity if requested
    if let Some(expected_hash) = sha256 {
        let expected = expected_hash.trim().to_lowercase();
        if !expected.is_empty() {
            let actual = compute_sha256(&archive_path)?;
            if actual != expected {
                let _ = std::fs::remove_dir_all(&temp_root);
                return Err(format!("Integrity check failed: Expected sha256 {}, but downloaded archive hash is {}", expected, actual));
            }
        }
    }

    // Step 3: Unpack archive
    unpack_archive_native(&archive_path, &extract_dir)?;

    // Step 4: Resolve source root (handle subpath or root wrapper folder e.g. my-plugin-main/)
    let source_dir = resolve_plugin_content_root(&extract_dir, subpath.as_deref())?;
    let manifest_file = source_dir.join("manifest.json");
    if !manifest_file.exists() {
        let _ = std::fs::remove_dir_all(&temp_root);
        return Err("Verification failed: No valid 'manifest.json' detected in plugin root".into());
    }

    // Parse manifest to retrieve display name and version
    let manifest_content = std::fs::read_to_string(&manifest_file)
        .map_err(|e| format!("Failed to read extracted manifest.json: {}", e))?;
    let raw_manifest: crate::commands::system_cmd::RawPluginManifest = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Malformed manifest.json in package: {}", e))?;

    let plugin_name = raw_manifest.name.unwrap_or_else(|| id.clone());
    let plugin_version = raw_manifest.version.unwrap_or_else(|| "1.0.0".into());

    // Step 5: Safely deploy to final plugins/<id> destination
    let final_dest = plugins_dir.join(&id);
    if final_dest.exists() {
        let _ = crate::system::plugin_service::stop_service(&id);
        move_dir_to_recycle_bin(&final_dest)?;
    }

    // Copy directory tree to final destination
    copy_dir_recursive(&source_dir, &final_dest)?;
    let _ = std::fs::remove_dir_all(&temp_root);

    // Step 6: Enable plugin in config
    let mut cfg = crate::system::config::load_config();
    cfg.custom_plugin_toggles.insert(id.clone(), true);
    crate::system::config::save_config(&cfg);

    Ok(InstallReport {
        id,
        name: plugin_name,
        version: plugin_version,
        path: final_dest.to_string_lossy().to_string(),
    })
}

// -----------------------------------------------------------------------------
// Internal Security & Filesystem Helpers
// -----------------------------------------------------------------------------

fn is_safe_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !id.contains("..")
        && !id.starts_with('.')
}

fn canonicalize_safe(p: &Path) -> std::io::Result<PathBuf> {
    let p = std::fs::canonicalize(p)?;
    let s = p.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        Ok(PathBuf::from(stripped))
    } else {
        Ok(p)
    }
}

fn move_dir_to_recycle_bin(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let path_str = path.to_string_lossy();
        let script = format!(
            "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
            path_str.replace("'", "''")
        );

        let status = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("Failed to execute Recycle Bin command: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to safely delete folder to Recycle Bin".into())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())
    }
}

fn download_file_native(url: &str, dest: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Try Windows native curl.exe first
        let curl_res = std::process::Command::new("curl.exe")
            .args(&["-fsSL", url, "-o", &dest.to_string_lossy()])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        if let Ok(status) = curl_res {
            if status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                return Ok(());
            }
        }

        // Fallback to PowerShell Invoke-WebRequest
        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
            url.replace("'", "''"),
            dest.to_string_lossy().replace("'", "''")
        );

        let ps_res = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-Command", &ps_cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("Download failed: {}", e))?;

        if ps_res.success() && dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Ok(());
        }

        Err("Failed to download package archive via curl or powershell".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Unsupported operating system for download".into())
    }
}

fn unpack_archive_native(archive: &Path, dest: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Try native Windows tar.exe (fast and builtin on Windows 10/11)
        let tar_res = std::process::Command::new("tar.exe")
            .args(&["-xf", &archive.to_string_lossy(), "-C", &dest.to_string_lossy()])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        if let Ok(status) = tar_res {
            if status.success() {
                return Ok(());
            }
        }

        // Fallback to PowerShell Expand-Archive
        let ps_cmd = format!(
            "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
            archive.to_string_lossy().replace("'", "''"),
            dest.to_string_lossy().replace("'", "''")
        );

        let ps_res = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-Command", &ps_cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("Unpack failed: {}", e))?;

        if ps_res.success() {
            return Ok(());
        }

        Err("Failed to unpack archive via tar or Expand-Archive".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Unsupported operating system for unpack".into())
    }
}

fn compute_sha256(path: &Path) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let script = format!(
            "(Get-FileHash -LiteralPath '{}' -Algorithm SHA256).Hash.ToLower()",
            path.to_string_lossy().replace("'", "''")
        );

        let output = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to compute file hash: {}", e))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        Err("Get-FileHash returned error".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Unsupported OS for sha256".into())
    }
}

fn resolve_plugin_content_root(extracted_dir: &Path, subpath: Option<&str>) -> Result<PathBuf, String> {
    let mut base = extracted_dir.to_path_buf();

    // 1. If explicit subpath is declared, navigate into it safely
    if let Some(sub) = subpath {
        let clean = sub.trim().trim_start_matches('/').trim_start_matches('\\');
        if clean.contains("..") {
            return Err("Security violation: subpath cannot contain '..'".into());
        }
        base = base.join(clean);
    } else {
        // 2. Auto-detect wrapper folder (e.g. GitHub zip: <repo>-<branch>/manifest.json)
        if !base.join("manifest.json").exists() {
            if let Ok(entries) = std::fs::read_dir(&base) {
                let dirs: Vec<PathBuf> = entries
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.path())
                    .collect();
                if dirs.len() == 1 && dirs[0].join("manifest.json").exists() {
                    base = dirs[0].clone();
                }
            }
        }
    }

    if base.join("manifest.json").exists() {
        Ok(base)
    } else {
        Err(format!("manifest.json not found in resolved content directory: {:?}", base))
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("Failed to create destination dir: {}", e))?;

    let entries = std::fs::read_dir(src).map_err(|e| format!("Failed to read source dir: {}", e))?;
    for entry in entries.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| format!("Failed to copy file {:?}: {}", src_path, e))?;
        }
    }
    Ok(())
}
