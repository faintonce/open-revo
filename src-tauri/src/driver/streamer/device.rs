// OpenRevo - BetterRGB HID Transport & Low-Level Hardware Communication
// Manages Win32 Overlapped I/O, device enumeration, 65-byte chunk transmission, and MCU states.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;
use crate::driver::hid_ffi::*;
use crate::driver::lighting::parse_hex_color;
use crate::driver::wmi_ec::ADDR_AP_OEM_BYTE;
use super::layout::{is_perkey_pid, set_active_layout, BUF_SIZE, ITE_PID_KEYBOARD_CE00, ITE_VID, LAYOUT_CE00, LAYOUT_DEFAULT, USAGE_PAGE_ITE};

pub struct StreamerDeviceInfo {
    pub handle: HANDLE,
    pub path: String,
    pub feature_report_len: usize,
    pub output_report_len: usize,
}

pub struct StreamerDevice {
    handle: HANDLE,
    event: HANDLE,
    path: String,
    feature_report_len: usize,
    output_report_len: usize,
}

unsafe impl Send for StreamerDevice {}
unsafe impl Sync for StreamerDevice {}

impl Drop for StreamerDevice {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE && !self.handle.is_null() {
            unsafe { CloseHandle(self.handle); }
            self.handle = INVALID_HANDLE_VALUE;
        }
        if self.event != INVALID_HANDLE_VALUE && !self.event.is_null() {
            unsafe { CloseHandle(self.event); }
            self.event = INVALID_HANDLE_VALUE;
        }
    }
}

impl StreamerDevice {
    pub fn open() -> Option<Self> {
        match open_raw_ite_keyboard_device() {
            Some(info) => {
                // Auto-reset event (bManualReset = 0) aligns with BetterRGB hidapi to prevent spurious completions
                let event = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
                if event == INVALID_HANDLE_VALUE || event.is_null() {
                    crate::log_error!("STREAMER", "Failed to create Win32 Event for Overlapped I/O");
                    unsafe { CloseHandle(info.handle); }
                    return None;
                }
                crate::log_info!(
                    "STREAMER",
                    "StreamerDevice (Overlapped) opened successfully. feat_len={}, out_len={}, path={}",
                    info.feature_report_len,
                    info.output_report_len,
                    &info.path
                );
                Some(Self {
                    handle: info.handle,
                    event,
                    path: info.path,
                    feature_report_len: info.feature_report_len,
                    output_report_len: info.output_report_len,
                })
            }
            None => {
                static DEV_OPEN_LOGGED: AtomicBool = AtomicBool::new(false);
                if !DEV_OPEN_LOGGED.swap(true, Ordering::SeqCst) {
                    crate::log_info!("STREAMER", "StreamerDevice::open() failed to find compatible device");
                }
                None
            }
        }
    }

    pub fn write_feature(&self, report: &[u8]) -> bool {
        if self.handle == INVALID_HANDLE_VALUE || self.handle.is_null() {
            crate::log_debug!("STREAMER", "write_feature: device handle is null or invalid");
            return false;
        }
        unsafe {
            // Step 1: Follow official BetterRGB hidapi standard:
            // Windows expects at least caps.FeatureReportByteLength bytes passed to HidD_SetFeature.
            // Any less sent causes ERROR_INVALID_PARAMETER (87). Pad with zeros to self.feature_report_len.
            let target_len = std::cmp::max(report.len(), self.feature_report_len);
            let mut stack_buf = [0u8; 64];
            let buf_ptr = if target_len <= 64 {
                stack_buf[..report.len()].copy_from_slice(report);
                stack_buf.as_ptr()
            } else {
                return false;
            };

            let ok = HidD_SetFeature(
                self.handle,
                buf_ptr as *const c_void,
                target_len as u32,
            );
            if ok != 0 {
                return true;
            }

            // Step 2: Fallback to exact report length if a non-standard driver strictly rejects padding
            if report.len() != target_len {
                let fallback_ok = HidD_SetFeature(
                    self.handle,
                    report.as_ptr() as *const c_void,
                    report.len() as u32,
                );
                if fallback_ok != 0 {
                    return true;
                }
            }

            let err = GetLastError();
            crate::log_debug!(
                "STREAMER",
                "HidD_SetFeature failed! err={}, report_len={}, feat_len={}, report[0..3]={:02X?}",
                err,
                report.len(),
                self.feature_report_len,
                &report[..std::cmp::min(4, report.len())]
            );
            false
        }
    }

