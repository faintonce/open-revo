// OpenRevo PBO service — companion sidecar for the pbo-tuner plugin.
//
// Why this exists: the PBO feature originally lived in the OpenRevo host
// (`smu_*` commands in src-tauri), which meant every release needed host
// source changes. This binary carries the same code so the plugin is
// self-contained: the host only has to launch it via the manifest `service`
// block, and nothing in src-tauri has to know about AMD SMU.
//
// Protocol: plain HTTP on 127.0.0.1, one endpoint per former Tauri command.
// The page's `rpc()` helper posts JSON and reads `result` back. No framework,
// no async runtime — std::net plus two serde crates, so the build is fast and
// the binary stays small.

mod pawnio;
mod smu;

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

const VERSION: &str = "2.0.0";
const DEFAULT_PORT: u16 = 18955;

/// Port this instance is actually listening on; reported by /api/health and
/// used when rewriting the manifest on a port change.
static CURRENT_PORT: AtomicU16 = AtomicU16::new(DEFAULT_PORT);

fn main() {
    load_log_flag();
    let port = parse_port();
    log(&format!("pbo-service {VERSION} starting on 127.0.0.1:{port}"));

    // Announce the profile path early: the only way to debug a "profile not
    // found" report is to know which directory the service resolved.
    log(&format!("config dir: {}", config_dir().display()));

    // Port handover: /api/config_port spawns a replacement before the old
    // process releases the listener, so retry the bind briefly to absorb the
    // gap. Cold starts succeed on the first attempt.
    let listener = match bind_with_retry(port) {
        Ok(l) => l,
        Err(e) => {
            log(&format!("bind failed: {e}"));
            std::process::exit(2);
        }
    };
    CURRENT_PORT.store(port, Ordering::Relaxed);
    log(&format!("pbo-service listening on 127.0.0.1:{port}"));

    // The host launches this at boot with auto_start, before any plugin tab
    // exists — that is what makes "apply my profile on startup" possible now
    // that the host no longer runs it.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(8));
        match auto_apply_profile() {
            Ok(msg) => log(&format!("auto-apply: {msg}")),
            Err(e) => log(&format!("auto-apply skipped: {e}")),
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(move || {
                    let _ = handle(s);
                });
            }
            Err(e) => log(&format!("accept failed: {e}")),
        }
    }
}

fn parse_port() -> u16 {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 0;
    while i < args.len() {
        if let Some(v) = args[i].strip_prefix("--port=") {
            return v.parse().unwrap_or(DEFAULT_PORT);
        }
        if args[i] == "--port" {
            if let Some(v) = args.get(i + 1) {
                return v.parse().unwrap_or(DEFAULT_PORT);
            }
        }
        i += 1;
    }
    DEFAULT_PORT
}

fn bind_with_retry(port: u16) -> std::io::Result<TcpListener> {
    let mut last: Option<std::io::Error> = None;
    for _ in 0..25 {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => return Ok(l),
            Err(e) => {
                last = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    }
    Err(last.unwrap_or_else(|| std::io::Error::other("bind failed")))
}

// ---- HTTP plumbing ----

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(15)))?;

    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    let head_end;
    loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(p) = find(&buf, b"\r\n\r\n") {
            head_end = p + 4;
            break;
        }
        if buf.len() > 128 * 1024 {
            return Ok(());
        }
    }

    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("").to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();

    let content_length = lines
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.eq_ignore_ascii_case("content-length") {
                v.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .next()
        .unwrap_or(0);

    let mut body = buf[head_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }

    // A browser page on a different origin (file:// or the host's webview)
    // always preflights a JSON POST.
    if method == "OPTIONS" {
        return write_response(&mut stream, 204, "");
    }

    let result = route(&method, &path, &body);
    match result {
        Ok(v) => write_response(&mut stream, 200, &v.to_string()),
        Err(e) => write_response(&mut stream, 200, &json!({ "__error": e }).to_string()),
    }
}

