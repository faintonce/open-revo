use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::GUID;
use windows::Devices::Bluetooth::Advertisement::{
    BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
    BluetoothLEScanningMode,
};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus, GattDeviceService, GattSharingMode, GattValueChangedEventArgs,
    GattWriteOption, GattSession,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothConnectionStatus, BluetoothLEDevice};
use windows::Storage::Streams::DataWriter;
use crate::driver::wmi_ec::ADDR_WATER_COOLER_FLAG;

const NUS_SERVICE_UUID: GUID = GUID::from_u128(0x6e400001_b5a3_f393_e0a9_e50e24dcca9e);
const NUS_TX_WRITE_UUID: GUID = GUID::from_u128(0x6e400002_b5a3_f393_e0a9_e50e24dcca9e);
const NUS_RX_NOTIFY_UUID: GUID = GUID::from_u128(0x6e400003_b5a3_f393_e0a9_e50e24dcca9e);

// Default Water Cooler MAC address (0 = auto-scan on first connection)
pub const DEFAULT_WATER_COOLER_MAC: u64 = 0;

pub fn mac_str_to_u64(mac_str: &str) -> Option<u64> {
    let clean = mac_str.replace(':', "").replace('-', "").trim().to_string();
    u64::from_str_radix(&clean, 16).ok()
}

pub fn mac_u64_to_str(mac: u64) -> String {
    let b = mac.to_be_bytes();
    format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", b[2], b[3], b[4], b[5], b[6], b[7])
}

/// Discover all Mechrevo Water Cooler devices registered in Windows paired PnP registry.
pub fn find_all_paired_water_coolers() -> Vec<(u64, String)> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    let mut list = Vec::new();
    let bthle_path: Vec<u16> = "SYSTEM\\CurrentControlSet\\Enum\\BTHLE\0".encode_utf16().collect();
    let mut root_key = HKEY::default();
    unsafe {
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(bthle_path.as_ptr()), 0, KEY_READ, &mut root_key).is_err() {
            return list;
        }

        let mut index = 0u32;
        let mut key_name_buf = [0u16; 256];
        let mut key_name_len = key_name_buf.len() as u32;

        while RegEnumKeyExW(
            root_key,
            index,
            windows::core::PWSTR(key_name_buf.as_mut_ptr()),
            &mut key_name_len,
            None,
            windows::core::PWSTR::null(),
            None,
            None,
        ).is_ok() {
            index += 1;
            let len = (key_name_len as usize).min(key_name_buf.len());
            let dev_key_str = String::from_utf16_lossy(&key_name_buf[..len]);
            key_name_len = key_name_buf.len() as u32;

            // Subkey looks like "Dev_cacbc3029718"
            if dev_key_str.to_lowercase().starts_with("dev_") {
                let mac_part = &dev_key_str[4..];
                if let Some(parsed_mac) = mac_str_to_u64(mac_part) {
                    let wide_dev_path: Vec<u16> = format!("SYSTEM\\CurrentControlSet\\Enum\\BTHLE\\{}\0", dev_key_str)
                        .encode_utf16()
                        .collect();
                    let mut dev_hkey = HKEY::default();
                    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_dev_path.as_ptr()), 0, KEY_READ, &mut dev_hkey).is_ok() {
                        let mut sub_index = 0u32;
                        let mut sub_buf = [0u16; 256];
                        let mut sub_len_inout = sub_buf.len() as u32;
                        while RegEnumKeyExW(
                            dev_hkey,
                            sub_index,
                            windows::core::PWSTR(sub_buf.as_mut_ptr()),
                            &mut sub_len_inout,
                            None,
                            windows::core::PWSTR::null(),
                            None,
                            None,
                        ).is_ok() {
                            sub_index += 1;
                            let sub_len = (sub_len_inout as usize).min(sub_buf.len());
                            let inst_str = String::from_utf16_lossy(&sub_buf[..sub_len]);
                            sub_len_inout = sub_buf.len() as u32;

                            let wide_inst_path: Vec<u16> = format!("SYSTEM\\CurrentControlSet\\Enum\\BTHLE\\{}\\{}\0", dev_key_str, inst_str)
                                .encode_utf16()
                                .collect();
                            let mut inst_hkey = HKEY::default();
                            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(wide_inst_path.as_ptr()), 0, KEY_READ, &mut inst_hkey).is_ok() {
                                let mut val_buf = [0u16; 256];
                                let mut val_len = (val_buf.len() * 2) as u32;
                                let mut val_type = REG_SZ;
                                let fn_prop: Vec<u16> = "FriendlyName\0".encode_utf16().collect();
                                if RegQueryValueExW(
                                    inst_hkey,
                                    PCWSTR(fn_prop.as_ptr()),
                                    None,
                                    Some(&mut val_type),
                                    Some(val_buf.as_mut_ptr() as *mut _),
                                    Some(&mut val_len),
                                ).is_ok() {
                                    let str_len = (val_len as usize / 2).saturating_sub(1);
                                    let raw_name = String::from_utf16_lossy(&val_buf[..str_len]);
                                    let upper = raw_name.to_uppercase();
                                    if upper.contains("COOLING") || upper.contains("LCT") || upper.contains("CS_") {
                                        if !list.iter().any(|(m, _)| *m == parsed_mac) {
                                            crate::log_info!("BLE_COOLER", "Found paired water cooler from Windows PnP: '{}' -> {:012X}", raw_name, parsed_mac);
                                            list.push((parsed_mac, raw_name));
                                        }
                                    }
                                }
                                let _ = RegCloseKey(inst_hkey);
                            }
                        }
                        let _ = RegCloseKey(dev_hkey);
                    }
                }
            }
        }
        let _ = RegCloseKey(root_key);
    }
    list
}

/// Discovers Mechrevo Water Cooler MAC address directly from Windows paired PnP device registry (0ms, 100% reliable).
pub fn find_paired_water_cooler_mac() -> Option<u64> {
    find_all_paired_water_coolers().first().map(|(mac, _)| *mac)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct WaterCoolerDevice {
    pub mac_address: String,
    pub mac_u64: u64,
    pub name: String,
    pub rssi: Option<i16>,
    pub is_paired: bool,
    pub is_current: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CoolerPumpTier {
    /// 0: 停泵 (0V / 0%)
    Stop = 0,
    /// 1: 超静音微巡航 (预留档位, 待未来捕获更低巡航注入)
    Silent = 1,
    /// 2: 标准巡航 (7V @ 45%, 实测稳定 U 模式)
    Standard = 2,
    /// 3: 狂暴推水 (12V @ 100%, 极限高压压温)
    Turbo = 3,
}

impl CoolerPumpTier {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Stop,
            1 => Self::Silent,
            2 => Self::Standard,
            3 => Self::Turbo,
            // 兼容旧配置/旧前端传值 (7, 8, 11/12)
            7 | 8 => Self::Standard,
            11 | 12 => Self::Turbo,
            _ => Self::Standard,
        }
    }

    /// 返回硬件物理帧三元组: (OnOff, Volt, Duty)
    pub fn to_hardware_params(self) -> (u8, u8, u8) {
        match self {
            Self::Stop => (0x00, 0, 0),
            Self::Silent => (0x01, 7, 45), // 预留静音档安全回退为 7V @ 45%
            Self::Standard => (0x01, 7, 45), // 7V @ 45% (实测稳定标准巡航，切回时微顿平稳运转)
            Self::Turbo => (0x01, 2, 0), // 触发单片机防失控应急 100% 狂暴极速啸叫 (Byte 4 = 0)
        }
    }
}

pub struct WaterCoolerState {
    pub connected: AtomicBool,
    pub is_flow_ok: AtomicBool,
    pub meter_state: AtomicU8,  // Raw meter state from device: 0=No Flow, 1=Normal, 2=Priming/Trickle
    pub pump_voltage: AtomicU8, // Retained for backwards compatibility (0, 7, 8, 11/12)
    pub pump_tier: AtomicU8,    // 0=Stop, 1=Silent, 2=Standard, 3=Turbo
    pub fan_duty: AtomicU8,     // 0 - 100% (default 75)
    pub pump_rpm: AtomicU32,    // Calculated/estimated RPM
    pub running: AtomicBool,
    // Strategy & Curve
    pub strategy: Arc<parking_lot::RwLock<String>>,
    pub curve_points: Arc<parking_lot::RwLock<Vec<crate::system::config::CoolerCurvePoint>>>,
    pub last_applied_temp: AtomicU8,
    pub pump_start_temp: AtomicU8, // 启泵阈值（默认 55°C）
    pub pump_turbo_temp: AtomicU8, // 狂暴阈值（默认 90°C）
    // Safety protection & Priming
    pub safety_tripped: AtomicBool,
    pub is_priming: AtomicBool,
    // RGB LED State (Mode 2: Colorful Flow by default, Mode 3: Breathe Color, Mode 1: Breathe)
    pub led_on: AtomicBool,
    pub led_r: AtomicU8,
    pub led_g: AtomicU8,
    pub led_b: AtomicU8,
    pub led_mode: AtomicU8,     // 2: Colorful (Default), 3: Breathe Color, 1: Single Color Breathe
    pub led_dirty: AtomicBool,  // Immediate send flag
    pub params_dirty: AtomicBool, // On-change trigger for pump & fan
    pub is_suspended: AtomicBool, // System in sleep / hibernate / standby / lid closed
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CoolerTelemetry {
    pub connected: bool,
    pub is_flow_ok: bool,
    pub meter_state: u8, // 0=No Flow, 1=Normal, 2=Priming/Trickle
    pub pump_voltage: u8,
    pub pump_tier: u8,   // 0=Stop, 1=Silent, 2=Standard, 3=Turbo
    pub fan_duty: u8,
    pub pump_rpm: u32,
    pub running: bool,
    pub safety_tripped: bool,
    pub is_priming: bool,
    pub led_on: bool,
    pub led_r: u8,
    pub led_g: u8,
    pub led_b: u8,
    pub led_mode: u8,
    pub mac_address: String,
    pub strategy: String,
    pub curve_points: Vec<crate::system::config::CoolerCurvePoint>,
    pub pump_start_temp: u8,
    pub pump_turbo_temp: u8,
}

pub struct BleCoolerController {
    state: Arc<WaterCoolerState>,
    mac_address: Arc<AtomicU64>,
    active_char: Arc<parking_lot::Mutex<Option<GattCharacteristic>>>,
    supervisor_started: AtomicBool,
    wakeup_notify: Arc<(parking_lot::Mutex<bool>, parking_lot::Condvar)>,
}

static BLE_TX_MUTEX: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
static LAST_USER_CMD_MS: AtomicU64 = AtomicU64::new(0);

fn mark_user_cmd_sent() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    LAST_USER_CMD_MS.store(now, Ordering::SeqCst);
}

