// process-service — OpenRevo process-manager companion service.
//
// Standalone HTTP sidecar exposing system-level introspection over localhost:
//   GET  /api/health                  liveness probe
//   GET  /api/processes               process list (cpu%, memory, path, ppid)
//   POST /api/process/kill            {pid, tree}     end process (optionally whole tree)
//   GET  /api/connections             TCP/UDP endpoint table with owning PID
//   POST /api/connections/close       {local_addr, local_port, remote_addr?, remote_port?} force TCP close
//   GET  /api/file/occupiers?path=    Restart Manager: which processes lock this file
//   GET  /api/port/check?port=N       is a TCP port free on 127.0.0.1 (with owner lookup)
//   POST /api/config/port             {port} rewrite manifest.json service port and
//                                     hand over to a fresh instance listening there
//
// The host (open-revo daemon) starts/stops this process per manifest.json
// `service` configuration and spawns it elevated when requested. This binary
// never touches host code.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// Port this instance is actually listening on (reported by /api/health and
/// used for manifest rewrites). Defaults to the manifest port.
static CURRENT_PORT: AtomicU16 = AtomicU16::new(18956);

// ------------------------------------------------------------ app state -----

struct AppState {
    sys: System,
    started: Instant,
    /// Last request that requires fresh data (anything except /api/health).
    last_activity: Instant,
    /// True while the background ticker is paused because no page is polling.
    dormant: bool,
    /// PIDs owning at least one visible top-level window — the Task-Manager
    /// notion of "Apps". Recomputed together with the process snapshot.
    gui: HashSet<u32>,
}

impl AppState {
    fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        AppState {
            sys,
            started: Instant::now(),
            last_activity: Instant::now(),
            dormant: false,
            gui: visible_window_pids(),
        }
    }

    // Only the process table is tracked: process CPU% is computed by sysinfo
    // from GetProcessTimes deltas between two consecutive refreshes, so the
    // fixed 1s cadence below is what makes the numbers stable.
    fn refresh_all(&mut self) {
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.gui = visible_window_pids();
    }

    fn trim_working_set() {
        // SetProcessWorkingSetSize(-1,-1) == EmptyWorkingSet: pages just
        // soft-fault back in on demand, so trimming costs nothing functionally.
        #[cfg(windows)]
        unsafe {
            SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
        }
    }
}

static STATE: std::sync::OnceLock<Mutex<AppState>> = std::sync::OnceLock::new();

fn state() -> &'static Mutex<AppState> {
    STATE.get_or_init(|| Mutex::new(AppState::new()))
}

// --------------------------------------------------- visible window probing --

// Task-Manager-style "Apps": processes owning at least one visible top-level
// window. EnumWindows over ~100 top-level windows costs microseconds, so this
// rides along with the 1s refresh cadence without measurably waking the CPU.
fn visible_window_pids() -> HashSet<u32> {
    #[cfg(windows)]
    {
        extern "system" fn collect(
            hwnd: windows_sys::Win32::Foundation::HWND,
            lparam: isize,
        ) -> i32 {
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GetWindowThreadProcessId, IsWindowVisible,
                };
                if IsWindowVisible(hwnd) != 0 {
                    let mut pid: u32 = 0;
                    GetWindowThreadProcessId(hwnd, &mut pid);
                    if pid != 0 {
                        (*(lparam as *mut HashSet<u32>)).insert(pid);
                    }
                }
            }
            1 // keep enumerating
        }
        let mut set: HashSet<u32> = HashSet::new();
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;
            EnumWindows(Some(collect), &mut set as *mut _ as isize);
        }
        set
    }
    #[cfg(not(windows))]
    {
        HashSet::new()
    }
}

// ------------------------------------------------------------------ main ----

fn main() {
    let mut port: u16 = 18956;
    let args: Vec<String> = std::env::args().collect();
    let mut serve = false;
    let mut i = 1;
    while i < args.len() {
        if let Some(v) = args[i].strip_prefix("--port=") {
            port = v.parse().unwrap_or(port);
        } else if args[i] == "--port" && i + 1 < args.len() {
            i += 1;
            port = args[i].parse().unwrap_or(port);
        } else if args[i] == "--serve" {
            serve = true;
        }
        i += 1;
    }
    if !serve {
        // Started without --serve (e.g. manual probe): just report and exit.
        println!("process-service v{} (pass --serve --port N to run)", env!("CARGO_PKG_VERSION"));
        return;
    }

    // Pre-warm the process snapshot so the first /api/processes call has deltas.
    let _ = state();

    // Scheduled-kill ticker: independent of page activity so tasks fire
    // even while the plugin panel is closed.
    spawn_scheduler();

    // Background ticker — activity-driven so the service stays idle while no
    // plugin page is open. The page polls every 1.5s; while requests keep
    // arriving the snapshot is refreshed on a fixed 1s cadence (stable CPU%
    // windows). After 10s without data requests (page closed) the ticker
    // stops refreshing and trims the working set, dropping reported memory
    // to ~1MB; the first request after waking refreshes synchronously.
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(1));
        let mut st = match state().lock() {
            Ok(g) => g,
            Err(_) => continue,
        };
        if st.last_activity.elapsed() > Duration::from_secs(10) {
            if !st.dormant {
                st.dormant = true;
                AppState::trim_working_set();
            }
            continue;
        }
        st.dormant = false;
        st.refresh_all();
    });

    // Port handover: when /api/config/port spawns a replacement instance the
    // old process still holds the listener for a few hundred ms, so retry the
    // bind for up to ~5s before giving up. Cold starts bind on the first try.
    let listener = match bind_with_retry(port) {
        Ok(l) => l,
        Err(_) => std::process::exit(1),
    };
    CURRENT_PORT.store(port, Ordering::Relaxed);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                // Thread per connection: keeps the listener responsive even
                // while a slow /api/processes refresh is in flight.
                std::thread::spawn(move || handle_client(s));
            }
            Err(_) => continue,
        }
    }
}

