// OpenRevo - Auto Updater Checker & Dual-Source Race Engine
// Pure ASCII compliant

use std::time::Duration;
use semver::Version;
use super::types::{AssetInfo, ReleaseManifest, UpdateCheckResult};

pub const DEFAULT_GITHUB_URL: &str = "https://github.com/faintonce/open-revo/releases/latest/download/latest.json";
pub const DEFAULT_GITEE_URL: &str = "https://gitee.com/faintonce/open-revo/raw/master/latest.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionType {
    Installer,
    Portable,
}

/// Detects whether current running binary is an installed copy or portable copy
pub fn detect_distribution_type() -> DistributionType {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            if parent.join("uninstall.exe").exists() || parent.join("uninst.exe").exists() {
                return DistributionType::Installer;
            }
            let path_str = parent.to_string_lossy().to_lowercase();
            if path_str.contains("program files") {
                return DistributionType::Installer;
            }
        }
    }
    DistributionType::Portable
}

/// Creates a standard ureq agent configured with timeouts and native TLS
pub fn is_local_debug_url(url: &str) -> bool {
    cfg!(debug_assertions) && tauri::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "http" && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
            && url.username().is_empty() && url.password().is_none()
    })
}

pub fn create_http_agent(timeout_ms: u64, url: &str) -> ureq::Agent {
    let mut builder = ureq::builder()
        .https_only(!is_local_debug_url(url))
        .timeout(Duration::from_millis(timeout_ms))
        .timeout_connect(Duration::from_millis(timeout_ms))
        .timeout_read(Duration::from_millis(timeout_ms))
        .user_agent("OpenRevo-Updater");

    match ureq::native_tls::TlsConnector::new() {
        Ok(tls) => {
            builder = builder.tls_connector(std::sync::Arc::new(tls));
        }
        Err(e) => {
            crate::log_error!("UPDATER", "Failed to initialize native TLS connector: {}", e);
        }
    }

    builder.build()
}

/// Single manifest fetch helper with timeout and branch fallback (master <-> main)
fn fetch_single_manifest(url: &str, timeout_ms: u64) -> Result<ReleaseManifest, String> {
    let agent = create_http_agent(timeout_ms, url);

    let try_fetch = |target: &str| -> Result<Option<ReleaseManifest>, String> {
        let resp = match agent.get(target).call() {
            Ok(r) => r,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(e) => return Err(format!("Network request failed: {}", e)),
        };

        if resp.status() == 404 {
            return Ok(None);
        }

        if resp.status() != 200 {
            return Err(format!("Server returned HTTP {}", resp.status()));
        }

        resp.into_json::<ReleaseManifest>()
            .map(Some)
            .map_err(|e| format!("Failed to parse release manifest JSON: {}", e))
    };

    // 1. Primary attempt
    if let Some(manifest) = try_fetch(url)? {
        return Ok(manifest);
    }

    // 2. Branch fallback: master <-> main
    let alt_url = if url.contains("/raw/master/") {
        Some(url.replace("/raw/master/", "/raw/main/"))
    } else if url.contains("/raw/main/") {
        Some(url.replace("/raw/main/", "/raw/master/"))
    } else {
        None
    };

    if let Some(alt) = alt_url {
        crate::log_info!("UPDATER", "URL 404 encountered, attempting alternative branch: {}", alt);
        if let Some(manifest) = try_fetch(&alt)? {
            return Ok(manifest);
        }
    }

    Err(format!("Network request failed: {}: status code 404", url))
}

