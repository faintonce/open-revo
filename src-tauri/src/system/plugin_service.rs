// OpenRevo - Plugin Companion Service Engine
// Manages local daemon/agent background processes declared in plugin manifest.json.
// Handles lifecycle supervision, process sandboxing, port health probing, and error reporting.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginServiceConfig {
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_true")]
    pub elevated: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_true")]
    pub auto_stop: bool,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub health_url: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginServiceStatus {
    pub plugin_id: String,
    pub status: String, // "running" | "starting" | "stopped" | "error"
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub code: Option<String>,
    pub message: Option<String>,
    pub detail: Option<String>,
}

#[allow(dead_code)]
struct ManagedService {
    plugin_id: String,
    pid: u32,
    exe_name: String,
    port: Option<u16>,
    auto_stop: bool,
}

static ACTIVE_SERVICES: Mutex<Option<HashMap<String, ManagedService>>> = Mutex::new(None);

fn with_active_services<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, ManagedService>) -> R,
{
    let mut lock = ACTIVE_SERVICES.lock().unwrap();
    if lock.is_none() {
        *lock = Some(HashMap::new());
    }
    f(lock.as_mut().unwrap())
}

/// Fast non-blocking port probing on loopback interface
pub fn is_port_listening(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

/// Kill process tree by PID using native Win32 API
fn kill_process_by_pid(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                let res = TerminateProcess(handle, 1).is_ok();
                let _ = CloseHandle(handle);
                return res;
            }
        }
    }
    false
}

/// Retrieve the declared service configuration for a plugin
pub fn get_plugin_service_config(plugin_id: &str) -> Option<(PathBuf, PluginServiceConfig)> {
    let plugin_dir = crate::commands::system_cmd::find_plugin_dir(plugin_id);
    let manifest_path = plugin_dir.join("manifest.json");
    if !manifest_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: crate::commands::system_cmd::RawPluginManifest = serde_json::from_str(&content).ok()?;
    manifest.service.map(|svc| (plugin_dir, svc))
}