fn route(method: &str, path: &str, body: &[u8]) -> Result<Value, String> {
    if method == "GET" && path == "/api/health" {
        return Ok(json!({
            "status": "ok",
            "version": VERSION,
            "port": CURRENT_PORT.load(Ordering::Relaxed),
            "cpu": smu::cpu_name(),
        }));
    }
    // Debug aid: the tail of the service log, so a bad CO read/write can be
    // diagnosed straight from the page without hunting for the log file.
    // Accepts any method — the page's rpc() helper always POSTs.
    if path == "/api/debug_log" {
        return Ok(debug_log_tail());
    }
    if method != "POST" {
        return Err(format!("unsupported {method} {path}"));
    }

    let req: Value = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| format!("bad json: {e}"))?
    };

    match path {
        "/api/port_check" => port_check(&req),
        "/api/log_config" => {
            // No `enabled` in the body = status query (the page's rpc() always POSTs).
            match params(&req).get("enabled").and_then(|v| v.as_bool()) {
                Some(on) => {
                    set_log_enabled(on)?;
                    Ok(json!({
                        "enabled": on,
                        "path": log_path().display().to_string(),
                        "max_bytes": LOG_MAX_BYTES,
                    }))
                }
                None => Ok(json!({
                    "enabled": LOG_ENABLED.load(Ordering::Relaxed),
                    "path": log_path().display().to_string(),
                    "max_bytes": LOG_MAX_BYTES,
                })),
            }
        }
        "/api/config_port" => config_port(&req),
        "/api/smu_init" => Ok(serde_json::to_value(smu::init_smu()?).unwrap_or(Value::Null)),
        "/api/smu_get_core_layout" => {
            Ok(serde_json::to_value(smu::get_core_layout()).unwrap_or(Value::Null))
        }
        "/api/smu_get_co_single_core" => {
            let (core, ccd, ccx, phys) = placement(&req)?;
            Ok(json!(smu::get_co_single_core(core, ccd, ccx, phys)?))
        }
        "/api/smu_set_co_single_core" => {
            let (core, ccd, ccx, phys) = placement(&req)?;
            let margin = params(&req)
                .get("margin")
                .and_then(|v| v.as_i64())
                .ok_or("margin missing")? as i32;
            Ok(json!(smu::set_co_single_core(core, ccd, ccx, phys, margin)?))
        }
        "/api/smu_set_co_all_cores" => {
            let margin = req
                .get("margin")
                .and_then(|v| v.as_i64())
                .ok_or("margin missing")? as i32;
            Ok(json!(smu::set_co_all_cores(margin)?))
        }
        "/api/smu_get_pbo_scalar" => Ok(json!(smu::get_pbo_scalar()?)),
        "/api/smu_set_pbo_scalar" => {
            let v = req
                .get("scalarX1000")
                .or_else(|| req.get("scalar_x1000"))
                .and_then(|v| v.as_u64())
                .ok_or("scalarX1000 missing")? as u32;
            Ok(json!(smu::set_pbo_scalar(v)?))
        }
        "/api/smu_get_fmax" => Ok(json!(smu::get_fmax()?)),
        "/api/smu_set_fmax" => {
            let v = req
                .get("frequencyMhz")
                .or_else(|| req.get("frequency_mhz"))
                .and_then(|v| v.as_u64())
                .ok_or("frequencyMhz missing")? as u32;
            Ok(json!(smu::set_fmax(v)?))
        }
        "/api/smu_profile_save" => {
            let content = req
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or("content missing")?;
            Ok(json!(save_profile(content)?))
        }
        "/api/smu_profile_load" => Ok(json!(load_profile()?)),
        other => Err(format!("unknown endpoint {other}")),
    }
}

/// Endpoints take a flat JSON body (`{core, ccd, ccx, margin}`). The page used
/// to wrap them in a `{"request": {...}}` envelope — the shape the host-scoped
/// invoke expected — and a cached page can still send that, so unwrap it here
/// instead of answering "core missing" and silently failing every write.
fn params(req: &Value) -> &Value {
    let flat = req.get("core").is_some() || req.get("margin").is_some();
    match req.get("request") {
        Some(inner) if !flat && inner.is_object() => inner,
        _ => req,
    }
}

