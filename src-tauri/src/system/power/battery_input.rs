// 供电输入信息：
//   - 适配器类型与额定功率：EC 供电类型字节 + 适配器额定功率编码
//   - 电池充放功率：Windows 电池设备 IOCTL（BATTERY_QUERY_TAG / BATTERY_QUERY_STATUS）
//
// 只读实现，不写 EC / 不写 ACPI。
use crate::driver::wmi_ec;

const IOCTL_BATTERY_QUERY_TAG: u32 = 0x294040;
const IOCTL_BATTERY_QUERY_STATUS: u32 = 0x29404C;
const IOCTL_BATTERY_QUERY_INFORMATION: u32 = 0x294044;
const BATTERY_INFORMATION_LEVEL: u32 = 0;
const FILE_DEVICE_BATTERY: u32 = 0x29;
const IOCTL_BATTERY_QUERY_TAG_CODE: u32 = (FILE_DEVICE_BATTERY << 16) | (0x10 << 2) | 1;
const IOCTL_BATTERY_QUERY_STATUS_CODE: u32 = (FILE_DEVICE_BATTERY << 16) | (0x13 << 2) | 1;

const GENERIC_READ_WRITE: u32 = 0xC0000000;
const FILE_SHARE_READ_WRITE: u32 = 0x00000003;
const OPEN_EXISTING: u32 = 3;
const DIGCF_PRESENT: u32 = 0x00000002;
const DIGCF_DEVICEINTERFACE: u32 = 0x00000010;
const BATTERY_UNKNOWN_CAPACITY: u32 = 0xFFFFFFFF;
const BATTERY_UNKNOWN_RATE: i32 = i32::MIN;

#[repr(C)]
#[derive(Copy, Clone)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

const GUID_DEVCLASS_BATTERY: Guid = Guid {
    data1: 0x72631E54,
    data2: 0x78A4,
    data3: 0x11D0,
    data4: [0xBC, 0xF7, 0x00, 0xAA, 0x00, 0xB7, 0xB3, 0x2A],
};

#[repr(C)]
struct SpDeviceInterfaceData {
    cb_size: u32,
    interface_class_guid: Guid,
    flags: u32,
    reserved: usize,
}

#[repr(C)]
struct SpDeviceInterfaceDetailData {
    cb_size: u32,
    device_path: [u16; 1],
}

#[repr(C)]
struct BatteryWaitStatus {
    tag: u32,
    timeout: u32,
    power_state: u32,
    low_capacity: u32,
    high_capacity: u32,
}

#[repr(C)]
#[derive(Default)]
struct BatteryStatus {
    power_state: u32,
    capacity: u32,
    voltage: u32,
    rate: i32,
}
#[repr(C)]
struct BatteryQueryInformation {
    tag: u32,
    information_level: u32,
    at_rate: i32,
}

#[repr(C)]
#[derive(Default)]
struct BatteryInformation {
    capabilities: u32,
    technology: u8,
    reserved: [u8; 3],
    // 部分 ACPI 电池驱动会在 8..12 写入化学类型字符串（如 "LION"）
    chemistry: [u8; 4],
    designed_capacity: u32,
    full_charged_capacity: u32,
    default_alert1: u32,
    default_alert2: u32,
    critical_bias: u32,
    cycle_count: u32,
}

#[link(name = "setupapi")]
extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const Guid,
        enumerator: *const u16,
        hwnd_parent: *mut core::ffi::c_void,
        flags: u32,
    ) -> *mut core::ffi::c_void;
    fn SetupDiEnumDeviceInterfaces(
        device_info_set: *mut core::ffi::c_void,
        device_info_data: *const core::ffi::c_void,
        interface_class_guid: *const Guid,
        member_index: u32,
        device_interface_data: *mut SpDeviceInterfaceData,
    ) -> i32;
    fn SetupDiGetDeviceInterfaceDetailW(
        device_info_set: *mut core::ffi::c_void,
        device_interface_data: *const SpDeviceInterfaceData,
        device_interface_detail_data: *mut SpDeviceInterfaceDetailData,
        device_interface_detail_data_size: u32,
        required_size: *mut u32,
        device_info_data: *mut core::ffi::c_void,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(device_info_set: *mut core::ffi::c_void) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    #[link_name = "CreateFileW"]
    fn bat_CreateFileW(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut core::ffi::c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: *mut core::ffi::c_void,
    ) -> isize;
    #[link_name = "DeviceIoControl"]
    fn bat_DeviceIoControl(
        device: isize,
        io_control_code: u32,
        in_buffer: *const core::ffi::c_void,
        in_buffer_size: u32,
        out_buffer: *mut core::ffi::c_void,
        out_buffer_size: u32,
        bytes_returned: *mut u32,
        overlapped: *mut core::ffi::c_void,
    ) -> i32;
    #[link_name = "CloseHandle"]
    fn bat_CloseHandle(handle: isize) -> i32;
}

