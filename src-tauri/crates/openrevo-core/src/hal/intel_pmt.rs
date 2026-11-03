//! Read the GPU thermal sensor from the installed Intel PMT driver.
//! No physical-memory mapping or write IOCTLs. Only the verified PTL XML GUID
//! is accepted; other platform layouts must be added with their own evidence.
//!
//! https://github.com/intel/Intel-PMT/blob/main/xml/PTL/0/ptl_aggregator.xml
//! https://github.com/intel/compute-runtime/blob/master/level_zero/sysman/source/shared/windows/pmt/sysman_pmt_helper.h

use super::vault::intel_pmt::*;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
struct Region {
    index: u32,
}

fn decode_discovery(data: &[u8]) -> Result<Option<Region>, String> {
    fn word(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
    }
    if data.len() < DISCOVERY_HEADER_BYTES || word(data, 0) != 0 {
        return Err("Invalid PMT discovery header".into());
    }
    let count = word(data, 4) as usize;
    if count > (data.len() - DISCOVERY_HEADER_BYTES) / DISCOVERY_ENTRY_BYTES {
        return Err("Truncated PMT discovery entries".into());
    }
    for entry in data[DISCOVERY_HEADER_BYTES..]
        .chunks_exact(DISCOVERY_ENTRY_BYTES)
        .take(count)
    {
        let version = word(entry, 0);
        let index = word(entry, 4);
        let guid = word(entry, 8);
        let dword_count = word(entry, 12);
        if version == 0 && guid == PTL_NORMAL_GUID && dword_count >= (PTL_TEMPERATURE_QWORD + 1) * 2
        {
            return Ok(Some(Region { index }));
        }
    }
    Ok(None)
}

fn decode_gpu_temperature(data: u64) -> Option<u32> {
    // PTL Container_15, GT_MAX bits 56..63, signed S8.7.0 Celsius.
    // Zero and negative/disabled sensor encodings are unavailable.
    let value = (data >> PTL_GT_MAX_SHIFT) as u8 as i8;
    (value > 0).then_some(value as u32)
}

#[derive(Default)]
pub(super) struct GpuTemperature {
    source: Option<native::Source>,
    failed_at: Option<Instant>,
}

impl GpuTemperature {
    pub(super) fn read(&mut self) -> Option<u32> {
        if self
            .failed_at
            .is_some_and(|at| at.elapsed() < Duration::from_secs(30))
        {
            return None;
        }
        let result = (|| {
            if self.source.is_none() {
                self.source = Some(native::Source::connect()?);
            }
            self.source.as_ref().unwrap().read()
        })();
        match result {
            Ok(value) => {
                self.failed_at = None;
                value
            }
            Err(_) => {
                self.source = None;
                self.failed_at = Some(Instant::now());
                None
            }
        }
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows::core::{GUID, PCWSTR};
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Device_Interface_ListW, CM_Get_Device_Interface_List_SizeW,
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CR_BUFFER_SMALL, CR_SUCCESS,
    };

