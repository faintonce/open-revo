//! Internal PawnIO transport shared by Intel temperature and package power.
use super::vault::intel::*;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;

const INTEL_MSR_MODULE: &[u8] = include_bytes!("../../resources/pawnio/IntelMSR.bin");

#[link(name = "kernel32")]
extern "system" {
    fn DeviceIoControl(
        device: isize,
        control_code: u32,
        input: *const std::ffi::c_void,
        input_size: u32,
        output: *mut std::ffi::c_void,
        output_size: u32,
        returned: *mut u32,
        overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn GetCurrentThread() -> *mut std::ffi::c_void;
    fn SetThreadAffinityMask(thread: *mut std::ffi::c_void, mask: usize) -> usize;
}

pub(super) struct ThreadAffinity(usize);

impl ThreadAffinity {
    pub(super) fn pin() -> io::Result<Self> {
        // Both MSRs must be read on the same CPU/package.
        let previous = unsafe { SetThreadAffinityMask(GetCurrentThread(), 1) };
        if previous == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(previous))
    }
}

impl Drop for ThreadAffinity {
    fn drop(&mut self) {
        unsafe {
            SetThreadAffinityMask(GetCurrentThread(), self.0);
        }
    }
}

/// Internal signed-module transport. Only the RAPL policy exposes a write operation.
pub(super) struct IntelMsr {
    device: File,
}

impl IntelMsr {
    /// Connect to an already installed driver. This never installs or starts services.
    pub(super) fn open() -> io::Result<Self> {
        if !is_intel() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Intel CPU required",
            ));
        }
        let sensor = Self {
            device: OpenOptions::new()
                .read(true)
                .write(true)
                .access_mode(3)
                .share_mode(3)
                .open(r"\\?\GLOBALROOT\Device\PawnIO")?,
        };
        sensor.ioctl(IOCTL_LOAD_BINARY, INTEL_MSR_MODULE, &mut [])?;
        Ok(sensor)
    }

    fn ioctl(&self, code: u32, input: &[u8], output: &mut [u8]) -> io::Result<usize> {
        let mut returned = 0;
        let success = unsafe {
            DeviceIoControl(
                self.device.as_raw_handle() as isize,
                code,
                input.as_ptr().cast(),
                input.len() as u32,
                if output.is_empty() {
                    std::ptr::null_mut()
                } else {
                    output.as_mut_ptr().cast()
                },
                output.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(returned as usize)
    }

    pub(super) fn read_msr(&self, index: u64) -> io::Result<u64> {
        // PawnIO ABI: 32-byte, NUL-padded function name followed by 64-bit cells.
        let mut input = [0u8; 40];
        let operation = b"ioctl_read_msr";
        input[..operation.len()].copy_from_slice(operation);
        input[32..].copy_from_slice(&index.to_le_bytes());
        let mut output = [0u8; 8];
        if self.ioctl(IOCTL_EXECUTE, &input, &mut output)? != output.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Incomplete Intel MSR read",
            ));
        }
        Ok(u64::from_le_bytes(output))
    }

    pub(super) fn write_package_limit(&self, value: u64) -> io::Result<()> {
        let mut input = [0u8; 48];
        let operation = b"ioctl_write_msr";
        input[..operation.len()].copy_from_slice(operation);
        input[32..40].copy_from_slice(&MSR_PKG_POWER_LIMIT.to_le_bytes());
        input[40..].copy_from_slice(&value.to_le_bytes());
        if self.ioctl(IOCTL_EXECUTE, &input, &mut [])? != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unexpected MSR write output",
            ));
        }
        Ok(())
    }
}
pub(super) fn is_intel() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        let v = std::arch::x86_64::__cpuid(0);
        return v.ebx == u32::from_le_bytes(*b"Genu")
            && v.edx == u32::from_le_bytes(*b"ineI")
            && v.ecx == u32::from_le_bytes(*b"ntel");
    }
    #[cfg(not(target_arch = "x86_64"))]
    false
}