/// 读适配器类型（"圆口" / "Type-C" / "圆口+Type-C" / "未知"）
pub fn read_adapter_type() -> String {
    let raw = match wmi_ec::wmi_read_ec(wmi_ec::ADDR_POWER_STATUS) {
        Some(v) => v,
        None => return "未知".to_string(),
    };
    let has_round = (raw & wmi_ec::MASK_ADAPTER_ROUND) != 0;
    let has_type_c = (raw & wmi_ec::MASK_ADAPTER_TYPEC) != 0;
    match (has_round, has_type_c) {
        (true, true) => "圆口+Type-C".to_string(),
        (true, false) => "圆口".to_string(),
        (false, true) => "Type-C".to_string(),
        (false, false) => "未知".to_string(),
    }
}

/// 适配器额定功率编码的原始 EC 值（用于显示 "(EC 0xXX)"）
pub fn read_adapter_raw() -> Option<u8> {
    wmi_ec::wmi_read_ec(wmi_ec::ADDR_ADAPTER_RATED_WATTS)
}



/// 适配器额定功率（W）。ADDR_ADAPTER_RATED_WATTS 的 bit3~6 编码，未识别返回 None。
pub fn read_adapter_rated_watts() -> Option<u16> {
    let raw = wmi_ec::wmi_read_ec(wmi_ec::ADDR_ADAPTER_RATED_WATTS)?;
    let code = raw & 0x78;
    let watts = match code {
        0x00 => 330,
        0x08 => 230,
        0x10 => 180,
        0x18 => 150,
        0x20 => 120,
        0x28 => 90,
        0x30 => 65,
        0x38 => 40,
        0x40 => 280,
        0x48 => 210,
        0x50 => 250,
        0x58 => 420,
        _ => return None,
    };
    Some(watts)
}