/// `{core, ccd, ccx}` — the page always sends all three. `physical_core` is
/// optional: the SMU core slot inside the CCD, which on multi-CCD parts with
/// fused-off cores differs from `core % 8` (the legacy assumption).
fn placement(req: &Value) -> Result<(u32, u32, u32, u32), String> {
    let p = params(req);
    let get = |k: &str| -> Result<u32, String> {
        p.get(k)
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .ok_or_else(|| format!("{k} missing"))
    };
    let core = get("core")?;
    let phys = p
        .get("physical_core")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(core % 8);
    Ok((core, get("ccd")?, get("ccx")?, phys))
}

/// Last ~300 lines of the service log for the page's debug panel.
fn debug_log_tail() -> Value {
    let path = log_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(300);
    json!({
        "enabled": LOG_ENABLED.load(Ordering::Relaxed),
        "path": path.display().to_string(),
        "total": lines.len(),
        "lines": lines[start..],
    })
}

fn write_response(stream: &mut TcpStream, code: u16, body: &str) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        204 => "No Content",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\n\
         Content-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

// ---- Port config (custom listening port, persisted in manifest.json) ----
//
// Same mechanism as the process-manager plugin: the host re-reads the plugin
// manifest on every service start/status call, so rewriting `service.port`,
// `health_url` and `args` there is the single source of truth. Apply spawns a
// replacement instance on the new port, then this process exits; the host
// cleans up by exe name on panel close, so the handover leaves no orphans.

fn port_check(req: &Value) -> Result<Value, String> {
    let p = req
        .get("port")
        .and_then(|v| v.as_u64())
        .ok_or("port missing")?;
    if p == 0 || p > 65535 {
        return Err("invalid port".into());
    }
    let p = p as u16;
    let cur = CURRENT_PORT.load(Ordering::Relaxed);
    if p == cur {
        // Our own listener holds it — usable by definition.
        return Ok(json!({ "port": p, "free": true, "self": true }));
    }
    match TcpListener::bind(("127.0.0.1", p)) {
        Ok(_) => Ok(json!({ "port": p, "free": true })),
        Err(_) => {
            let owner = winnet::listener_owner(p);
            Ok(match owner {
                Some((name, pid)) => json!({ "port": p, "free": false, "owner": name, "pid": pid }),
                None => json!({ "port": p, "free": false, "owner": "", "pid": null }),
            })
        }
    }
}

fn config_port(req: &Value) -> Result<Value, String> {
    let p = req
        .get("port")
        .and_then(|v| v.as_u64())
        .ok_or("port missing")?;
    if p == 0 || p > 65535 {
        return Err("invalid port".into());
    }
    let p = p as u16;
    let cur = CURRENT_PORT.load(Ordering::Relaxed);

    if p != cur && TcpListener::bind(("127.0.0.1", p)).is_err() {
        return Err(format!("端口 {p} 已被占用，无法应用"));
    }

    update_manifest_port(p)?;

    if p == cur {
        // Same port: manifest is now consistent, nothing to restart.
        return Ok(json!({ "ok": true, "port": p, "restarting": false }));
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(&exe)
        .args(["--serve", "--port", &p.to_string()])
        .spawn()
        .map_err(|e| format!("manifest updated but replacement spawn failed: {e}"))?;
    // Let the HTTP response flush before terminating.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(400));
        std::process::exit(0);
    });
    Ok(json!({ "ok": true, "port": p, "restarting": true }))
}