// ---------------------------------------------------------- http plumbing ---

fn bind_with_retry(port: u16) -> std::io::Result<TcpListener> {
    let mut last: Option<std::io::Error> = None;
    for _ in 0..25 {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => return Ok(l),
            Err(e) => {
                last = Some(e);
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    Err(last.unwrap_or_else(|| std::io::Error::other("bind failed")))
}

fn handle_client(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));

    let req = match read_request(&mut stream) {
        Some(r) => r,
        None => {
            respond(&mut stream, 400, &json!({"error": "bad request"}));
            return;
        }
    };

    let (method, target) = (req.0, req.1);

    // CORS preflight: the browser sends OPTIONS before any POST with a
    // non-safelisted Content-Type (application/json). Without this the
    // preflight fails and the page sees "Failed to fetch" on every POST.
    if method == "OPTIONS" {
        let preflight = "HTTP/1.1 204 No Content\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\n\
            Access-Control-Allow-Headers: Content-Type\r\n\
            Access-Control-Max-Age: 86400\r\n\
            Connection: close\r\n\
            Content-Length: 0\r\n\r\n";
        let _ = stream.write_all(preflight.as_bytes());
        let _ = stream.flush();
        return;
    }

    let (path, query) = split_target(&target);
    // Activity marker: /api/health is just a liveness probe (the page also
    // fires it every 30s), it must NOT keep the service awake — only data
    // requests do. This is what makes the service idle once the page closes.
    if path != "/api/health" && path != "/" {
        if let Ok(mut st) = state().lock() {
            st.last_activity = Instant::now();
        }
    }
    let started = Instant::now();
    let (status, value) = route(method.as_str(), path.as_str(), &query, &req.2);
    let _ = started.elapsed();
    respond(&mut stream, status, &value);
}

