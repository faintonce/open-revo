use std::ffi::c_void;
use std::thread::sleep;
use std::time::{Duration, Instant};

// --- Windows Win32 FFI ---

type HANDLE = *mut c_void;
const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
const GENERIC_READ: u32 = 0x80000000;
const GENERIC_WRITE: u32 = 0x40000000;
const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const OPEN_EXISTING: u32 = 3;
const FILE_FLAG_OVERLAPPED: u32 = 0x40000000;
const ERROR_IO_PENDING: u32 = 997;
const WAIT_OBJECT_0: u32 = 0;

#[link(name = "hid")]
extern "system" {
    fn HidD_GetHidGuid(hid_guid: *mut [u8; 16]);
    fn HidD_GetAttributes(hid_device_object: HANDLE, attributes: *mut Attr) -> i32;
    fn HidD_SetFeature(hid_device_object: HANDLE, report_buffer: *const c_void, report_buffer_length: u32) -> i32;
    fn HidD_GetPreparsedData(hid_device_object: HANDLE, preparsed_data: *mut isize) -> i32;
    fn HidD_FreePreparsedData(preparsed_data: isize) -> i32;
    fn HidP_GetCaps(preparsed_data: isize, capabilities: *mut Caps) -> i32;
}

#[repr(C)]
struct Attr { size: u32, vendor_id: u16, product_id: u16, version_number: u16 }

#[repr(C)]
struct Caps {
    usage: u16, usage_page: u16, input_report_byte_length: u16, output_report_byte_length: u16,
    feature_report_byte_length: u16, reserved: [u16; 17], number_link_collection_nodes: u16,
    number_input_button_caps: u16, number_input_value_caps: u16, number_input_data_indices: u16,
    number_output_button_caps: u16, number_output_value_caps: u16, number_output_data_indices: u16,
    number_feature_button_caps: u16, number_feature_value_caps: u16, number_feature_data_indices: u16,
}

#[repr(C)]
struct SpDeviceInterfaceData { cb_size: u32, interface_class_guid: [u8; 16], flags: u32, reserved: usize }

#[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    h_event: HANDLE,
}

#[link(name = "setupapi")]
extern "system" {
    fn SetupDiGetClassDevsW(class_guid: *const [u8; 16], enumerator: *const u16, hwnd_parent: *mut c_void, flags: u32) -> *mut c_void;
    fn SetupDiEnumDeviceInterfaces(device_info_set: *mut c_void, device_info_data: *mut c_void, interface_class_guid: *const [u8; 16], member_index: u32, device_interface_data: *mut SpDeviceInterfaceData) -> i32;
    fn SetupDiGetDeviceInterfaceDetailW(device_info_set: *mut c_void, device_interface_data: *mut SpDeviceInterfaceData, device_interface_detail_data: *mut u8, device_interface_detail_data_size: u32, required_size: *mut u32, device_info_data: *mut c_void) -> i32;
    fn SetupDiDestroyDeviceInfoList(device_info_set: *mut c_void) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(file_name: *const u16, desired_access: u32, share_mode: u32, security_attributes: *mut c_void, creation_disposition: u32, flags_and_attributes: u32, template_file: *mut c_void) -> HANDLE;
    fn CloseHandle(object: HANDLE) -> i32;
    fn WriteFile(file: HANDLE, buffer: *const c_void, number_of_bytes_to_write: u32, number_of_bytes_written: *mut u32, overlapped: *mut c_void) -> i32;
    fn CreateEventW(lpEventAttributes: *mut c_void, bManualReset: i32, bInitialState: i32, lpName: *const u16) -> HANDLE;
    fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: u32) -> u32;
    fn GetOverlappedResult(hFile: HANDLE, lpOverlapped: *mut c_void, lpNumberOfBytesTransferred: *mut u32, bWait: i32) -> i32;
    fn ResetEvent(hEvent: HANDLE) -> i32;
    fn GetLastError() -> u32;
}

