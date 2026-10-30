// OpenRevo - BetterRGB Per-Key High-FPS Lighting Streamer Driver
// Modular architecture for ITE 8291 keyboard real-time software streaming (15-60 FPS).

pub mod layout;
pub mod pacer;
pub mod tracker;
pub mod device;
pub mod effects;

pub use layout::*;
pub use pacer::*;
pub use tracker::*;
pub use device::*;
pub use effects::*;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};
use std::sync::Arc;
use std::thread::{self, sleep, JoinHandle};
use std::time::Duration;
use crate::driver::hid_ffi::*;
use crate::driver::lighting::parse_hex_color;
use crate::driver::wmi_ec::ADDR_BACKLIGHT_WHITE;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamerParams {
    pub enabled: bool,
    pub effect: u8,         // 0..=13 (14 种极客高刷动效)
    pub fps: u32,           // 15, 30, 45, 60
    pub brightness_pct: u8, // 0..=100
    pub speed: u8,          // 1..=10
    pub direction: u8,      // 0..=3 (Right, Left, Up, Down)
    pub color_hex: String,  // "#RRGGBB"
}

impl Default for StreamerParams {
    fn default() -> Self {
        Self {
            enabled: true,
            effect: 1, // Wave
            fps: 30,   // Default 30 FPS, super smooth & power efficient
            brightness_pct: 75,
            speed: 5,
            direction: 0,
            color_hex: "#6366F1".to_string(),
        }
    }
}

pub struct IteStreamerController {
    params: Arc<Mutex<StreamerParams>>,
    running: Arc<AtomicBool>,
    has_started: Arc<AtomicBool>,
    worker_handle: Mutex<Option<JoinHandle<()>>>,
}

pub static GLOBAL_STREAMER: std::sync::OnceLock<Arc<IteStreamerController>> = std::sync::OnceLock::new();
static GLOBAL_KEYBOARD_PROBE_CACHE: AtomicI8 = AtomicI8::new(-1);
static STREAMER_UNSUPPORTED: AtomicBool = AtomicBool::new(false);

pub fn get_global_streamer() -> Arc<IteStreamerController> {
    GLOBAL_STREAMER.get_or_init(|| {
        Arc::new(IteStreamerController::new())
    }).clone()
}

impl IteStreamerController {
    pub fn new() -> Self {
        Self {
            params: Arc::new(Mutex::new(StreamerParams::default())),
            running: Arc::new(AtomicBool::new(false)),
            has_started: Arc::new(AtomicBool::new(false)),
            worker_handle: Mutex::new(None),
        }
    }

    pub fn invalidate_probe_cache() {
        GLOBAL_KEYBOARD_PROBE_CACHE.store(-1, Ordering::Relaxed);
    }