type Request = (String, String, Vec<u8>); // method, target, body

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut tmp = [0u8; 8192];
    let mut header_end: Option<usize> = None;

    loop {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            header_end = Some(pos);
            break;
        }
        if buf.len() > 1_048_576 {
            return None; // header too large
        }
    }
    let header_end = header_end?;
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let reqline = lines.next()?.to_string();
    let mut parts = reqline.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();

    let mut content_length: usize = 0;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    if content_length > 1_048_576 {
        return None; // body too large
    }

    let mut body: Vec<u8> = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);
    Some((method, target, body))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn split_target(target: &str) -> (String, HashMap<String, String>) {
    let mut it = target.splitn(2, '?');
    let path = it.next().unwrap_or("/").to_string();
    let mut query = HashMap::new();
    if let Some(qs) = it.next() {
        for pair in qs.split('&') {
            let mut kv = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                query.insert(url_decode(k), url_decode(v));
            }
        }
    }
    (path, query)
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn respond(stream: &mut TcpStream, status: u16, value: &Value) {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{\"error\":\"serialize\"}".into());
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        status,
        reason,
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

// ---------------------------------------------------------------- routing ---

fn route(method: &str, path: &str, query: &HashMap<String, String>, body: &[u8]) -> (u16, Value) {
    match (method, path) {
        ("GET", "/api/health") => (
            200,
            json!({
                "status": "ok",
                "service": "process-service",
                "version": env!("CARGO_PKG_VERSION"),
                "port": CURRENT_PORT.load(Ordering::Relaxed),
                "uptime_s": state().lock().map(|s| s.started.elapsed().as_secs()).unwrap_or(0),
                "elevated": is_elevated()
            }),
        ),
        ("GET", "/api/processes") => api_processes(),
        ("POST", "/api/process/kill") => with_json_body(body, api_process_kill),
        ("GET", "/api/schedule") => api_schedule_list(),
        ("POST", "/api/schedule") => with_json_body(body, api_schedule_add),
        ("DELETE", "/api/schedule") => match query.get("id").and_then(|s| s.parse::<u64>().ok()) {
            Some(id) => api_schedule_delete(id),
            None => (400, json!({ "error": "missing id" })),
        },
        ("GET", "/api/connections") => api_connections(),
        ("POST", "/api/connections/close") => with_json_body(body, api_connection_close),
        ("GET", "/api/file/occupiers") => match query.get("path") {
            Some(p) if !p.trim().is_empty() => api_file_occupiers(p.trim()),
            _ => (400, json!({"error": "missing ?path= parameter"})),
        },
        ("GET", "/api/port/check") => match query.get("port") {
            Some(p) => api_port_check(p),
            None => (400, json!({"error": "missing ?port= parameter"})),
        },
        ("POST", "/api/config/port") => with_json_body(body, api_config_port),
        ("GET", "/") => (200, json!({"service": "process-service", "version": env!("CARGO_PKG_VERSION")})),
        _ => (404, json!({"error": format!("no route: {} {}", method, path)})),
    }
}

fn with_json_body(body: &[u8], f: fn(&Value) -> (u16, Value)) -> (u16, Value) {
    if body.is_empty() {
        return (400, json!({"error": "empty body"}));
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(v) => f(&v),
        Err(e) => (400, json!({"error": format!("invalid JSON body: {}", e)})),
    }
}

// ------------------------------------------------------------ port config --

/// Is `port` bindable on 127.0.0.1? If not, report who holds it by looking up
/// the LISTEN row in the TCP table.
fn find_listener_owner(port: u16) -> (String, Option<u32>) {
    unsafe {
        let mut size: u32 = 0;
        let rc = GetExtendedTcpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            2, // AF_INET
            TCP_TABLE_OWNER_PID_ALL as i32,
            0,
        );
        if rc == 122 {
            let mut buf = vec![0u8; size as usize];
            let rc = GetExtendedTcpTable(
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                &mut size,
                0,
                2,
                TCP_TABLE_OWNER_PID_ALL as i32,
                0,
            );
            if rc == 0 {
                let base = buf.as_mut_ptr() as *const u32;
                let count = *base as usize;
                let row_ptr = base.add(1) as *const TcpRow;
                for i in 0..count {
                    let r = std::ptr::read(row_ptr.add(i));
                    if r.state == 2 && port_from_dw(r.local_port) == port {
                        // r.local_addr 0 == 0.0.0.0 (any interface) wins over a
                        // specific one, but either way the first match is fine.
                        let name = pid_names(&[r.pid])
                            .get(&r.pid)
                            .cloned()
                            .unwrap_or_default();
                        return (name, Some(r.pid));
                    }
                }
            }
        }
    }
    (String::new(), None)
}

fn api_port_check(port_param: &str) -> (u16, Value) {
    let Ok(p) = port_param.trim().parse::<u16>() else {
        return (400, json!({"error": "invalid port"}));
    };
    if p == 0 {
        return (400, json!({"error": "invalid port"}));
    }
    let cur = CURRENT_PORT.load(Ordering::Relaxed);
    if p == cur {
        // Our own listener holds it — usable by definition.
        return (200, json!({"port": p, "free": true, "self": true}));
    }
    match TcpListener::bind(("127.0.0.1", p)) {
        Ok(l) => {
            drop(l);
            (200, json!({"port": p, "free": true, "self": false}))
        }
        Err(_) => {
            let (owner, pid) = find_listener_owner(p);
            (
                200,
                json!({"port": p, "free": false, "owner": owner, "pid": pid}),
            )
        }
    }
}

/// Rewrite <plugin>/manifest.json so `service.port`, `service.health_url` and
/// `service.args` all point at `new_port`. The host re-reads the manifest on
/// every status/start call, so the change takes effect immediately.
fn update_manifest_port(new_port: u16) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    // Layout: <plugin>/sidecar/process-service.exe -> <plugin>/manifest.json
    let manifest_path = exe
        .parent()
        .and_then(|d| d.parent())
        .map(|d| d.join("manifest.json"))
        .ok_or_else(|| "cannot resolve plugin dir".to_string())?;

    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {}", manifest_path.display(), e))?;
    let mut v: Value =
        serde_json::from_str(&raw).map_err(|e| format!("parse manifest: {}", e))?;

    let cur = CURRENT_PORT.load(Ordering::Relaxed);
    let svc = v
        .get_mut("service")
        .and_then(|s| s.as_object_mut())
        .ok_or_else(|| "manifest has no service object".to_string())?;

    svc.insert("port".into(), json!(new_port));

    if let Some(hu) = svc.get("health_url").and_then(|x| x.as_str()).map(String::from) {
        let updated = hu.replace(&format!("127.0.0.1:{}", cur), &format!("127.0.0.1:{}", new_port));
        svc.insert("health_url".into(), json!(updated));
    }

    // Rebuild args: keep everything except port tokens, then ensure --serve
    // and the new --port are present.
    let mut kept: Vec<String> = Vec::new();
    let old_args: Vec<String> = svc
        .get("args")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
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

    let out = serde_json::to_string_pretty(&v)
        .map_err(|e| format!("serialize manifest: {}", e))?;
    std::fs::write(&manifest_path, out)
        .map_err(|e| format!("write {}: {}", manifest_path.display(), e))?;
    Ok(())
}