fn write_frame(c: &GattCharacteristic, bytes: &[u8]) -> Result<(), String> {
    let _guard = BLE_TX_MUTEX.lock();
    let hex = bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
    crate::log_debug!("BLE_TX", "-> TX [{}]", hex);

    let writer = DataWriter::new().map_err(|e| e.to_string())?;
    writer.WriteBytes(bytes).map_err(|e| e.to_string())?;
    let buf = writer.DetachBuffer().map_err(|e| e.to_string())?;
    let op = c.WriteValueWithOptionAsync(&buf, GattWriteOption::WriteWithoutResponse)
        .map_err(|e| e.to_string())?;

    let op_clone = op.clone();
    let (tx, rx) = std::sync::mpsc::channel::<Result<GattCommunicationStatus, String>>();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            op_clone.get().map_err(|e| e.to_string())
        }))
        .unwrap_or_else(|_| Err("Panic caught during WinRT op.get()".to_string()));
        let _ = tx.send(result);
    });

    let status = match rx.recv_timeout(Duration::from_millis(1500)) {
        Ok(result) => result?,
        Err(_timeout) => {
            crate::log_error!("BLE_TX", "<- TIMEOUT (1500ms) for [{}]", hex);
            return Err("BLE write timeout (1500ms)".to_string());
        }
    };

    if status != GattCommunicationStatus::Success {
        crate::log_error!("BLE_TX", "<- FAILED {:?} for [{}]", status, hex);
        return Err(format!("Gatt write failed: {:?}", status));
    }
    crate::log_debug!("BLE_TX", "<- OK [{}]", hex);
    // 30ms pacing delay matching official PowerShell script
    std::thread::sleep(Duration::from_millis(30));
    Ok(())
}


// Global singleton controller instance for access across Tauri commands
static GLOBAL_COOLER: std::sync::OnceLock<Arc<BleCoolerController>> = std::sync::OnceLock::new();

pub fn get_global_cooler() -> Arc<BleCoolerController> {
    GLOBAL_COOLER.get_or_init(|| {
        let cfg = crate::system::config::load_config();
        let initial_mac = mac_str_to_u64(&cfg.cooler_mac)
            .filter(|&m| m != 0)
            .or_else(|| find_paired_water_cooler_mac())
            .unwrap_or(DEFAULT_WATER_COOLER_MAC);
        let cooler = BleCoolerController::new(initial_mac);
        *cooler.state.strategy.write() = cfg.cooler_strategy.clone();
        *cooler.state.curve_points.write() = cfg.cooler_curve_points.clone();

        let start_t = if cfg.cooler_pump_start_temp == 0 { 55 } else { cfg.cooler_pump_start_temp };
        let turbo_t = if cfg.cooler_pump_turbo_temp == 0 { 90 } else { cfg.cooler_pump_turbo_temp };
        cooler.state.pump_start_temp.store(start_t, Ordering::SeqCst);
        cooler.state.pump_turbo_temp.store(turbo_t, Ordering::SeqCst);

        // Calculate and immediately apply target parameters for the active strategy upon startup
        let (pump, fan) = calculate_strategy_params(
            &cfg.cooler_strategy,
            &cfg.cooler_curve_points,
            cfg.cooler_pump_voltage,
            cfg.cooler_fan_duty,
            start_t,
            turbo_t,
        );
        cooler.state.pump_voltage.store(pump, Ordering::SeqCst);
        cooler.state.pump_tier.store(CoolerPumpTier::from_u8(pump) as u8, Ordering::SeqCst);
        cooler.state.fan_duty.store(fan, Ordering::SeqCst);
        if cfg.cooler_strategy == "smart" {
            cooler.state.last_applied_temp.store(get_current_max_temp(), Ordering::SeqCst);
        }
        cooler.state.params_dirty.store(true, Ordering::SeqCst);

        cooler.state.led_on.store(cfg.cooler_led_on, Ordering::SeqCst);
        cooler.state.led_r.store(cfg.cooler_led_r, Ordering::SeqCst);
        cooler.state.led_g.store(cfg.cooler_led_g, Ordering::SeqCst);
        cooler.state.led_b.store(cfg.cooler_led_b, Ordering::SeqCst);
        // Default to Mode 2 (Colorful Flow) if mode is invalid or 0
        let mode = if cfg.cooler_led_mode == 0 { 2 } else { cfg.cooler_led_mode };
        cooler.state.led_mode.store(mode, Ordering::SeqCst);
        Arc::new(cooler)
    }).clone()
}

pub fn scan_for_water_cooler(timeout: Duration) -> Result<u64, String> {
    crate::log_info!("BLE_COOLER", "Starting BLE Advertisement Watcher to discover Mechrevo Water Cooler...");
    let watcher = BluetoothLEAdvertisementWatcher::new().map_err(|e| e.to_string())?;
    let _ = watcher.SetScanningMode(BluetoothLEScanningMode::Active);

    let found_mac = Arc::new(parking_lot::Mutex::new(None::<u64>));
    let found_clone = Arc::clone(&found_mac);

    let token = watcher.Received(&windows::Foundation::TypedEventHandler::new(
        move |_sender: &Option<BluetoothLEAdvertisementWatcher>, args: &Option<BluetoothLEAdvertisementReceivedEventArgs>| {
            if let Some(args) = args {
                let address = args.BluetoothAddress().unwrap_or(0);
                if address == 0 {
                    return Ok(());
                }

                let mut is_match = false;
                if let Ok(ad) = args.Advertisement() {
                    // Check LocalName strictly: e.g. "CoolingSystem", "LCT", "CS_"
                    // NOTE: We strictly verify device name to avoid hijacking arbitrary third-party devices
                    // that also use the standard Nordic UART Service (NUS) UUID.
                    if let Ok(name) = ad.LocalName() {
                        let name_str = name.to_string();
                        if !name_str.is_empty() {
                            let upper = name_str.to_uppercase();
                            if upper.contains("COOLING") || upper.contains("LCT") || upper.contains("CS_") {
                                crate::log_info!("BLE_COOLER", "Discovered Mechrevo cooler by name: '{}' (MAC: {:012X})", name_str, address);
                                is_match = true;
                            }
                        }
                    }
                }

                if is_match {
                    let mut lock = found_clone.lock();
                    if lock.is_none() {
                        *lock = Some(address);
                    }
                }
            }
            Ok(())
        },
    )).map_err(|e| e.to_string())?;

    let _ = watcher.Start();

    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(mac) = *found_mac.lock() {
            let _ = watcher.Stop();
            let _ = watcher.RemoveReceived(token);
            return Ok(mac);
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    let _ = watcher.Stop();
    let _ = watcher.RemoveReceived(token);
    Err("No Mechrevo Water Cooler found within timeout".into())
}

/// Scan for all nearby Mechrevo water coolers and merge with Windows PnP paired devices.
pub fn scan_all_water_coolers(timeout: Duration) -> Result<Vec<WaterCoolerDevice>, String> {
    let watcher = BluetoothLEAdvertisementWatcher::new().map_err(|e| e.to_string())?;
    let _ = watcher.SetScanningMode(BluetoothLEScanningMode::Active);

    let discovered = Arc::new(parking_lot::Mutex::new(std::collections::HashMap::<u64, (String, Option<i16>)>::new()));
    let discovered_clone = Arc::clone(&discovered);

    let token = watcher.Received(&windows::Foundation::TypedEventHandler::new(
        move |_sender: &Option<BluetoothLEAdvertisementWatcher>, args: &Option<BluetoothLEAdvertisementReceivedEventArgs>| {
            if let Some(args) = args {
                let address = args.BluetoothAddress().unwrap_or(0);
                if address == 0 {
                    return Ok(());
                }

                let mut is_match = false;
                let mut dev_name = String::new();
                if let Ok(ad) = args.Advertisement() {
                    if let Ok(name) = ad.LocalName() {
                        let name_str = name.to_string();
                        if !name_str.is_empty() {
                            let upper = name_str.to_uppercase();
                            if upper.contains("COOLING") || upper.contains("LCT") || upper.contains("CS_") {
                                dev_name = name_str;
                                is_match = true;
                            }
                        }
                    }
                }

                if is_match {
                    let rssi = args.RawSignalStrengthInDBm().ok();
                    let mut map = discovered_clone.lock();
                    map.insert(address, (dev_name, rssi));
                }
            }
            Ok(())
        },
    )).map_err(|e| e.to_string())?;

    let _ = watcher.Start();
    std::thread::sleep(timeout);
    let _ = watcher.Stop();
    let _ = watcher.RemoveReceived(token);

    let current_mac = get_global_cooler().get_mac();
    let paired_list = find_all_paired_water_coolers();
    let map = discovered.lock();

    let mut result: Vec<WaterCoolerDevice> = Vec::new();
    let mut seen_macs = std::collections::HashSet::new();

    for (&mac, (name, rssi)) in map.iter() {
        seen_macs.insert(mac);
        let is_paired = paired_list.iter().any(|(p_mac, _)| *p_mac == mac);
        let display_name = if name.trim().is_empty() {
            paired_list.iter().find(|(p_mac, _)| *p_mac == mac)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| format!("水冷魔盒 ({:04X})", mac & 0xFFFF))
        } else {
            name.clone()
        };
        result.push(WaterCoolerDevice {
            mac_address: mac_u64_to_str(mac),
            mac_u64: mac,
            name: display_name,
            rssi: *rssi,
            is_paired,
            is_current: mac == current_mac && current_mac != 0,
        });
    }

    for (p_mac, p_name) in paired_list {
        if !seen_macs.contains(&p_mac) {
            seen_macs.insert(p_mac);
            result.push(WaterCoolerDevice {
                mac_address: mac_u64_to_str(p_mac),
                mac_u64: p_mac,
                name: p_name,
                rssi: None,
                is_paired: true,
                is_current: p_mac == current_mac && current_mac != 0,
            });
        }
    }

    if current_mac != 0 && !seen_macs.contains(&current_mac) {
        result.push(WaterCoolerDevice {
            mac_address: mac_u64_to_str(current_mac),
            mac_u64: current_mac,
            name: "已绑定水冷魔盒 (离线)".to_string(),
            rssi: None,
            is_paired: false,
            is_current: true,
        });
    }

    result.sort_by(|a, b| {
        b.is_current.cmp(&a.is_current)
            .then_with(|| b.rssi.unwrap_or(-120).cmp(&a.rssi.unwrap_or(-120)))
            .then_with(|| b.is_paired.cmp(&a.is_paired))
    });

    Ok(result)
}