    /// 判断 HID 接口路径（须先 to_lowercase）是否属于 ITE 逐键 RGB 键盘。
    /// 判据：VID 048D + MI_01 接口 + PID 属于 is_perkey_pid()。
    /// 与 `StreamerDevice::open()` 使用同一套规则，杜绝"探测得到但打不开"的分裂。
    pub(crate) fn is_perkey_path(path_lower: &str) -> bool {
        if !path_lower.contains("vid_048d") || !path_lower.contains("mi_01") {
            return false;
        }
        let idx = match path_lower.find("pid_") {
            Some(i) => i,
            None => return false,
        };
        // HID 接口路径里的 PID 是十六进制文本，Windows 习惯补零成 4 位（pid_600b / pid_ce00），
        // 但不能假定定长：若遇到 pid_600 这种非补零写法，旧的 idx+4..idx+8 会切到分隔符而漏判。
        // 这里扫描连续的十六进制字符，最多 4 位，不足 4 位也照常解析。
        let hex: String = path_lower[idx + 4..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .take(4)
            .collect();
        if hex.is_empty() {
            return false;
        }
        u16::from_str_radix(&hex, 16).map(is_perkey_pid).unwrap_or(false)
    }

    pub fn probe_exists() -> bool {
        let cached = GLOBAL_KEYBOARD_PROBE_CACHE.load(Ordering::Relaxed);
        if cached != -1 {
            return cached == 1;
        }

        let streamer = GLOBAL_STREAMER.get();
        if let Some(s) = streamer {
            if s.is_running() {
                GLOBAL_KEYBOARD_PROBE_CACHE.store(1, Ordering::Relaxed);
                return true;
            }
        }
        let mut hid_guid = GUID::default();
        unsafe {
            HidD_GetHidGuid(&mut hid_guid);
            let dev_info = SetupDiGetClassDevsW(&hid_guid, std::ptr::null(), std::ptr::null_mut(), 0x12);
            if dev_info == INVALID_HANDLE_VALUE {
                return false;
            }
            let mut iface_data = SP_DEVICE_INTERFACE_DATA {
                cb_size: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                interface_class_guid: GUID::default(),
                flags: 0,
                reserved: 0,
            };
            let mut index = 0;
            let mut found = false;
            while SetupDiEnumDeviceInterfaces(dev_info, std::ptr::null_mut(), &hid_guid, index, &mut iface_data) != 0 {
                index += 1;
                let mut req_size: u32 = 0;
                SetupDiGetDeviceInterfaceDetailW(dev_info, &mut iface_data, std::ptr::null_mut(), 0, &mut req_size, std::ptr::null_mut());
                if req_size > 0 {
                    let mut buf = vec![0u8; req_size as usize];
                    let cb_size_field = buf.as_mut_ptr() as *mut u32;
                    #[cfg(target_pointer_width = "64")]
                    let cb_size = 8;
                    #[cfg(target_pointer_width = "32")]
                    let cb_size = 5;
                    *cb_size_field = cb_size;
                    if SetupDiGetDeviceInterfaceDetailW(dev_info, &mut iface_data, buf.as_mut_ptr(), req_size, std::ptr::null_mut(), std::ptr::null_mut()) != 0 {
                        let path_ptr = buf.as_ptr().add(4) as *const u16;
                        let mut path_len = 0;
                        while *path_ptr.add(path_len) != 0 { path_len += 1; }
                        let path_str = String::from_utf16_lossy(std::slice::from_raw_parts(path_ptr, path_len)).to_lowercase();
                        // ITE 8291 逐键 RGB 控制接口：048D:xxxx & MI_01。
                        // 不能只认 0x600B —— 蛟龙 GM7ZG7S 等机型用 0xCE00，同为 0x33 逐键协议（实机验证）。
                        if Self::is_perkey_path(&path_str) {
                            found = true;
                            break;
                        }
                    }
                }
            }
            SetupDiDestroyDeviceInfoList(dev_info);
            if found {
                GLOBAL_KEYBOARD_PROBE_CACHE.store(1, Ordering::Relaxed);
            }
            found
        }
    }

    pub fn is_unsupported(&self) -> bool {
        STREAMER_UNSUPPORTED.load(Ordering::SeqCst)
    }

    pub fn set_unsupported(&self, val: bool) {
        STREAMER_UNSUPPORTED.store(val, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Returns true only if the worker thread had successfully started, but later exited unexpectedly
    /// without an intentional stop() call by the user or power manager.
    pub fn was_abruptly_stopped(&self) -> bool {
        self.has_started.load(Ordering::SeqCst) && !self.is_running()
    }

    pub fn get_params(&self) -> StreamerParams {
        self.params.lock().clone()
    }

    pub fn update_params(&self, new_params: StreamerParams) {
        let should_start = new_params.enabled && new_params.brightness_pct > 0;
        {
            let mut p = self.params.lock();
            *p = new_params;
        }
        if should_start && !self.is_running() {
            self.start();
        }
    }

    pub fn apply_hardware(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        direction_idx: u8,
        color_hex: &str,
    ) -> bool {
        self.stop();

        if !Self::probe_exists() {
            return false;
        }

        if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE) {
            if (val & 0x02) != 0 {
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, val & !0x02);
            }
        }

        let dev = match StreamerDevice::open() {
            Some(d) => d,
            None => {
                static APPLY_WARN_ONCE: AtomicBool = AtomicBool::new(false);
                if !APPLY_WARN_ONCE.swap(true, Ordering::SeqCst) {
                    crate::log_info!("STREAMER", "apply_hardware: No compatible ITE keyboard HID device found");
                }
                return false;
            }
        };

        dev.apply_hardware_effect(enabled, effect, brightness_idx, speed_idx, direction_idx, color_hex)
    }

    pub fn turn_off(&self) {
        self.stop();
        Self::invalidate_probe_cache();
        if !Self::probe_exists() {
            return;
        }
        if let Some(dev) = StreamerDevice::open() {
            dev.turn_off_mcu();
            sleep(Duration::from_millis(20));
            dev.stop_mcu();
            sleep(Duration::from_millis(15));
            drop(dev);
        }
    }

    pub fn init_from_saved_config() {
        if !Self::probe_exists() {
            crate::log_info!("KEYBOARD", "init_from_saved_config: No ITE 8291 RGB keyboard device detected, skipping streamer/mcu initialization");
            return;
        }
        let cfg = crate::system::config::load_config();
        let streamer = get_global_streamer();

        if cfg.lighting.kb_engine == "hardware" {
            let hw_effect = if cfg.lighting.kb_effect > 0 && cfg.lighting.kb_effect < 100 {
                cfg.lighting.kb_effect
            } else {
                3
            };
            crate::log_info!("KEYBOARD", "init_from_saved_config: activating Hardware MCU Engine (effect={})", hw_effect);
            streamer.apply_hardware(
                cfg.lighting.kb_enabled,
                hw_effect,
                cfg.lighting.kb_brightness,
                cfg.lighting.kb_speed,
                cfg.lighting.kb_direction,
                &cfg.lighting.kb_color,
            );
        } else {
            crate::log_info!("KEYBOARD", "init_from_saved_config: activating BetterRGB Streamer Engine (effect={})", cfg.lighting.streamer_effect);
            let brightness_pct = match cfg.lighting.kb_brightness {
                0 => 0,
                1 => 20,
                2 => 45,
                3 => 75,
                4 => 100,
                _ => 75,
            };
            let sp = StreamerParams {
                enabled: cfg.lighting.kb_enabled,
                effect: cfg.lighting.streamer_effect,
                fps: if cfg.lighting.kb_fps >= 15 && cfg.lighting.kb_fps <= 60 { cfg.lighting.kb_fps } else { 30 },
                brightness_pct,
                speed: cfg.lighting.kb_speed,
                direction: cfg.lighting.kb_direction,
                color_hex: if !cfg.lighting.kb_color.is_empty() { cfg.lighting.kb_color } else { "#6366F1".to_string() },
            };
            streamer.update_params(sp);
        }
    }

    pub fn set_fps(&self, fps: u32) {
        let mut p = self.params.lock();
        p.fps = fps;
    }

    pub fn start(&self) -> bool {
        if !Self::probe_exists() {
            return false;
        }

        if self.is_running() {
            return true;
        }

        self.stop();

        crate::log_info!("STREAMER", "start() called. Initiating device opening...");
        let mut dev = match StreamerDevice::open() {
            Some(d) => d,
            None => {
                crate::log_info!("STREAMER", "start(): No compatible ITE keyboard HID device found or bus settling.");
                return false;
            }
        };

        if !dev.enter_custom_mode() {
            crate::log_info!(
                "STREAMER",
                "start(): Handshake 0x33 not ready or unsupported on current interface. Deferring to recovery pipeline."
            );
            return false;
        }

        crate::log_info!("STREAMER", "start(): Handshake 0x33 succeeded! Spawning streaming worker thread...");
        self.set_unsupported(false);
        self.running.store(true, Ordering::SeqCst);
        self.has_started.store(true, Ordering::SeqCst);
        let running_clone = Arc::clone(&self.running);
        let params_ref = Arc::clone(&self.params);

        let handle = thread::spawn(move || {
            crate::log_info!("STREAMER", "⚡ BetterRGB Stream Engine Thread STARTED!");
            unsafe {
                let cur_thread = GetCurrentThread();
                let ok = SetThreadPriority(cur_thread, THREAD_PRIORITY_ABOVE_NORMAL);
                crate::log_info!("STREAMER", "🚀 Elevating stream thread priority to ABOVE_NORMAL (res={})", ok);
            }
            let key_rx = register_key_event_receiver();
            let mut pacer = FramePacer::new(30);
            let mut t: f64 = 0.0;
            let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
            let mut frame_count: u64 = 0;
            let mut err_streak: u32 = 0;
            let mut key_tracker = KeyTracker::new();
            let mut last_effect: Option<u8> = None;
            let mut is_interactive = false;

            while running_clone.load(Ordering::SeqCst) {
                let current_params = params_ref.lock().clone();
                if last_effect != Some(current_params.effect) {
                    last_effect = Some(current_params.effect);
                    t = 0.0;
                    key_tracker = KeyTracker::new();
                    buf.fill(0);
                    is_interactive = is_interactive_effect(current_params.effect);
                    set_interactive_mode(is_interactive);
                    crate::log_info!(
                        "STREAMER",
                        "Mode switched to effect #{} (interactive={}) -> animation timeline & buffers reset",
                        current_params.effect,
                        is_interactive
                    );
                }
                if !current_params.enabled || current_params.brightness_pct == 0 {
                    buf.fill(0);
                    dev.send_frame(&buf, 0);
                    sleep(Duration::from_millis(100));
                    continue;
                }

                if current_params.fps != pacer.target_fps {
                    pacer.set_fps(current_params.fps);
                }
                if is_interactive {
                    key_tracker.process_key_events(&key_rx);
                    key_tracker.poll_fallback_keys();
                }

                let (pr, pg, pb) = parse_hex_color(&current_params.color_hex);
                render_streamer_frame(&mut buf, &current_params, &mut t, &mut key_tracker, pr, pg, pb);

                let send_start = std::time::Instant::now();
                let (send_ok, send_err) = dev.send_frame(&buf, current_params.brightness_pct);
                let send_time_ms = send_start.elapsed().as_millis() as u64;
                pacer.record_transport_result(send_ok, send_time_ms);
                frame_count += 1;

                if frame_count <= 5 || frame_count == 30 || frame_count == 100 || frame_count % 300 == 0 {
                    crate::log_info!(
                        "STREAMER",
                        "Frame #{} status={}, reason={}, effect={}, target_fps={}, effective_fps={}, transport={}ms, br={}",
                        frame_count,
                        send_ok,
                        send_err,
                        current_params.effect,
                        current_params.fps,
                        pacer.effective_fps,
                        send_time_ms,
                        current_params.brightness_pct
                    );
                }

                if !send_ok {
                    err_streak += 1;
                    if err_streak == 1 || err_streak == 10 || err_streak % 100 == 0 {
                        crate::log_error!(
                            "STREAMER",
                            "send_frame FAILED at frame #{} (streak={}) reason={}",
                            frame_count,
                            err_streak,
                            send_err
                        );
                    }
                    if err_streak == 30 {
                        if let Some(new_dev) = StreamerDevice::open() {
                            // Seamless in-place reconnect: send stream setup & custom mode directly WITHOUT the 65-byte all-zero clear_frame that causes black flash!
                            let step1: [u8; 9] = [0x00, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00];
                            let _ = new_dev.write_feature(&step1);
                            let step2: [u8; 9] = [0x00, 0x08, 0x02, 0x33, 0x00, 0x32, 0x00, 0x00, 0x00];
                            if new_dev.write_feature(&step2) {
                                dev = new_dev;
                                err_streak = 0;
                                crate::log_info!("STREAMER", "In-place USB device reconnected successfully without black frame flash");
                            }
                        }
                    }
                    if err_streak >= 150 {
                        crate::log_warn!(
                            "STREAMER",
                            "send_frame persistent failure (streak={}). Device likely suspended/disconnected. Exiting worker thread.",
                            err_streak
                        );
                        running_clone.store(false, Ordering::SeqCst);
                        break;
                    }
                } else {
                    err_streak = 0;
                }

                pacer.pace();
            }

            unregister_key_event_sender();
            dev.stop_mcu();
            sleep(Duration::from_millis(20));
            drop(dev);
            crate::log_info!("STREAMER", "🛑 BetterRGB Stream Engine Thread STOPPED! (total rendered frames: {})", frame_count);
        });

        *self.worker_handle.lock() = Some(handle);
        true
    }

    pub fn stop(&self) {
        self.has_started.store(false, Ordering::SeqCst);
        if !self.running.swap(false, Ordering::SeqCst) {
            return;
        }

        let mut handle_guard = self.worker_handle.lock();
        if let Some(h) = handle_guard.take() {
            let _ = h.join();
        }
        sleep(Duration::from_millis(50));
    }

    pub fn handover_to_hardware_on_exit(&self) {
        self.stop();

        if !Self::probe_exists() {
            return;
        }

        let cfg = crate::system::config::load_config();
        if cfg.lighting.kb_enabled && cfg.lighting.kb_brightness > 0 {
            let hardware_effect = if cfg.lighting.kb_effect > 0 && cfg.lighting.kb_effect < 100 {
                cfg.lighting.kb_effect
            } else {
                3
            };
            let speed = if cfg.lighting.kb_speed == 0 { 2 } else { cfg.lighting.kb_speed };
            let color = if !cfg.lighting.kb_color.is_empty() {
                cfg.lighting.kb_color.as_str()
            } else {
                "#6366F1"
            };

            let ok = self.apply_hardware(
                true,
                hardware_effect,
                cfg.lighting.kb_brightness,
                speed,
                cfg.lighting.kb_direction,
                color,
            );
            crate::log_info!(
                "STREAMER",
                "handover_to_hardware_on_exit: handed over to hardware MCU (user_hw_effect={}, br={}, ok={})",
                hardware_effect,
                cfg.lighting.kb_brightness,
                ok
            );
        } else {
            self.turn_off();
            crate::log_info!("STREAMER", "handover_to_hardware_on_exit: keyboard is configured OFF, turning off MCU");
        }
    }

    /// 把键盘控制权交还 OEM / 官方控制台。
    ///
    /// 与 `handover_to_hardware_on_exit()` 不同：那个是「退出时让 MCU 自己跑效果」，
    /// 这个是「彻底让出」—— 停流式线程、让 MCU 退出 0x33 自定义模式、释放独占的 HID 句柄。
    ///
    /// 归还时必须调用，否则官方控制台即使服务已恢复也抢不回灯控：HID 是单点独占，
    /// 且 MCU 停在 0x33 自定义模式下不会响应 OEM 指令（实机验证：只调 resume_oem_services()
    /// 后官方控制台点效果仍然无效，必须重启本程序才能夺回）。
    pub fn release_to_oem(&self) {
        self.stop();
        Self::invalidate_probe_cache();
        if !Self::probe_exists() {
            crate::log_info!("STREAMER", "release_to_oem: no ITE keyboard present, nothing to release");
            return;
        }
        // worker 线程收尾时已经发过 stop_mcu；这里再补一次，覆盖「流式线程没在跑，
        // 但 MCU 被 apply_hardware 停在自定义模式」的情况（kb_engine == "hardware"）。
        if let Some(dev) = StreamerDevice::open() {
            let _ = dev.stop_mcu();
            sleep(Duration::from_millis(20));
            drop(dev);
        }
        crate::log_info!(
            "STREAMER",
            "release_to_oem: streamer stopped, MCU left 0x33 mode, HID handle released back to OEM"
        );
    }

    pub fn suspend_for_sleep(&self) {
        crate::log_info!("STREAMER", "suspend_for_sleep: stopping streamer thread");
        self.stop();
    }

    pub fn resume_after_wake(&self) {
        Self::invalidate_probe_cache();

        static RESUMING: AtomicBool = AtomicBool::new(false);
        if RESUMING.swap(true, Ordering::SeqCst) {
            return;
        }
        struct ResumeGuard;
        impl Drop for ResumeGuard {
            fn drop(&mut self) {
                RESUMING.store(false, Ordering::SeqCst);
            }
        }
        let _guard = ResumeGuard;

        crate::log_info!("STREAMER", "resume_after_wake called: gracefully restoring keyboard lighting");
        if self.is_running() {
            crate::log_info!("STREAMER", "resume_after_wake: Streamer already running and active. Keeping smooth session.");
            return;
        }
        self.stop();
        self.set_unsupported(false);

        if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE) {
            if (val & 0x02) != 0 {
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, val & !0x02);
            }
        }

