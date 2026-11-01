use std::sync::LazyLock;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

use crate::system::query_cache::ExpiringCache;

/// Both of these are asked on every telemetry heartbeat (1.5 s). A Toolhelp32
/// process snapshot and a dozen registry/directory probes are cheap individually
/// but pointless at that rate: OEM software only appears or disappears when the
/// user installs, uninstalls, takes over or restores it.
static OEM_RUNNING_CACHE: LazyLock<ExpiringCache<bool>> =
    LazyLock::new(|| ExpiringCache::new(Duration::from_secs(5)));
static OEM_INSTALLED_CACHE: LazyLock<ExpiringCache<bool>> =
    LazyLock::new(|| ExpiringCache::new(Duration::from_secs(60)));

/// Drop cached OEM detection results. Call after taking over or restoring the
/// official control center, so the UI reflects the change immediately.
pub fn invalidate_oem_detection_cache() {
    OEM_RUNNING_CACHE.invalidate();
    OEM_INSTALLED_CACHE.invalidate();
}

/// Check if official Mechrevo/Uniwill background processes are actively running.
/// We check every companion process (including GCUService, GCUBridge, AiStoneService, etc.).
/// If even ONE of them is still running (e.g., user disabled GCUService with Huorong but GCUBridge is still active),
/// this returns true so the user is given the option to take over and stop it!
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

pub fn is_oem_process_name(name: &str) -> bool {
    matches!(
        name,
        "gcubridge.exe"
            | "gcuservice.exe"
            | "aistoneservice.exe"
            | "aistone.exe"
            | "mycontrolcenter.exe"
            | "controlcenteru.exe"
            | "gamingcenteru.exe"
            | "controlcenter.exe"
            | "gamingcenter.exe"
            | "gamingcenter3_cross.exe"
            | "systraycomponent.exe"
            | "ccuwinui.exe"
            | "systemhelper.exe"
            | "defaulttool.exe"
            | "uniwillservice.exe"
            | "unwservice.exe"
            | "osdtpdetect.exe"
            | "displayutil.exe"
            | "sysconutil.exe"
            | "bldhotkeyservice.exe"
            | "bldfnhotkeyutility.exe"
            | "蛟龙游戏控制中心.exe"
            | "jiaolongcc.exe"
    )
}

/// Force terminate all official OEM background processes via pure Win32 API.
/// Takes < 2ms without creating any external taskkill.exe subprocesses!
pub fn kill_oem_processes_native() -> usize {
    let mut killed = 0;
    unsafe {
        if let Ok(handle) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            if handle == INVALID_HANDLE_VALUE {
                return 0;
            }

            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(handle, &mut entry).is_ok() {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();

                    if is_oem_process_name(&name) {
                        if let Ok(p_handle) = OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID) {
                            if TerminateProcess(p_handle, 1).is_ok() {
                                killed += 1;
                            }
                            let _ = CloseHandle(p_handle);
                        }
                    }

                    if Process32NextW(handle, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(handle);
        }
    }
    killed
}

/// Check if official Mechrevo/Uniwill background processes are actively running.
/// We check every companion process (including GCUService, GCUBridge, AiStoneService, etc.).
/// If even ONE of them is still running (e.g., user disabled GCUService with Huorong but GCUBridge is still active),
/// this returns true so the user is given the option to take over and stop it!
///
/// TTL-cached (see [`OEM_RUNNING_CACHE`]) for hot polling paths; use
/// [`probe_oem_service_running`] when an immediate answer is required.
pub fn is_oem_service_running() -> bool {
    OEM_RUNNING_CACHE.get_or_compute(probe_oem_service_running)
}