/// 计算水泵档位（带 5°C 迟滞回差，防启停震荡）
pub fn calculate_pump_tier_with_hysteresis(
    current_temp: u8,
    last_tier: CoolerPumpTier,
    start_temp: u8,
    turbo_temp: u8,
) -> CoolerPumpTier {
    let stop_threshold = start_temp.saturating_sub(5);
    let turbo_fallback = turbo_temp.saturating_sub(5);

    match last_tier {
        CoolerPumpTier::Turbo => {
            // 降温回差：必须降至 <= turbo_fallback 才回落到 Standard
            if current_temp <= turbo_fallback {
                if current_temp < stop_threshold {
                    CoolerPumpTier::Stop
                } else {
                    CoolerPumpTier::Standard
                }
            } else {
                CoolerPumpTier::Turbo
            }
        }
        CoolerPumpTier::Standard | CoolerPumpTier::Silent => {
            // 升温：>= turbo_temp 直冲 Turbo
            if current_temp >= turbo_temp {
                CoolerPumpTier::Turbo
            } else if current_temp <= stop_threshold { // 降温回差：降至 <= stop_threshold 才停泵
                CoolerPumpTier::Stop
            } else {
                CoolerPumpTier::Standard
            }
        }
        CoolerPumpTier::Stop => {
            // 升温：>= turbo_temp 直冲 Turbo；>= start_temp 启动 Standard
            if current_temp >= turbo_temp {
                CoolerPumpTier::Turbo
            } else if current_temp >= start_temp {
                CoolerPumpTier::Standard
            } else {
                CoolerPumpTier::Stop
            }
        }
    }
}

/// 连续平滑线性风扇曲线插值（融合实测声学舒适区：<=60% 静音舒适，>=70% 性能风噪）
pub fn evaluate_cooler_fan_duty_smooth(temp: u8) -> u8 {
    if temp < 50 {
        // < 50°C: 配合停泵实现 0 dBA 绝对静音
        0
    } else if temp <= 65 {
        // 50°C -> 30%, 65°C -> 50% (微风无感轻载区)
        let ratio = (temp - 50) as f32 / 15.0;
        (30.0 + ratio * 20.0).round().clamp(0.0, 100.0) as u8
    } else if temp <= 80 {
        // 65°C -> 50%, 80°C -> 60% (高效舒适区，牢牢控制在 60% 噪音线内)
        let ratio = (temp - 65) as f32 / 15.0;
        (50.0 + ratio * 10.0).round().clamp(0.0, 100.0) as u8
    } else if temp <= 90 {
        // 80°C -> 60%, 90°C -> 75% (性能压温区)
        let ratio = (temp - 80) as f32 / 10.0;
        (60.0 + ratio * 15.0).round().clamp(0.0, 100.0) as u8
    } else {
        // > 90°C: 85% ~ 100% (配合狂暴推水极限救命)
        let extra = (temp.saturating_sub(90) as f32 * 3.0).min(15.0);
        (85.0 + extra).round().clamp(0.0, 100.0) as u8
    }
}

pub fn evaluate_cooler_curve(points: &[crate::system::config::CoolerCurvePoint], temp: u8) -> (u8, u8) {
    if points.is_empty() {
        return (evaluate_cooler_fan_duty_smooth(temp), CoolerPumpTier::Standard as u8);
    }
    let mut sorted = points.to_vec();
    sorted.sort_by_key(|p| p.temp);

    let first = match sorted.first() {
        Some(f) => f,
        None => return (evaluate_cooler_fan_duty_smooth(temp), CoolerPumpTier::Standard as u8),
    };
    let last = match sorted.last() {
        Some(l) => l,
        None => return (first.fan_duty, first.pump_voltage),
    };

    if sorted.len() == 1 || temp <= first.temp {
        return (first.fan_duty, first.pump_voltage);
    }
    if temp >= last.temp {
        return (last.fan_duty, last.pump_voltage);
    }

    for i in 0..(sorted.len() - 1) {
        let p1 = &sorted[i];
        let p2 = &sorted[i + 1];
        if temp >= p1.temp && temp <= p2.temp {
            let temp_range = (p2.temp - p1.temp) as f32;
            let ratio = if temp_range > 0.0 {
                (temp - p1.temp) as f32 / temp_range
            } else {
                0.0
            };
            let fan_diff = (p2.fan_duty as f32) - (p1.fan_duty as f32);
            let fan_duty = (p1.fan_duty as f32 + ratio * fan_diff).round().clamp(0.0, 100.0) as u8;
            let pump_v = if ratio >= 0.5 { p2.pump_voltage } else { p1.pump_voltage };
            return (fan_duty, pump_v);
        }
    }

    (last.fan_duty, last.pump_voltage)
}

pub fn get_current_max_temp() -> u8 {
    let cpu_t = crate::system::cpu_monitor::get_global_cpu_monitor().get_temp().max(0) as u8;
    let gpu_t = {
        let gpu = crate::driver::gpu_bridge::get_global_gpu();
        let g = gpu.lock();
        if g.get_gpu_count() > 0 { g.get_temp(0) as u8 } else { 0 }
    };
    cpu_t.max(gpu_t).max(30)
}

pub fn calculate_strategy_params(
    strategy: &str,
    _curve_points: &[crate::system::config::CoolerCurvePoint],
    fallback_pump: u8,
    fallback_fan: u8,
    pump_start_temp: u8,
    pump_turbo_temp: u8,
) -> (u8, u8) {
    match strategy {
        "quiet" => (CoolerPumpTier::Standard as u8, 40),
        "balance" => (CoolerPumpTier::Standard as u8, 60),
        "performance" => (CoolerPumpTier::Turbo as u8, 90),
        "smart" => {
            let cur_temp = get_current_max_temp();
            let fan = evaluate_cooler_fan_duty_smooth(cur_temp);
            // Default smart uses hysteresis-driven standard tier
            let tier = calculate_pump_tier_with_hysteresis(cur_temp, CoolerPumpTier::Standard, pump_start_temp, pump_turbo_temp);
            (tier as u8, fan)
        }
        _ => {
            let valid_pump = CoolerPumpTier::from_u8(fallback_pump) as u8;
            (valid_pump, fallback_fan.clamp(0, 100))
        }
    }
}

