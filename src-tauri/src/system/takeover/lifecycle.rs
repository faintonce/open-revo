//! OEM Service and Process Lifecycle Controller
//!
//! Enforces Level 1.1 Specification from Sovereign Architecture Blueprint:
//! - Complete pause and resume sequences for official control centers
//! - Dynamic SCM state synchronization and watchdog cascading
//! - Multi-tier Truth Sniffer execution for seamless restoration
//! - Automatic bypass on machines without OEM software (Thin & Light / APU)

use std::process::Command;
use std::time::Instant;

#[cfg(target_os = "windows")]
use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;

use super::sandbox::{
    disable_scheduled_tasks, enable_scheduled_tasks, neutralize_run_keys, refresh_tray_notification_area,
    reg_key_exists, restore_run_keys, set_reg_dword, set_uwp_appmodel_startup_state, update_uwp_startup_task,
};
use super::sniffer::{find_official_control_center, ControlCenterLocation};

pub const OEM_SERVICES: &[&str] = &[
    "GCUBridge",
    "GCUService",
    "AiStoneService",
    "UniwillService",
    "OEMService",
    "BLDHotKeyService",
];

pub const OEM_PROCESSES: &[&str] = &[
    "gamingcenter.exe",
    "gamingcenteru.exe",
    "controlcenter.exe",
    "controlcenteru.exe",
    "ccu.exe",
    "ccu.winui.exe",
    "gcuservice.exe",
    "gcubridge.exe",
    "aistone.exe",
    "aistoneservice.exe",
    "mycontrolcenter.exe",
    "uniwillservice.exe",
    "osdtpdetect.exe",
    "systraycomponent.exe",
    "bldhotkeyservice.exe",
    "bldfnhotkeyutility.exe",
    "蛟龙游戏控制中心.exe",
    "jiaolongcc.exe",
];

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
fn run_cmd_captured(cmd: &str, args: &[&str]) -> (Option<i32>, String, String) {
    use std::os::windows::process::CommandExt;
    match Command::new(cmd).args(args).creation_flags(CREATE_NO_WINDOW).output() {
        Ok(out) => {
            let code = out.status.code();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            (code, stdout, stderr)
        }
        Err(e) => (None, String::new(), e.to_string()),
    }
}

/// Checks whether any official companion service or process is currently active
pub fn is_oem_running() -> bool {
    crate::system::process::is_oem_service_running()
}

/// Checks whether official control center software is installed on this machine
pub fn is_oem_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        for svc in OEM_SERVICES {
            let key_path = format!("SYSTEM\\CurrentControlSet\\Services\\{}", svc);
            if reg_key_exists(HKEY_LOCAL_MACHINE, &key_path) {
                return true;
            }
        }
        find_official_control_center().is_some()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Pauses official OEM services, tasks, processes, and neutralizes autostart (Takeover)
pub fn pause_oem_services() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        crate::log_info!("TAKEOVER", "=== Initiating OEM Service Takeover Sequence (Pause) ===");

        // 1. Neutralize autostart Run registry keys
        neutralize_run_keys();

        // 2. Disable UWP Modern AppModel tasks
        set_uwp_appmodel_startup_state(true);
        update_uwp_startup_task(true);

        // 3. Disable all 12 scheduled tasks
        disable_scheduled_tasks();

        // 4. Configure services to demand (Start=3) and stop them
        for svc in OEM_SERVICES {
            let key_path = format!(r"SYSTEM\CurrentControlSet\Services\{}", svc);
            if reg_key_exists(HKEY_LOCAL_MACHINE, &key_path) {
                // Set to Demand (3)
                set_reg_dword(HKEY_LOCAL_MACHINE, &key_path, "Start", 3);
                let _ = run_cmd_captured("sc.exe", &["config", svc, "start=", "demand"]);
                let _ = run_cmd_captured("sc.exe", &["stop", svc]);
            }
        }

        // 5. Terminate all companion OEM processes natively
        let killed = crate::system::process::kill_oem_processes_native();
        crate::log_info!("TAKEOVER", "Terminated {} lingering OEM processes natively.", killed);

        // 6. Refresh notification area to eliminate ghost tray icons
        refresh_tray_notification_area();

        crate::log_info!("TAKEOVER", "=== OEM Service Takeover Sequence Completed Successfully ===");
        // OEM processes were just terminated; drop the cached detection results so the
        // UI reflects the takeover on the very next heartbeat instead of after the TTL.
        crate::system::process::invalidate_oem_detection_cache();
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

