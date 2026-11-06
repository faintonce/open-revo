pub static IS_FILE_DIALOG_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static RECORDED_SLINT_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
pub fn get_openrevo_main_hwnd() -> isize {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextW, IsWindow, IsWindowVisible,
    };

    unsafe {
        // 0. Primary: use recorded HWND directly communicated by Slint companion via IPC
        let recorded = RECORDED_SLINT_HWND.load(std::sync::atomic::Ordering::SeqCst);
        if recorded != 0 && IsWindow(HWND(recorded as *mut _)).as_bool() {
            return recorded;
        }

        // 1. If foreground window is titled OpenRevo and visible, fast-path
        let fg = GetForegroundWindow();
        if !fg.0.is_null() && IsWindowVisible(fg).as_bool() {
            let mut title = [0u16; 64];
            let len = GetWindowTextW(fg, &mut title);
            if len > 0 {
                let s = String::from_utf16_lossy(&title[..len as usize]);
                if s == "OpenRevo" {
                    return fg.0 as isize;
                }
            }
        }

        // 2. Enumerate top-level windows to find visible window titled "OpenRevo"
        unsafe extern "system" fn enum_wnd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if IsWindowVisible(hwnd).as_bool() {
                let mut title = [0u16; 64];
                let len = GetWindowTextW(hwnd, &mut title);
                if len > 0 {
                    let s = String::from_utf16_lossy(&title[..len as usize]);
                    if s == "OpenRevo" {
                        let out_ptr = lparam.0 as *mut isize;
                        *out_ptr = hwnd.0 as isize;
                        return BOOL(0); // Stop enumeration
                    }
                }
            }
            BOOL(1) // Continue enumeration
        }

        let mut found_hwnd: isize = 0;
        let _ = EnumWindows(Some(enum_wnd_proc), LPARAM(&mut found_hwnd as *mut _ as isize));
        if found_hwnd != 0 {
            return found_hwnd;
        }

        // 3. Fallback to foreground window if non-null
        if !fg.0.is_null() {
            return fg.0 as isize;
        }

        0
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_openrevo_main_hwnd() -> isize {
    0
}

#[cfg(target_os = "windows")]
pub fn open_file_dialog(filter_name: &str, filter_ext: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, SetForegroundWindow};

    // Concurrency protection: ignore re-entrant clicks when dialog is already visible
    if IS_FILE_DIALOG_ACTIVE.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst).is_err() {
        crate::log_warn!("DIALOG", "File dialog is already active; ignoring duplicate invocation");
        return None;
    }

    struct DialogGuard;
    impl Drop for DialogGuard {
        fn drop(&mut self) {
            IS_FILE_DIALOG_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let _guard = DialogGuard;

    let hwnd_owner = get_openrevo_main_hwnd();
    if hwnd_owner != 0 {
        unsafe {
            let hwnd = HWND(hwnd_owner as *mut _);
            let _ = SetForegroundWindow(hwnd);
            let _ = BringWindowToTop(hwnd);
        }
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwndOwner: isize,
        hInstance: isize,
        lpstrFilter: *const u16,
        lpstrCustomFilter: *mut u16,
        nMaxCustFilter: u32,
        nFilterIndex: u32,
        lpstrFile: *mut u16,
        nMaxFile: u32,
        lpstrFileTitle: *mut u16,
        nMaxFileTitle: u32,
        lpstrInitialDir: *const u16,
        lpstrTitle: *const u16,
        Flags: u32,
        nFileOffset: u16,
        nFileExtension: u16,
        lpstrDefExt: *const u16,
        lCustData: isize,
        lpfnHook: isize,
        lpTemplateName: *const u16,
        pvReserved: *mut std::ffi::c_void,
        dwReserved: u32,
        FlagsEx: u32,
    }

    extern "system" {
        fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    }

    let mut filter_spec = Vec::new();
    filter_spec.extend(OsStr::new(filter_name).encode_wide());
    filter_spec.push(0);
    filter_spec.extend(OsStr::new(&format!("*.{}", filter_ext)).encode_wide());
    filter_spec.push(0);
    filter_spec.extend(OsStr::new("所有文件 (*.*)").encode_wide());
    filter_spec.push(0);
    filter_spec.extend(OsStr::new("*.*").encode_wide());
    filter_spec.push(0);
    filter_spec.push(0);

    let mut file_buf = vec![0u16; 1024];

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd_owner,
        hInstance: 0,
        lpstrFilter: filter_spec.as_ptr(),
        lpstrCustomFilter: std::ptr::null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: 1,
        lpstrFile: file_buf.as_mut_ptr(),
        nMaxFile: file_buf.len() as u32,
        lpstrFileTitle: std::ptr::null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: std::ptr::null(),
        lpstrTitle: std::ptr::null(),
        Flags: 0x00000800 | 0x00001000 | 0x00080000, // OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_EXPLORER
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: std::ptr::null(),
        lCustData: 0,
        lpfnHook: 0,
        lpTemplateName: std::ptr::null(),
        pvReserved: std::ptr::null_mut(),
        dwReserved: 0,
        FlagsEx: 0,
    };

    let ret = unsafe { GetOpenFileNameW(&mut ofn) };
    if ret != 0 {
        let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
        let path_str = String::from_utf16_lossy(&file_buf[..len]);
        if !path_str.is_empty() {
            return Some(std::path::PathBuf::from(path_str));
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn open_file_dialog(_filter_name: &str, _filter_ext: &str) -> Option<std::path::PathBuf> {
    None
}