impl BleCoolerController {
    pub fn new(mac: u64) -> Self {
        Self {
            state: Arc::new(WaterCoolerState {
                connected: AtomicBool::new(false),
                is_flow_ok: AtomicBool::new(false),
                meter_state: AtomicU8::new(0),
                pump_voltage: AtomicU8::new(7),
                pump_tier: AtomicU8::new(CoolerPumpTier::Standard as u8),
                fan_duty: AtomicU8::new(0),
                pump_rpm: AtomicU32::new(2200),
                running: AtomicBool::new(true),
                strategy: Arc::new(parking_lot::RwLock::new("smart".to_string())),
                curve_points: Arc::new(parking_lot::RwLock::new(crate::system::config::default_cooler_curve_points())),
                last_applied_temp: AtomicU8::new(0),
                pump_start_temp: AtomicU8::new(55),
                pump_turbo_temp: AtomicU8::new(90),
                safety_tripped: AtomicBool::new(false),
                is_priming: AtomicBool::new(false),
                led_on: AtomicBool::new(true),
                led_r: AtomicU8::new(0),
                led_g: AtomicU8::new(128),
                led_b: AtomicU8::new(0),
                led_mode: AtomicU8::new(2), // Mode 2: Colorful Flow default
                led_dirty: AtomicBool::new(true),
                params_dirty: AtomicBool::new(true),
                is_suspended: AtomicBool::new(false),
            }),
            mac_address: Arc::new(AtomicU64::new(mac)),
            active_char: Arc::new(parking_lot::Mutex::new(None)),
            supervisor_started: AtomicBool::new(false),
            wakeup_notify: Arc::new((parking_lot::Mutex::new(false), parking_lot::Condvar::new())),
        }
    }

    pub fn notify_supervisor(&self) {
        let (lock, cvar) = &*self.wakeup_notify;
        let mut ready = lock.lock();
        *ready = true;
        cvar.notify_all();
    }

    pub fn get_mac(&self) -> u64 {
        self.mac_address.load(Ordering::SeqCst)
    }

    pub fn set_mac(&self, new_mac: u64) {
        self.mac_address.store(new_mac, Ordering::SeqCst);
        let mut cfg = crate::system::config::load_config();
        cfg.cooler_mac = mac_u64_to_str(new_mac);
        crate::system::config::save_config(&cfg);
        crate::log_info!("BLE_COOLER", "Updated target MAC address to: {} ({:012X})", cfg.cooler_mac, new_mac);
        self.notify_supervisor();
    }

    pub fn state(&self) -> Arc<WaterCoolerState> {
        Arc::clone(&self.state)
    }

    pub fn get_telemetry(&self) -> CoolerTelemetry {
        let v = self.state.pump_voltage.load(Ordering::SeqCst);
        let tier = self.state.pump_tier.load(Ordering::SeqCst);
        let is_connected = self.state.connected.load(Ordering::SeqCst);
        let is_safety = self.state.safety_tripped.load(Ordering::SeqCst);
        let rpm = if is_connected && !is_safety {
            match CoolerPumpTier::from_u8(tier) {
                CoolerPumpTier::Stop => 0,
                CoolerPumpTier::Silent => 1800,
                CoolerPumpTier::Standard => 2400,
                CoolerPumpTier::Turbo => 4200,
            }
        } else {
            0
        };
        self.state.pump_rpm.store(rpm, Ordering::SeqCst);

        CoolerTelemetry {
            connected: is_connected,
            is_flow_ok: if is_connected { self.state.is_flow_ok.load(Ordering::SeqCst) } else { false },
            meter_state: if is_connected { self.state.meter_state.load(Ordering::SeqCst) } else { 0 },
            pump_voltage: v,
            pump_tier: tier,
            fan_duty: self.state.fan_duty.load(Ordering::SeqCst),
            pump_rpm: rpm,
            running: self.state.running.load(Ordering::SeqCst),
            safety_tripped: is_safety,
            is_priming: self.state.is_priming.load(Ordering::SeqCst),
            led_on: self.state.led_on.load(Ordering::SeqCst),
            led_r: self.state.led_r.load(Ordering::SeqCst),
            led_g: self.state.led_g.load(Ordering::SeqCst),
            led_b: self.state.led_b.load(Ordering::SeqCst),
            led_mode: self.state.led_mode.load(Ordering::SeqCst),
            mac_address: mac_u64_to_str(self.mac_address.load(Ordering::SeqCst)),
            strategy: self.state.strategy.read().clone(),
            curve_points: self.state.curve_points.read().clone(),
            pump_start_temp: self.state.pump_start_temp.load(Ordering::SeqCst),
            pump_turbo_temp: self.state.pump_turbo_temp.load(Ordering::SeqCst),
        }
    }

    pub fn set_strategy(&self, strategy: &str, curve: Option<Vec<crate::system::config::CoolerCurvePoint>>) {
        *self.state.strategy.write() = strategy.to_string();
        if let Some(ref pts) = curve {
            *self.state.curve_points.write() = pts.clone();
        }

        let cur_pump = self.state.pump_voltage.load(Ordering::SeqCst);
        let cur_fan = self.state.fan_duty.load(Ordering::SeqCst);
        let curve_pts = self.state.curve_points.read().clone();
        let start_t = self.state.pump_start_temp.load(Ordering::SeqCst);
        let turbo_t = self.state.pump_turbo_temp.load(Ordering::SeqCst);

        let (target_pump, target_fan) = calculate_strategy_params(
            strategy,
            &curve_pts,
            cur_pump,
            cur_fan,
            start_t,
            turbo_t,
        );

        let pump_tier = CoolerPumpTier::from_u8(target_pump);
        self.state.safety_tripped.store(false, Ordering::SeqCst);
        self.state.pump_tier.store(pump_tier as u8, Ordering::SeqCst);
        self.state.pump_voltage.store(target_pump, Ordering::SeqCst);
        self.state.fan_duty.store(target_fan, Ordering::SeqCst);
        if strategy == "smart" {
            let temp = get_current_max_temp();
            self.state.last_applied_temp.store(temp, Ordering::SeqCst);
        }
        self.state.params_dirty.store(true, Ordering::SeqCst);

        // Single clean atomic save to config.json, with no stale clobbering!
        let mut cfg = crate::system::config::load_config();
        cfg.cooler_strategy = strategy.to_string();
        cfg.cooler_pump_voltage = target_pump;
        cfg.cooler_fan_duty = target_fan;
        if let Some(pts) = curve {
            cfg.cooler_curve_points = pts;
        }
        crate::system::config::save_config(&cfg);
    }

    pub fn set_pump_thresholds(&self, start_temp: u8, turbo_temp: u8) -> CoolerTelemetry {
        let start_clamped = start_temp.clamp(35, 80);
        let turbo_clamped = turbo_temp.clamp(start_clamped + 5, 98);
        self.state.pump_start_temp.store(start_clamped, Ordering::SeqCst);
        self.state.pump_turbo_temp.store(turbo_clamped, Ordering::SeqCst);

        let mut cfg = crate::system::config::load_config();
        cfg.cooler_pump_start_temp = start_clamped;
        cfg.cooler_pump_turbo_temp = turbo_clamped;
        crate::system::config::save_config(&cfg);

        if self.state.strategy.read().as_str() == "smart" {
            let max_temp = get_current_max_temp();
            let cur_tier = CoolerPumpTier::from_u8(self.state.pump_tier.load(Ordering::SeqCst));
            let next_tier = calculate_pump_tier_with_hysteresis(max_temp, cur_tier, start_clamped, turbo_clamped);
            if cur_tier != next_tier {
                self.state.pump_tier.store(next_tier as u8, Ordering::SeqCst);
                self.state.pump_voltage.store(next_tier as u8, Ordering::SeqCst);
                self.state.params_dirty.store(true, Ordering::SeqCst);
            }
        }

        self.get_telemetry()
    }

    pub fn set_params(&self, pump_v: Option<u8>, fan_d: Option<u8>) {
        mark_user_cmd_sent();
        // Clearing safety trip on manual user action
        self.state.safety_tripped.store(false, Ordering::SeqCst);
        let mut cfg = crate::system::config::load_config();
        let mut target_pump_params = None;
        if let Some(v) = pump_v {
            let tier = CoolerPumpTier::from_u8(v);
            self.state.pump_tier.store(tier as u8, Ordering::SeqCst);
            self.state.pump_voltage.store(v, Ordering::SeqCst);
            target_pump_params = Some(tier.to_hardware_params());
            cfg.cooler_pump_voltage = v;
        }
        let mut _target_fan_params = None;
        if let Some(f) = fan_d {
            self.state.fan_duty.store(f, Ordering::SeqCst);
            let fan_on = if f == 0 { 0x00 } else { 0x01 };
            _target_fan_params = Some((fan_on, f));
            cfg.cooler_fan_duty = f;
        }
        cfg.cooler_strategy = self.state.strategy.read().clone();
        crate::system::config::save_config(&cfg);

        // Route parameter transmission through the supervisor daemon loop
        // to guarantee serialized writes, 80ms hardware-safe pacing, and zero UART buffer overrun.
        crate::log_warn!(
            "BLE_COOLER",
            "[set_params] v={:?}, f={:?}, target_pump={:?}, queuing params_dirty for supervisor",
            pump_v,
            fan_d,
            target_pump_params
        );
        self.state.params_dirty.store(true, Ordering::SeqCst);
    }