        let cfg = crate::system::config::load_config();
        if cfg.lighting.kb_engine == "hardware" {
            let hw_effect = if cfg.lighting.kb_effect > 0 && cfg.lighting.kb_effect < 100 {
                cfg.lighting.kb_effect
            } else {
                3
            };
            let _ = self.apply_hardware(
                cfg.lighting.kb_enabled,
                hw_effect,
                cfg.lighting.kb_brightness,
                cfg.lighting.kb_speed,
                cfg.lighting.kb_direction,
                &cfg.lighting.kb_color,
            );
            return;
        }

        // Streamer Engine startup & recovery pipeline:
        let brightness_pct = match cfg.lighting.kb_brightness {
            0 => 0,
            1 => 20,
            2 => 45,
            3 => 75,
            4 => 100,
            _ => 75,
        };
        let sp = StreamerParams {
            enabled: cfg.lighting.kb_enabled,
            effect: cfg.lighting.streamer_effect,
            fps: if cfg.lighting.kb_fps >= 15 && cfg.lighting.kb_fps <= 60 { cfg.lighting.kb_fps } else { 30 },
            brightness_pct,
            speed: cfg.lighting.kb_speed,
            direction: cfg.lighting.kb_direction,
            color_hex: if !cfg.lighting.kb_color.is_empty() { cfg.lighting.kb_color.clone() } else { "#6366F1".to_string() },
        };
        {
            let mut p = self.params.lock();
            *p = sp;
        }