/// Rewrite <plugin>/manifest.json so `service.port`, `service.health_url` and
/// `service.args` all point at `new_port`.
fn update_manifest_port(new_port: u16) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    // Layout: <plugin>/sidecar/pbo-service.exe -> <plugin>/manifest.json
    let manifest_path = exe
        .parent()
        .and_then(|d| d.parent())
        .map(|d| d.join("manifest.json"))
        .ok_or_else(|| "cannot resolve plugin dir".to_string())?;

    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let mut v: Value = serde_json::from_str(&raw).map_err(|e| format!("parse manifest: {e}"))?;

    let cur = CURRENT_PORT.load(Ordering::Relaxed);
    let svc = v
        .get_mut("service")
        .and_then(|s| s.as_object_mut())
        .ok_or("manifest has no service object")?;

    svc.insert("port".into(), json!(new_port));

    if let Some(hu) = svc
        .get("health_url")
        .and_then(|x| x.as_str())
        .map(String::from)
    {
        let updated =
            hu.replace(&format!("127.0.0.1:{cur}"), &format!("127.0.0.1:{new_port}"));
        svc.insert("health_url".into(), json!(updated));
    }

    // Rebuild args: keep everything except port tokens, then ensure --serve
    // and the new --port are present.
    let old_args: Vec<String> = svc
        .get("args")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let mut kept: Vec<String> = Vec::new();
    let mut skip_next = false;
    for a in &old_args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--port" {
            skip_next = true;
            continue;
        }
        if a.starts_with("--port=") || a == "--serve" {
            continue;
        }
        kept.push(a.clone());
    }
    let mut args = vec!["--serve".to_string()];
    args.extend(kept);
    args.push("--port".to_string());
    args.push(new_port.to_string());
    svc.insert("args".into(), json!(args));

    let out =
        serde_json::to_string_pretty(&v).map_err(|e| format!("serialize manifest: {e}"))?;
    std::fs::write(&manifest_path, out)
        .map_err(|e| format!("write {}: {e}", manifest_path.display()))?;
    Ok(())
}

// Manual Win32 externs (no windows crate in this crate's deps): who owns a
// LISTEN socket? iphlpapi's GetExtendedTcpTable finds the PID, kernel32's
// Toolhelp32Snapshot resolves it to an image name.
#[cfg(windows)]
mod winnet {
    const AF_INET: u32 = 2;
    const TCP_TABLE_OWNER_PID_ALL: u32 = 5;
    const TH32CS_SNAPPROCESS: u32 = 0x2;
    const INVALID_HANDLE_VALUE: isize = -1;