    pub fn write_chunk(&self, chunk_65: &[u8; 65]) -> bool {
        if self.handle == INVALID_HANDLE_VALUE || self.handle.is_null() {
            crate::log_error!("STREAMER", "write_chunk: device handle is null or invalid");
            return false;
        }
        unsafe {
            // Zero heap allocation: Use stack buffer for 65-byte chunks
            let target_len = std::cmp::max(65, self.output_report_len);
            let mut stack_buf = [0u8; 128];
            let buf_ptr = if target_len <= 128 {
                stack_buf[..65].copy_from_slice(chunk_65);
                stack_buf.as_ptr()
            } else {
                return false;
            };

            let mut ol = OVERLAPPED {
                internal: 0,
                internal_high: 0,
                offset: 0,
                offset_high: 0,
                h_event: self.event,
            };
            ResetEvent(self.event);

            let mut written: u32 = 0;
            let res = WriteFile(
                self.handle,
                buf_ptr as *const c_void,
                target_len as u32,
                &mut written,
                &mut ol as *mut OVERLAPPED as *mut c_void,
            );

            if res != 0 {
                return true;
            }

            let err = GetLastError();
            if err == ERROR_IO_PENDING {
                // BetterRGB sets write timeout to 1000ms. 200ms was prematurely timing out on bus congestion
                let wait = WaitForSingleObject(self.event, 1000);
                if wait == WAIT_OBJECT_0 {
                    let mut transferred: u32 = 0;
                    if GetOverlappedResult(self.handle, &mut ol as *mut OVERLAPPED as *mut c_void, &mut transferred, 0) != 0 {
                        return true;
                    }
                }
                CancelIoEx(self.handle, &mut ol as *mut OVERLAPPED as *mut c_void);
                crate::log_warn!("STREAMER", "write_chunk OVERLAPPED timeout/failed! wait={}, err={}", wait, GetLastError());
                return false;
            }

            crate::log_warn!("STREAMER", "write_chunk WriteFile failed immediately! err={}", err);
            false
        }
    }

    pub const HW_SPEED_MAP: [u8; 5] = [10, 7, 5, 3, 1];
    pub const HW_BRIGHTNESS_MAP: [u8; 5] = [0, 8, 22, 36, 50];
    pub const HW_DIRECTION_MAP: [u8; 4] = [1, 2, 3, 4];
    pub const HW_RAINBOW_COLORS: [(u8, u8, u8); 7] = [
        (255, 0, 0),     // Red
        (255, 165, 0),   // Orange
        (255, 255, 0),   // Yellow
        (0, 255, 0),     // Green
        (0, 255, 255),   // Cyan
        (0, 0, 255),     // Blue
        (128, 0, 128),   // Purple
    ];