        Self::invalidate_probe_cache();
        if self.start() {
            crate::log_info!("KEYBOARD", "resume_after_wake: BetterRGB Streamer connected immediately on fast path");
            return;
        }

        // Fast-path missed due to Windows USB bus settling / PnP re-enumeration.
        // DO NOT blink or cut hardware MCU power! Launch background silent recovery worker.
        crate::log_info!("KEYBOARD", "resume_after_wake: Fast-path USB connection pending, spawning silent smooth recovery worker");
        std::thread::spawn(move || {
            for attempt in 1..=4 {
                sleep(Duration::from_millis(600));
                let cur_cfg = crate::system::config::load_config();
                if cur_cfg.lighting.kb_engine != "streamer" || !cur_cfg.lighting.kb_enabled {
                    return;
                }
                let streamer = crate::driver::ite_streamer::get_global_streamer();
                if streamer.is_running() {
                    return;
                }
                Self::invalidate_probe_cache();
                if streamer.start() {
                    crate::log_info!("KEYBOARD", "resume_after_wake: BetterRGB Streamer silent recovery SUCCEEDED on attempt {}", attempt);
                    return;
                }
            }
            crate::log_info!("KEYBOARD", "resume_after_wake: Streamer hardware not detected after settling period. Safely defaulting to Hardware MCU state.");
            let cur_cfg = crate::system::config::load_config();
            let hw_effect = if cur_cfg.lighting.kb_effect > 0 && cur_cfg.lighting.kb_effect < 100 {
                cur_cfg.lighting.kb_effect
            } else {
                3
            };
            let streamer = crate::driver::ite_streamer::get_global_streamer();
            streamer.set_unsupported(true);
            streamer.apply_hardware(
                cur_cfg.lighting.kb_enabled,
                hw_effect,
                cur_cfg.lighting.kb_brightness,
                cur_cfg.lighting.kb_speed,
                cur_cfg.lighting.kb_direction,
                &cur_cfg.lighting.kb_color,
            );
        });
    }
}