/// Dual-source race and fallback engine using lightweight threads & mpsc channel
pub fn race_manifest_fetch_sync(
    preferred_source: Option<&str>,
) -> Result<(String, ReleaseManifest), String> {
    // 1. Check local debug mock override (supports both http(s) URL and direct file path)
    if let Ok(mock_url) = std::env::var("OPENREVO_UPDATE_URL") {
        let trimmed = mock_url.trim();
        if !trimmed.is_empty() {
            if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                crate::log_info!("UPDATER", "Using debug mock update URL: {}", trimmed);
                let manifest = fetch_single_manifest(trimmed, 5000)?;
                return Ok(("local_mock".to_string(), manifest));
            } else {
                let clean_path = trimmed.trim_start_matches("file:///").replace('/', "\\");
                let path = std::path::PathBuf::from(&clean_path);
                if path.is_file() {
                    crate::log_info!("UPDATER", "Using debug mock local file: {:?}", path);
                    let content = std::fs::read_to_string(&path)
                        .map_err(|e| format!("Failed to read mock file {:?}: {}", path, e))?;
                    let manifest: ReleaseManifest = serde_json::from_str(&content)
                        .map_err(|e| format!("Failed to parse mock JSON: {}", e))?;
                    return Ok(("local_mock".to_string(), manifest));
                }
            }
        }
    }

    // 2. User explicit preference
    if let Some(pref) = preferred_source {
        match pref.to_lowercase().as_str() {
            "gitee" => {
                let m = fetch_single_manifest(DEFAULT_GITEE_URL, 5000)?;
                return Ok(("gitee".to_string(), m));
            }
            "github" => {
                let m = fetch_single_manifest(DEFAULT_GITHUB_URL, 5000)?;
                return Ok(("github".to_string(), m));
            }
            _ => {}
        }
    }

    // 3. Dual-source race: probe both simultaneously with 2000ms threshold
    crate::log_info!("UPDATER", "Starting dual-source race between Gitee and GitHub via lightweight threads...");

    let (tx, rx) = std::sync::mpsc::channel();
    let tx_gitee = tx.clone();
    let tx_github = tx;

    let _ = std::thread::Builder::new()
        .name("updater-race-gitee".to_string())
        .spawn(move || {
            let res = fetch_single_manifest(DEFAULT_GITEE_URL, 2000);
            let _ = tx_gitee.send(("gitee".to_string(), res));
        });

    let _ = std::thread::Builder::new()
        .name("updater-race-github".to_string())
        .spawn(move || {
            let res = fetch_single_manifest(DEFAULT_GITHUB_URL, 2000);
            let _ = tx_github.send(("github".to_string(), res));
        });

    newest_manifest(rx, Duration::from_millis(5500))
}

fn newest_manifest(
    responses: std::sync::mpsc::Receiver<(String, Result<ReleaseManifest, String>)>,
    timeout: Duration,
) -> Result<(String, ReleaseManifest), String> {
    let deadline = std::time::Instant::now() + timeout;
    let mut newest: Option<(Version, String, ReleaseManifest)> = None;
    let mut errors = Vec::new();
    for _ in 0..2 {
        let response = responses.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()));
        let (source, manifest) = match response {
            Ok(value) => value,
            Err(_) => { errors.push("Manifest probe timed out or disconnected".into()); break; }
        };
        match manifest.and_then(|manifest| {
            Version::parse(&manifest.version).map(|version| (version, manifest)).map_err(|e| e.to_string())
        }) {
            Ok((version, manifest)) => {
                if newest.as_ref().is_none_or(|(old, old_source, _)| version > *old || (version == *old && source == "github" && old_source != "github")) {
                    newest = Some((version, source, manifest));
                }
            }
            Err(error) => errors.push(format!("{}: {}", source, error)),
        }
    }
    newest.map(|(_, source, manifest)| (source, manifest)).ok_or_else(|| errors.join("; "))
}

/// Checks whether an update is available based strictly on SemVer
pub fn evaluate_update(
    local_version: &str,
    _local_commit: &str,
    manifest: &ReleaseManifest,
) -> bool {
    let local_v = match Version::parse(local_version) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let remote_v = match Version::parse(&manifest.version) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Strict semantic version bump (remote_v > local_v)
    remote_v > local_v
}

fn select_update_asset(manifest: &ReleaseManifest, dist_type: DistributionType) -> Result<(&'static str, &AssetInfo), String> {
    let keys = if dist_type == DistributionType::Installer {
        ["installer", "binary_only"]
    } else {
        ["binary_only", "installer"]
    };
    keys.into_iter().find_map(|key| {
        manifest.assets.get(key)
            .filter(|asset| asset.filename.to_ascii_lowercase().ends_with(".exe"))
            .map(|asset| (key, asset))
    }).ok_or_else(|| "No executable update asset found; archive updates are not supported".into())
}

