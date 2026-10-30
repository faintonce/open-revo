// OpenRevo - Tongfang Cloud ICC Color Profile Discovery & Retrieval Engine
// Pure ASCII Policy strictly enforced.
// Probes local hardware identity (EDID PnP ID, Panel SN, Motherboard BIOS SN, Physical MAC)
// and queries Uniwill / Aistone OEM cloud calibration servers to retrieve factory ICC/ICM profiles.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const SERVER_UNIWILL: &str = "http://iccprofile.uniwill.com.tw/api/iccprofile/";
pub const SERVER_AISTONE: &str = "http://iccprofile.aistoneglobal.com:8001/api/iccprofile/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudIccMonitorInfo {
    pub pnp: String,
    pub panel_serial: String,
    pub friendly_name: String,
    pub instance_name: String,
    pub is_internal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudIccIdentity {
    pub bios_serial: String,
    pub mac_addresses: Vec<String>,
    pub monitors: Vec<CloudIccMonitorInfo>,
    pub is_50_series: bool,
    pub arch_name: String,
    pub recommended_method: String,
    pub candidates_method1: Vec<String>, // <pnp>_<sn> (50 series)
    pub candidates_method2: Vec<String>, // <pnp>_<mac> (40 series)
    pub candidates_fallback: Vec<String>, // <pnp> / friendly
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedIccItem {
    pub file_name: String,
    pub file_size: usize,
    pub file_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudIccFetchHit {
    pub candidate: String,
    pub server_url: String,
    pub full_url: String,
    pub file_name: String,
    pub is_zip: bool,
    pub file_size: usize,
    pub raw_bytes: Vec<u8>,
    pub icc_profiles: Vec<ExtractedIccItem>,
}

#[derive(Debug, Deserialize)]
struct PsRawMonitor {
    pnp: Option<String>,
    serial: Option<String>,
    friendly: Option<String>,
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PsRawIdentity {
    bios_serial: Option<String>,
    macs: Option<Vec<String>>,
    monitors: Option<Vec<PsRawMonitor>>,
}

/// Retrieve machine identity for cloud ICC lookup.
pub fn get_cloud_icc_identity() -> Result<CloudIccIdentity, String> {
    // 1. Prepare UTF-16 Base64 encoded PowerShell script to avoid shell quotation parsing issues
    let ps_code = r#"
$b = (Get-CimInstance Win32_BIOS -ErrorAction SilentlyContinue).SerialNumber;
$m = @(Get-NetAdapter -Physical -ErrorAction SilentlyContinue | ForEach-Object { ($_.MacAddress -replace '[-:]','').ToUpperInvariant() } | Where-Object { $_ });
$d = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID -ErrorAction SilentlyContinue | ForEach-Object {
    $p = (($_.ManufacturerName | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join '') + (($_.ProductCodeID | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join '');
    $s = (($_.SerialNumberID | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join '');
    if ($s -eq '0') { $s = '' }
    $f = (($_.UserFriendlyName | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join '');
    [pscustomobject]@{ pnp = $p; serial = $s; friendly = $f; raw = $_.InstanceName }
});
[pscustomobject]@{ bios_serial = $b; macs = $m; monitors = $d } | ConvertTo-Json -Compress
"#;

    let utf16_bytes: Vec<u8> = ps_code
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    
    // Custom base64 encoder without external crate dependency
    const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut b64 = String::new();
    let chunks = utf16_bytes.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        let n = (b0 << 16) | (b1 << 8) | b2;
        b64.push(B64_CHARS[(n >> 18) & 63] as char);
        b64.push(B64_CHARS[(n >> 12) & 63] as char);
        if chunk.len() > 1 {
            b64.push(B64_CHARS[(n >> 6) & 63] as char);
        } else {
            b64.push('=');
        }
        if chunk.len() > 2 {
            b64.push(B64_CHARS[n & 63] as char);
        } else {
            b64.push('=');
        }
    }

    #[cfg(target_os = "windows")]
    let output = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("powershell.exe")
            .args(&["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to spawn PowerShell for hardware probe: {}", e))?
    };

    #[cfg(not(target_os = "windows"))]
    return Err("Cloud ICC probe is only supported on Windows".into());

    let raw_stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw_stdout.is_empty() {
        return Err("Empty response from identity probe".into());
    }

    let parsed: PsRawIdentity = serde_json::from_str(&raw_stdout)
        .map_err(|e| format!("Failed to parse identity JSON: {} (raw: {})", e, raw_stdout))?;

    let bios_serial = parsed.bios_serial.unwrap_or_default().trim().to_string();
    let macs: Vec<String> = parsed
        .macs
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    let mut monitors = Vec::new();
    for m in parsed.monitors.unwrap_or_default() {
        let pnp = m.pnp.unwrap_or_default().trim().to_string();
        let serial = m.serial.unwrap_or_default().trim().to_string();
        let friendly = m.friendly.unwrap_or_default().trim().to_string();
        let raw = m.raw.unwrap_or_default().trim().to_string();
        if pnp.is_empty() && friendly.is_empty() {
            continue;
        }
        let is_internal = raw.to_uppercase().contains("UID4353") || raw.to_uppercase().contains("INTERNAL") || monitors.is_empty();
        monitors.push(CloudIccMonitorInfo {
            pnp,
            panel_serial: serial,
            friendly_name: friendly,
            instance_name: raw,
            is_internal,
        });
    }

    // Build candidate arrays
    let mut c1 = Vec::new();
    let mut c2 = Vec::new();
    let mut c3 = Vec::new();

    for mon in &monitors {
        let id = &mon.pnp;
        if id.is_empty() {
            continue;
        }

        // Method 1 (50 series): <pnp>_<sn>
        if !mon.panel_serial.is_empty() {
            c1.push(format!("{}_{}", id, mon.panel_serial));
        }
        if !bios_serial.is_empty() {
            c1.push(format!("{}_{}", id, bios_serial));
        }

        // Method 2 (40 series): <pnp>_<mac>
        for mac in &macs {
            c2.push(format!("{}_{}", id, mac));
        }

        // Fallback: <pnp> / <friendly>
        c3.push(id.clone());
        if !mon.friendly_name.is_empty() && &mon.friendly_name != id {
            c3.push(mon.friendly_name.clone());
        }
    }

    // Deduplicate while preserving order
    let dedup = |vec: Vec<String>| -> Vec<String> {
        let mut out = Vec::new();
        for item in vec {
            if !out.contains(&item) {
                out.push(item);
            }
        }
        out
    };

    // Query declarative hardware ground truth (OemMagicVariable vs UniWillVariable)
    let is_50 = crate::driver::wmi_ec::is_nextgen_oem_magic_ec();
    let arch_name = if is_50 {
        "OemMagicVariable (50系)".to_string()
    } else {
        "UniWillVariable (40系)".to_string()
    };
    let recommended_method = if is_50 { "50".to_string() } else { "40".to_string() };

    Ok(CloudIccIdentity {
        bios_serial,
        mac_addresses: macs,
        monitors,
        is_50_series: is_50,
        arch_name,
        recommended_method,
        candidates_method1: dedup(c1),
        candidates_method2: dedup(c2),
        candidates_fallback: dedup(c3),
    })
}

/// Unpack archive via native Windows tar.exe or PowerShell Expand-Archive
fn unpack_zip_native(archive: &Path, dest: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let tar_res = std::process::Command::new("tar.exe")
            .args(&["-xf", &archive.to_string_lossy(), "-C", &dest.to_string_lossy()])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        if let Ok(status) = tar_res {
            if status.success() {
                return Ok(());
            }
        }

        let ps_cmd = format!(
            "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
            archive.to_string_lossy().replace("'", "''"),
            dest.to_string_lossy().replace("'", "''")
        );

        let ps_res = std::process::Command::new("powershell.exe")
            .args(&["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &ps_cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("PowerShell unzip invocation failed: {}", e))?;

        if ps_res.success() {
            return Ok(());
        }
        Err("Native archive expansion failed".into())
    }
    #[cfg(not(target_os = "windows"))]
    Err("Unpack zip only supported on Windows".into())
}

/// Collect all .icm and .icc files recursively from a directory
fn collect_icc_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(collect_icc_files_recursive(&path));
            } else if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if ext == "icm" || ext == "icc" {
                    results.push(path);
                }
            }
        }
    }
    results
}