    pub fn set_led(&self, on: bool, r: u8, g: u8, b: u8, mode: u8) {
        let valid_mode = if mode == 0 { 2 } else { mode };
        // Baseline color safeguard: if on and RGB is all zero, inject official emerald green (0, 128, 0)
        let (final_r, final_g, final_b) = if on && r == 0 && g == 0 && b == 0 {
            (0, 128, 0)
        } else {
            (r, g, b)
        };
        self.state.led_on.store(on, Ordering::SeqCst);
        self.state.led_r.store(final_r, Ordering::SeqCst);
        self.state.led_g.store(final_g, Ordering::SeqCst);
        self.state.led_b.store(final_b, Ordering::SeqCst);
        self.state.led_mode.store(valid_mode, Ordering::SeqCst);
        self.state.led_dirty.store(true, Ordering::SeqCst);

        // Save into config.json
        let mut cfg = crate::system::config::load_config();
        cfg.cooler_led_on = on;
        cfg.cooler_led_r = final_r;
        cfg.cooler_led_g = final_g;
        cfg.cooler_led_b = final_b;
        cfg.cooler_led_mode = valid_mode;
        crate::system::config::save_config(&cfg);
    }

    pub fn reconnect(&self) {
        self.state.safety_tripped.store(false, Ordering::SeqCst);
        self.state.is_suspended.store(false, Ordering::SeqCst);
        self.state.running.store(true, Ordering::SeqCst);

        let strategy = self.state.strategy.read().clone();
        let curve_pts = self.state.curve_points.read().clone();
        let cur_pump = self.state.pump_voltage.load(Ordering::SeqCst);
        let cur_fan = self.state.fan_duty.load(Ordering::SeqCst);
        let start_t = self.state.pump_start_temp.load(Ordering::SeqCst);
        let turbo_t = self.state.pump_turbo_temp.load(Ordering::SeqCst);
        let (target_pump, target_fan) = calculate_strategy_params(&strategy, &curve_pts, cur_pump, cur_fan, start_t, turbo_t);
        self.state.pump_voltage.store(target_pump, Ordering::SeqCst);
        self.state.fan_duty.store(target_fan, Ordering::SeqCst);
        if strategy == "smart" {
            self.state.last_applied_temp.store(get_current_max_temp(), Ordering::SeqCst);
        }

        self.state.params_dirty.store(true, Ordering::SeqCst);
        self.state.led_dirty.store(true, Ordering::SeqCst);
        self.notify_supervisor();
    }

    /// Called on system resume from sleep / hibernate or lid open
    pub fn resume_after_wake(&self) {
        crate::log_info!("BLE_COOLER", "System resumed / lid opened! Reactivating water cooler supervisor...");
        self.state.safety_tripped.store(false, Ordering::SeqCst);
        self.state.is_suspended.store(false, Ordering::SeqCst);
        self.state.running.store(true, Ordering::SeqCst);

        let strategy = self.state.strategy.read().clone();
        let curve_pts = self.state.curve_points.read().clone();
        let cur_pump = self.state.pump_voltage.load(Ordering::SeqCst);
        let cur_fan = self.state.fan_duty.load(Ordering::SeqCst);
        let start_t = self.state.pump_start_temp.load(Ordering::SeqCst);
        let turbo_t = self.state.pump_turbo_temp.load(Ordering::SeqCst);
        let (target_pump, target_fan) = calculate_strategy_params(&strategy, &curve_pts, cur_pump, cur_fan, start_t, turbo_t);
        self.state.pump_voltage.store(target_pump, Ordering::SeqCst);
        self.state.fan_duty.store(target_fan, Ordering::SeqCst);
        if strategy == "smart" {
            self.state.last_applied_temp.store(get_current_max_temp(), Ordering::SeqCst);
        }

        self.state.params_dirty.store(true, Ordering::SeqCst);
        self.state.led_dirty.store(true, Ordering::SeqCst);
        self.notify_supervisor();
    }

