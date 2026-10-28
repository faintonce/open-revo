use parking_lot::{Mutex, RwLock};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::LazyLock;
use std::time::SystemTime;

static LOG_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Log levels:
/// 0 = Off (No disk writing)
/// 1 = Error only (Default: always records ERROR, WARN, and PANIC with 0 runtime overhead)
/// 2 = All logs (Verbose: records DEBUG, INFO, WARN, ERROR, and PANIC)
pub const LOG_LEVEL_OFF: u8 = 0;
pub const LOG_LEVEL_ERROR: u8 = 1;
pub const LOG_LEVEL_ALL: u8 = 2;

pub static CLI_LOG_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn is_cli_log_active() -> bool {
    CLI_LOG_MODE.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn set_cli_log_active(active: bool) {
    CLI_LOG_MODE.store(active, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_debug_mode() -> bool {
    std::env::args().any(|arg| {
        arg == "--debug" || arg == "--verbose" || arg.starts_with("--debug=") || arg.starts_with("--debug:")
    })
}

/// Parses CLI arguments and initializes logger level and filters based on flags.
pub fn init_from_cli() -> u8 {
    let args: Vec<String> = std::env::args().collect();
    let mut has_cli_log = false;
    let mut requested_level: Option<u8> = None;
    let mut cli_filter: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--debug" || arg == "--verbose" {
            has_cli_log = true;
            requested_level = Some(LOG_LEVEL_ALL);
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                i += 1;
                cli_filter = Some(args[i].clone());
            }
        } else if let Some(val) = arg.strip_prefix("--debug=").or_else(|| arg.strip_prefix("--debug:")) {
            has_cli_log = true;
            requested_level = Some(LOG_LEVEL_ALL);
            cli_filter = Some(val.to_string());
        } else if arg == "--log" {
            has_cli_log = true;
            requested_level = Some(LOG_LEVEL_ERROR);
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                i += 1;
                cli_filter = Some(args[i].clone());
            }
        } else if let Some(val) = arg.strip_prefix("--log=").or_else(|| arg.strip_prefix("--log:")) {
            has_cli_log = true;
            requested_level = Some(LOG_LEVEL_ERROR);
            cli_filter = Some(val.to_string());
        } else if arg == "--trace-custom" || arg == "--trace-tuning" {
            has_cli_log = true;
            requested_level = Some(LOG_LEVEL_ALL);
            cli_filter = Some("custom".to_string());
        } else if arg == "--filter" || arg == "--log-filter" || arg == "--module" {
            has_cli_log = true;
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                i += 1;
                cli_filter = Some(args[i].clone());
            }
        } else if let Some(val) = arg
            .strip_prefix("--filter=")
            .or_else(|| arg.strip_prefix("--filter:"))
            .or_else(|| arg.strip_prefix("--log-filter="))
            .or_else(|| arg.strip_prefix("--log-filter:"))
        {
            has_cli_log = true;
            cli_filter = Some(val.to_string());
        }
        i += 1;
    }

    let mut cfg = crate::system::config::load_config();
    crate::system::boot_trace::boot_log(
        "CONFIG",
        &format!(
            "Config loaded (power_mode={}, log_enabled={})",
            cfg.power_mode, cfg.log_enabled
        ),
    );

    // The logging feature is strictly hidden from regular users and writes 0 disk logs
    // unless explicitly launched with --log, --debug, --verbose, or a filter flag.
    let effective_level = if has_cli_log {
        set_cli_log_active(true);
        cfg.log_enabled = true;
        if let Some(lvl) = requested_level {
            cfg.log_level = lvl;
        } else if cfg.log_level == 0 {
            cfg.log_level = LOG_LEVEL_ALL;
        }

        if let Some(ref f) = cli_filter {
            cfg.log_filter = Some(f.clone());
        }
        crate::system::config::save_config(&cfg);
        cfg.log_level
    } else {
        set_cli_log_active(false);
        LOG_LEVEL_OFF
    };

    let effective_filter = if has_cli_log {
        cli_filter.as_deref().or(cfg.log_filter.as_deref())
    } else {
        None
    };

    init_logger_mode(effective_level, effective_filter);

    crate::log_info!("MAIN", "========================================");
    crate::log_info!("MAIN", "OpenRevo backend starting up...");
    if effective_level != LOG_LEVEL_OFF {
        let filter_display = get_log_filter()
            .map(|f| format!("FILTER [{}]", f))
            .unwrap_or_else(|| "ALL MODULES (No Filter)".to_string());
        crate::log_info!(
            "MAIN",
            "Logging ACTIVE -> Level: {}, Filter: {}, LogPath: {:?}",
            effective_level,
            filter_display,
            get_log_path()
        );
    } else {
        crate::log_info!("MAIN", "Logging DISABLED (0 disk wear for normal users)");
    }
    crate::log_info!("MAIN", "========================================");

    effective_level
}