/// Uncached Toolhelp32 process scan for OEM companion processes.
pub fn probe_oem_service_running() -> bool {
    unsafe {
        if let Ok(handle) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            if handle == INVALID_HANDLE_VALUE {
                return false;
            }

            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(handle, &mut entry).is_ok() {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();

                    if is_oem_process_name(&name) {
                        let _ = CloseHandle(handle);
                        return true;
                    }

                    if Process32NextW(handle, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(handle);
        }
    }
    false
}

/// Check if official Mechrevo/Uniwill Control Center is installed on this system.
/// Uses 3-tier validation: Windows Service registry keys, physical directory presence, and active processes.
/// If the machine has NEVER installed official software (clean Windows install), this returns false
/// and the "接管/恢复官方控制台" button will be completely hidden from the UI!
///
/// TTL-cached (see [`OEM_INSTALLED_CACHE`]) for hot polling paths; use
/// [`probe_oem_installed`] when an immediate answer is required.
pub fn is_oem_installed() -> bool {
    OEM_INSTALLED_CACHE.get_or_compute(probe_oem_installed)
}

/// Uncached 3-tier OEM installation probe (service registry keys, install
/// directories, UWP AppModel keys, running companion processes).
pub fn probe_oem_installed() -> bool {
    use std::path::Path;
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    // Tier 1: Windows Service Registry Keys
    // Even if disabled by Huorong/antivirus or stopped in services.msc, the registry entry remains intact.
    let service_keys = [
        r"SYSTEM\CurrentControlSet\Services\GCUBridge",
        r"SYSTEM\CurrentControlSet\Services\GCUService",
        r"SYSTEM\CurrentControlSet\Services\AiStoneService",
        r"SYSTEM\CurrentControlSet\Services\UniwillService",
        r"SYSTEM\CurrentControlSet\Services\OEMService",
    ];

    for key in service_keys {
        let hkey = HSTRING::from(key);
        let mut subkey = windows::Win32::System::Registry::HKEY::default();
        let res = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                windows::core::PCWSTR(hkey.as_ptr()),
                0,
                KEY_READ,
                &mut subkey,
            )
        };
        if res.is_ok() {
            let _ = unsafe { RegCloseKey(subkey) };
            return true;
        }
    }

    // Tier 2: Physical OEM Installation Directories
    let known_paths = [
        r"C:\Program Files\OEM\智控中心",
        r"C:\Program Files\OEM\机械革命电竞控制台",
        r"C:\Program Files\OEM\机械革命控制中心",
        r"C:\Program Files\OEM\AiStoneService",
        r"C:\Program Files\OEM\ControlCenter",
        r"C:\Program Files\OEM\GamingCenter",
        r"C:\Program Files (x86)\OEM",
        r"C:\Program Files\Uniwill",
        r"C:\Program Files (x86)\Uniwill",
        r"C:\Program Files\ControlCenter",
        r"C:\Program Files (x86)\ControlCenter",
    ];

    for path in known_paths {
        if Path::new(path).exists() {
            return true;
        }
    }

    // Check if C:\Program Files\OEM exists and contains any non-empty subdirectories
    if let Ok(entries) = std::fs::read_dir(r"C:\Program Files\OEM") {
        if entries.filter_map(|e| e.ok()).next().is_some() {
            return true;
        }
    }

    // Tier 3: Check UWP AppModel registry (CCU.WinUI on 50 series, ControlCenter3 on 40/50 series)
    let uwp_keys = [
        r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\SystemAppData\CCU.WinUI_wrbgcf7aesyd8",
        r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\SystemAppData\ControlCenter3_wrbgcf7aesyd8",
        r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\SystemAppData\ControlCenter3_h329z55cwnj8g",
    ];
    for key in uwp_keys {
        let hkey = HSTRING::from(key);
        let mut subkey = windows::Win32::System::Registry::HKEY::default();
        let res = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                windows::core::PCWSTR(hkey.as_ptr()),
                0,
                KEY_READ,
                &mut subkey,
            )
        };
        if res.is_ok() {
            let _ = unsafe { RegCloseKey(subkey) };
            return true;
        }
    }

    // Tier 4: If any OEM process is actively running in memory, it is definitely installed
    if is_oem_service_running() {
        return true;
    }

    false
}

/// Check if a specific process is actively running (case-insensitive), returning its PID if found.
pub fn is_process_running(target_name: &str) -> Option<u32> {
    let lower_target = target_name.to_lowercase();
    unsafe {
        if let Ok(handle) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            if handle == INVALID_HANDLE_VALUE {
                return None;
            }

            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(handle, &mut entry).is_ok() {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();

                    if name == lower_target {
                        let pid = entry.th32ProcessID;
                        let _ = CloseHandle(handle);
                        return Some(pid);
                    }

                    if Process32NextW(handle, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(handle);
        }
    }
    None
}

/// Retrieve all currently running OEM processes with their names and PIDs.
pub fn get_running_oem_processes() -> Vec<(String, u32)> {
    let mut procs = Vec::new();
    unsafe {
        if let Ok(handle) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            if handle != INVALID_HANDLE_VALUE {
                let mut entry: PROCESSENTRY32W = std::mem::zeroed();
                entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

                if Process32FirstW(handle, &mut entry).is_ok() {
                    loop {
                        let len = entry
                            .szExeFile
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(entry.szExeFile.len());
                        let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();

                        if is_oem_process_name(&name) {
                            procs.push((name, entry.th32ProcessID));
                        }

                        if Process32NextW(handle, &mut entry).is_err() {
                            break;
                        }
                    }
                }
                let _ = CloseHandle(handle);
            }
        }
    }
    procs
}