    #[link(name = "kernel32")]
    extern "system" {
        fn DeviceIoControl(
            device: isize,
            control: u32,
            input: *const std::ffi::c_void,
            input_size: u32,
            output: *mut std::ffi::c_void,
            output_size: u32,
            returned: *mut u32,
            overlapped: *mut std::ffi::c_void,
        ) -> i32;
    }
    fn query(
        device: &File,
        control: u32,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, String> {
        let mut returned = 0;
        let success = unsafe {
            DeviceIoControl(
                device.as_raw_handle() as isize,
                control,
                if input.is_empty() {
                    std::ptr::null()
                } else {
                    input.as_ptr().cast()
                },
                input.len() as u32,
                output.as_mut_ptr().cast(),
                output.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if success == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if returned as usize > output.len() {
            return Err("Invalid PMT reply size".into());
        }
        Ok(returned as usize)
    }
    fn open(path: &str) -> Result<File, String> {
        // GENERIC_READ only; driver handles are closed after each short query.
        OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .map_err(|e| e.to_string())
    }
    fn interfaces() -> Result<Vec<String>, String> {
        let guid = GUID::from_u128(INTERFACE_GUID);
        for _ in 0..3 {
            let mut size = 0;
            if unsafe {
                CM_Get_Device_Interface_List_SizeW(
                    &mut size,
                    &guid,
                    PCWSTR::null(),
                    CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
                )
            } != CR_SUCCESS
                || size > 65536
            {
                return Err("Cannot enumerate Intel PMT interfaces".into());
            }
            if size == 0 {
                return Ok(Vec::new());
            }
            let mut buffer = vec![0u16; size as usize];
            let status = unsafe {
                CM_Get_Device_Interface_ListW(
                    &guid,
                    PCWSTR::null(),
                    &mut buffer,
                    CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
                )
            };
            if status == CR_BUFFER_SMALL {
                continue;
            }
            if status != CR_SUCCESS {
                return Err("Cannot read Intel PMT interface list".into());
            }
            return Ok(buffer
                .split(|v| *v == 0)
                .filter(|v| !v.is_empty())
                .map(String::from_utf16_lossy)
                .collect());
        }
        Err("Intel PMT interface list changed repeatedly".into())
    }
    pub(super) struct Source {
        path: String,
        region: Region,
    }
    fn discover(device: &File) -> Result<Option<Region>, String> {
        let mut size = [0u8; 4];
        if query(device, DISCOVERY_SIZE, &[], &mut size)? != size.len() {
            return Err("Truncated PMT discovery size".into());
        }
        let size = u32::from_le_bytes(size) as usize;
        if !(DISCOVERY_HEADER_BYTES..=65536).contains(&size) {
            return Err("Invalid PMT discovery size".into());
        }
        let mut discovery = vec![0; size];
        let returned = query(device, DISCOVERY, &[], &mut discovery)?;
        decode_discovery(&discovery[..returned])
    }
    impl Source {
        pub(super) fn connect() -> Result<Self, String> {
            for path in interfaces()? {
                let Ok(device) = open(&path) else {
                    continue;
                };
                if let Ok(Some(region)) = discover(&device) {
                    return Ok(Self { path, region });
                }
            }
            Err("Verified Panther Lake normal PMT telemetry is unavailable".into())
        }
        pub(super) fn read(&self) -> Result<Option<u32>, String> {
            let device = open(&self.path)?;
            // Recheck discovery on this handle, including after resume or a
            // driver replacement; never reuse an offset with a changed layout.
            if discover(&device)?.as_ref() != Some(&self.region) {
                return Err("Intel PMT telemetry layout changed".into());
            }
            let request: Vec<u8> = [0, self.region.index, PTL_TEMPERATURE_QWORD, 1]
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect();
            let mut data = [0u8; 8];
            if query(&device, READ_QWORD, &request, &mut data)? != data.len() {
                return Err("Truncated PMT GPU temperature".into());
            }
            Ok(decode_gpu_temperature(u64::from_le_bytes(data)))
        }
    }
}

#[cfg(not(windows))]
mod native {
    pub(super) struct Source;
    impl Source {
        pub(super) fn connect() -> Result<Self, String> {
            Err("Intel PMT requires Windows".into())
        }
        pub(super) fn read(&self) -> Result<Option<u32>, String> {
            Err("Intel PMT requires Windows".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn discovery(guid: u32, words: u32) -> Vec<u8> {
        [0, 2, 0, 0, 0x03086100, 186, 0, 1, guid, words]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect()
    }
    #[test]
    fn matches_xml_guid_and_region_bounds_instead_of_first_interface() {
        assert_eq!(
            decode_discovery(&discovery(PTL_NORMAL_GUID, 838)),
            Ok(Some(Region { index: 1 }))
        );
        assert_eq!(decode_discovery(&discovery(0x03086001, 838)), Ok(None));
        assert_eq!(decode_discovery(&discovery(PTL_NORMAL_GUID, 31)), Ok(None));
        assert_eq!(
            decode_discovery(&discovery(PTL_NORMAL_GUID, 32)),
            Ok(Some(Region { index: 1 }))
        );
    }
    #[test]
    fn rejects_truncated_and_versioned_discovery_without_reading_offsets() {
        for length in 0..40 {
            assert!(decode_discovery(&discovery(PTL_NORMAL_GUID, 838)[..length]).is_err());
        }
        let mut data = discovery(PTL_NORMAL_GUID, 838);
        data[0] = 1;
        assert!(decode_discovery(&data).is_err());
        data[0] = 0;
        data[24] = 1;
        assert_eq!(decode_discovery(&data), Ok(None));
    }
    #[test]
    fn reads_gt_max_not_cpu_pch_or_graphics_min_and_rejects_signed_sentinels() {
        assert_eq!(decode_gpu_temperature(0x30002c2c2d303117), Some(48));
        assert_eq!(decode_gpu_temperature(0x32002c2c2d303117), Some(50));
        for temperature in [0u64, 128, 255] {
            assert_eq!(
                decode_gpu_temperature(temperature << PTL_GT_MAX_SHIFT),
                None
            );
        }
    }
}