fn battery_device_paths() -> Vec<Vec<u16>> {
    let mut paths = Vec::new();
    unsafe {
        let mut guid = GUID_DEVCLASS_BATTERY;
        let info_set = SetupDiGetClassDevsW(
            &mut guid,
            core::ptr::null(),
            core::ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if info_set.is_null() || info_set as isize == -1 {
            return paths;
        }
        let mut index = 0u32;
        loop {
            let mut did = SpDeviceInterfaceData {
                cb_size: core::mem::size_of::<SpDeviceInterfaceData>() as u32,
                interface_class_guid: GUID_DEVCLASS_BATTERY,
                flags: 0,
                reserved: 0,
            };
            if SetupDiEnumDeviceInterfaces(
                info_set,
                core::ptr::null(),
                &mut guid,
                index,
                &mut did,
            ) == 0
            {
                break;
            }
            index += 1;

            let mut required: u32 = 0;
            let _ = SetupDiGetDeviceInterfaceDetailW(
                info_set,
                &mut did,
                core::ptr::null_mut(),
                0,
                &mut required,
                core::ptr::null_mut(),
            );
            if required == 0 {
                continue;
            }
            let mut buffer = vec![0u8; required as usize];
            let detail = buffer.as_mut_ptr() as *mut SpDeviceInterfaceDetailData;
            (*detail).cb_size = core::mem::size_of::<SpDeviceInterfaceDetailData>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(
                info_set,
                &mut did,
                detail,
                required,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            ) == 0
            {
                continue;
            }
            let path_ptr = (*detail).device_path.as_ptr();
            let mut len = 0usize;
            while len < 512 && *path_ptr.add(len) != 0 {
                len += 1;
            }
            paths.push(std::slice::from_raw_parts(path_ptr, len + 1).to_vec());
        }
        let _ = SetupDiDestroyDeviceInfoList(info_set);
    }
    paths
}

/// 读电池当前充放功率（W）。只返回充电时的正值；读不到返回 None。
pub fn read_battery_rate_watts() -> Option<f32> {
    for path in battery_device_paths() {
        unsafe {
            let handle = bat_CreateFileW(
                path.as_ptr(),
                GENERIC_READ_WRITE,
                FILE_SHARE_READ_WRITE,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                core::ptr::null_mut(),
            );
            if handle == -1 || handle == 0 {
                continue;
            }

            let mut tag: u32 = 0;
            let mut returned: u32 = 0;
            let wait_ms: u32 = 0;
            let tag_ok = bat_DeviceIoControl(
                handle,
                IOCTL_BATTERY_QUERY_TAG_CODE,
                &wait_ms as *const u32 as *const core::ffi::c_void,
                4,
                &mut tag as *mut u32 as *mut core::ffi::c_void,
                4,
                &mut returned,
                core::ptr::null_mut(),
            ) != 0;

            if tag_ok {
                let wait = BatteryWaitStatus {
                    tag,
                    timeout: 0,
                    power_state: 0,
                    low_capacity: BATTERY_UNKNOWN_CAPACITY,
                    high_capacity: BATTERY_UNKNOWN_CAPACITY,
                };
                let mut status = BatteryStatus::default();
                let status_ok = bat_DeviceIoControl(
                    handle,
                    IOCTL_BATTERY_QUERY_STATUS_CODE,
                    &wait as *const BatteryWaitStatus as *const core::ffi::c_void,
                    core::mem::size_of::<BatteryWaitStatus>() as u32,
                    &mut status as *mut BatteryStatus as *mut core::ffi::c_void,
                    core::mem::size_of::<BatteryStatus>() as u32,
                    &mut returned,
                    core::ptr::null_mut(),
                ) != 0;
                let _ = bat_CloseHandle(handle);
                if status_ok && status.rate != BATTERY_UNKNOWN_RATE {
                    // Rate 单位毫瓦；充电为正、放电为负，保留 1 位小数
                    let watts = status.rate as f32 / 1000.0;
                    if watts > 0.0 {
                        return Some((watts * 10.0).round() / 10.0);
                    }
                    return Some(0.0);
                }
                continue;
            }
            let _ = bat_CloseHandle(handle);
        }
    }
    None
}

/// 读电池循环次数：优先电池 IOCTL，读不到/为 0 时用 powercfg 电池报告兜底（仅一次，缓存）。
pub fn read_battery_cycle_count() -> Option<u32> {
    static CACHE: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        // 与厂商控制台同源：EC ADDR_BATTERY_CYCLE_LOW 低字节 / ADDR_BATTERY_CYCLE_HIGH 高字节
        if let Some(low) = wmi_ec::wmi_read_ec(wmi_ec::ADDR_BATTERY_CYCLE_LOW) {
            let high = wmi_ec::wmi_read_ec(wmi_ec::ADDR_BATTERY_CYCLE_HIGH).unwrap_or(0);
            let value = ((high as u32) << 8) | (low as u32);
            if value > 0 && value < 100_000 {
                return Some(value);
            }
        }
        if let Some(c) = ioctl_battery_cycle_count() {
            if c > 0 {
                return Some(c);
            }
        }
        powercfg_battery_cycle_count()
    })
}