/// Query current companion service status for a plugin
pub fn get_service_status(plugin_id: &str) -> PluginServiceStatus {
    let Some((_plugin_dir, svc)) = get_plugin_service_config(plugin_id) else {
        return PluginServiceStatus {
            plugin_id: plugin_id.to_string(),
            status: "stopped".to_string(),
            pid: None,
            port: None,
            code: Some("NO_SERVICE_CONFIGURED".to_string()),
            message: Some("插件未声明 companion service 伴随服务".to_string()),
            detail: None,
        };
    };

    let exe_file_name = Path::new(&svc.executable)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 1. Check if recorded in local manager
    let recorded_pid = with_active_services(|map| map.get(plugin_id).map(|m| m.pid));

    // 2. Check if process is actively running in memory
    let active_pid = recorded_pid.or_else(|| {
        if !exe_file_name.is_empty() {
            crate::system::process::is_process_running(&exe_file_name)
        } else {
            None
        }
    });

    // 3. Check port connectivity if port declared
    let port_ok = svc.port.map_or(false, is_port_listening);

    if port_ok || active_pid.is_some() {
        PluginServiceStatus {
            plugin_id: plugin_id.to_string(),
            status: "running".to_string(),
            pid: active_pid,
            port: svc.port,
            code: Some("RUNNING".to_string()),
            message: Some("服务运行中".to_string()),
            detail: None,
        }
    } else {
        PluginServiceStatus {
            plugin_id: plugin_id.to_string(),
            status: "stopped".to_string(),
            pid: None,
            port: svc.port,
            code: Some("STOPPED".to_string()),
            message: Some("服务未运行".to_string()),
            detail: None,
        }
    }
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

/// Start a plugin companion service with sandbox path validation
pub fn start_service(plugin_id: &str) -> Result<PluginServiceStatus, String> {
    if plugin_id == "openrevo-plugin-power" && crate::commands::tuning_cmd::uses_native_cpu_power() {
        return Err("本机已使用内置 CPU 功耗控制，请关闭功耗插件代理以避免同时写入".into());
    }
    let Some((plugin_dir, svc)) = get_plugin_service_config(plugin_id) else {
        return Err(format!("插件 '{}' 未在 manifest.json 中声明 companion service", plugin_id));
    };

    let exe_path = plugin_dir.join(&svc.executable);
    let canon_dir = match canonicalize_safe(&plugin_dir) {
        Ok(d) => d,
        Err(e) => return Ok(PluginServiceStatus {
            plugin_id: plugin_id.to_string(),
            status: "error".to_string(),
            pid: None,
            port: svc.port,
            code: Some("PLUGIN_DIR_NOT_FOUND".to_string()),
            message: Some(format!("插件根目录无效: {}", e)),
            detail: Some(plugin_dir.display().to_string()),
        }),
    };

    let canon_exe = match canonicalize_safe(&exe_path) {
        Ok(p) => p,
        Err(_) => {
            return Ok(PluginServiceStatus {
                plugin_id: plugin_id.to_string(),
                status: "error".to_string(),
                pid: None,
                port: svc.port,
                code: Some("BINARY_NOT_FOUND".to_string()),
                message: Some(format!("未找到服务执行程序: {}", svc.executable)),
                detail: Some(format!("完整搜索路径: {:?}", exe_path)),
            });
        }
    };

    // Rule 1: Anti-Path-Traversal Sandbox Verification
    if !canon_exe.starts_with(&canon_dir) {
        return Ok(PluginServiceStatus {
            plugin_id: plugin_id.to_string(),
            status: "error".to_string(),
            pid: None,
            port: svc.port,
            code: Some("PATH_SECURITY_VIOLATION".to_string()),
            message: Some("安全拦截：伴随服务执行程序必须位于插件根目录树内部".to_string()),
            detail: Some(format!("目标: {:?}, 限制目录: {:?}", canon_exe, canon_dir)),
        });
    }

    let exe_name = canon_exe.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // Check if already running
    let existing_status = get_service_status(plugin_id);
    if existing_status.status == "running" {
        return Ok(existing_status);
    }

    // Launch process silently without opening black console windows
    let working_dir = canon_exe.parent().unwrap_or(&canon_dir);
    let mut cmd = std::process::Command::new(&canon_exe);
    cmd.args(&svc.args);
    cmd.current_dir(working_dir);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let (code, msg) = if e.raw_os_error() == Some(1223) {
                ("UAC_REJECTED", "用户取消了系统管理员提权请求".to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                ("PERMISSION_DENIED", "执行权限被系统拒绝（可能被杀毒软件拦截）".to_string())
            } else {
                ("SPAWN_FAILED", format!("启动进程失败: {}", e))
            };

            return Ok(PluginServiceStatus {
                plugin_id: plugin_id.to_string(),
                status: "error".to_string(),
                pid: None,
                port: svc.port,
                code: Some(code.to_string()),
                message: Some(msg),
                detail: Some(format!("执行目标: {:?}, 错误码: {:?}", canon_exe, e.raw_os_error())),
            });
        }
    };

    let pid = child.id();
    with_active_services(|map| {
        map.insert(plugin_id.to_string(), ManagedService {
            plugin_id: plugin_id.to_string(),
            pid,
            exe_name: exe_name.clone(),
            port: svc.port,
            auto_stop: svc.auto_stop,
        });
    });

    // If port is specified, poll for readiness (up to 2.5 seconds)
    if let Some(port) = svc.port {
        for _ in 0..15 {
            std::thread::sleep(Duration::from_millis(150));
            if is_port_listening(port) {
                break;
            }
        }
    } else {
        std::thread::sleep(Duration::from_millis(150));
    }

    Ok(PluginServiceStatus {
        plugin_id: plugin_id.to_string(),
        status: "running".to_string(),
        pid: Some(pid),
        port: svc.port,
        code: Some("OK".to_string()),
        message: Some("伴随服务启动成功".to_string()),
        detail: None,
    })
}

/// Stop a plugin companion service
pub fn stop_service(plugin_id: &str) -> Result<PluginServiceStatus, String> {
    let svc_info = get_plugin_service_config(plugin_id);
    let exe_name = svc_info.as_ref().and_then(|(_, s)| {
        Path::new(&s.executable).file_name().map(|n| n.to_string_lossy().to_string())
    });
    let port = svc_info.as_ref().and_then(|(_, s)| s.port);

    // 1. Terminate recorded PID
    let recorded_pid = with_active_services(|map| map.remove(plugin_id).map(|m| m.pid));
    if let Some(pid) = recorded_pid {
        kill_process_by_pid(pid);
    }

    // 2. Kill by executable name fallback if still alive
    if let Some(ref name) = exe_name {
        if let Some(pid) = crate::system::process::is_process_running(name) {
            kill_process_by_pid(pid);
        }
    }

    std::thread::sleep(Duration::from_millis(150));

    Ok(PluginServiceStatus {
        plugin_id: plugin_id.to_string(),
        status: "stopped".to_string(),
        pid: None,
        port,
        code: Some("STOPPED".to_string()),
        message: Some("伴随服务已停止".to_string()),
        detail: None,
    })
}

/// Shutdown all managed plugin services on OpenRevo process exit
pub fn shutdown_all_plugin_services() {
    let to_kill = with_active_services(|map| {
        let items: Vec<(u32, String)> = map.values()
            .filter(|m| m.auto_stop)
            .map(|m| (m.pid, m.exe_name.clone()))
            .collect();
        map.clear();
        items
    });

    for (pid, name) in to_kill {
        kill_process_by_pid(pid);
        if let Some(running_pid) = crate::system::process::is_process_running(&name) {
            kill_process_by_pid(running_pid);
        }
    }
}