    #[link(name = "iphlpapi")]
    extern "system" {
        fn GetExtendedTcpTable(
            table: *mut core::ffi::c_void,
            size: *mut u32,
            order: u32,
            family: u32,
            class: u32,
            reserved: u32,
        ) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
        fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    #[repr(C)]
    struct ProcessEntry32W {
        size: u32,
        usage: u32,
        process_id: u32,
        default_heap: usize,
        module_id: u32,
        threads: u32,
        parent_process_id: u32,
        base_priority: i32,
        flags: u32,
        exe_file: [u16; 260],
    }

    /// First LISTEN row bound to `port` -> (image name, pid).
    pub fn listener_owner(port: u16) -> Option<(String, u32)> {
        unsafe {
            let mut size: u32 = 0;
            let rc = GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if rc != 122 {
                return None; // ERROR_INSUFFICIENT_BUFFER expected on probe
            }
            let mut buf = vec![0u8; size as usize];
            let rc = GetExtendedTcpTable(
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if rc != 0 {
                return None;
            }
            let base = buf.as_mut_ptr() as *const u32;
            let count = *base as usize;
            // MIB_TCPROW_OWNER_PID: state, localAddr, localPort, remoteAddr,
            // remotePort, owningPid — six u32s per row.
            let words = std::slice::from_raw_parts(base.add(1), count * 6);
            let mut pid: Option<u32> = None;
            for i in 0..count {
                let r = &words[i * 6..i * 6 + 6];
                if r[0] == 2 {
                    // TCP_STATE_LISTEN; ports are in network byte order.
                    let lp = (((r[2] >> 8) & 0xff) | ((r[2] & 0xff) << 8)) as u16;
                    if lp == port {
                        pid = Some(r[5]);
                        break;
                    }
                }
            }
            let pid = pid?;
            let name = process_image_name(pid)?;
            Some((name, pid))
        }
    }

    fn process_image_name(pid: u32) -> Option<String> {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE || snap == 0 {
                return None;
            }
            let mut entry: ProcessEntry32W = std::mem::zeroed();
            entry.size = std::mem::size_of::<ProcessEntry32W>() as u32;
            let mut found: Option<String> = None;
            if Process32FirstW(snap, &mut entry) != 0 {
                loop {
                    if entry.process_id == pid {
                        let len = entry
                            .exe_file
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(entry.exe_file.len());
                        found = Some(String::from_utf16_lossy(&entry.exe_file[..len]));
                        break;
                    }
                    if Process32NextW(snap, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snap);
            found
        }
    }
}

#[cfg(not(windows))]
mod winnet {
    pub fn listener_owner(_port: u16) -> Option<(String, u32)> {
        None
    }
}

// ---- Config dir / profile ----
//
// Mirrors the host's resolution order so an existing profile keeps working:
// portable `data/` next to the executable, then `data_dir.ptr`, then the
// HKCU\Software\OpenRevo\DataPath value, then %APPDATA%\OpenRevo.
fn config_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let roaming = if appdata.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(&appdata).join("OpenRevo")
    };

    if let Some(dir) = &exe_dir {
        let portable = dir.join("data");
        if portable.is_dir() {
            return portable;
        }
    }

    if let Ok(ptr) = std::fs::read_to_string(roaming.join("data_dir.ptr")) {
        let t = ptr.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }

    if let Some(p) = registry_datapath() {
        return PathBuf::from(p);
    }

    roaming
}

fn profile_path() -> PathBuf {
    config_dir().join("pbo_tuner_profile.json")
}

fn save_profile(content: &str) -> Result<String, String> {
    // Reject anything that is not valid JSON before it lands on disk: a broken
    // profile would otherwise only surface at the next boot.
    serde_json::from_str::<Value>(content).map_err(|e| format!("profile is not valid JSON: {e}"))?;
    let path = profile_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

fn load_profile() -> Result<String, String> {
    let path = profile_path();
    if !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Apply the saved profile at startup when the user opted in.
///
/// Runs from this process, not the host: the service is launched by the host's
/// plugin bootstrap before any tab exists, which is exactly the moment the
/// margins need to be restored.
fn auto_apply_profile() -> Result<String, String> {
    let path = profile_path();
    if !path.exists() {
        return Ok("no profile saved".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let profile: Value = serde_json::from_str(&text).map_err(|e| format!("profile corrupt: {e}"))?;

    if profile.get("autoApply").and_then(|v| v.as_bool()) != Some(true) {
        return Ok("auto-apply is off for this profile".into());
    }

    let info = smu::init_smu()?;
    if !info.curve_optimizer_supported {
        return Err(format!("{} does not expose Curve Optimizer", info.code_name));
    }

    let layout = smu::get_core_layout();
    if layout.is_empty() {
        return Err("core topology unavailable".into());
    }

    let co = profile
        .get("co")
        .and_then(|v| v.as_object())
        .ok_or("profile has no core map")?;

    let (mut applied, mut failed) = (0usize, 0usize);
    for core in &layout {
        let Some(margin) = co.get(&core.core.to_string()).and_then(|v| v.as_i64()) else {
            continue;
        };
        match smu::set_co_single_core(core.core, core.ccd, core.ccx, core.physical_core, margin as i32) {
            Ok(_) => applied += 1,
            Err(_) => failed += 1,
        }
    }

    if let Some(fmax) = profile.get("fmax").and_then(|v| v.as_i64()) {
        if fmax > 0 && info.fmax_supported {
            let _ = smu::set_fmax(fmax as u32);
        }
    }
    if let Some(scalar) = profile.get("scalar").and_then(|v| v.as_i64()) {
        if scalar >= 1 && info.pbo_scalar_supported {
            let _ = smu::set_pbo_scalar((scalar as u32) * 1000);
        }
    }

    Ok(format!("applied {applied} cores ({failed} failed)"))
}

// ---- Logging ----

/// File logging is opt-in (default OFF): the page has a switch in the
/// 服务日志 card. The flag persists in the plugin manifest as
/// `service.log_enabled`, so the service remembers it across restarts.
static LOG_ENABLED: AtomicBool = AtomicBool::new(false);
/// Log file size cap; exceeding it rotates the current file to `.log.old`.
const LOG_MAX_BYTES: u64 = 1024 * 1024;

/// The log lives next to pbo-service.exe (portable, easy to grab when a
/// remote user reports a problem), falling back to the config dir if the
/// exe path cannot be resolved.
fn data_dir_for_log() -> PathBuf {
    if let Some(d) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        if !d.as_os_str().is_empty() {
            return d;
        }
    }
    config_dir()
}

fn log_path() -> PathBuf {
    data_dir_for_log().join("pbo_service.log")
}

/// <plugin>/sidecar/pbo-service.exe -> <plugin>/manifest.json
fn manifest_path_for_log() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    exe.parent()
        .and_then(|d| d.parent())
        .map(|d| d.join("manifest.json"))
        .ok_or_else(|| "cannot resolve plugin dir".to_string())
}

fn load_log_flag() {
    let on = manifest_path_for_log()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| {
            v.get("service")?
                .get("log_enabled")?
                .as_bool()
        })
        .unwrap_or(false);
    LOG_ENABLED.store(on, Ordering::Relaxed);
}

fn set_log_enabled(on: bool) -> Result<(), String> {
    let path = manifest_path_for_log()?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut v: Value =
        serde_json::from_str(&raw).map_err(|e| format!("parse manifest: {e}"))?;
    let svc = v
        .get_mut("service")
        .and_then(|s| s.as_object_mut())
        .ok_or("manifest has no service object")?;
    svc.insert("log_enabled".into(), json!(on));
    let out =
        serde_json::to_string_pretty(&v).map_err(|e| format!("serialize manifest: {e}"))?;
    std::fs::write(&path, out).map_err(|e| format!("write {}: {e}", path.display()))?;
    LOG_ENABLED.store(on, Ordering::Relaxed);
    Ok(())
}

pub(crate) fn log(msg: &str) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("[{ts}] {msg}\n");
    let path = log_path();
    // 1 MB cap: rotate the current file to .log.old once, keeping the newest
    // window on disk while total usage stays bounded (~2 MB).
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() >= LOG_MAX_BYTES {
            let old = path.with_extension("log.old");
            let _ = std::fs::remove_file(&old);
            let _ = std::fs::rename(&path, &old);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = f.write_all(line.as_bytes());
    }
    eprintln!("{}", line.trim_end());
}

// ---- Registry (HKCU\Software\OpenRevo DataPath) ----

#[cfg(windows)]
fn registry_datapath() -> Option<String> {
    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(hkey: isize, sub: *const u16, opt: u32, sam: u32, out: *mut isize) -> i32;
        fn RegQueryValueExW(
            hkey: isize,
            name: *const u16,
            reserved: *mut u32,
            ty: *mut u32,
            data: *mut u8,
            len: *mut u32,
        ) -> i32;
        fn RegCloseKey(hkey: isize) -> i32;
    }