// Simple HSV to RGB conversion
fn hsv_to_rgb(hue: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((hue % 360.0) + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 { (c, x, 0.0) }
    else if h < 120.0 { (x, c, 0.0) }
    else if h < 180.0 { (0.0, c, x) }
    else if h < 240.0 { (0.0, x, c) }
    else if h < 300.0 { (x, 0.0, c) }
    else { (c, 0.0, x) };
    (
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

fn write_chunk_ov(h: HANDLE, ev: HANDLE, data: &[u8; 65], out_len: usize) -> bool {
    let target_len = std::cmp::max(65, out_len);
    let mut buf = vec![0u8; target_len];
    buf[..65].copy_from_slice(data);

    let mut ol = Overlapped {
        internal: 0, internal_high: 0, offset: 0, offset_high: 0, h_event: ev,
    };
    unsafe {
        ResetEvent(ev);
        let mut written = 0u32;
        let res = WriteFile(h, buf.as_ptr() as *const c_void, target_len as u32, &mut written, &mut ol as *mut Overlapped as *mut c_void);
        if res != 0 { return true; }
        if GetLastError() == ERROR_IO_PENDING {
            let wait = WaitForSingleObject(ev, 300);
            if wait == WAIT_OBJECT_0 {
                let mut transferred = 0u32;
                if GetOverlappedResult(h, &mut ol as *mut Overlapped as *mut c_void, &mut transferred, 0) != 0 {
                    return true;
                }
            }
        }
        false
    }
}

fn write_feature(h: HANDLE, report: &[u8], feat_len: usize) -> bool {
    let target_len = std::cmp::max(report.len(), feat_len);
    let mut buf = vec![0u8; target_len];
    buf[..report.len()].copy_from_slice(report);
    let ok = unsafe { HidD_SetFeature(h, buf.as_ptr() as *const c_void, target_len as u32) };
    ok != 0
}

fn main() {
    println!("=================================================================");
    println!("   OpenRevo - BetterRGB ITE 8291 Standalone Minimal Diagnostic   ");
    println!("=================================================================");
    println!("This tool bypasses OpenRevo GUI, services, and IPC.");
    println!("It directly tests ITE 8291 USB HID communication and pushes a 5s RGB Rainbow.\n");

    let mut hid_guid = [0u8; 16];
    unsafe { HidD_GetHidGuid(&mut hid_guid); }

    let dev_info = unsafe { SetupDiGetClassDevsW(&hid_guid, std::ptr::null(), std::ptr::null_mut(), 0x12) };
    if dev_info.is_null() || dev_info as isize == -1 {
        eprintln!("[ERROR] SetupDiGetClassDevsW failed to retrieve HID device class set!");
        return;
    }

    let mut iface_data = SpDeviceInterfaceData {
        cb_size: std::mem::size_of::<SpDeviceInterfaceData>() as u32,
        interface_class_guid: [0; 16],
        flags: 0,
        reserved: 0,
    };

    let mut index = 0;
    let mut best_handle = INVALID_HANDLE_VALUE;
    let mut best_path = String::new();
    let mut best_feat_len = 9usize;
    let mut best_out_len = 65usize;
    let mut best_is_exclusive = false;

    println!("[1/4] Scanning all ITE 8291 HID endpoints on the system...");

    while unsafe { SetupDiEnumDeviceInterfaces(dev_info, std::ptr::null_mut(), &hid_guid, index, &mut iface_data) } != 0 {
        index += 1;
        let mut req_size = 0u32;
        unsafe { SetupDiGetDeviceInterfaceDetailW(dev_info, &mut iface_data, std::ptr::null_mut(), 0, &mut req_size, std::ptr::null_mut()); }
        if req_size > 0 {
            let mut buf = vec![0u8; req_size as usize];
            let cb_size_field = buf.as_mut_ptr() as *mut u32;
            unsafe {
                *cb_size_field = if std::mem::size_of::<usize>() == 8 { 8 } else { 5 };
            }

            if unsafe { SetupDiGetDeviceInterfaceDetailW(dev_info, &mut iface_data, buf.as_mut_ptr(), req_size, std::ptr::null_mut(), std::ptr::null_mut()) } != 0 {
                let path_ptr = unsafe { buf.as_ptr().add(4) as *const u16 };
                let mut path_len = 0;
                while unsafe { *path_ptr.add(path_len) } != 0 { path_len += 1; }
                let path_str = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(path_ptr, path_len) });
                let path_lower = path_str.to_lowercase();

                // Only inspect ITE 8291 per-key RGB keyboards.
                // 不能只认 pid_600b：蛟龙 GM7ZG7S 等机型用 0xCE00，同为 0x33 逐键协议。
                // 第二层仍由 UsagePage 0xFF03 / MI_01 / OutputLen >= 65 把关，不会误收别的设备。
                if path_lower.contains("vid_048d")
                    && (path_lower.contains("pid_600") || path_lower.contains("pid_ce00")) {
                    // Try exclusive first
                    let mut is_excl = true;
                    let mut h = unsafe {
                        CreateFileW(path_ptr, GENERIC_READ | GENERIC_WRITE, 0, std::ptr::null_mut(), OPEN_EXISTING, FILE_FLAG_OVERLAPPED, std::ptr::null_mut())
                    };
                    if h == INVALID_HANDLE_VALUE || h.is_null() {
                        is_excl = false;
                        h = unsafe {
                            CreateFileW(path_ptr, GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null_mut(), OPEN_EXISTING, FILE_FLAG_OVERLAPPED, std::ptr::null_mut())
                        };
                    }

                    if h != INVALID_HANDLE_VALUE && !h.is_null() {
                        let mut attrs = Attr { size: std::mem::size_of::<Attr>() as u32, vendor_id: 0, product_id: 0, version_number: 0 };
                        if unsafe { HidD_GetAttributes(h, &mut attrs) } != 0 {
                            let mut preparsed: isize = 0;
                            if unsafe { HidD_GetPreparsedData(h, &mut preparsed) } != 0 {
                                let mut caps: Caps = unsafe { std::mem::zeroed() };
                                unsafe { HidP_GetCaps(preparsed, &mut caps); }
                                unsafe { HidD_FreePreparsedData(preparsed); }

                                println!("  -> Found endpoint: Page=0x{:04X} Usage=0x{:04X} OutLen={} FeatLen={} Excl={}",
                                    caps.usage_page, caps.usage, caps.output_report_byte_length, caps.feature_report_byte_length, is_excl);
                                println!("     Path: {}", &path_str);

                                // Select MI_01 / Output >= 65 for streamer
                                if (path_lower.contains("mi_01") || caps.output_report_byte_length >= 65) && caps.usage_page == 0xFF03 {
                                    if best_handle != INVALID_HANDLE_VALUE {
                                        unsafe { CloseHandle(best_handle); }
                                    }
                                    best_handle = h;
                                    best_path = path_str;
                                    best_feat_len = caps.feature_report_byte_length as usize;
                                    best_out_len = caps.output_report_byte_length as usize;
                                    best_is_exclusive = is_excl;
                                    continue;
                                }
                            }
                        }
                        unsafe { CloseHandle(h); }
                    } else {
                        println!("  -> Found ITE path but could not open (Error={}): {}", unsafe { GetLastError() }, &path_str);
                    }
                }
            }
        }
    }
    unsafe { SetupDiDestroyDeviceInfoList(dev_info); }

    if best_handle == INVALID_HANDLE_VALUE {
        eprintln!("\n[FATAL] No suitable ITE 8291 Streamer endpoint (MI_01 / OutputLen >= 65 / Page 0xFF03) found!");
        eprintln!("Is OpenRevo or an OEM service holding an exclusive lock? Please check task manager.");
        return;
    }

    println!("\n[2/4] Successfully acquired target device:");
    println!("  Path: {}", &best_path);
    println!("  Lock Type: {}", if best_is_exclusive { "EXCLUSIVE (100% Guaranteed Single Owner)" } else { "SHARED (Shared with another process)" });
    println!("  Report Lengths: Feature={}, Output={}", best_feat_len, best_out_len);

    let ev = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
    if ev.is_null() || ev == INVALID_HANDLE_VALUE {
        eprintln!("[FATAL] Failed to create Win32 Event for Overlapped IO");
        unsafe { CloseHandle(best_handle); }
        return;
    }

    println!("\n[3/4] Executing BetterRGB 0x33 Handshake...");
    // Step 1
    let step1: [u8; 9] = [0x00, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00];
    let ok1 = write_feature(best_handle, &step1, best_feat_len);
    println!("  Step 1 SetFeature [0x12, 0x03]: result={} (Err={})", ok1, unsafe { GetLastError() });
    sleep(Duration::from_millis(20));

    // Clear frame
    let clear_chunk: [u8; 65] = [0; 65];
    let ok_clear = write_chunk_ov(best_handle, ev, &clear_chunk, best_out_len);
    println!("  Step Clear Frame chunk: result={} (Err={})", ok_clear, unsafe { GetLastError() });
    sleep(Duration::from_millis(20));

    // Step 2 (0x33 Custom Mode)
    let step2: [u8; 9] = [0x00, 0x08, 0x02, 0x33, 0x00, 0x32, 0x00, 0x00, 0x00];
    let ok2 = write_feature(best_handle, &step2, best_feat_len);
    println!("  Step 2 SetFeature [0x08, 0x02, 0x33]: result={} (Err={})", ok2, unsafe { GetLastError() });
    sleep(Duration::from_millis(30));

    if !ok1 || !ok2 {
        eprintln!("[WARNING] Handshake failed on step1 or step2!");
    } else {
        println!("  >>> Handshake SUCCESS! MCU entered 0x33 Per-Key Mode.");
    }

    println!("\n[4/4] Streaming smooth 60FPS Rainbow Wave for 5 seconds...");
    println!("Look at your physical keyboard right now! Does it light up with animated rainbow?\n");

    let step3: [u8; 9] = [0x00, 0x12, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00];
    let start_time = Instant::now();
    let duration = Duration::from_secs(5);
    let mut frame_count = 0;
    let mut t: f64 = 0.0;

    while start_time.elapsed() < duration {
        let frame_start = Instant::now();

        // 1. Send Frame Header
        if !write_feature(best_handle, &step3, best_feat_len) {
            eprintln!("  [WARN] Step 3 SetFeature failed at frame #{}", frame_count);
        }

        // 2. Build 512 bytes buffer (8 chunks of 64 bytes)
        let mut buf = [0u8; 512];
        for i in 0..126 {
            let offset = i * 4;
            if offset + 2 < 512 {
                let hue = (t + (i as f64) * 8.0) % 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
                buf[offset] = r;
                buf[offset + 1] = g;
                buf[offset + 2] = b;
            }
        }

        // 3. Write 8 chunks
        for chunk_idx in 0..8 {
            let mut chunk = [0u8; 65];
            chunk[0] = 0x00;
            chunk[1..65].copy_from_slice(&buf[chunk_idx * 64..(chunk_idx + 1) * 64]);
            if !write_chunk_ov(best_handle, ev, &chunk, best_out_len) {
                // Ignore transient write drops
            }
        }

        frame_count += 1;
        t += 8.0;

        if frame_count % 30 == 0 {
            print!("\r  -> Streaming active: frame #{} (Elapsed: {:.1}s)   ", frame_count, start_time.elapsed().as_secs_f32());
        }

        // 60 FPS pacing (~16.6ms)
        let elapsed_ms = frame_start.elapsed().as_millis() as u64;
        if elapsed_ms < 16 {
            sleep(Duration::from_millis(16 - elapsed_ms));
        }
    }

    println!("\n\n[DONE] 5-second streaming completed (Total frames: {}).", frame_count);
    println!("Cleaning up handles and closing device...");
    unsafe {
        CloseHandle(ev);
        CloseHandle(best_handle);
    }
    println!("Diagnostic finished. If keyboard illuminated with fluid rainbow wave, our Rust driver is 100% functional!");
}