fn api_config_port(v: &Value) -> (u16, Value) {
    let Some(p) = v.get("port").and_then(|x| x.as_u64()) else {
        return (400, json!({"error": "missing numeric \"port\""}));
    };
    if p == 0 || p > 65535 {
        return (400, json!({"error": "invalid port"}));
    }
    let p = p as u16;
    let cur = CURRENT_PORT.load(Ordering::Relaxed);

    if p != cur && TcpListener::bind(("127.0.0.1", p)).is_err() {
        let (owner, pid) = find_listener_owner(p);
        return (
            409,
            json!({
                "error": format!("port {} is occupied by {} (pid {:?})", p, owner, pid),
                "free": false,
            }),
        );
    }

    if let Err(e) = update_manifest_port(p) {
        return (500, json!({"error": format!("manifest update failed: {}", e)}));
    }

    if p == cur {
        // Same port: manifest is now consistent, nothing to restart.
        return (200, json!({"ok": true, "port": p, "restarting": false}));
    }

    // Handover: spawn a replacement on the new port, then exit. The old
    // listener is released when this process dies; the replacement retries
    // its bind for a few seconds to absorb the gap. The host kills our
    // replacement by exe name on panel close / host exit, so no orphans.
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("process-service.exe"));
    let spawned = std::process::Command::new(&exe)
        .args(["--serve", "--port", &p.to_string()])
        .spawn()
        .is_ok();
    if !spawned {
        return (
            500,
            json!({"error": "manifest updated but replacement instance failed to spawn"}),
        );
    }
    // Let the HTTP response flush before terminating.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
        std::process::exit(0);
    });
    (200, json!({"ok": true, "port": p, "restarting": true}))
}

// ----------------------------------------------------------- /api/processes --

fn api_processes() -> (u16, Value) {
    // Wake-up path: the ticker is dormant (page was closed / idle). Refresh
    // synchronously so the first response after reopening the page is fresh;
    // while awake the 1s ticker owns refreshing and this is a no-op.
    if let Ok(mut st) = state().lock() {
        if st.dormant {
            st.refresh_all();
            st.dormant = false;
        }
    }
    let st = match state().lock() {
        Ok(g) => g,
        Err(_) => return (500, json!({"error": "state poisoned"})),
    };
    // CPU% is normalized by the ticker's fixed 1s windows; serving the cached
    // snapshot keeps values identical across consecutive polls.
    // sysinfo reports per-core semantics (100% = one full core, up to
    // nb_cpus * 100); divide by the logical core count to match Task
    // Manager's whole-machine scale (100% = all cores).
    let nb_cpus = std::thread::available_parallelism()
        .map(|n| n.get() as f32)
        .unwrap_or(1.0)
        .max(1.0);

    let mut list: Vec<Value> = Vec::with_capacity(st.sys.processes().len());
    for (pid, p) in st.sys.processes() {
        list.push(json!({
            "pid": pid.as_u32(),
            "ppid": p.parent().map(|x| x.as_u32()),
            "name": p.name().to_string_lossy(),
            "cpu": p.cpu_usage() / nb_cpus,
            "mem": p.memory(),
            "exe": p.exe().map(|e| e.to_string_lossy()).unwrap_or_default(),
            "app": st.gui.contains(&pid.as_u32()),
        }));
    }

    (200, json!({ "count": list.len(), "processes": list }))
}

// ------------------------------------------------------- process actions ----

fn kill_pid(pid: u32) -> Result<(), String> {
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if h.is_null() {
            return Err(format!("OpenProcess({}) failed (err {})", pid, GetLastError()));
        }
        let ok = TerminateProcess(h, 1);
        CloseHandle(h);
        if ok == 0 {
            return Err(format!("TerminateProcess({}) failed (err {})", pid, GetLastError()));
        }
    }
    Ok(())
}

fn api_process_kill(v: &Value) -> (u16, Value) {
    let pid = match v.get("pid").and_then(|x| x.as_u64()) {
        Some(p) if p > 0 => p as u32,
        _ => return (400, json!({"error": "missing pid"})),
    };
    let tree = v.get("tree").and_then(|x| x.as_bool()).unwrap_or(false);

    if !tree {
        return match kill_pid(pid) {
            Ok(()) => (200, json!({"ok": true, "killed": [pid]})),
            Err(e) => (500, json!({"error": e})),
        };
    }

    let (killed, failed) = kill_tree(pid);
    if failed.is_empty() {
        (200, json!({ "ok": true, "killed": killed }))
    } else {
        (500, json!({ "killed": killed, "failed": failed }))
    }
}