    // HKEY_CURRENT_USER is 0xFFFFFFFF80000001 on 64-bit; sign-extend from i32
    // instead of widening the bare u32, which would address the wrong root.
    const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
    const KEY_READ: u32 = 0x20019;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe {
        let mut hkey: isize = 0;
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide("Software\\OpenRevo").as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) != 0
        {
            return None;
        }
        let mut ty: u32 = 0;
        let mut len: u32 = 4096;
        let mut buf = vec![0u8; len as usize];
        let rc = RegQueryValueExW(
            hkey,
            wide("DataPath").as_ptr(),
            std::ptr::null_mut(),
            &mut ty,
            buf.as_mut_ptr(),
            &mut len,
        );
        RegCloseKey(hkey);
        if rc != 0 || len < 2 {
            return None;
        }
        // REG_SZ (1) or REG_EXPAND_SZ (2): UTF-16LE, trailing NUL.
        let units = len as usize / 2;
        let mut v: Vec<u16> = Vec::with_capacity(units);
        for i in 0..units {
            v.push(u16::from_le_bytes([buf[i * 2], buf[i * 2 + 1]]));
        }
        while matches!(v.last(), Some(0)) {
            v.pop();
        }
        let s = String::from_utf16_lossy(&v);
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

#[cfg(not(windows))]
fn registry_datapath() -> Option<String> {
    None
}
