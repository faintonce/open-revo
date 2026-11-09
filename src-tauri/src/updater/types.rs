// OpenRevo - Auto Updater Data Models and Contracts
// Pure ASCII compliant

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceUrls {
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub gitee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub filename: String,
    #[serde(default)]
    pub size: u64,
    pub sha256: String,
    pub sources: SourceUrls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    #[serde(default)]
    pub commit: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub display_version: String,
    #[serde(default)]
    pub build_timestamp: i64,
    #[serde(default)]
    pub pub_date: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub assets: HashMap<String, AssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub channel: String,       // "stable" | "beta"
    pub current_version: String,
    pub current_commit: String,
    pub latest_version: String,
    pub latest_commit: String,
    pub display_version: String,
    pub notes: String,
    pub chosen_source: String, // "gitee", "github", or "local_mock"
    pub asset_type: String,    // "binary_only", "portable", "installer"
    pub filename: String,
    pub download_url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgressPayload {
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    pub speed_bytes_per_sec: u64,
}
