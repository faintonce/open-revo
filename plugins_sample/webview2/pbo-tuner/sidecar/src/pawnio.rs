// OpenRevo - PawnIO kernel driver bridge (Ring0 hardware access)
// Adapted from ZenStates-Core (SMUDebugTool) PawnIo.cs.
//
// PawnIO is a scriptable universal kernel driver (a modern, safer WinRing0
// replacement). This module talks to it directly through DeviceIoControl on
// the device object `\\.\GLOBALROOT\Device\PawnIO`, loading the RyzenSMU.bin
// blob and executing its exported SMU functions.

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;

    fn CreateFileA(
        lpFileName: *const u8,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut std::ffi::c_void,
    ) -> isize;

    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;

    fn DeviceIoControl(
        hDevice: isize,
        dwIoControlCode: u32,
        lpInBuffer: *const std::ffi::c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut std::ffi::c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut std::ffi::c_void,
    ) -> i32;
}

// PawnIO control codes, reproduced from ZenStates-Core.
// DEVICE_TYPE = 41394u << 16 = 0xA1C20000
const DEVICE_TYPE: u32 = 41394u32 << 16;
const IOCTL_PIO_LOAD_BINARY: u32 = 0x821 << 2;
const IOCTL_PIO_EXECUTE_FN: u32 = 0x841 << 2;
const CTL_LOAD_BINARY: u32 = DEVICE_TYPE | IOCTL_PIO_LOAD_BINARY;
const CTL_EXECUTE: u32 = DEVICE_TYPE | IOCTL_PIO_EXECUTE_FN;
const FN_NAME_LENGTH: usize = 32;

const DEVICE_PATH: &[u8] = b"\\\\.\\GLOBALROOT\\Device\\PawnIO\0";

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ_WRITE: u32 = 3;
const OPEN_EXISTING: u32 = 3;
const INVALID_HANDLE: isize = -1;

/// A handle to a loaded PawnIO executor (a PawnIO module, e.g. RyzenSMU.bin).
pub struct PawnIo {
    handle: isize,
    loaded: bool,
}

impl PawnIo {
    /// Opens the PawnIO device and loads the given module blob.
    pub fn load_module_from_bytes(blob: &[u8]) -> Result<Self, String> {
        let handle = unsafe { open_device()? };
        let loaded = unsafe { load_binary(handle, blob) };

        if !loaded {
            unsafe { CloseHandle(handle as *mut _) };
            return Err("PawnIO failed to load module blob".into());
        }

        Ok(PawnIo { handle, loaded: true })
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Executes a named function exported by the loaded module.
    /// Returns the output u64 array.
    pub fn execute(&self, name: &str, input: &[u64], out_len: usize) -> Result<Vec<u64>, String> {
        if !self.loaded {
            return Err("PawnIO module not loaded".into());
        }
        unsafe { execute_fn(self.handle, name, input, out_len) }
    }

    /// Executes a named function and returns the raw HRESULT together with output.
    pub fn execute_hr(
        &self,
        name: &str,
        input: &[u64],
        out_len: usize,
    ) -> Result<(u32, Vec<u64>), String> {
        if !self.loaded {
            return Err("PawnIO module not loaded".into());
        }
        unsafe { execute_fn_hr(self.handle, name, input, out_len) }
    }
}

impl Drop for PawnIo {
    fn drop(&mut self) {
        if self.loaded {
            unsafe {
                CloseHandle(self.handle as *mut _);
            }
        }
    }
}

unsafe fn open_device() -> Result<isize, String> {
    let h = CreateFileA(
        DEVICE_PATH.as_ptr(),
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ_WRITE,
        std::ptr::null_mut(),
        OPEN_EXISTING,
        0,
        std::ptr::null_mut(),
    );
    if h == INVALID_HANDLE || h == 0 {
        return Err(format!(
            "Failed to open PawnIO device (error {})",
            GetLastError()
        ));
    }
    Ok(h)
}

unsafe fn load_binary(handle: isize, blob: &[u8]) -> bool {
    let mut returned: u32 = 0;
    let ok = DeviceIoControl(
        handle,
        CTL_LOAD_BINARY,
        blob.as_ptr() as *const std::ffi::c_void,
        blob.len() as u32,
        std::ptr::null_mut(),
        0,
        &mut returned,
        std::ptr::null_mut(),
    );
    ok != 0
}

unsafe fn execute_fn(
    handle: isize,
    name: &str,
    input: &[u64],
    out_len: usize,
) -> Result<Vec<u64>, String> {
    let (hr, out) = execute_fn_hr(handle, name, input, out_len)?;
    if hr != 0 {
        return Err(format!("PawnIO execute failed HRESULT 0x{:08X}", hr));
    }
    Ok(out)
}

unsafe fn execute_fn_hr(
    handle: isize,
    name: &str,
    input: &[u64],
    out_len: usize,
) -> Result<(u32, Vec<u64>), String> {
    // Input buffer layout: [FN_NAME_LENGTH bytes of ASCII name][input u64 array]
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len().min(FN_NAME_LENGTH - 1);
    let mut in_buf = vec![0u8; FN_NAME_LENGTH + input.len() * 8];
    in_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);
    for (i, v) in input.iter().enumerate() {
        let off = FN_NAME_LENGTH + i * 8;
        in_buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }

    let mut out_buf = vec![0u8; out_len * 8];
    let mut returned: u32 = 0;
    let ok = DeviceIoControl(
        handle,
        CTL_EXECUTE,
        in_buf.as_mut_ptr() as *const std::ffi::c_void,
        in_buf.len() as u32,
        out_buf.as_mut_ptr() as *mut std::ffi::c_void,
        out_buf.len() as u32,
        &mut returned,
        std::ptr::null_mut(),
    );

    if ok == 0 {
        return Err(format!(
            "PawnIO DeviceIoControl failed (error {})",
            GetLastError()
        ));
    }

    let count = (returned as usize / 8).min(out_len);
    let mut result = vec![0u64; out_len];
    for i in 0..count {
        let mut b = [0u8; 8];
        b.copy_from_slice(&out_buf[i * 8..i * 8 + 8]);
        result[i] = u64::from_le_bytes(b);
    }

    Ok((0, result))
}