/// Tree kill: snapshot, walk descendants, terminate leaves-first.
/// Returns (killed pids, failures excluding an already-exited root).
fn kill_tree(pid: u32) -> (Vec<u32>, Vec<Value>) {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for (p, info) in sys.processes() {
        if let Some(pp) = info.parent() {
            children.entry(pp.as_u32()).or_default().push(p.as_u32());
        }
    }
    let mut order: Vec<u32> = Vec::new();
    let mut stack = vec![pid];
    let mut seen = std::collections::HashSet::new();
    while let Some(cur) = stack.pop() {
        if !seen.insert(cur) {
            continue;
        }
        order.push(cur);
        if let Some(kids) = children.get(&cur) {
            for k in kids.clone() {
                stack.push(k);
            }
        }
    }
    // Descendants first, root last.
    order.reverse();

    let mut killed = Vec::new();
    let mut failed: Vec<Value> = Vec::new();
    for target in &order {
        match kill_pid(*target) {
            Ok(()) => killed.push(*target),
            Err(e) => {
                // Self-kill of an already-exited child is not an error.
                if *target != pid {
                    failed.push(json!({ "pid": target, "error": e }));
                }
            }
        }
    }
    (killed, failed)
}

// ------------------------------------------------------- scheduled kills ----
// Tasks persist in schedule.json next to the executable; the scheduler thread
// checks every 15s even while the service is dormant, so kills fire without
// any plugin page open.

static SCHEDULE: std::sync::OnceLock<std::sync::Mutex<Vec<Value>>> = std::sync::OnceLock::new();

fn schedule_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("schedule.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("schedule.json"))
}

fn schedule() -> &'static std::sync::Mutex<Vec<Value>> {
    SCHEDULE.get_or_init(|| {
        let tasks = std::fs::read_to_string(schedule_path())
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<Value>>(&s).ok())
            .unwrap_or_default();
        std::sync::Mutex::new(tasks)
    })
}

fn save_schedule(tasks: &[Value]) {
    let s = serde_json::to_string_pretty(tasks).unwrap_or_else(|_| "[]".into());
    let _ = std::fs::write(schedule_path(), s);
}

#[cfg(windows)]
fn local_now() -> (String, String) {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st) };
    (
        format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay),
        format!("{:02}:{:02}", st.wHour, st.wMinute),
    )
}

#[cfg(not(windows))]
fn local_now() -> (String, String) {
    (String::new(), String::new())
}

/// Normalize a process name for matching: lowercase, drop ".exe".
fn proc_key(s: &str) -> String {
    s.trim().to_ascii_lowercase().trim_end_matches(".exe").to_string()
}

/// Kill every process whose name matches `name` (case-insensitive, ".exe"
/// optional). Returns (killed, failed) across all matched process trees.
/// Never matches pid 0 (System Idle Process) or the service itself.
fn kill_by_name(name: &str, tree: bool) -> (Vec<u32>, Vec<Value>) {
    let target = proc_key(name);
    if target.is_empty() {
        return (vec![], vec![]);
    }
    let me = std::process::id();
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let mut pids: Vec<u32> = sys
        .processes()
        .iter()
        .filter(|(pid, p)| {
            pid.as_u32() != 0
                && pid.as_u32() != me
                && proc_key(&p.name().to_string_lossy()) == target
        })
        .map(|(pid, _)| pid.as_u32())
        .collect();
    pids.sort_unstable();
    pids.dedup();
    let mut killed = Vec::new();
    let mut failed = Vec::new();
    for pid in pids {
        let (k, f) = if tree {
            kill_tree(pid)
        } else {
            match kill_pid(pid) {
                Ok(()) => (vec![pid], vec![]),
                Err(e) => (vec![], vec![json!({ "pid": pid, "error": e })]),
            }
        };
        killed.extend(k);
        failed.extend(f);
    }
    (killed, failed)
}

fn api_schedule_list() -> (u16, Value) {
    let tasks = match schedule().lock() {
        Ok(g) => g.clone(),
        Err(e) => return (500, json!({ "error": e.to_string() })),
    };
    (200, json!({ "count": tasks.len(), "tasks": tasks }))
}