fn powercfg_battery_cycle_count() -> Option<u32> {
    let tmp = std::env::temp_dir().join("openrevo-battery-report.xml");
    let _ = std::fs::remove_file(&tmp);
    let mut cmd = std::process::Command::new("powercfg");
    cmd.args([
        "/batteryreport",
        "/xml",
        "/output",
        &tmp.to_string_lossy(),
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let status = cmd.status().ok();
    let bytes = match std::fs::read(&tmp) {
        Ok(b) => b,
        Err(_) => {
            crate::log_warn!(
                "POWER_INPUT",
                "powercfg batteryreport failed: status={:?}, file missing",
                status
            );
            return None;
        }
    };
    let _ = std::fs::remove_file(&tmp);

    // powercfg 的 XML 可能是 UTF-8(BOM) 或 UTF-16LE，这里统一转成可搜索文本
    let text = if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };

    let mut value = None;
    if let Some(pos) = text.find("CycleCount") {
        let tail = &text[pos + "CycleCount".len()..];
        if let Some(gt) = tail.find('>') {
            let num: String = tail[gt + 1..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            value = num.parse::<u32>().ok();
        }
    }
    crate::log_info!(
        "POWER_INPUT",
        "powercfg batteryreport: status={:?}, parsed cycle_count={:?}",
        status,
        value
    );
    value.filter(|v| *v > 0)
}

/// 读电池循环次数（Windows 电池信息 IOCTL）。
fn ioctl_battery_cycle_count() -> Option<u32> {
    for path in battery_device_paths() {
        unsafe {
            let handle = bat_CreateFileW(
                path.as_ptr(),
                GENERIC_READ_WRITE,
                FILE_SHARE_READ_WRITE,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                core::ptr::null_mut(),
            );
            if handle == -1 || handle == 0 {
                continue;
            }
            let mut tag: u32 = 0;
            let mut returned: u32 = 0;
            let wait_ms: u32 = 0;
            let tag_ok = bat_DeviceIoControl(
                handle,
                IOCTL_BATTERY_QUERY_TAG_CODE,
                &wait_ms as *const u32 as *const core::ffi::c_void,
                4,
                &mut tag as *mut u32 as *mut core::ffi::c_void,
                4,
                &mut returned,
                core::ptr::null_mut(),
            ) != 0;
            if tag_ok {
                let query = BatteryQueryInformation {
                    tag,
                    information_level: BATTERY_INFORMATION_LEVEL,
                    at_rate: 0,
                };
                let mut info = BatteryInformation::default();
                let ok = bat_DeviceIoControl(
                    handle,
                    IOCTL_BATTERY_QUERY_INFORMATION,
                    &query as *const BatteryQueryInformation as *const core::ffi::c_void,
                    core::mem::size_of::<BatteryQueryInformation>() as u32,
                    &mut info as *mut BatteryInformation as *mut core::ffi::c_void,
                    core::mem::size_of::<BatteryInformation>() as u32,
                    &mut returned,
                    core::ptr::null_mut(),
                ) != 0;
                let _ = bat_CloseHandle(handle);
                if ok && info.cycle_count < 100_000 {
                    return Some(info.cycle_count);
                }
                continue;
            }
            let _ = bat_CloseHandle(handle);
        }
    }
    None
}

/// 汇总供电输入信息
#[derive(Clone, Debug, Default)]
pub struct PowerInputInfo {
    pub adapter_type: String,
    pub adapter_raw: Option<u8>,
    pub adapter_watts: Option<u16>,
    pub battery_rate_watts: Option<f32>,
    pub battery_cycle_count: Option<u32>,
}

pub fn read_power_input() -> PowerInputInfo {
    // 只有供电类型字节能明确识别出圆口 / Type-C 时，才信任额定功率与电池充电功率；
    // 宝龙达模具不提供 ADDR_POWER_STATUS/ADDR_ADAPTER_RATED_WATTS，读到的 0x00 不能当成 330W 适配器，
    // 此时全部返回未知，由界面回落到原「已连接电源适配器」文案。
    let adapter_type = read_adapter_type();
    let adapter_known = adapter_type != "未知";
    let barrel = adapter_type == "圆口" || adapter_type == "圆口+Type-C";
    // 适配器瓦数以 EC ADDR_ADAPTER_RATED_WATTS 为准（适配器索引 只是同一索引的另一种表示）；
    // ADDR_BATTERY_LIMIT_UP 在多数同方机型上是充电上限，不能当适配器瓦数用。
    // Type-C 暂时不显示瓦数，只显示类型（等实际机型验证完成后再放开）。
    let (adapter_raw, adapter_watts) = if adapter_known && barrel {
        (read_adapter_raw(), read_adapter_rated_watts())
    } else {
        (None, None)
    };
    let battery_rate_watts = if adapter_known { read_battery_rate_watts() } else { None };
    let battery_cycle_count = read_battery_cycle_count();
    PowerInputInfo {
        adapter_raw,
        adapter_type,
        adapter_watts,
        battery_rate_watts,
        battery_cycle_count,
    }
}