/// Authoritatively restores official OEM services, tasks, autostarts, and relaunches control center
pub fn resume_oem_services() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let start_instant = Instant::now();
        crate::log_info!("TAKEOVER", "=== Initiating OEM Service Restoration Sequence (Resume) ===");

        // Fast Bypass: If machine does not have OEM control center, exit silently
        if !is_oem_installed() {
            crate::log_info!("TAKEOVER", "No OEM software installed on this platform. Restoration bypassed cleanly.");
            return Ok(());
        }

        // 1. Terminate remaining OEM processes to guarantee a clean slate
        let _ = crate::system::process::kill_oem_processes_native();

        // 2. Refresh tray notification area
        refresh_tray_notification_area();

        // 3. Restore Run registry keys from OpenRevoBackup_
        restore_run_keys();

        // 4. Re-enable UWP Modern Startup Tasks
        set_uwp_appmodel_startup_state(false);
        update_uwp_startup_task(false);

        // 5. Re-enable all 12 official Scheduled Tasks
        enable_scheduled_tasks();

        // 6. Reconfigure services to Automatic startup (Registry Start=2 + sc.exe config)
        for svc in OEM_SERVICES {
            let key_path = format!(r"SYSTEM\CurrentControlSet\Services\{}", svc);
            if reg_key_exists(HKEY_LOCAL_MACHINE, &key_path) {
                set_reg_dword(HKEY_LOCAL_MACHINE, &key_path, "Start", 2);
                let _ = run_cmd_captured("sc.exe", &["config", svc, "start=", "auto"]);
            }
        }

        // 7. Start primary service (GCUBridge) and track GCUService auto-spawn
        let gcubridge_installed = reg_key_exists(HKEY_LOCAL_MACHINE, r"SYSTEM\CurrentControlSet\Services\GCUBridge");
        let mut gcubridge_active = false;

        if gcubridge_installed {
            if let Some(_) = crate::system::process::is_process_running("gcubridge.exe") {
                gcubridge_active = true;
            } else {
                let _ = run_cmd_captured("sc.exe", &["start", "GCUBridge"]);
                let wait_start = Instant::now();
                while wait_start.elapsed().as_millis() < 3000 {
                    if crate::system::process::is_process_running("gcubridge.exe").is_some() {
                        gcubridge_active = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
            }
        }

        // 8. Track GCUService auto-spawn
        let mut gcuservice_active = crate::system::process::is_process_running("gcuservice.exe").is_some();
        if !gcuservice_active && gcubridge_active {
            let gcu_wait_start = Instant::now();
            while gcu_wait_start.elapsed().as_millis() < 6000 {
                if crate::system::process::is_process_running("gcuservice.exe").is_some() {
                    gcuservice_active = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }

        // 9. Start remaining companion OEM services
        for svc in OEM_SERVICES {
            if *svc != "GCUBridge" && *svc != "GCUService" {
                let key_path = format!(r"SYSTEM\CurrentControlSet\Services\{}", svc);
                if reg_key_exists(HKEY_LOCAL_MACHINE, &key_path) {
                    let _ = run_cmd_captured("sc.exe", &["start", svc]);
                }
            }
        }

        // 10. Service readiness grace period (500ms)
        std::thread::sleep(std::time::Duration::from_millis(500));

        // 11. Trigger official Tray / companion Scheduled Tasks
        let tray_tasks = ["ControlCenterTray", "OSDTpDetect", "AiStone", "CCU", "CCU.WinUI"];
        for task in tray_tasks {
            let key_path = format!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Schedule\TaskCache\Tree\{}", task);
            if reg_key_exists(HKEY_LOCAL_MACHINE, &key_path) {
                let _ = run_cmd_captured("schtasks.exe", &["/run", "/tn", task]);
            }
        }

        // 12. Multi-tier Truth Sniffer execution to relaunch official UI
        if let Some(location) = find_official_control_center() {
            match location {
                ControlCenterLocation::Win32Exe(exe_path) => {
                    crate::log_info!("TAKEOVER", "Relaunching official desktop control center: {:?}", exe_path);
                    use std::os::windows::process::CommandExt;
                    let spawn_res = if let Some(parent_dir) = exe_path.parent() {
                        Command::new(&exe_path)
                            .current_dir(parent_dir)
                            .creation_flags(CREATE_NO_WINDOW)
                            .spawn()
                    } else {
                        Command::new(&exe_path)
                            .creation_flags(CREATE_NO_WINDOW)
                            .spawn()
                    };
                    if let Err(e) = spawn_res {
                        crate::log_warn!("TAKEOVER", "Failed to launch {:?}: {}", exe_path, e);
                    }
                }
                ControlCenterLocation::UwpApp(uwp_pkg) => {
                    crate::log_info!("TAKEOVER", "Relaunching official UWP control center: shell:AppsFolder\\{}", uwp_pkg);
                    use std::os::windows::process::CommandExt;
                    let _ = Command::new("explorer.exe")
                        .args([format!("shell:AppsFolder\\{}", uwp_pkg)])
                        .creation_flags(CREATE_NO_WINDOW)
                        .spawn();
                }
            }
        } else {
            crate::log_warn!("TAKEOVER", "Truth Sniffer could not locate official UI executable to relaunch.");
        }

        // 13. Refresh notification area
        refresh_tray_notification_area();

        // OEM services and processes were just restarted; drop cached detection results
        // so the UI reflects the restoration on the very next heartbeat.
        crate::system::process::invalidate_oem_detection_cache();
        crate::log_info!(
            "TAKEOVER",
            "=== OEM Restoration Completed in {}ms: GCUBridge={}, GCUService={} ===",
            start_instant.elapsed().as_millis(),
            gcubridge_active,
            gcuservice_active
        );
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}