fn api_schedule_add(v: &Value) -> (u16, Value) {
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    if name.is_empty() {
        return (400, json!({ "error": "missing name" }));
    }
    let mode = v.get("mode").and_then(|x| x.as_str()).unwrap_or("daily").trim().to_string();
    if mode != "daily" && mode != "once" {
        return (400, json!({ "error": "mode must be daily or once" }));
    }
    let time = v.get("time").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let hhmm_ok = time.len() == 5
        && time.as_bytes()[2] == b':'
        && time[..2].parse::<u8>().map(|h| h < 24).unwrap_or(false)
        && time[3..].parse::<u8>().map(|m| m < 60).unwrap_or(false);
    if !hhmm_ok {
        return (400, json!({ "error": "time must be HH:MM" }));
    }
    let date = v.get("date").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    if mode == "once" && date.len() != 10 {
        return (400, json!({ "error": "once tasks need date YYYY-MM-DD" }));
    }
    let tree = v.get("tree").and_then(|x| x.as_bool()).unwrap_or(true);

    let mut tasks = match schedule().lock() {
        Ok(g) => g,
        Err(e) => return (500, json!({ "error": e.to_string() })),
    };
    let mut id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(1);
    while tasks.iter().any(|t| t.get("id").and_then(|x| x.as_u64()) == Some(id)) {
        id += 1;
    }
    let task = json!({ "id": id, "name": name, "mode": mode, "time": time, "date": date, "tree": tree });
    tasks.push(task);
    save_schedule(&tasks);
    (200, json!({ "ok": true, "count": tasks.len() }))
}

fn api_schedule_delete(id: u64) -> (u16, Value) {
    let mut tasks = match schedule().lock() {
        Ok(g) => g,
        Err(e) => return (500, json!({ "error": e.to_string() })),
    };
    let before = tasks.len();
    tasks.retain(|t| t.get("id").and_then(|x| x.as_u64()) != Some(id));
    save_schedule(&tasks);
    (200, json!({ "ok": true, "removed": before - tasks.len() }))
}

/// Background scheduler: every 15s, fire any due task. Runs even while the
/// service is dormant so kills happen with no plugin page open.
fn spawn_scheduler() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(15));
        let (today, hhmm) = local_now();
        if hhmm.is_empty() {
            continue;
        }
        let mut tasks = match schedule().lock() {
            Ok(g) => g,
            Err(_) => continue,
        };
        let mut changed = false;
        tasks.retain_mut(|t| {
            let mode = t.get("mode").and_then(|x| x.as_str()).unwrap_or("");
            let time = t.get("time").and_then(|x| x.as_str()).unwrap_or("");
            if time.len() != 5 {
                return true;
            }
            let due = match mode {
                // Daily fires once per calendar day (guarded by last_run).
                "daily" => {
                    time == hhmm.as_str()
                        && t.get("last_run").and_then(|x| x.as_str()) != Some(today.as_str())
                }
                // One-shot fires when the moment has passed, then is removed.
                "once" => {
                    let date = t.get("date").and_then(|x| x.as_str()).unwrap_or("");
                    !date.is_empty()
                        && (date < today.as_str() || (date == today.as_str() && time <= hhmm.as_str()))
                }
                _ => false,
            };
            if !due {
                return true;
            }
            let name = t.get("name").and_then(|x| x.as_str()).unwrap_or("");
            let tree = t.get("tree").and_then(|x| x.as_bool()).unwrap_or(true);
            let (killed, failed) = kill_by_name(name, tree);
            if mode == "once" {
                changed = true;
                return false; // fired: drop the one-shot task
            }
            if let Some(obj) = t.as_object_mut() {
                obj.insert("last_run".into(), json!(today));
                obj.insert("last_killed".into(), json!(killed.len()));
                obj.insert("last_failed".into(), json!(failed.len()));
            }
            changed = true;
            true
        });
        if changed {
            save_schedule(&tasks);
        }
    });
}

// --------------------------------------------------------- /api/connections --

#[repr(C)]
struct TcpRow {
    state: u32,
    local_addr: u32,
    local_port: u32,
    remote_addr: u32,
    remote_port: u32,
    pid: u32,
}

#[repr(C)]
struct UdpRow {
    local_addr: u32,
    local_port: u32,
    pid: u32,
}

const TCP_TABLE_OWNER_PID_ALL: u32 = 5;
const UDP_TABLE_OWNER_PID: u32 = 1;
const MIB_TCP_STATE_DELETE_TCB: u32 = 12;

fn addr_to_string(dw: u32) -> String {
    format!("{}.{}.{}.{}", dw & 0xff, (dw >> 8) & 0xff, (dw >> 16) & 0xff, (dw >> 24) & 0xff)
}

// Ports in the tables are stored big-endian (network order) in the low word.
fn port_from_dw(dw: u32) -> u16 {
    (((dw >> 8) & 0xff) | ((dw & 0xff) << 8)) as u16
}

fn pid_names(pids: &[u32]) -> HashMap<u32, String> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let mut map = HashMap::new();
    for pid in pids {
        if let Some(p) = sys.process(Pid::from_u32(*pid)) {
            map.insert(*pid, p.name().to_string_lossy().to_string());
        }
    }
    map
}