    /// Graceful and immediate shutdown of physical water cooler:
    /// Cuts off pump power (0V), stops radiator fan (0%), extinguishes LEDs,
    /// clears EC water cooling flag, and pauses supervisor.
    pub fn stop_and_shutdown(&self) {
        crate::log_info!("BLE_COOLER", "Executing graceful shutdown (Quit / Sleep / Hibernate / Lid)...");
        self.state.is_suspended.store(true, Ordering::SeqCst);
        self.state.connected.store(false, Ordering::SeqCst);
        self.state.is_flow_ok.store(false, Ordering::SeqCst);
        self.state.meter_state.store(0, Ordering::SeqCst);

        // 1. Physically send shutdown frames if active characteristic is connected
        if let Some(ref write_char) = *self.active_char.lock() {
            crate::log_info!("BLE_COOLER", "Sending power cut and shutoff frames to hardware...");
            let _ = write_frame(write_char, &[0xFE, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            std::thread::sleep(Duration::from_millis(40));
            let _ = write_frame(write_char, &[0xFE, 0x1B, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            std::thread::sleep(Duration::from_millis(40));
            let _ = write_frame(write_char, &[0xFE, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            std::thread::sleep(Duration::from_millis(40));
            let _ = write_frame(write_char, &[0xFE, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
        }
        *self.active_char.lock() = None;

        // 2. Clear EC Water Cooling status flag
        crate::driver::wmi_ec::wmi_write_ec(ADDR_WATER_COOLER_FLAG, 0);
        crate::log_info!("BLE_COOLER", "Graceful hardware shutdown complete.");
        self.notify_supervisor();
    }

    pub fn reset_defaults(&self) {
        self.state.safety_tripped.store(false, Ordering::SeqCst);
        self.state.is_priming.store(false, Ordering::SeqCst);
        self.state.pump_start_temp.store(55, Ordering::SeqCst);
        self.state.pump_turbo_temp.store(90, Ordering::SeqCst);
        let mut cfg = crate::system::config::load_config();
        cfg.cooler_pump_start_temp = 55;
        cfg.cooler_pump_turbo_temp = 90;
        crate::system::config::save_config(&cfg);
        self.set_strategy("smart", Some(crate::system::config::default_cooler_curve_points()));
        self.set_led(true, 0, 128, 0, 2); // Default to Mode 2: Colorful flow
        self.state.led_dirty.store(true, Ordering::SeqCst);
    }

    pub fn trigger_fill_water(&self) {
        self.state.safety_tripped.store(false, Ordering::SeqCst);
        self.state.is_priming.store(true, Ordering::SeqCst);
    }

    /// Single Persistent Supervisor Daemon for BLE connection and keep-alive loop.
    /// Non-blocking, fails gracefully if cooler is disconnected, automatically reconnects.
    pub fn start_active_cooling(self: Arc<Self>) {
        if !crate::system::config::is_water_cooler_enabled() {
            crate::log_info!("BLE_COOLER", "Water cooler module is disabled. Supervisor will not launch.");
            return;
        }

        self.state.is_suspended.store(false, Ordering::SeqCst);
        self.state.running.store(true, Ordering::SeqCst);
        self.state.params_dirty.store(true, Ordering::SeqCst);
        self.state.led_dirty.store(true, Ordering::SeqCst);

        // Ensure ONLY ONE supervisor daemon ever runs!
        if self.supervisor_started.swap(true, Ordering::SeqCst) {
            self.notify_supervisor();
            return;
        }

        std::thread::spawn(move || {
            let mut consecutive_errors: u32 = 0;
            let mut fast_retries_remaining: u32 = 4; // Start with active burst on boot/launch

            loop {
                // If paused (e.g. system in sleep, lid closed) or module disabled in config, wait quietly
                if !self.state.running.load(Ordering::SeqCst) || self.state.is_suspended.load(Ordering::SeqCst) || !crate::system::config::is_water_cooler_enabled() {
                    let (lock, cvar) = &*self.wakeup_notify;
                    let mut ready = lock.lock();
                    if !*ready {
                        let _ = cvar.wait_for(&mut ready, Duration::from_millis(500));
                    }
                    continue;
                }

                // If user or external/watcher trigger requested connection, reset fast retries
                {
                    let (lock, _) = &*self.wakeup_notify;
                    let mut req = lock.lock();
                    if *req {
                        *req = false;
                        fast_retries_remaining = 3;
                        consecutive_errors = 0;
                    }
                }

                crate::log_info!(
                    "BLE_COOLER",
                    "Initiating BLE connection session (burst remaining: {}, total errors: {})...",
                    fast_retries_remaining,
                    consecutive_errors
                );

                let session_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.run_connection_session()
                }));

                match session_res {
                    Ok(Ok(_)) => {
                        consecutive_errors = 0;
                        fast_retries_remaining = 3;
                        crate::log_info!("BLE_COOLER", "Session finished normally.");
                    }
                    Ok(Err(e)) => {
                        consecutive_errors = consecutive_errors.saturating_add(1);
                        self.state.connected.store(false, Ordering::SeqCst);
                        self.state.is_flow_ok.store(false, Ordering::SeqCst);
                        *self.active_char.lock() = None;

                        crate::log_warn!("BLE_COOLER", "Connection attempt failed: {}", e);

                        // Zero-overhead registry check (0ms, no RF scan): if current MAC failed, auto-sync with Windows PnP paired cooler
                        if let Some(paired) = find_paired_water_cooler_mac() {
                            if paired != self.get_mac() {
                                crate::log_warn!("BLE_COOLER", "Current MAC {:012X} failed, auto-recovering to Windows PnP paired cooler: {:012X}", self.get_mac(), paired);
                                self.set_mac(paired);
                            }
                        }

                        if fast_retries_remaining > 0 {
                            fast_retries_remaining -= 1;
                            let retry_secs = match fast_retries_remaining {
                                2 => 2,
                                1 => 3,
                                _ => 4,
                            };
                            crate::log_info!("BLE_COOLER", "Active burst retry in {}s (remaining: {})...", retry_secs, fast_retries_remaining);

                            let (lock, cvar) = &*self.wakeup_notify;
                            let mut ready = lock.lock();
                            if !*ready {
                                let _ = cvar.wait_for(&mut ready, Duration::from_secs(retry_secs));
                            }
                            *ready = false;
                        } else {
                            // Active burst exhausted. Water cooler is offline / powered off.
                            // Switch to Passive Advertisement Watcher Standby (0% CPU, 0 RF TX).
                            crate::log_info!(
                                "BLE_COOLER",
                                "Water cooler offline. Entering Passive Advertisement Standby (zero RF/CPU, auto-wakes on cooler power-on)..."
                            );
                            self.standby_listen_for_cooler(Duration::from_secs(45));
                        }
                    }
                    Err(panic_err) => {
                        consecutive_errors = consecutive_errors.saturating_add(1);
                        crate::log_error!("BLE_COOLER", "BLE session crashed/panicked: {:?}", panic_err);
                        self.state.connected.store(false, Ordering::SeqCst);
                        self.state.is_flow_ok.store(false, Ordering::SeqCst);
                        *self.active_char.lock() = None;
                        std::thread::sleep(Duration::from_secs(4));
                    }
                }
            }
        });
    }

    /// Low-power passive BLE advertisement listener:
    /// Zero RF broadcast transmission, 0% CPU overhead, zero interference with Bluetooth audio/mice.
    /// Wakes up instantly when the physical water cooler powers on and broadcasts advertising packets.
    fn standby_listen_for_cooler(&self, max_wait: Duration) {
        let target_mac = self.get_mac();
        let watcher_res = BluetoothLEAdvertisementWatcher::new();
        let watcher = match watcher_res {
            Ok(w) => w,
            Err(e) => {
                crate::log_warn!("BLE_COOLER", "Failed to create BLE Advertisement Watcher for standby: {}", e);
                let (lock, cvar) = &*self.wakeup_notify;
                let mut ready = lock.lock();
                if !*ready {
                    let _ = cvar.wait_for(&mut ready, max_wait);
                }
                return;
            }
        };

        let _ = watcher.SetScanningMode(BluetoothLEScanningMode::Passive);

        let notify_clone = Arc::clone(&self.wakeup_notify);
        let mac_clone = Arc::clone(&self.mac_address);

        let token_res = watcher.Received(&windows::Foundation::TypedEventHandler::new(
            move |_sender: &Option<BluetoothLEAdvertisementWatcher>, args: &Option<BluetoothLEAdvertisementReceivedEventArgs>| {
                if let Some(args) = args {
                    let address = args.BluetoothAddress().unwrap_or(0);
                    if address == 0 {
                        return Ok(());
                    }

                    let target = mac_clone.load(Ordering::SeqCst);
                    let mut matched = false;

                    if target != 0 && address == target {
                        matched = true;
                    } else if let Ok(ad) = args.Advertisement() {
                        if let Ok(name) = ad.LocalName() {
                            let name_str = name.to_string();
                            if !name_str.is_empty() {
                                let upper = name_str.to_uppercase();
                                if upper.contains("COOLING") || upper.contains("LCT") || upper.contains("CS_") {
                                    crate::log_info!(
                                        "BLE_COOLER",
                                        "Standby watcher detected cooler by name: '{}' (MAC: {:012X})",
                                        name_str,
                                        address
                                    );
                                    if target == 0 {
                                        mac_clone.store(address, Ordering::SeqCst);
                                    }
                                    matched = true;
                                }
                            }
                        }
                    }

                    if matched {
                        crate::log_info!(
                            "BLE_COOLER",
                            "Standby watcher matched cooler {:012X}! Waking supervisor...",
                            address
                        );
                        let (lock, cvar) = &*notify_clone;
                        let mut ready = lock.lock();
                        *ready = true;
                        cvar.notify_all();
                    }
                }
                Ok(())
            },
        ));

        let token = match token_res {
            Ok(t) => t,
            Err(e) => {
                crate::log_warn!("BLE_COOLER", "Failed to attach Received handler to watcher: {}", e);
                let (lock, cvar) = &*self.wakeup_notify;
                let mut ready = lock.lock();
                if !*ready {
                    let _ = cvar.wait_for(&mut ready, max_wait);
                }
                return;
            }
        };

        if let Err(e) = watcher.Start() {
            crate::log_warn!("BLE_COOLER", "Failed to start BLE Advertisement Watcher: {}", e);
            let _ = watcher.RemoveReceived(token);
            let (lock, cvar) = &*self.wakeup_notify;
            let mut ready = lock.lock();
            if !*ready {
                let _ = cvar.wait_for(&mut ready, max_wait);
            }
            return;
        }

        crate::log_info!(
            "BLE_COOLER",
            "Passive BLE Advertisement Watcher active (target: {:012X}, max standby sleep: {:?})",
            target_mac,
            max_wait
        );

        // Park thread on condvar until watcher signals or timeout or user clicks reconnect
        {
            let (lock, cvar) = &*self.wakeup_notify;
            let mut ready = lock.lock();
            if !*ready {
                let _ = cvar.wait_for(&mut ready, max_wait);
            }
        }

        // Stop watcher once awakened
        let _ = watcher.Stop();
        let _ = watcher.RemoveReceived(token);
        crate::log_info!("BLE_COOLER", "Passive BLE Advertisement Watcher stopped, resuming active connection path.");
    }

    fn run_connection_session(&self) -> Result<(), String> {
        self.state.connected.store(false, Ordering::SeqCst);

        let mut target_mac = self.mac_address.load(Ordering::SeqCst);

        // Auto-discovery fallback: if no stored MAC, try Windows PnP paired device
        if target_mac == 0 {
            if let Some(paired) = find_paired_water_cooler_mac() {
                crate::log_warn!("BLE_COOLER", "No stored MAC, resolved paired water cooler from Windows PnP: {:012X}", paired);
                self.set_mac(paired);
                target_mac = paired;
            } else {
                return Err("No water cooler MAC configured and none paired in Windows PnP".to_string());
            }
        }

        let dev_res = BluetoothLEDevice::FromBluetoothAddressAsync(target_mac)
            .and_then(|op| op.get());

        if dev_res.is_err() {
            return Err(format!("Water cooler {:012X} is unreachable or powered off", target_mac));
        }

        let dev = dev_res.map_err(|e| format!("Device resolution failed: {}", e))?;

        // Maintain connection to prevent Windows BLE stack from dropping link or sleeping.
        // MUST hold _gatt_session alive throughout run_connection_session to prevent automatic teardown!
        let _gatt_session = if let Ok(device_id) = dev.BluetoothDeviceId() {
            if let Ok(session_op) = GattSession::FromDeviceIdAsync(&device_id) {
                if let Ok(session) = session_op.get() {
                    let _ = session.SetMaintainConnection(true);
                    Some(session)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Monitor real physical Bluetooth connection status changes
        let state_for_event = Arc::clone(&self.state);
        let _ = dev.ConnectionStatusChanged(&windows::Foundation::TypedEventHandler::new(
            move |sender: &Option<BluetoothLEDevice>, _| {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if let Some(d) = sender {
                        if let Ok(status) = d.ConnectionStatus() {
                            let is_conn = status == BluetoothConnectionStatus::Connected;
                            if is_conn {
                                state_for_event.connected.store(true, Ordering::SeqCst);
                            }
                        }
                    }
                }));
                Ok(())
            },
        ));

        // Attempt Cached service discovery first, then Uncached if needed
        let mut service_opt = None;
        for cache_mode in [BluetoothCacheMode::Cached, BluetoothCacheMode::Uncached] {
            if let Ok(svc_op) = dev.GetGattServicesForUuidWithCacheModeAsync(NUS_SERVICE_UUID, cache_mode) {
                if let Ok(svc_res) = svc_op.get() {
                    if svc_res.Status().unwrap_or(GattCommunicationStatus::Unreachable) == GattCommunicationStatus::Success {
                        if let Ok(services) = svc_res.Services() {
                            if services.Size().unwrap_or(0) > 0 {
                                if let Ok(s) = services.GetAt(0) {
                                    service_opt = Some(s);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(300));
        }

        let service: GattDeviceService = match service_opt {
            Some(s) => s,
            None => {
                let _ = dev.Close();
                crate::log_warn!("BLE_COOLER", "GATT service unreachable on {:012X}.", target_mac);
                return Err("NUS GATT Service unreachable or cooler disconnected".to_string());
            }
        };

        // Open service in shared read/write mode
        let _ = service.OpenAsync(GattSharingMode::SharedReadAndWrite).map(|op| op.get());

        // Discover characteristics
        let mut write_char = None;
        let mut notify_char = None;

        for cache_mode in [BluetoothCacheMode::Cached, BluetoothCacheMode::Uncached] {
            if let Ok(chars_op) = service.GetCharacteristicsWithCacheModeAsync(cache_mode) {
                if let Ok(chars_res) = chars_op.get() {
                    if chars_res.Status().unwrap_or(GattCommunicationStatus::Unreachable) == GattCommunicationStatus::Success {
                        if let Ok(chars) = chars_res.Characteristics() {
                            for i in 0..chars.Size().unwrap_or(0) {
                                if let Ok(c) = chars.GetAt(i) {
                                    if let Ok(uuid) = c.Uuid() {
                                        if uuid == NUS_TX_WRITE_UUID {
                                            write_char = Some(c);
                                        } else if uuid == NUS_RX_NOTIFY_UUID {
                                            notify_char = Some(c);
                                        }
                                    }
                                }
                            }
                            if write_char.is_some() {
                                break;
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(300));
        }

        let write_char = match write_char {
            Some(w) => w,
            None => {
                let _ = service.Close();
                let _ = dev.Close();
                return Err("NUS TX characteristic not found".into());
            }
        };

        let _held_notify_char = notify_char;

        // Enable notifications on RX & handle telemetry callbacks (0x31 unsolicited, 0x32 query response)
        if let Some(ref nc) = _held_notify_char {
            let state_clone = Arc::clone(&self.state);
            let _ = nc.ValueChanged(&windows::Foundation::TypedEventHandler::new(
                move |_sender, args: &Option<GattValueChangedEventArgs>| {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        if let Some(args) = args {
                            if let Ok(buf) = args.CharacteristicValue() {
                                let len = buf.Length().unwrap_or(0) as usize;
                                if len >= 4 {
                                    use windows::Storage::Streams::DataReader;
                                    if let Ok(reader) = DataReader::FromBuffer(&buf) {
                                        let mut bytes = vec![0u8; len];
                                        if reader.ReadBytes(&mut bytes).is_ok() {
                                            crate::log_info!("BLE_COOLER", "[Notify] Received {} bytes: {:02X?}", bytes.len(), &bytes);
                                            // Official protocol: FE [31|32] XX SS 00 00 00 EF
                                            // byte[3] is SS (Meter state from LC_Event_Meter_State)
                                            // state=2: Normal Flow (OK) ✓ (From official disassembled LiquidCooling.cs)
                                            // state=1: Normal Flow / Flowing (OK) ✓
                                            // state=0: No Flow / device not ready / fault
                                            if bytes[0] == 0xFE && bytes.len() >= 4 {
                                                let meter_state = bytes[3];
                                                let flow_ok = meter_state == 2 || meter_state == 1;
                                                state_clone.meter_state.store(meter_state, Ordering::SeqCst);
                                                state_clone.is_flow_ok.store(flow_ok, Ordering::SeqCst);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }));
                    Ok(())
                },
            ));

            let _ = nc.WriteClientCharacteristicConfigurationDescriptorAsync(
                GattClientCharacteristicConfigurationDescriptorValue::Notify,
            ).map(|op| op.get());
        }

        // Handshake "sw"
        if let Err(e) = write_frame(&write_char, &[0x73, 0x77]) {
            let _ = service.Close();
            let _ = dev.Close();
            return Err(format!("Handshake failed: {}", e));
        }
        // Give MCU 300ms to process handshake and stabilize
        std::thread::sleep(Duration::from_millis(300));

        // Mark connected and establish flowing state
        self.state.connected.store(true, Ordering::SeqCst);
        *self.active_char.lock() = Some(write_char.clone());

        // Re-evaluate strategy parameters immediately before force pushing
        let strategy = self.state.strategy.read().clone();
        let curve_pts = self.state.curve_points.read().clone();
        let cur_pump = self.state.pump_voltage.load(Ordering::SeqCst);
        let cur_fan = self.state.fan_duty.load(Ordering::SeqCst);
        let start_t = self.state.pump_start_temp.load(Ordering::SeqCst);
        let turbo_t = self.state.pump_turbo_temp.load(Ordering::SeqCst);
        let (pump_v, fan_d) = calculate_strategy_params(&strategy, &curve_pts, cur_pump, cur_fan, start_t, turbo_t);
        self.state.pump_voltage.store(pump_v, Ordering::SeqCst);
        self.state.fan_duty.store(fan_d, Ordering::SeqCst);
        if strategy == "smart" {
            self.state.last_applied_temp.store(get_current_max_temp(), Ordering::SeqCst);
        }

        if pump_v > 0 {
            self.state.is_flow_ok.store(true, Ordering::SeqCst);
            self.state.meter_state.store(2, Ordering::SeqCst);
        }
        crate::log_info!("BLE_COOLER", "Connected! Initializing pump, fan and flow query...");

        // Inform EC of water cooler connected
        crate::driver::wmi_ec::wmi_write_ec(ADDR_WATER_COOLER_FLAG, 1);

        // Gently apply pump, fan, LED with sufficient pacing so UART RX buffer doesn't overflow
        let cur_tier = CoolerPumpTier::from_u8(self.state.pump_tier.load(Ordering::SeqCst));
        let (pump_on, pump_volt, pump_duty) = cur_tier.to_hardware_params();
        let fan_on = if fan_d == 0 { 0x00 } else { 0x01 };

        crate::log_info!("BLE_COOLER", "Pushing initial state: Tier={:?}, Volt={}V, Duty={}%, Fan={}%", cur_tier, pump_volt, pump_duty, fan_d);
        // Correct Mechrevo protocol: [0xFE, 0x1C, OnOff, Volt, Duty, 0, 0, 0xEF]
        let _ = write_frame(&write_char, &[0xFE, 0x1C, pump_on, pump_volt, pump_duty, 0x00, 0x00, 0xEF]);
        std::thread::sleep(Duration::from_millis(50));
        let _ = write_frame(&write_char, &[0xFE, 0x1B, fan_on, fan_d, 0x00, 0x00, 0x00, 0xEF]);
        std::thread::sleep(Duration::from_millis(50));

        let led_on = self.state.led_on.load(Ordering::SeqCst);
        if led_on {
            let r = self.state.led_r.load(Ordering::SeqCst);
            let g = self.state.led_g.load(Ordering::SeqCst);
            let b = self.state.led_b.load(Ordering::SeqCst);
            let raw_mode = self.state.led_mode.load(Ordering::SeqCst);
            let mode = if raw_mode == 0 { 2 } else { raw_mode };
            // Ensure RGB is never all-zero to prevent MCU from cutting LED PWM duty to 0
            let (final_r, final_g, final_b) = if r == 0 && g == 0 && b == 0 {
                (0, 128, 0)
            } else {
                (r, g, b)
            };
            let _ = write_frame(&write_char, &[0xFE, 0x1E, 0x01, final_r, final_g, final_b, mode, 0xEF]);
            std::thread::sleep(Duration::from_millis(50));
            let _ = write_frame(&write_char, &[0xFE, 0x33, 0x01, final_r, final_g, final_b, mode, 0xEF]);
            std::thread::sleep(Duration::from_millis(50));
        } else {
            let _ = write_frame(&write_char, &[0xFE, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            std::thread::sleep(Duration::from_millis(50));
            let _ = write_frame(&write_char, &[0xFE, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            std::thread::sleep(Duration::from_millis(50));
        }
        self.state.params_dirty.store(false, Ordering::SeqCst);
        self.state.led_dirty.store(false, Ordering::SeqCst);

        // Initial Water Flow Query
        let _ = write_frame(&write_char, &[0xFE, 0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
        std::thread::sleep(Duration::from_millis(50));

        let mut loop_count = 1u64; // Start at 1 so iteration 0 doesn't duplicate 0x32
        let mut disconnect_strikes = 0u8;
        let mut last_error = None;
        let mut last_cmd_ms = 0u64;

        while self.state.running.load(Ordering::SeqCst) && !self.state.is_suspended.load(Ordering::SeqCst) {
            // Check physical connection status with 8-second persistence debounce
            // to tolerate transient link parameter updates from Windows BLE stack
            if let Ok(status) = dev.ConnectionStatus() {
                if status == BluetoothConnectionStatus::Disconnected {
                    disconnect_strikes += 1;
                    if disconnect_strikes >= 8 {
                        last_error = Some("Physical device disconnected (8s persistent)".to_string());
                        break;
                    }
                } else {
                    disconnect_strikes = 0;
                    self.state.connected.store(true, Ordering::SeqCst);
                }
            }

            // Check if user requested Fill Water (Priming Air Purge)
            if self.state.is_priming.load(Ordering::SeqCst) {
                crate::log_warn!("BLE_COOLER", "Executing Fill Water (Priming) routine...");
                let _ = write_frame(&write_char, &[0xFE, 0x19, 0x01, 0x04, 0x50, 0x00, 0x00, 0xEF]);
                let _ = write_frame(&write_char, &[0xFE, 0x1B, 0x01, 50, 0x00, 0x00, 0x00, 0xEF]);

                for _pulse in 1..=8 {
                    if !self.state.is_priming.load(Ordering::SeqCst) || !self.state.running.load(Ordering::SeqCst) {
                        break;
                    }
                    let _ = write_frame(&write_char, &[0xFE, 0x1C, 0x01, 7, 45, 0x00, 0x00, 0xEF]);
                    std::thread::sleep(Duration::from_secs(5));

                    let _ = write_frame(&write_char, &[0xFE, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
                    std::thread::sleep(Duration::from_millis(2500));
                }

                let _ = write_frame(&write_char, &[0xFE, 0x19, 0x00, 0x04, 0x00, 0x00, 0x00, 0xEF]);
                self.state.is_priming.store(false, Ordering::SeqCst);
                self.state.is_flow_ok.store(true, Ordering::SeqCst);
            }

            // 0. Smart Adaptive Temperature Linkage Strategy (CPU/GPU Max Temp with Hysteresis)
            if self.state.strategy.read().as_str() == "smart" {
                let max_temp = get_current_max_temp();
                let cur_tier = CoolerPumpTier::from_u8(self.state.pump_tier.load(Ordering::SeqCst));
                let start_temp = self.state.pump_start_temp.load(Ordering::SeqCst);
                let turbo_temp = self.state.pump_turbo_temp.load(Ordering::SeqCst);
                let next_tier = calculate_pump_tier_with_hysteresis(max_temp, cur_tier, start_temp, turbo_temp);
                let target_fan = evaluate_cooler_fan_duty_smooth(max_temp);
                let cur_fan = self.state.fan_duty.load(Ordering::SeqCst);

                if cur_tier != next_tier || (target_fan as i16 - cur_fan as i16).abs() >= 2 {
                    crate::log_debug!(
                        "BLE_SMART",
                        "[Smart RAMP] max_temp={}C, tier {:?} -> {:?}, fan {} -> {}",
                        max_temp,
                        cur_tier,
                        next_tier,
                        cur_fan,
                        target_fan
                    );
                    self.state.pump_tier.store(next_tier as u8, Ordering::SeqCst);
                    self.state.pump_voltage.store(next_tier as u8, Ordering::SeqCst);
                    self.state.fan_duty.store(target_fan, Ordering::SeqCst);
                    self.state.params_dirty.store(true, Ordering::SeqCst);
                }
                self.state.last_applied_temp.store(max_temp, Ordering::SeqCst);
            }

            // 1. On-Change Pump & Fan control: only send when user changes params or smart triggers
            if self.state.params_dirty.swap(false, Ordering::SeqCst) {
                let tier = CoolerPumpTier::from_u8(self.state.pump_tier.load(Ordering::SeqCst));
                let (pump_on, pump_volt, pump_duty) = tier.to_hardware_params();
                let fan_d = self.state.fan_duty.load(Ordering::SeqCst);
                let fan_on = if fan_d == 0 { 0x00 } else { 0x01 };

                crate::log_debug!(
                    "BLE_LOOP",
                    "[Loop params_dirty] Updating HW: Tier={:?}, Volt={}V, Duty={}%, Fan={}%",
                    tier,
                    pump_volt,
                    pump_duty,
                    fan_d
                );
                // Correct Mechrevo protocol: [0xFE, 0x1C, OnOff, Volt, Duty, 0, 0, 0xEF]
                let mut write_aborted = false;
                if let Err(e) = write_frame(&write_char, &[0xFE, 0x1C, pump_on, pump_volt, pump_duty, 0x00, 0x00, 0xEF]) {
                    crate::log_warn!("BLE_LOOP", "[Loop Write Error] Pump write warning: {}", e);
                    if e.contains("0x80000013") {
                        last_error = Some("Physical device closed during pump write (0x80000013)".to_string());
                        write_aborted = true;
                    }
                }
                if write_aborted {
                    break;
                }
                std::thread::sleep(Duration::from_millis(80));

                if let Err(e) = write_frame(&write_char, &[0xFE, 0x1B, fan_on, fan_d, 0x00, 0x00, 0x00, 0xEF]) {
                    crate::log_warn!("BLE_LOOP", "[Loop Write Error] Fan write warning: {}", e);
                    if e.contains("0x80000013") {
                        last_error = Some("Physical device closed during fan write (0x80000013)".to_string());
                        write_aborted = true;
                    }
                }
                if write_aborted {
                    break;
                }
                std::thread::sleep(Duration::from_millis(80));
                last_cmd_ms = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
            }

            // 2. On-Change RGB LED control: only send when user changes lighting
            if self.state.led_dirty.swap(false, Ordering::SeqCst) {
                let led_on = self.state.led_on.load(Ordering::SeqCst);
                if !led_on {
                    let _ = write_frame(&write_char, &[0xFE, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
                    std::thread::sleep(Duration::from_millis(50));
                    let _ = write_frame(&write_char, &[0xFE, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
                    std::thread::sleep(Duration::from_millis(50));
                } else {
                    let r = self.state.led_r.load(Ordering::SeqCst);
                    let g = self.state.led_g.load(Ordering::SeqCst);
                    let b = self.state.led_b.load(Ordering::SeqCst);
                    let raw_mode = self.state.led_mode.load(Ordering::SeqCst);
                    let mode = if raw_mode == 0 { 2 } else { raw_mode };

                    let (final_r, final_g, final_b) = if r == 0 && g == 0 && b == 0 {
                        (0, 128, 0)
                    } else {
                        (r, g, b)
                    };

                    let _ = write_frame(&write_char, &[0xFE, 0x1E, 0x01, final_r, final_g, final_b, mode, 0xEF]);
                    std::thread::sleep(Duration::from_millis(50));
                    let _ = write_frame(&write_char, &[0xFE, 0x33, 0x01, final_r, final_g, final_b, mode, 0xEF]);
                    std::thread::sleep(Duration::from_millis(50));
                }
            }

            // 3. Gentle Telemetry Heartbeat: query flow meter state every 3 seconds (avoiding interference during user command settling)
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if loop_count % 3 == 0 && (now_ms.saturating_sub(last_cmd_ms) > 2000) {
                if let Err(e) = write_frame(&write_char, &[0xFE, 0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]) {
                    if e.contains("0x80000013") {
                        last_error = Some("Physical device closed during flow query (0x80000013)".to_string());
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            }

            loop_count += 1;

            // Responsive interruptible sleep: 40 chunks of 25ms (total 1000ms, max 25ms latency)
            for _ in 0..40 {
                if !self.state.running.load(Ordering::SeqCst)
                    || self.state.params_dirty.load(Ordering::SeqCst)
                    || self.state.led_dirty.load(Ordering::SeqCst)
                    || self.state.is_priming.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        // Clean shutdown or error cleanup
        self.state.connected.store(false, Ordering::SeqCst);
        self.state.is_flow_ok.store(false, Ordering::SeqCst);
        self.state.meter_state.store(0, Ordering::SeqCst);

        // If stopped gracefully, send OFF frames
        if !self.state.running.load(Ordering::SeqCst) {
            let _ = write_frame(&write_char, &[0xFE, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            let _ = write_frame(&write_char, &[0xFE, 0x1B, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            let _ = write_frame(&write_char, &[0xFE, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            let _ = write_frame(&write_char, &[0xFE, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF]);
            crate::driver::wmi_ec::wmi_write_ec(ADDR_WATER_COOLER_FLAG, 0);
        }

        *self.active_char.lock() = None;
        let _ = service.Close();
        let _ = dev.Close();

        if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_params_fixed_modes() {
        let dummy_curve = vec![];
        assert_eq!(calculate_strategy_params("quiet", &dummy_curve, 2, 0, 55, 90), (CoolerPumpTier::Standard as u8, 40));
        assert_eq!(calculate_strategy_params("balance", &dummy_curve, 2, 0, 55, 90), (CoolerPumpTier::Standard as u8, 60));
        assert_eq!(calculate_strategy_params("performance", &dummy_curve, 2, 0, 55, 90), (CoolerPumpTier::Turbo as u8, 90));
    }

    #[test]
    fn test_pump_tier_hysteresis() {
        // From Stop
        assert_eq!(calculate_pump_tier_with_hysteresis(45, CoolerPumpTier::Stop, 55, 90), CoolerPumpTier::Stop);
        assert_eq!(calculate_pump_tier_with_hysteresis(55, CoolerPumpTier::Stop, 55, 90), CoolerPumpTier::Standard);
        assert_eq!(calculate_pump_tier_with_hysteresis(92, CoolerPumpTier::Stop, 55, 90), CoolerPumpTier::Turbo);

        // Downward hysteresis from Standard
        assert_eq!(calculate_pump_tier_with_hysteresis(52, CoolerPumpTier::Standard, 55, 90), CoolerPumpTier::Standard);
        assert_eq!(calculate_pump_tier_with_hysteresis(48, CoolerPumpTier::Standard, 55, 90), CoolerPumpTier::Stop);

        // Downward hysteresis from Turbo
        assert_eq!(calculate_pump_tier_with_hysteresis(88, CoolerPumpTier::Turbo, 55, 90), CoolerPumpTier::Turbo);
        assert_eq!(calculate_pump_tier_with_hysteresis(84, CoolerPumpTier::Turbo, 55, 90), CoolerPumpTier::Standard);
    }

    #[test]
    fn test_smooth_fan_curve() {
        assert_eq!(evaluate_cooler_fan_duty_smooth(40), 0);
        assert_eq!(evaluate_cooler_fan_duty_smooth(65), 50);
        assert_eq!(evaluate_cooler_fan_duty_smooth(80), 60);
        assert!(evaluate_cooler_fan_duty_smooth(92) >= 85);
    }

    #[test]
    fn test_diagnose_cooler() {
        println!("=== TESTING PAIRED REGISTRY DISCOVERY ===");
        let paired_mac = find_paired_water_cooler_mac();
        println!("find_paired_water_cooler_mac: {:?}", paired_mac.map(|m| format!("{:012X}", m)));
        // Registry discovery should execute without panic
        if let Some(mac) = paired_mac {
            assert!(mac > 0);
        }
    }
}