fn check_for_updates_sync(preferred_source: Option<&str>) -> Result<UpdateCheckResult, String> {
    let (chosen_source, manifest) = race_manifest_fetch_sync(preferred_source)?;

    let local_version = env!("CARGO_PKG_VERSION");
    let local_commit = option_env!("OPENREVO_GIT_COMMIT").unwrap_or("");
    let dist_type = detect_distribution_type();

    let has_update = evaluate_update(local_version, local_commit, &manifest);

    // Pick suitable asset
    let (asset_key, asset_info) = select_update_asset(&manifest, dist_type)?;

    // Pick download URL based on chosen source
    let download_url = if chosen_source == "gitee" && !asset_info.sources.gitee.is_empty() {
        asset_info.sources.gitee.clone()
    } else if !asset_info.sources.github.is_empty() {
        asset_info.sources.github.clone()
    } else if let Ok(mock_url) = std::env::var("OPENREVO_UPDATE_URL") {
        let base = mock_url.rsplit_once('/').map(|(b, _)| b).unwrap_or("http://127.0.0.1:8080");
        format!("{}/{}", base, asset_info.filename)
    } else {
        asset_info.sources.gitee.clone()
    };

    let display_ver = if !manifest.display_version.is_empty() {
        manifest.display_version.clone()
    } else if !manifest.commit.is_empty() {
        format!("v{}-{}", manifest.version, manifest.commit)
    } else {
        format!("v{}", manifest.version)
    };

    let filename = asset_info.filename.clone();
    let size = asset_info.size;
    let sha256 = asset_info.sha256.clone();
    Ok(UpdateCheckResult {
        has_update,
        current_version: local_version.to_string(),
        current_commit: local_commit.to_string(),
        latest_version: manifest.version,
        latest_commit: manifest.commit,
        display_version: display_ver,
        notes: manifest.notes,
        chosen_source,
        asset_type: asset_key.to_string(),
        filename,
        download_url,
        size,
        sha256,
    })
}

/// Main entry point for checking updates (asynchronous wrapper using spawn_blocking)
pub async fn check_for_updates(preferred_source: Option<String>) -> Result<UpdateCheckResult, String> {
    tokio::task::spawn_blocking(move || {
        check_for_updates_sync(preferred_source.as_deref())
    })
    .await
    .map_err(|e| format!("Worker thread error: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_http_update_exceptions_are_limited_to_local_debug_servers() {
        assert_eq!(is_local_debug_url("http://127.0.0.1:8080/latest.json"), cfg!(debug_assertions));
        for url in ["http://example.com/latest.json", "http://localhost@example.com/latest.json", "http://user@localhost:8080/latest.json", "https://localhost:8080/latest.json"] {
            assert!(!is_local_debug_url(url));
        }
    }

    #[test]
    fn regression_update_probe_does_not_let_a_fast_stale_mirror_hide_newer_releases() {
        let make = |version: &str| serde_json::from_value::<ReleaseManifest>(serde_json::json!({"version":version})).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(("gitee".into(), Ok(make("0.9.3")))).unwrap();
        let next = make("0.9.6");
        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            tx.send(("github".into(), Ok(next))).unwrap();
        });
        assert_eq!(newest_manifest(rx, Duration::from_secs(1)).unwrap().1.version, "0.9.6");
        worker.join().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(("github".into(), Ok(make("invalid")))).unwrap();
        tx.send(("gitee".into(), Ok(make("1.0.0-rc.1")))).unwrap();
        assert_eq!(newest_manifest(rx, Duration::from_secs(1)).unwrap().0, "gitee");
    }

    #[test]
    fn regression_selects_executable_updates_without_archive_fallback() {
        let mut manifest: ReleaseManifest = serde_json::from_value(serde_json::json!({
            "version": "1.0.0",
            "assets": {
                "portable": { "filename": "portable.7z", "sha256": "", "sources": {} }
            }
        })).unwrap();
        assert!(select_update_asset(&manifest, DistributionType::Portable).is_err());
        assert!(select_update_asset(&manifest, DistributionType::Installer).is_err());
        for (key, filename) in [("binary_only", "open-revo.exe"), ("installer", "setup.exe")] {
            manifest.assets.insert(key.into(), AssetInfo {
                filename: filename.into(), size: 0, sha256: "a".repeat(64),
                sources: super::super::types::SourceUrls { github: String::new(), gitee: String::new() },
            });
        }
        assert_eq!(select_update_asset(&manifest, DistributionType::Portable).unwrap().0, "binary_only");
        assert_eq!(select_update_asset(&manifest, DistributionType::Installer).unwrap().0, "installer");
        manifest.assets.remove("binary_only");
        assert_eq!(select_update_asset(&manifest, DistributionType::Portable).unwrap().0, "installer");
    }
}