fn api_connections() -> (u16, Value) {
    let mut tcp: Vec<TcpRow> = Vec::new();
    let mut udp: Vec<UdpRow> = Vec::new();

    unsafe {
        // ---- TCP ----
        let mut size: u32 = 0;
        let rc = GetExtendedTcpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            2, // AF_INET
            TCP_TABLE_OWNER_PID_ALL as i32,
            0,
        );
        if rc == 122 {
            // ERROR_INSUFFICIENT_BUFFER
            let mut buf = vec![0u8; size as usize];
            let rc = GetExtendedTcpTable(
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                &mut size,
                0,
                2,
                TCP_TABLE_OWNER_PID_ALL as i32,
                0,
            );
            if rc == 0 {
                let base = buf.as_mut_ptr() as *const u32;
                let count = *base as usize;
                let row_ptr = base.add(1) as *const TcpRow;
                for i in 0..count {
                    tcp.push(std::ptr::read(row_ptr.add(i)));
                }
            }
        }

        // ---- UDP ----
        let mut size: u32 = 0;
        let rc = GetExtendedUdpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            2,
            UDP_TABLE_OWNER_PID as i32,
            0,
        );
        if rc == 122 {
            let mut buf = vec![0u8; size as usize];
            let rc = GetExtendedUdpTable(
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                &mut size,
                0,
                2,
                UDP_TABLE_OWNER_PID as i32,
                0,
            );
            if rc == 0 {
                let base = buf.as_mut_ptr() as *const u32;
                let count = *base as usize;
                let row_ptr = base.add(1) as *const UdpRow;
                for i in 0..count {
                    udp.push(std::ptr::read(row_ptr.add(i)));
                }
            }
        }
    }

    let mut all_pids: Vec<u32> = tcp.iter().map(|r| r.pid).collect();
    all_pids.extend(udp.iter().map(|r| r.pid));
    let names = pid_names(&all_pids);

    let tcp_json: Vec<Value> = tcp
        .iter()
        .map(|r| {
            json!({
                "proto": "tcp",
                "state": tcp_state_name(r.state),
                "local": format!("{}:{}", addr_to_string(r.local_addr), port_from_dw(r.local_port)),
                "remote": format!("{}:{}", addr_to_string(r.remote_addr), port_from_dw(r.remote_port)),
                "pid": r.pid,
                "process": names.get(&r.pid).cloned().unwrap_or_default(),
            })
        })
        .collect();

    let udp_json: Vec<Value> = udp
        .iter()
        .map(|r| {
            json!({
                "proto": "udp",
                "local": format!("{}:{}", addr_to_string(r.local_addr), port_from_dw(r.local_port)),
                "pid": r.pid,
                "process": names.get(&r.pid).cloned().unwrap_or_default(),
            })
        })
        .collect();

    (200, json!({ "tcp": tcp_json, "udp": udp_json }))
}

fn tcp_state_name(s: u32) -> String {
    let name = match s {
        1 => "CLOSED",
        2 => "LISTEN",
        3 => "SYN_SENT",
        4 => "SYN_RCVD",
        5 => "ESTABLISHED",
        6 => "FIN_WAIT1",
        7 => "FIN_WAIT2",
        8 => "CLOSE_WAIT",
        9 => "CLOSING",
        10 => "LAST_ACK",
        11 => "TIME_WAIT",
        12 => "DELETE_TCB",
        _ => "UNKNOWN",
    };
    name.to_string()
}

fn api_connection_close(v: &Value) -> (u16, Value) {
    let parse_ip = |s: &str| -> Option<u32> {
        let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
        if parts.len() != 4 {
            return None;
        }
        // Network byte order DWORD
        Some(parts[0] | (parts[1] << 8) | (parts[2] << 16) | (parts[3] << 24))
    };
    let port_dw = |p: u16| -> u32 { ((p >> 8) & 0xff) as u32 | ((p & 0xff) << 8) as u32 };

    let laddr = match v.get("local_addr").and_then(|x| x.as_str()).and_then(parse_ip) {
        Some(a) => a,
        None => return (400, json!({"error": "missing/invalid local_addr"})),
    };
    let lport = match v.get("local_port").and_then(|x| x.as_u64()) {
        Some(p) if p > 0 && p < 65536 => port_dw(p as u16),
        _ => return (400, json!({"error": "missing/invalid local_port"})),
    };
    let raddr = v.get("remote_addr").and_then(|x| x.as_str()).and_then(parse_ip).unwrap_or(0);
    let rport = v.get("remote_port").and_then(|x| x.as_u64()).map(|p| port_dw(p as u16)).unwrap_or(0);

    let row = MIB_TCPROW_W2K {
        dwState: MIB_TCP_STATE_DELETE_TCB,
        dwLocalAddr: laddr,
        dwLocalPort: lport,
        dwRemoteAddr: raddr,
        dwRemotePort: rport,
    };
    // windows-sys 0.59 has no plain MIB_TCPROW; MIB_TCPROW_W2K is layout-identical
    // to MIB_TCPROW_LH (what SetTcpEntry actually takes).
    let rc = unsafe { SetTcpEntry(&row as *const MIB_TCPROW_W2K as *const MIB_TCPROW_LH) };
    if rc != 0 {
        return (500, json!({"error": format!("SetTcpEntry failed (err {}), requires admin", rc)}));
    }
    (200, json!({"ok": true}))
}