/// Verify whether byte array is a valid ICC color profile (signature 'acsp' at offset 36..40)
pub fn is_valid_icc_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 128 {
        return false;
    }
    // ICC Specification: bytes 36..40 contains ASCII signature 'acsp'
    &bytes[36..40] == b"acsp"
}

/// Probe a single candidate against a specific server URL across 3 extensions ('', '.zip', '.icm').
pub fn probe_single_candidate(
    server_base: &str,
    candidate: &str,
    timeout_secs: u64,
) -> Option<CloudIccFetchHit> {
    let base = if server_base.ends_with('/') {
        server_base.to_string()
    } else {
        format!("{}/", server_base)
    };

    let suffixes = ["", ".zip", ".icm"];
    for suffix in suffixes {
        let full_url = format!("{}{}{}", base, candidate, suffix);
        let resp = match ureq::get(&full_url)
            .timeout(Duration::from_secs(timeout_secs))
            .call()
        {
            Ok(r) => r,
            Err(_) => continue,
        };

        if resp.status() != 200 {
            continue;
        }

        let mut bytes = Vec::new();
        if resp.into_reader().read_to_end(&mut bytes).is_err() || bytes.is_empty() {
            continue;
        }

        let is_zip = bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B;
        let is_icc = is_valid_icc_bytes(&bytes);

        if is_zip {
            // Write to temporary zip and extract
            let temp_dir = std::env::temp_dir().join(format!("icc_extract_{}_{}", std::process::id(), fastrand()));
            let temp_zip = temp_dir.join("archive.zip");
            let _ = fs::create_dir_all(&temp_dir);
            if fs::write(&temp_zip, &bytes).is_err() {
                let _ = fs::remove_dir_all(&temp_dir);
                continue;
            }

            let extract_dir = temp_dir.join("out");
            let _ = fs::create_dir_all(&extract_dir);
            let mut extracted_items = Vec::new();

            if unpack_zip_native(&temp_zip, &extract_dir).is_ok() {
                let icc_paths = collect_icc_files_recursive(&extract_dir);
                for path in icc_paths {
                    if path.metadata().map(|m| m.len()).unwrap_or(0) > 128 {
                        if let Ok(content) = fs::read(&path) {
                            if is_valid_icc_bytes(&content) {
                                let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("profile.icm").to_string();
                                let size = content.len();
                                extracted_items.push(ExtractedIccItem {
                                    file_name: fname,
                                    file_size: size,
                                    file_bytes: content,
                                });
                            }
                        }
                    }
                }
            }
            let _ = fs::remove_dir_all(&temp_dir);

            if !extracted_items.is_empty() {
                let safe_zip_name = format!("{}.zip", candidate.trim_end_matches(".zip"));
                let total_size = bytes.len();
                return Some(CloudIccFetchHit {
                    candidate: candidate.to_string(),
                    server_url: server_base.to_string(),
                    full_url,
                    file_name: safe_zip_name,
                    is_zip: true,
                    file_size: total_size,
                    raw_bytes: bytes,
                    icc_profiles: extracted_items,
                });
            }
        } else if is_icc {
            let size = bytes.len();
            let safe_name = format!("{}.icm", candidate.trim_end_matches(".icm"));
            let single_item = ExtractedIccItem {
                file_name: safe_name.clone(),
                file_size: size,
                file_bytes: bytes.clone(),
            };
            return Some(CloudIccFetchHit {
                candidate: candidate.to_string(),
                server_url: server_base.to_string(),
                full_url,
                file_name: safe_name,
                is_zip: false,
                file_size: size,
                raw_bytes: bytes,
                icc_profiles: vec![single_item],
            });
        }
    }
    None
}

fn fastrand() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