    /// Stop MCU animation loop and reset state machine
    pub fn stop_mcu(&self) -> bool {
        let stop_pkt: [u8; 9] = [0x00, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        self.write_feature(&stop_pkt)
    }

    /// Turn off keyboard backlight LEDs completely
    pub fn turn_off_mcu(&self) -> bool {
        let stop_pkt: [u8; 9] = [0x00, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        self.write_feature(&stop_pkt);
        sleep(Duration::from_millis(15));
        let off_pkt: [u8; 9] = [0x00, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        self.write_feature(&off_pkt)
    }

    /// Apply hardware MCU built-in effect directly to ITE 8291
    pub fn apply_hardware_effect(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        direction_idx: u8,
        color_hex: &str,
    ) -> bool {
        if !enabled || brightness_idx == 0 {
            let off_pkt: [u8; 9] = [0x00, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            return self.write_feature(&off_pkt);
        }

        // Step 1: Send CONTROL_STOP to reset MCU state machine
        let stop_pkt: [u8; 9] = [0x00, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        self.write_feature(&stop_pkt);
        sleep(Duration::from_millis(20));

        let speed = Self::HW_SPEED_MAP.get(speed_idx as usize).copied().unwrap_or(5);
        let light = Self::HW_BRIGHTNESS_MAP.get(brightness_idx as usize).copied().unwrap_or(36);

        let actual_effect = match effect {
            22 => 6,
            18 => 17,
            e => e,
        };

        let is_on_key_pressed = actual_effect == 4 || actual_effect == 6 || actual_effect == 14 || actual_effect == 17;
        if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(ADDR_AP_OEM_BYTE) {
            let target = if is_on_key_pressed { val | 0x08 } else { val & !0x08 };
            if target != val {
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_AP_OEM_BYTE, target);
            }
        }

        let direction = if is_on_key_pressed {
            1
        } else {
            Self::HW_DIRECTION_MAP.get(direction_idx as usize).copied().unwrap_or(1)
        };

        let is_circular = actual_effect == 3
            || actual_effect == 5
            || actual_effect == 6
            || actual_effect == 9
            || actual_effect == 10
            || actual_effect == 14
            || actual_effect == 17;

        let color_index = if is_circular { 8 } else { 1 };
        let (r, g, b) = parse_hex_color(color_hex);

        // Step 2: Wake up channel 2 with target brightness
        let wake_pkt: [u8; 9] = [0x00, 0x08, 0x02, 0x00, 0x00, light, 0x00, 0x00, 0x00];
        self.write_feature(&wake_pkt);
        sleep(Duration::from_millis(15));

        // Step 3: Color palette slots (Save = 0x00 to protect 8291 eFlash)
        if is_circular {
            for (idx, &(cr, cg, cb)) in Self::HW_RAINBOW_COLORS.iter().enumerate() {
                let slot1 = (idx + 1) as u8;
                let pkt1: [u8; 9] = [0x00, 0x14, 0x00, slot1, cr, cg, cb, 0x00, 0x00];
                self.write_feature(&pkt1);

                let slot2 = (idx + 9) as u8;
                let pkt2: [u8; 9] = [0x00, 0x14, 0x00, slot2, cr, cg, cb, 0x00, 0x00];
                self.write_feature(&pkt2);
            }
        } else {
            let color_pkt: [u8; 9] = [0x00, 0x14, 0x00, 0x01, r, g, b, 0x00, 0x00];
            self.write_feature(&color_pkt);
        }
        sleep(Duration::from_millis(15));

        // Step 4: Write hardware effect packet (Save = 0x00 for RAM execution)
        let effect_pkt: [u8; 9] = [
            0x00,
            0x08,
            0x02,
            actual_effect,
            speed,
            light,
            color_index,
            direction,
            0x00,
        ];
        let ok = self.write_feature(&effect_pkt);

        crate::log_info!("STREAMER", "apply_hardware_effect: effect={}, speed={}, light={}, dir={}, ok={}", actual_effect, speed, light, direction, ok);
        ok
    }

    /// Enter Custom Per-Key Streaming Mode (0x33) following BetterRGB official 3-step protocol
    pub fn enter_custom_mode(&self) -> bool {
        crate::log_debug!(
            "STREAMER",
            "enter_custom_mode: initiating BetterRGB handshake on {} (feat_len={}, out_len={})",
            &self.path,
            self.feature_report_len,
            self.output_report_len
        );

        for attempt in 1..=3 {
            // Step 1: Feature report 00 12 00 03 00 00 00 00 00 (stream setup)
            let step1: [u8; 9] = [0x00, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00];
            if !self.write_feature(&step1) {
                crate::log_debug!("STREAMER", "enter_custom_mode: step 1 (0x12 0x03) failed on attempt {}, retrying...", attempt);
                sleep(Duration::from_millis(50));
                continue;
            }

            // Step 2: Clear buffer only after step1 confirmed (Report ID 0x00 + 64 zero bytes)
            let clear_frame: [u8; 65] = [0; 65];
            let _ = self.write_chunk(&clear_frame);
            sleep(Duration::from_millis(15));

            // Step 3: Feature report 00 08 02 33 00 32 00 00 00 (enable custom mode 0x33)
            let step2: [u8; 9] = [0x00, 0x08, 0x02, 0x33, 0x00, 0x32, 0x00, 0x00, 0x00];
            if self.write_feature(&step2) {
                crate::log_info!("STREAMER", "enter_custom_mode: successfully entered 0x33 mode on attempt {}", attempt);
                sleep(Duration::from_millis(20));
                return true;
            } else {
                crate::log_debug!("STREAMER", "enter_custom_mode: step 2 (0x33) failed on attempt {}", attempt);
                sleep(Duration::from_millis(50));
            }
        }

        crate::log_info!("STREAMER", "enter_custom_mode: handshake not ready on current interface attempt");
        false
    }

    /// Send one full 512-byte RGB frame
    pub fn send_frame(&self, buf: &[u8; BUF_SIZE], brightness_pct: u8) -> (bool, &'static str) {
        let step3: [u8; 9] = [0x00, 0x12, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00];
        if !self.write_feature(&step3) {
            return (false, "step3_feature_failed");
        }

        let bright_factor = brightness_pct.clamp(0, 100) as u16;
        for i in 0..8 {
            let mut chunk: [u8; 65] = [0; 65];
            chunk[0] = 0x00;
            let src = &buf[i * 64..(i + 1) * 64];
            if bright_factor == 100 {
                chunk[1..65].copy_from_slice(src);
            } else {
                for (dst, &s) in chunk[1..65].iter_mut().zip(src.iter()) {
                    *dst = ((s as u16 * bright_factor) / 100) as u8;
                }
            }
            if !self.write_chunk(&chunk) {
                return (false, "chunk_write_failed");
            }
        }
        (true, "ok")
    }
}

pub fn open_raw_ite_keyboard_device() -> Option<StreamerDeviceInfo> {
    unsafe {
        let mut hid_guid = GUID::default();
        HidD_GetHidGuid(&mut hid_guid);
        let dev_info = SetupDiGetClassDevsW(&hid_guid, std::ptr::null(), std::ptr::null_mut(), 0x12);
        if dev_info == INVALID_HANDLE_VALUE {
            crate::log_error!("STREAMER", "SetupDiGetClassDevsW failed to get HID device set");
            return None;
        }

        let mut iface_data = SP_DEVICE_INTERFACE_DATA {
            cb_size: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            interface_class_guid: GUID::default(),
            flags: 0,
            reserved: 0,
        };

        let mut best_handle = INVALID_HANDLE_VALUE;
        let mut best_score = -1;
        let mut best_path = String::new();
        let mut best_feat_len = 9;
        let mut best_out_len = 65;
        // 与 best_handle 绑定的键位表，只有在候选真正胜出时才生效，
        // 否则被跳过的候选会把 layout 改成下一台机器不该用的表。
        let mut best_layout = LAYOUT_DEFAULT;

        for idx in 0..128 {
            if SetupDiEnumDeviceInterfaces(dev_info, std::ptr::null_mut(), &hid_guid, idx, &mut iface_data) == 0 {
                break;
            }

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

                    // 与 probe_exists() 共用判据：VID 048D + MI_01 + 逐键 PID（含 0xCE00）
                    if !super::IteStreamerController::is_perkey_path(&path_str) {
                        continue;
                    }

                    let mut h = CreateFileW(
                        path_ptr,
                        GENERIC_READ | GENERIC_WRITE,
                        0, // exclusive
                        std::ptr::null_mut(),
                        OPEN_EXISTING,
                        FILE_FLAG_OVERLAPPED,
                        std::ptr::null_mut(),
                    );
                    if h == INVALID_HANDLE_VALUE || h.is_null() {
                        h = CreateFileW(
                            path_ptr,
                            GENERIC_READ | GENERIC_WRITE,
                            FILE_SHARE_READ | FILE_SHARE_WRITE,
                            std::ptr::null_mut(),
                            OPEN_EXISTING,
                            FILE_FLAG_OVERLAPPED,
                            std::ptr::null_mut(),
                        );
                    }
                    if h != INVALID_HANDLE_VALUE && !h.is_null() {
                        let mut attrs = HIDD_ATTRIBUTES { size: std::mem::size_of::<HIDD_ATTRIBUTES>() as u32, vendor_id: 0, product_id: 0, version_number: 0 };
                        if HidD_GetAttributes(h, &mut attrs) != 0 && attrs.vendor_id == ITE_VID && is_perkey_pid(attrs.product_id) {
                            let mut preparsed: isize = 0;
                            if HidD_GetPreparsedData(h, &mut preparsed) != 0 {
                                let mut caps: HIDP_CAPS = std::mem::zeroed();
                                HidP_GetCaps(preparsed, &mut caps);
                                HidD_FreePreparsedData(preparsed);

                                if caps.usage_page == 0x0001 && caps.usage == 0x0006 {
                                    CloseHandle(h);
                                    continue;
                                }

                                let mut score = 0;
                                if caps.usage_page == USAGE_PAGE_ITE || caps.usage_page == 0xFF03 {
                                    score += 50;
                                }
                                if path_str.contains("&mi_01") || path_str.contains("mi_01") {
                                    score += 30;
                                }
                                if caps.output_report_byte_length >= 65 {
                                    score += 20;
                                }
                                // Standard ITE 8291 RGB keyboard control uses exact 9-byte feature report
                                if caps.feature_report_byte_length == 9 {
                                    score += 30;
                                } else if caps.feature_report_byte_length > 9 {
                                    score += 5;
                                }

                                crate::log_info!(
                                    "STREAMER",
                                    "Discovered Candidate: score={}, page=0x{:04X}, usage=0x{:04X}, out_len={}, feat_len={}, path={}",
                                    score,
                                    caps.usage_page,
                                    caps.usage,
                                    caps.output_report_byte_length,
                                    caps.feature_report_byte_length,
                                    &path_str
                                );

                                if score > best_score {
                                    if best_handle != INVALID_HANDLE_VALUE {
                                        CloseHandle(best_handle);
                                    }
                                    best_score = score;
                                    best_handle = h;
                                    // 0xCE00 的灯位与 0x600B 基准表有 4 处实测差异，
                                    // 直接套用基准表会出现「右 Shift 不亮、上方向键/右 Ctrl 缺灯」。
                                    best_layout = if attrs.product_id == ITE_PID_KEYBOARD_CE00 {
                                        LAYOUT_CE00
                                    } else {
                                        LAYOUT_DEFAULT
                                    };
                                    best_path = path_str;
                                    best_feat_len = if caps.feature_report_byte_length > 0 {
                                        caps.feature_report_byte_length as usize
                                    } else {
                                        9
                                    };
                                    best_out_len = if caps.output_report_byte_length > 0 {
                                        caps.output_report_byte_length as usize
                                    } else {
                                        65
                                    };
                                    continue;
                                }
                            }
                        }
                        CloseHandle(h);
                    }
                }
            }
        }
        SetupDiDestroyDeviceInfoList(dev_info);

        if best_handle != INVALID_HANDLE_VALUE {
            crate::log_info!(
                "STREAMER",
                "Selected Best ITE Streamer Device (score={}, feat_len={}, out_len={}, path={})",
                best_score,
                best_feat_len,
                best_out_len,
                best_path
            );
            // 键位表必须在流式线程启动前确定：effects.rs / custom_fx_engine.rs 每帧都读它。
            set_active_layout(best_layout);
            Some(StreamerDeviceInfo {
                handle: best_handle,
                path: best_path,
                feature_report_len: best_feat_len,
                output_report_len: best_out_len,
            })
        } else {
            static LOGGED_ONCE: AtomicBool = AtomicBool::new(false);
            if !LOGGED_ONCE.swap(true, Ordering::SeqCst) {
                crate::log_info!("STREAMER", "No suitable ITE RGB streamer device found (need MI_01 / OutputReport >= 65)");
            }
            None
        }
    }
}