// ------------------------------------------------------ file occupancy (RM) --

fn api_file_occupiers(path: &str) -> (u16, Value) {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let path_ptr = [wide.as_ptr()];

    unsafe {
        let mut session: u32 = 0;
        // Session key: build a pseudo-GUID string (must be unique per session).
        let tick = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let key_str = format!(
            "{{{:08X}-{:04X}-{:04X}-{:04X}-{:012X}}}",
            std::process::id(),
            (tick >> 48) as u16,
            (tick >> 32) as u16,
            (tick >> 16) as u16,
            tick & 0xFFFF_FFFF_FFFF
        );
        let mut key: Vec<u16> = key_str.encode_utf16().chain(std::iter::once(0)).collect();

        let rc = RmStartSession(&mut session, 0, key.as_mut_ptr());
        if rc != 0 {
            return (500, json!({"error": format!("RmStartSession failed (err {})", rc)}));
        }

        // NOTE: closures never inherit the enclosing unsafe block, so the
        // whole body is wrapped in its own `unsafe { ... }`.
        let result = |session: u32| -> (u16, Value) { unsafe {
            let rc = RmRegisterResources(session, 1, path_ptr.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null());
            if rc != 0 {
                return (500, json!({"error": format!("RmRegisterResources failed (err {}): verify the path exists", rc)}));
            }

            // Standard two-phase RmGetList: the probing call (NULL array,
            // count=0) returns ERROR_MORE_DATA (234) and fills `needed`;
            // allocate and retry. Loop in case the set grows between calls.
            const ERROR_MORE_DATA: u32 = 234;
            let mut needed: u32 = 0;
            let mut count: u32 = 0;
            let mut reasons: u32 = 0;
            let mut list: Vec<RM_PROCESS_INFO> = Vec::new();
            let mut rc: u32 = 0;
            for _ in 0..3 {
                let array_ptr = if list.is_empty() {
                    std::ptr::null_mut()
                } else {
                    list.as_mut_ptr()
                };
                rc = RmGetList(session, &mut needed, &mut count, array_ptr, &mut reasons);
                if rc == 0 {
                    break;
                }
                if rc == ERROR_MORE_DATA {
                    list = (0..needed.max(1)).map(|_| std::mem::zeroed()).collect();
                    count = list.len() as u32;
                    continue;
                }
                return (500, json!({"error": format!("RmGetList failed (err {})", rc)}));
            }
            if rc != 0 {
                return (500, json!({"error": format!("RmGetList failed (err {})", rc)}));
            }

            let pids: Vec<u32> = list.iter().take(count as usize).map(|i| i.Process.dwProcessId).collect();
            let names = pid_names(&pids);

            let occupiers: Vec<Value> = list
                .iter()
                .take(count as usize)
                .map(|i| {
                    json!({
                        "pid": i.Process.dwProcessId,
                        "app": wide_to_string(&i.strAppName),
                        "service": wide_to_string(&i.strServiceShortName),
                        "app_type": rm_app_type_name(i.ApplicationType as i32),
                        "process": names.get(&i.Process.dwProcessId).cloned().unwrap_or_default(),
                    })
                })
                .collect();

            (200, json!({ "path": path, "count": occupiers.len(), "occupiers": occupiers }))
            }
        };

        let out = result(session);
        let _ = RmEndSession(session);
        out
    }
}

fn wide_to_string(w: &[u16]) -> String {
    let len = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..len])
}

fn rm_app_type_name(t: i32) -> String {
    let name = match t {
        1 => "Console",
        2 => "Windows GUI",
        3 => "Service",
        4 => "Explorer",
        5 => "Console (legacy)",
        6 => "Windows (legacy)",
        7 => "Service (legacy)",
        8 => "Critical",
        100 => "Other",
        _ => "Unknown",
    };
    name.to_string()
}

// --------------------------------------------------------------- elevated ----

fn is_elevated() -> bool {
    #[cfg(windows)]
    unsafe {
        let mut token: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elev: TOKEN_ELEVATION = std::mem::zeroed();
        let mut len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elev as *mut _ as *mut core::ffi::c_void,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        CloseHandle(token);
        ok != 0 && elev.TokenIsElevated != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

// -------------------------------------------------- windows-sys re-exports ---

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
#[cfg(windows)]
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, SetTcpEntry, MIB_TCPROW_LH, MIB_TCPROW_W2K,
};
#[cfg(windows)]
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
#[cfg(windows)]
use windows_sys::Win32::System::RestartManager::{
    RmEndSession, RmGetList, RmRegisterResources, RmStartSession, RM_PROCESS_INFO,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, SetProcessWorkingSetSize, TerminateProcess,
    PROCESS_TERMINATE,
};