static CURRENT_LOG_LEVEL: AtomicU8 = AtomicU8::new(LOG_LEVEL_ERROR);
static ACTIVE_FILTER_RAW: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));
static ACTIVE_MODULE_PATTERNS: LazyLock<RwLock<Option<Vec<String>>>> = LazyLock::new(|| RwLock::new(None));

const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024; // 10 MB threshold

/// Expands user-friendly category aliases or keywords into concrete uppercase module tag patterns.
pub fn expand_filter_patterns(raw: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    for part in raw.split([',', ';', '+', ' ', '|']) {
        let token = part.trim().to_lowercase();
        if token.is_empty() {
            continue;
        }
        match token.as_str() {
            "ec" => {
                for s in &["EC_DRIVER", "EC_BRIDGE", "EC_FAN", "EC_TUNING", "WMI_LISTENER", "ACPI"] {
                    patterns.push(s.to_string());
                }
            }
            "fan" => {
                for s in &["EC_FAN", "EC_TUNING", "BLD_FAN"] {
                    patterns.push(s.to_string());
                }
            }
            "bld" => {
                for s in &["BLD_FAN", "BLD_DRV", "BLD_RGB"] {
                    patterns.push(s.to_string());
                }
            }
            "light" | "rgb" | "lighting" | "led" => {
                for s in &["LIGHTING", "STREAMER", "STREAMER_WATCHDOG", "HARDWARE_MCU", "ITE_LIGHTING", "KEYBOARD"] {
                    patterns.push(s.to_string());
                }
            }
            "streamer" => {
                for s in &["STREAMER", "STREAMER_WATCHDOG"] {
                    patterns.push(s.to_string());
                }
            }
            "power" => {
                for s in &["POWER_MODE", "POWER_WATCHER", "EC_TUNING"] {
                    patterns.push(s.to_string());
                }
            }
            "custom" | "tuning" | "spc" | "throttle" | "lock" => {
                for s in &["CUSTOM_TRACE", "POWER_MODE", "EC_TUNING"] {
                    patterns.push(s.to_string());
                }
            }
            "device" | "switch" => {
                patterns.push("DEVICE_SWITCH".to_string());
            }
            "ui" | "win" | "window" | "tray" => {
                for s in &["WINDOW", "TRAY", "MAIN"] {
                    patterns.push(s.to_string());
                }
            }
            "boot" | "startup" | "init" => {
                for s in &["BOOT", "MAIN", "WINDOW", "TRAY", "SERVICE_MGR"] {
                    patterns.push(s.to_string());
                }
            }
            "ble" | "cooler" | "water" => {
                for s in &["BLE", "COOLER", "BLE_COOLER", "BLE_TX", "BLE_LOOP", "BLE_SMART", "COOLER_CMD", "COOLER_UI"] {
                    patterns.push(s.to_string());
                }
            }
            "hardware" => {
                for s in &["HARDWARE", "HARDWARE_MCU"] {
                    patterns.push(s.to_string());
                }
            }
            "hook" | "hooks" => {
                for s in &["HOOKS", "KEYBOARD", "DEVICE_SWITCH"] {
                    patterns.push(s.to_string());
                }
            }
            other => {
                patterns.push(other.to_uppercase());
            }
        }
    }
    patterns.sort();
    patterns.dedup();
    patterns
}

/// Check if a given module passes the current module filter.
pub fn is_module_allowed(module: &str) -> bool {
    let patterns_guard = ACTIVE_MODULE_PATTERNS.read();
    if let Some(patterns) = patterns_guard.as_ref() {
        let mod_upper = module.to_uppercase();
        patterns.iter().any(|p| mod_upper.contains(p) || p.contains(&mod_upper))
    } else {
        true
    }
}

/// Set active module filter. Passing None or "all" disables filtering (captures all modules).
pub fn set_log_filter(filter: Option<&str>) {
    let raw_val = filter.and_then(|f| {
        let trimmed = f.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("all") || trimmed.eq_ignore_ascii_case("none") {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let patterns = raw_val.as_deref().map(expand_filter_patterns);

    *ACTIVE_FILTER_RAW.write() = raw_val;
    *ACTIVE_MODULE_PATTERNS.write() = patterns;
}

/// Get active module filter string (e.g. "lighting,streamer" or None).
pub fn get_log_filter() -> Option<String> {
    ACTIVE_FILTER_RAW.read().clone()
}

/// Initialize logger mode with the given level and optional module filter.
pub fn init_logger_mode(level: u8, filter: Option<&str>) {
    CURRENT_LOG_LEVEL.store(level, Ordering::SeqCst);
    set_log_filter(filter);
    let path = get_log_path();

    // Check if existing log file exceeds threshold; if so, truncate to prevent runaway growth
    if let Ok(metadata) = std::fs::metadata(&path) {
        if metadata.len() > MAX_LOG_SIZE {
            let _ = std::fs::write(&path, b"[OpenRevo Log Truncated - Exceeded 10MB]\n");
        }
    }

    let mode_str = match level {
        LOG_LEVEL_ALL => "ALL (Verbose: DEBUG/INFO/WARN/ERROR/PANIC)",
        LOG_LEVEL_ERROR => "ERROR_ONLY (Safe: WARN/ERROR/PANIC)",
        _ => "OFF (Zero disk wear: WARN/INFO/DEBUG bypassed)",
    };
    let filter_str = match get_log_filter() {
        Some(f) => format!("Filter active -> [{}]", f),
        None => "Filter -> [ALL MODULES]".to_string(),
    };
    println!("[Logger] Log mode active -> {} | {} ({:?})", mode_str, filter_str, path);

    install_panic_hook();
}

/// Retrieve the dedicated crash log file path
pub fn get_crash_log_path() -> PathBuf {
    crate::system::config::get_config_dir().join("crash.log")
}

/// Install global crash panic hook that unconditionally captures and flushes panic details to crash.log
pub fn install_panic_hook() {
    static HOOK_INSTALLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if HOOK_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::panic::set_hook(Box::new(|info| {
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Unknown panic payload",
            },
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let thread_name = std::thread::current()
            .name()
            .unwrap_or("unnamed-thread")
            .to_string();

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let crash_report = format!(
            "\n================================================================================\n\
             [FATAL RUST PANIC / CRASH EVENT DETECTED]\n\
             Timestamp: {} (Unix Epoch Seconds)\n\
             Thread: {}\n\
             Location: {}\n\
             Panic Message: {}\n\
             ================================================================================\n",
            now, thread_name, location, msg
        );

        // 1. Immediately force-write to dedicated crash.log with disk sync
        let crash_path = get_crash_log_path();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&crash_path) {
            let _ = file.write_all(crash_report.as_bytes());
            let _ = file.flush();
            let _ = file.sync_all();
        }

        // 2. Also log to standard open-revo.log and console
        log_raw("PANIC", "CRASH", &format!("Fatal panic in thread '{}' at [{}]: {}", thread_name, location, msg));
    }));
}

pub fn get_log_level() -> u8 {
    CURRENT_LOG_LEVEL.load(Ordering::SeqCst)
}

pub fn set_log_level(level: u8) {
    CURRENT_LOG_LEVEL.store(level, Ordering::SeqCst);
}

pub fn is_file_logging_enabled() -> bool {
    CURRENT_LOG_LEVEL.load(Ordering::SeqCst) != LOG_LEVEL_OFF
}

pub fn get_log_path() -> PathBuf {
    crate::system::config::get_config_dir().join("open-revo.log")
}

pub fn log_raw(level: &str, module: &str, message: &str) {
    let is_fatal = level == "ERROR" || level == "PANIC" || module == "CRASH";
    let is_warn_or_fatal = is_fatal || level == "WARN";

    // 1. Module filter evaluation (fatal errors always bypass filter)
    if !is_fatal && !is_module_allowed(module) {
        return;
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();

    let formatted_time = format!("{}.{:03}", secs, millis);
    let log_line = format!("[{}] [{}] [{}] {}\n", formatted_time, level, module, message);

    // 2. Always print to standard console (when attached)
    print!("{}", log_line);

    // 3. Disk writing rules:
    // - LOG_LEVEL_ALL (2, via --debug): records DEBUG, INFO, WARN, ERROR, PANIC
    // - LOG_LEVEL_ERROR (1, via --log): records WARN, ERROR, PANIC
    // - LOG_LEVEL_OFF (0, default normal user): writes ONLY severe fatal ERROR / PANIC / CRASH (0 bytes disk writes during normal operation)
    let current_level = CURRENT_LOG_LEVEL.load(Ordering::Relaxed);
    let should_write = match current_level {
        LOG_LEVEL_ALL => true,
        LOG_LEVEL_ERROR => is_warn_or_fatal,
        _ => is_fatal, // Strictly zero disk writes for normal users; only severe fatal crashes/errors recorded
    };

    if should_write {
        let _guard = LOG_MUTEX.lock();
        let path = get_log_path();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        $crate::system::logger::log_raw("INFO", $module, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        $crate::system::logger::log_raw("WARN", $module, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        $crate::system::logger::log_raw("ERROR", $module, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => {
        $crate::system::logger::log_raw("DEBUG", $module, &format!($($arg)*))
    };
}
