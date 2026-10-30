// OpenRevo - Lighting Controller & Manager Architecture
// Coordinates Keyboard, Coastline Lightbar, Air Vent, and A-Cover Logo zones
// using Strategy pattern for hardware backends.

pub mod types;
pub mod device;
pub mod lightbar;
pub mod sleep_watcher;

pub use types::*;
pub use device::*;
pub use lightbar::*;
pub use sleep_watcher::*;
pub use crate::hal::vault::ite::*;

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use crate::driver::hid_ffi::{read_hklm_dword, set_hklm_dword, set_hklm_sz};
use crate::driver::wmi_ec::{
    ADDR_BACKLIGHT_WHITE, ADDR_SUPPORT_BYTE2, ADDR_MODE_INDEX,
    ADDR_TRIGGER_BYTE, ADDR_RGB_CONTROL, ADDR_RGB_RED, ADDR_RGB_GREEN, ADDR_RGB_BLUE,
};

pub type SharedIteLightingManager = Arc<IteLightingManager>;

pub struct IteLightingManager {
    kb_dev: Arc<Mutex<Option<IteLightingDevice>>>,
    lb_dev: Arc<Mutex<Option<IteLightingDevice>>>,
    hinge_dev: Arc<Mutex<Option<IteLightingDevice>>>,
    strip_dev: Arc<Mutex<Option<IteLightingDevice>>>,
    pub lb_strategy: Usb7001LightbarStrategy,
    pub hinge_strategy: Usb7000LightbarStrategy,
    pub ec_hinge_strategy: EcDirectLightbarStrategy,
    pub strip_strategy: Ite8291StripLightbarStrategy,
    pub state: Mutex<LightingPresetState>,
    pub is_sleeping: std::sync::atomic::AtomicBool,
}

unsafe impl Send for IteLightingManager {}
unsafe impl Sync for IteLightingManager {}

pub fn load_lighting_state() -> LightingPresetState {
    crate::system::config::load_config().lighting
}

pub fn save_lighting_state(st: &LightingPresetState) {
    let mut cfg = crate::system::config::load_config();
    let current_override = cfg.lighting.hardware_override.clone();
    cfg.lighting = st.clone();
    if !st.hardware_override.enabled && current_override.enabled {
        cfg.lighting.hardware_override = current_override;
    }
    crate::system::config::save_config(&cfg);
}

static GLOBAL_LIGHTING: std::sync::OnceLock<SharedIteLightingManager> = std::sync::OnceLock::new();
static IS_WELCOME_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Access global ITE lighting manager singleton across all Tauri modules and power listeners
pub fn get_global_lighting() -> SharedIteLightingManager {
    GLOBAL_LIGHTING.get_or_init(|| {
        let mgr = Arc::new(IteLightingManager::new());
        if !crate::hal::platform_probe::is_ultrabook() {
            let mgr_clone = Arc::clone(&mgr);
            std::thread::spawn(move || {
                let saved = mgr_clone.state.lock().clone();
                if saved.ceremony_enabled {
                    mgr_clone.trigger_welcome();
                } else {
                    mgr_clone.restore_all();
                }
            });
            mgr.start_idle_watcher();
        }
        mgr
    }).clone()
}

impl IteLightingManager {
    pub fn new() -> Self {
        let saved_state = load_lighting_state();
        let kb_dev = Arc::new(Mutex::new(None));
        let lb_dev = Arc::new(Mutex::new(None));
        let hinge_dev = Arc::new(Mutex::new(None));
        let strip_dev = Arc::new(Mutex::new(None));

        let lb_strategy = Usb7001LightbarStrategy::new(Arc::clone(&lb_dev));
        let hinge_strategy = Usb7000LightbarStrategy::new(Arc::clone(&hinge_dev));
        let ec_hinge_strategy = EcDirectLightbarStrategy;
        let strip_strategy = Ite8291StripLightbarStrategy::new(Arc::clone(&strip_dev));

        let mgr = Self {
            kb_dev,
            lb_dev,
            hinge_dev,
            strip_dev,
            lb_strategy,
            hinge_strategy,
            ec_hinge_strategy,
            strip_strategy,
            state: Mutex::new(saved_state),
            is_sleeping: std::sync::atomic::AtomicBool::new(false),
        };
        if !crate::hal::platform_probe::is_ultrabook() {
            mgr.refresh_devices();
        }
        mgr
    }

    pub fn has_keyboard_device(&self) -> bool {
        self.has_perkey_keyboard_device() || self.has_four_zone_keyboard_device()
    }

    pub fn has_perkey_keyboard_device(&self) -> bool {
        let contract = self.active_contract();
        if contract.backend != "USB_HID" || contract.keyboard_type != "PerKey" {
            return false;
        }
        crate::driver::ite_streamer::IteStreamerController::probe_exists()
    }

    pub fn has_four_zone_keyboard_device(&self) -> bool {
        let contract = self.active_contract();
        if contract.backend != "USB_HID" || contract.keyboard_type != "4Zone" {
            return false;
        }
        self.refresh_devices();
        self.kb_dev.lock().is_some()
    }

    pub fn has_lightbar_device(&self) -> bool {
        self.has_base_lightbar_device() || self.has_hinge_device()
    }

    pub fn has_base_lightbar_device(&self) -> bool {
        self.refresh_devices();
        self.lb_dev.lock().is_some() || self.strip_dev.lock().is_some() || Self::is_ec_lightbar_supported() || Self::probe_ambient_strip_exists()
    }

    pub fn has_usb_base_lightbar(&self) -> bool {
        self.refresh_devices();
        self.lb_dev.lock().is_some()
    }

    pub fn is_ec_lightbar_supported() -> bool {
        lightbar::EcDirectLightbarStrategy::probe_supported()
    }

    pub fn has_hinge_device(&self) -> bool {
        self.refresh_devices();
        self.hinge_dev.lock().is_some() || Self::probe_ec_hinge_supported()
    }

    /// Static probe to determine if any ITE RGB keyboard device is attached
    pub fn probe_keyboard_device_exists() -> bool {
        crate::driver::ite_streamer::IteStreamerController::probe_exists()
            || Self::probe_four_zone_keyboard_exists()
    }

    /// Static probe to determine if physical Four-Zone RGB keyboard device (048D:CE00) is attached
    pub fn probe_four_zone_keyboard_exists() -> bool {
        // Must strictly match UsagePage == 0xFF12 (USAGE_PAGE_FOURZONE) matching official LM_ITE_RGB.cs
        IteLightingDevice::open_exact(ITE_VID, ITE_PID_KEYBOARD_FOURZONE, 0, USAGE_PAGE_FOURZONE).is_some()
    }

    /// Static probe to determine if physical ITE 7001 base lightbar/logo device is attached
    pub fn probe_ambient_strip_exists() -> bool {
        Self::open_hid_device(ITE_VID, ITE_PID_AMBIENT_STRIP, 0).is_some()
    }

    pub fn probe_lightbar_device_exists() -> bool {
        if Self::probe_ambient_strip_exists() {
            return true;
        }
        if let Some(dev) = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_BASE, USAGE_LIGHTBAR) {
            if let Some(cap) = dev.probe_capability() {
                // Official LM_ITE_RGB.cs: Base coastline bar supported on 1, 3, 4, 5, 7, 9, 11
                return matches!(cap, 1 | 3 | 4 | 5 | 7 | 9 | 11);
            }
            return true;
        }
        false
    }

    /// Static probe to determine if physical Hinge / Ambient lightbar device is attached
    pub fn probe_hinge_device_exists() -> bool {
        // 1. Dedicated USB 7000 chip (e.g. Kuangshi 16 water-cooled series)
        if Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_HINGE, 0).is_some() {
            return true;
        }
        // 2. ITE 7001 secondary Hinge channel capability (official LM_ITE_RGB.cs logic)
        if let Some(dev) = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_BASE, 0) {
            if let Some(cap) = dev.probe_capability() {
                // Capabilities 0, 2, 3, 4, 6, 7, 10, 11 support Hinge
                return matches!(cap, 0 | 2 | 3 | 4 | 6 | 7 | 10 | 11);
            }
        }
        false
    }

    pub fn probe_ec_hinge_supported() -> bool {
        false
    }

    /// Static probe to determine if physical ITE 7001 base device has USB Logo capability (0xA2 byte 2 report)
    pub fn probe_7001_has_usb_logo() -> bool {
        if let Some(dev) = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_BASE, 0) {
            if let Some(cap) = dev.probe_capability() {
                // Official LM_ITE_RGB.cs: newlogosupport (Lettering streamer logo) supported ONLY on 5, 8, 9, 10, 11
                return matches!(cap, 5 | 8 | 9 | 10 | 11);
            }
        }
        false
    }

    /// Static probe to determine if physical USB Lettering Logo (streamer hardware) is supported
    pub fn probe_usb_logo_supported() -> bool {
        // 1. Dedicated USB 7000 chip (e.g. Kuangshi 16 water-cooled series)
        if Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_HINGE, 0).is_some() {
            return true;
        }
        // 2. USB 7001 secondary lettering logo channel
        Self::probe_7001_has_usb_logo()
    }

    /// Invalidate and drop cached HID device handles (e.g. before sleep or upon error)
    pub fn invalidate_devices(&self) {
        crate::driver::ite_streamer::IteStreamerController::invalidate_probe_cache();
        *self.kb_dev.lock() = None;
        *self.lb_dev.lock() = None;
        *self.hinge_dev.lock() = None;
        *self.strip_dev.lock() = None;
    }

    /// Gracefully release keyboard handle for BetterRGB streamer engine.
    pub fn release_keyboard_for_streamer(&self) {
        *self.kb_dev.lock() = None;
    }

    /// Resume from system sleep/hibernate with full device re-enumeration and engine-aware restoration
    pub fn resume_after_wake(&self) {
        if crate::hal::platform_probe::is_ultrabook() {
            return;
        }
        println!("[ITE Lighting] System resumed from Sleep / Hibernate! Reconnecting USB HID devices...");
        crate::driver::ite_streamer::get_global_streamer().set_unsupported(false);
        self.invalidate_devices();
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);
        // Wait 1.0s to allow Windows USB stack & power bus to stabilize
        std::thread::sleep(Duration::from_millis(1000));

        self.refresh_devices();
        let is_ceremony = self.state.lock().ceremony_enabled;
        if is_ceremony {
            self.trigger_welcome();
        } else {
            self.restore_all();
        }
    }

    /// Retrieve active Level 0 LightingContract from Machine Sovereign Profile
    pub fn active_contract(&self) -> crate::system::model_engine::LightingContract {
        crate::system::model_engine::get_active_lighting_contract()
    }

    /// Check if keyboard is permitted by Machine Constitution
    pub fn is_keyboard_allowed(&self) -> bool {
        let contract = self.active_contract();
        contract.backend != "None" && contract.keyboard_type != "None"
    }

    /// Check if Coastline Lightbar is physically asserted by Machine Constitution
    pub fn is_coastline_allowed(&self) -> bool {
        self.active_contract().has_coastline
    }

    /// Check if A-Cover Logo is physically asserted by Machine Constitution (Eliminates Ghost Logo Poisoning)
    pub fn is_logo_allowed(&self) -> bool {
        self.active_contract().has_logo
    }

    /// Check if Hinge / Air Vent is physically asserted by Machine Constitution
    pub fn is_air_vent_allowed(&self) -> bool {
        self.active_contract().has_air_vent
    }

    /// Scan and connect to hardware lighting devices, strictly gated by Level 0 Machine Constitution
    pub fn refresh_devices(&self) {
        let contract = self.active_contract();

        // 1. Keyboard USB Stack Isolation:
        // The USB HID bus may only be touched for the KEYBOARD when its backend actually
        // needs it. NOTE: this must NOT gate the lightbar / logo / hinge channels.
        // On hybrid machines (EC SingleZone keyboard + USB ITE 048D:7001 lettering logo,
        // e.g. GM6IX9B) gating everything on `backend` silently kills A-Cover LOGO control
        // while the Machine Constitution still declares has_logo = true.
        let keyboard_needs_usb = contract.backend == "USB_HID";

        // 2. Base Lightbar & Lettering Logo Channel:
        // Gated strictly by has_coastline or has_logo!
        if contract.has_coastline || contract.has_logo {
            let mut lb = self.lb_dev.lock();
            if lb.is_none() {
                *lb = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_BASE, USAGE_LIGHTBAR);
            }
        }

        // 2b. ITE 8291 Companion Ambient Strip Channel (048D:6008, e.g. Titan PLUS GM7TG0M):
        // Gated by has_coastline or has_air_vent!
        if contract.has_coastline || contract.has_air_vent {
            let mut strip = self.strip_dev.lock();
            if strip.is_none() {
                *strip = Self::open_hid_device(ITE_VID, ITE_PID_AMBIENT_STRIP, 0);
            }
        }

        // 3. Air Vent / Hinge Lightbar Channel:
        // Gated strictly by has_air_vent!
        if contract.has_air_vent {
            let mut hg = self.hinge_dev.lock();
            if hg.is_none() {
                *hg = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_HINGE, 0);
            }
        }

        // 4. Keyboard Channel:
        // Gated strictly by keyboard_type!
        if keyboard_needs_usb && contract.keyboard_type == "4Zone" && Self::probe_four_zone_keyboard_exists() {
            let mut kb = self.kb_dev.lock();
            if kb.is_none() {
                *kb = IteLightingDevice::open_exact(ITE_VID, ITE_PID_KEYBOARD_FOURZONE, 0, USAGE_PAGE_FOURZONE);
            }
        }
    }

    pub fn force_refresh_keyboard(&self) {
        let contract = self.active_contract();
        if contract.backend == "USB_HID" && contract.keyboard_type == "4Zone" && Self::probe_four_zone_keyboard_exists() {
            let mut kb = self.kb_dev.lock();
            *kb = IteLightingDevice::open_exact(ITE_VID, ITE_PID_KEYBOARD_FOURZONE, 0, USAGE_PAGE_FOURZONE);
        }
    }

    pub fn force_refresh_lightbar(&self) {
        let contract = self.active_contract();
        if contract.has_coastline || contract.has_logo {
            let mut lb = self.lb_dev.lock();
            *lb = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_BASE, USAGE_LIGHTBAR);
        }
        if contract.has_coastline || contract.has_air_vent {
            let mut strip = self.strip_dev.lock();
            *strip = Self::open_hid_device(ITE_VID, ITE_PID_AMBIENT_STRIP, 0);
        }
    }

    pub fn force_refresh_hinge(&self) {
        let contract = self.active_contract();
        if contract.has_air_vent {
            let mut hg = self.hinge_dev.lock();
            *hg = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_HINGE, 0);
        }
    }

    pub fn open_hid_device(vid: u16, pid: u16, target_usage: u16) -> Option<IteLightingDevice> {
        IteLightingDevice::open(vid, pid, target_usage)
    }

    pub fn open_hid_device_with_pids(vid: u16, pids: &[u16], target_usage: u16) -> Option<IteLightingDevice> {
        IteLightingDevice::open_with_pids(vid, pids, target_usage)
    }

    /// Enable or disable EC onKeyPressed event dispatching to ITE RGB keyboard
    #[allow(dead_code)]
    fn set_ec_on_key_pressed(&self, enable: bool) {
        if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_AP_OEM_BYTE) {
            let target = if enable { val | 0x08 } else { val & !0x08 };
            if target != val {
                crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_AP_OEM_BYTE, target);
            }
        }
    }

    /// Apply Monochrome White Backlight level directly via Motherboard EC (for Thin & Light laptops)
    pub fn apply_monochrome_white_ec(&self, enabled: bool, brightness_idx: u8) -> bool {
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);

        // Branch 1: Declarative HAL / Thin & Light models (IP3, Baolongda, Xingyao, etc.)
        let hal = crate::hal::get_hardware_driver();
        if hal.kind() != crate::hal::DriverKind::TongfangAcpi {
            let lvl = if enabled { brightness_idx } else { 0 };
            return hal.set_monochrome_backlight(lvl).is_ok();
        }

        let cur_val = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);

        if !enabled || brightness_idx == 0 {
            // Set disable bit (Bit 1 = 1)
            let target = cur_val | 0x02;
            let ok = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, target);
            if let Some(cur_trigger_byte) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, cur_trigger_byte | 0x20);
            }
            crate::log_info!("LIGHTING_ARBITER", "Monochrome white backlight disabled (EC=0x{:02X}) -> ok={}", target, ok);
            return ok;
        }

        // Map brightness index (1 -> 50% (0x20), 2..=4 -> 100% (0x40))
        let level_step = match brightness_idx {
            1 => 0x20,
            _ => 0x40,
        };
        let target = (cur_val & !0x02 & 0x0F) | level_step | 0x01;
        let ok = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, target);
        if let Some(cur_trigger_byte) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, cur_trigger_byte | 0x20);
        }
        crate::log_info!("LIGHTING_ARBITER", "Monochrome white backlight set: level_idx={} (EC=0x{:02X}) -> ok={}", brightness_idx, target, ok);
        ok
    }

    /// Apply Single-Zone EC RGB Backlight (for legacy gaming chassis)
    pub fn apply_single_zone_ec(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        _speed_idx: u8,
        color_hex: &str,
    ) -> bool {
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);

        // Branch 1: Declarative HAL / Non-Tongfang platforms (e.g. Baolongda MRID6)
        let hal = crate::hal::get_hardware_driver();
        if hal.kind() != crate::hal::DriverKind::TongfangAcpi {
            let is_on = enabled && brightness_idx > 0;
            let level = if is_on { brightness_idx.min(3) } else { 0 };
            let (r, g, b) = if is_on {
                parse_hex_color(color_hex)
            } else {
                (0, 0, 0)
            };
            if !is_on {
                let _ = hal.set_keyboard_effect(0, 0, 0, 0);
            } else {
                let _ = hal.set_keyboard_effect(effect, r, g, b);
            }
            let _ = hal.set_monochrome_backlight(level);
            crate::log_info!(
                "LIGHTING_ARBITER",
                "Applied single zone RGB to Declarative HAL: on={}, effect={}, level={}, rgb=({}, {}, {})",
                is_on, effect, level, r, g, b
            );
            return true;
        }

        let cur_bl_white = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);
        let is_on = enabled && brightness_idx > 0;

        let mut new_bl_white = cur_bl_white;
        if is_on {
            new_bl_white &= !0x02; // Bit 1 清零 = 启用背光供电
            new_bl_white &= 0x0F;  // 保留低 4 位，清零旧亮度高 4 位
            let high_nibble = match brightness_idx {
                1 => 0x30,
                2 => 0x50,
                3 => 0x70,
                _ => 0x90,
            };
            new_bl_white |= high_nibble;
        } else {
            // 关灯：Bit 1 置 1 (禁用供电)，Bit 4 必须维持 0x10 基底使能位
            new_bl_white = (new_bl_white & 0x0D) | 0x12;
        }
        let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, new_bl_white);

        // 锁存触发生效：向 ADDR_TRIGGER_BYTE 写入刷新位 (Bit 5, 0x20)，确保 EC 将 RAM 数据刷入硬件 PWM
        if let Some(cur_trigger_byte) = crate::driver::wmi_ec::wmi_read_ec(ADDR_TRIGGER_BYTE) {
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, cur_trigger_byte | 0x20);
        }

        // 彻底退出单色呼吸模式 (低3位清零，保留高位)，解除单呼吸蓝色锁定
        if let Some(cur_rgb_ctrl) = crate::driver::wmi_ec::wmi_read_ec(ADDR_RGB_CONTROL) {
            if (cur_rgb_ctrl & 0x07) != 0 {
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_CONTROL, cur_rgb_ctrl & 0xF8);
            }
        }

        let cur_trigger_byte = crate::driver::wmi_ec::wmi_read_ec(ADDR_TRIGGER_BYTE).unwrap_or(0);
        if !is_on {
            // 关闭背光：RGB 三通道调光写 0 (灭灯)，写入触发寄存器生效
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_RED, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_GREEN, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_BLUE, 0);
            let next_trigger = (cur_trigger_byte & !0x80) | 0x20;
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, next_trigger);
        } else if effect == 2 {
            // 彩虹循环模式:
            // 写入基准亮度种子色（官方为 50, 40, 40 或用户选定色彩），避免部分 EC 因 RGB=0 彻底黑灯
            let (r, g, b) = parse_hex_color(color_hex);
            let r_val = if r == 0 && g == 0 && b == 0 { 50 } else { ((r as u32 * 50) / 255).clamp(1, 50) as u8 };
            let g_val = if r == 0 && g == 0 && b == 0 { 40 } else { ((g as u32 * 50) / 255).clamp(1, 50) as u8 };
            let b_val = if r == 0 && g == 0 && b == 0 { 40 } else { ((b as u32 * 50) / 255).clamp(1, 50) as u8 };
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_RED, r_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_GREEN, g_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_BLUE, b_val);
            let next_trigger = cur_trigger_byte | 0x80 | 0x20;
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, next_trigger);
        } else {
            // 单色常亮: 清除彩虹标志，写入色彩档位 (0..50 刻度) 并提交锁存
            let (r, g, b) = parse_hex_color(color_hex);
            let (r_val, g_val, b_val) = if r == 255 && g == 255 && b == 255 {
                (50u8, 40u8, 40u8) // 官方特制白光平衡
            } else {
                let r_l = if r == 0 { 0 } else { ((r as u32 * 50) / 255).clamp(1, 50) as u8 };
                let g_l = if g == 0 { 0 } else { ((g as u32 * 50) / 255).clamp(1, 50) as u8 };
                let b_l = if b == 0 { 0 } else { ((b as u32 * 50) / 255).clamp(1, 50) as u8 };
                (r_l, g_l, b_l)
            };
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_RED, r_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_GREEN, g_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_BLUE, b_val);

            let next_trigger = (cur_trigger_byte & !0x80) | 0x20;
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, next_trigger);
        }
        true
    }

    /// Set Keyboard Backlight (Routed adaptively by Level 0 Machine Constitution)
    pub fn apply_keyboard(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        direction_idx: u8,
        color_hex: &str,
    ) -> bool {
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);

        let contract = self.active_contract();
        if contract.backend == "None" || contract.keyboard_type == "None" {
            crate::log_info!("LIGHTING_ARBITER", "Keyboard lighting bypassed: contract declares backend/keyboard = None");
            return false;
        }

        // Branch A: Pure EC-driven thinbook or EC Single-Zone RGB
        if contract.backend == "EC" {
            if contract.keyboard_type == "SingleZone" || contract.keyboard_type == "SingleZone_RGB" {
                return self.apply_single_zone_ec(enabled, effect, brightness_idx, speed_idx, color_hex);
            } else {
                return self.apply_monochrome_white_ec(enabled, brightness_idx);
            }
        }

        // Branch B: USB 4-Zone RGB keyboard
        if contract.keyboard_type == "4Zone" {
            let zones = self.state.lock().kb_zones.clone();
            return self.apply_four_zone_keyboard(enabled, effect, brightness_idx, speed_idx, direction_idx, color_hex, Some(&zones));
        }

        // Branch C: Per-Key RGB keyboard (ITE 8291 USB HID) -> Hardware MCU Execution!
        let streamer = crate::driver::ite_streamer::get_global_streamer();
        streamer.apply_hardware(
            enabled,
            effect,
            brightness_idx,
            speed_idx,
            direction_idx,
            color_hex,
        )
    }

    /// Set Four-Zone RGB Keyboard Backlight (Direct ITE 048D:CE00 MCU Execution)
    pub fn apply_four_zone_keyboard(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        direction_idx: u8,
        color_hex: &str,
        zone_colors: Option<&[String; 4]>,
    ) -> bool {
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);
        self.refresh_devices();

        // Hardware Power Guard: Ensure EC Backlight Disable Bit is cleared (0 = Enabled)
        if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE) {
            let target = if enabled && brightness_idx > 0 {
                val & !0x02
            } else {
                val | 0x02
            };
            if target != val {
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, target);
            }
        }

        let speed = SPEED_MAP.get(speed_idx as usize).copied().unwrap_or(5);
        let light = BRIGHTNESS_MAP.get(brightness_idx as usize).copied().unwrap_or(36);
        let direction = DIRECTION_MAP.get(direction_idx as usize).copied().unwrap_or(1);

        let kb = self.kb_dev.lock();
        if let Some(dev) = kb.as_ref() {
            if !enabled || brightness_idx == 0 {
                let off_pkt: [u8; 9] = [0, 8, 1, 0, 0, 0, 0, 0, 0];
                let _ = dev.write_feature(&off_pkt);
                // Save = 0 to prevent brown-out eFlash corruption during power state transitions
                let off_pkt2: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 255, 1, 50, 0, 0, 0];
                return dev.write_feature(&off_pkt2);
            }

            match effect {
                1 => {
                    // Static / Mono / 4-Zone Custom
                    if let Some(zones) = zone_colors {
                        for (i, c) in zones.iter().enumerate() {
                            let (r, g, b) = parse_hex_color(c);
                            let pkt = [0, 20, BLANK_KEYBOARD, (i + 1) as u8, r, g, b, 0, 0];
                            dev.write_feature(&pkt);
                        }
                    } else {
                        let (r, g, b) = parse_hex_color(color_hex);
                        for i in 1..=4 {
                            let pkt = [0, 20, BLANK_KEYBOARD, i as u8, r, g, b, 0, 0];
                            dev.write_feature(&pkt);
                        }
                    }
                    let end_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 1, 5, light, 8, 0, 0];
                    dev.write_feature(&end_pkt)
                }
                2 => {
                    // Breath
                    let (r, g, b) = parse_hex_color(color_hex);
                    for i in 1..=4 {
                        let pkt = [0, 20, BLANK_KEYBOARD, i as u8, r, g, b, 0, 0];
                        dev.write_feature(&pkt);
                    }
                    let end_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 2, speed, light, 8, 0, 0];
                    dev.write_feature(&end_pkt)
                }
                3 => {
                    // Wave
                    for (idx, &(r, g, b)) in RAINBOW_COLORS.iter().enumerate() {
                        let pkt = [0, 20, BLANK_KEYBOARD, (idx + 1) as u8, r, g, b, 0, 0];
                        dev.write_feature(&pkt);
                    }
                    let end_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 3, speed, light, 8, direction, 0];
                    dev.write_feature(&end_pkt)
                }
                5 => {
                    // Rainbow
                    let rainbow = [
                        [255, 0, 0],
                        [0, 180, 0],
                        [0, 0, 255],
                        [255, 0, 255],
                    ];
                    for (i, rgb) in rainbow.iter().enumerate() {
                        let pkt = [0, 20, BLANK_KEYBOARD, (i + 1) as u8, rgb[0], rgb[1], rgb[2], 0, 0];
                        dev.write_feature(&pkt);
                    }
                    let end_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 5, 5, light, 8, 0, 0];
                    dev.write_feature(&end_pkt)
                }
                12 | 18 => {
                    // Flash
                    for (idx, &(r, g, b)) in RAINBOW_COLORS.iter().enumerate() {
                        let pkt = [0, 20, BLANK_KEYBOARD, (idx + 1) as u8, r, g, b, 0, 0];
                        dev.write_feature(&pkt);
                    }
                    let end_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 0x12, speed, light, 8, direction, 0];
                    dev.write_feature(&end_pkt)
                }
                13 | 19 => {
                    // Mix
                    for (idx, &(r, g, b)) in RAINBOW_COLORS.iter().enumerate() {
                        let pkt = [0, 20, BLANK_KEYBOARD, (idx + 1) as u8, r, g, b, 0, 0];
                        dev.write_feature(&pkt);
                    }
                    let end_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 0x13, speed, light, 8, 0, 0];
                    dev.write_feature(&end_pkt)
                }
                _ => {
                    for (idx, &(r, g, b)) in RAINBOW_COLORS.iter().enumerate() {
                        let pkt = [0, 20, BLANK_KEYBOARD, (idx + 1) as u8, r, g, b, 0, 0];
                        dev.write_feature(&pkt);
                    }
                    let end_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 3, speed, light, 8, direction, 0];
                    dev.write_feature(&end_pkt)
                }
            }
        } else {
            false
        }
    }

    /// Set Coastline Lightbar (Dispatches cleanly via Usb7001 or Motherboard EC Direct Strategy)
    pub fn apply_lightbar(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        color_hex: &str,
    ) -> bool {
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);

        // Hardware Constitution Gate:
        if !self.is_coastline_allowed() {
            crate::log_warn!("LIGHTING_ARBITER", "apply_lightbar blocked: Machine Constitution declares has_coastline = false");
            return false;
        }

        self.refresh_devices();
        if self.lb_strategy.is_connected() {
            self.lb_strategy.apply_lightbar(enabled, effect, brightness_idx, speed_idx, color_hex)
        } else if self.strip_strategy.is_connected() {
            self.strip_strategy.apply_lightbar(enabled, effect, brightness_idx, speed_idx, color_hex)
        } else if Self::is_ec_lightbar_supported() {
            lightbar::EcDirectLightbarStrategy.apply_lightbar(enabled, effect, brightness_idx, speed_idx, color_hex)
        } else {
            false
        }
    }

    /// Set Air Vent / Hinge Coast Light (Dispatches cleanly via Usb7000 or EC strategy)
    pub fn apply_hinge(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        color_hex: &str,
    ) -> bool {
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);

        // Hardware Constitution Gate:
        if !self.is_air_vent_allowed() {
            crate::log_warn!("LIGHTING_ARBITER", "apply_hinge blocked: Machine Constitution declares has_air_vent = false");
            return false;
        }

        self.refresh_devices();

        if self.hinge_strategy.is_connected() {
            self.hinge_strategy.apply_lightbar(enabled, effect, brightness_idx, speed_idx, color_hex)
        } else if self.strip_strategy.is_connected() {
            self.strip_strategy.apply_lightbar(enabled, effect, brightness_idx, speed_idx, color_hex)
        } else if self.ec_hinge_strategy.is_connected() {
            self.ec_hinge_strategy.apply_lightbar(enabled, effect, brightness_idx, speed_idx, color_hex)
        } else {
            false
        }
    }

    /// Synchronize Motherboard EC Air Vent Lightbar registers
    pub fn sync_ec_hinge(enabled: bool, effect: u8, brightness_idx: u8, color_hex: &str) -> bool {
        EcDirectLightbarStrategy.apply_lightbar(enabled, effect, brightness_idx, 0, color_hex)
    }

    /// Check if machine has motherboard EC-controlled A-Cover Logo
    pub fn is_ec_logo_supported() -> bool {
        // Physical USB Lettering Logo hardware takes precedence
        if Self::probe_usb_logo_supported() {
            return false;
        }

        // SingleZone RGB or SingleColor keyboards physically wire registers 1897~1899 to keyboard backlighting PWM;
        // they NEVER have an independent EC Shield Logo (verified via official Tongfang NvramVariable.cs line 782).
        let raw_kb_support = crate::driver::wmi_ec::wmi_read_ec(ADDR_SUPPORT_BYTE2).unwrap_or(0);
        let is_single_zone = (raw_kb_support & 0x04) != 0;
        let raw_bl_white = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);
        let is_single_color = (raw_bl_white & 0x01) != 0;
        if is_single_zone || is_single_color {
            return false;
        }

        // Check operating mode index bit 0 (official isSupportECALogo hardware capability bit)
        let raw_mode_index = crate::driver::wmi_ec::wmi_read_ec(ADDR_MODE_INDEX).unwrap_or(0);
        if (raw_mode_index & 0x01) == 0x01 {
            return true;
        }

        // Check OEM registry Support\MBALogo
        if let Some(mba) = read_hklm_dword(r"SOFTWARE\OEM\GamingCenter2\Support", "MBALogo") {
            if mba == 1 {
                return true;
            }
        }

        // Check product project ID (e.g. Canglong Series ProjectID 25, 26)
        let project_id = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_PROJECT_ID).unwrap_or(0);
        if project_id == 25 || project_id == 26 {
            return true;
        }

        false
    }

    /// Restore EC A-Logo and OEM registry state on OpenRevo exit/handover
    pub fn handover_logo_on_exit() {
        let contract = crate::system::model_engine::get_active_lighting_contract();
        if !contract.has_logo {
            crate::log_info!("LIGHTING_ARBITER", "handover_logo_on_exit skipped: Machine Constitution declares has_logo = false. MBALogo poisoning safely prevented!");
            return;
        }

        if Self::probe_usb_logo_supported() {
            return;
        }
        if !Self::is_ec_logo_supported() {
            return;
        }

        let cur_mode = crate::driver::wmi_ec::wmi_read_ec(ADDR_MODE_INDEX).unwrap_or(0);
        let cur_bl = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);

        let current_br = cur_mode & 0xE0;
        let br_byte = if current_br == 0 { 0x40 } else { current_br };

        let exit_mode = (cur_mode & 0x1E) | br_byte | 0x01;
        let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_MODE_INDEX, exit_mode);
        let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, cur_bl | 0x10);

        set_hklm_dword(r"SOFTWARE\OEM\GamingCenter2\Support", "MBALogo", 1);
        set_hklm_sz(r"SOFTWARE\OEM\GamingCenter2\RGBKeyboard", "Lightbar_logo_PowerSwitch", "1");
        set_hklm_sz(r"SOFTWARE\OEM\GamingCenter2\RGBKeyboard", "Lightbar_logo_ACLight", "3");
        set_hklm_sz(r"SOFTWARE\OEM\GamingCenter2\RGBKeyboard", "Lightbar_logo_DCLight", "3");

        crate::log_info!("ITE_LIGHTING", "handover_logo_on_exit: restored A-Logo state=0x{:02X} & MBALogo=1", exit_mode);
    }

    /// Synchronize A-Cover LOGO with Motherboard EC (for models where A-Logo is wired to EC PWM/GPIO)
    pub fn sync_ec_logo(enabled: bool, effect: u8, brightness_idx: u8, color_hex: &str) -> bool {
        let contract = crate::system::model_engine::get_active_lighting_contract();
        if !contract.has_logo {
            crate::log_warn!("LIGHTING_ARBITER", "sync_ec_logo blocked: Machine Constitution declares has_logo = false");
            return false;
        }

        if !Self::is_ec_logo_supported() {
            return false;
        }

        let is_supported = true;
        let cur_mode_index = crate::driver::wmi_ec::wmi_read_ec(ADDR_MODE_INDEX).unwrap_or(0);
        let cur_bl_white = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);
        let cur_trigger_byte = crate::driver::wmi_ec::wmi_read_ec(ADDR_TRIGGER_BYTE).unwrap_or(0);
        let cur_rgb_ctrl = crate::driver::wmi_ec::wmi_read_ec(ADDR_RGB_CONTROL).unwrap_or(0);

        let support_bit = 0x01;

        let b_byte: u8 = if !enabled || brightness_idx == 0 {
            0
        } else {
            match brightness_idx {
                1 => 32,
                2 => 64,
                3 => 96,
                4 => 128,
                _ => 96,
            }
        };

        if !enabled || brightness_idx == 0 {
            let new_mode_index = (cur_mode_index & 0x1F) | support_bit;
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_MODE_INDEX, new_mode_index);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, cur_bl_white | 0x10);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_RED, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_GREEN, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_BLUE, 0);
            let next_trigger = cur_trigger_byte & !0x80;
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, next_trigger);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_CONTROL, cur_rgb_ctrl & 0xF8);
            crate::log_info!("ITE_LIGHTING", "A-Cover Logo EC synchronized: OFF (support_bit={})", support_bit);
        } else {
            let (r, g, b) = parse_hex_color(color_hex);

            match effect {
                1 => {
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, cur_trigger_byte & !0x80);
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_CONTROL, cur_rgb_ctrl & 0xF8);

                    let r_ec = ((r as u32 * 50) / 255).min(50) as u8;
                    let g_ec = ((g as u32 * 50) / 255).min(50) as u8;
                    let b_ec = ((b as u32 * 50) / 255).min(50) as u8;

                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_RED, r_ec);
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_GREEN, g_ec);
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_BLUE, b_ec);
                }
                2 => {
                    let next_trigger = cur_trigger_byte & !0x80;
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, next_trigger);

                    // EC supports 7 fixed breathing colors:
                    //   1=White  2=Red  3=Green  4=Blue  5=Yellow  6=Purple  7=Cyan
                    // Map the user's arbitrary RGB to the nearest EC palette entry
                    // using squared Euclidean distance (no sqrt needed for comparison).
                    // Reference EC palette (R, G, B):
                    let palette: &[(u8, u8, u8, u8)] = &[
                        (1, 255, 255, 255), // White
                        (2, 255,   0,   0), // Red
                        (3,   0, 255,   0), // Green
                        (4,   0,   0, 255), // Blue
                        (5, 255, 255,   0), // Yellow
                        (6, 136,   0, 255), // Purple  (OEM: r=136, b=255)
                        (7,   0, 255, 255), // Cyan
                    ];
                    let mut best_code: u8 = 1;
                    let mut best_dist: u32 = u32::MAX;
                    for &(code, pr, pg, pb) in palette {
                        let dr = (r as i32) - (pr as i32);
                        let dg = (g as i32) - (pg as i32);
                        let db = (b as i32) - (pb as i32);
                        let dist = (dr * dr + dg * dg + db * db) as u32;
                        if dist < best_dist {
                            best_dist = dist;
                            best_code = code;
                        }
                    }

                    let new_rgb_ctrl = (cur_rgb_ctrl & 0xF8) | best_code;
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_CONTROL, new_rgb_ctrl);
                    crate::log_info!(
                        "ITE_LIGHTING",
                        "EC Shield Logo Breathing: color=({},{},{}) -> palette_code={} (dist={})",
                        r, g, b, best_code, best_dist
                    );
                }
                _ => {
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_RGB_CONTROL, cur_rgb_ctrl & 0xF8);
                    let next_trigger = cur_trigger_byte | 0x80;
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, next_trigger);
                }
            }

            let new_mode_index = (cur_mode_index & 0x1F) | b_byte | support_bit;
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_MODE_INDEX, new_mode_index);
            let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, cur_bl_white | 0x10);

            crate::log_info!(
                "ITE_LIGHTING",
                "A-Cover Logo EC synchronized: effect={}, brightness={}, b_byte=0x{:02X}, color={}, mode_idx=0x{:02X}",
                effect,
                brightness_idx,
                b_byte,
                color_hex,
                new_mode_index
            );
        }

        if is_supported {
            set_hklm_dword(r"SOFTWARE\OEM\GamingCenter2\Support", "MBALogo", 1);
        }

        true
    }

    /// Set A-Cover Logo
    pub fn apply_logo(&self, enabled: bool, effect: u8, brightness_idx: u8, color_hex: &str) -> bool {
        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);

        // Hardware Constitution Gate:
        if !self.is_logo_allowed() {
            crate::log_warn!("LIGHTING_ARBITER", "apply_logo blocked: Machine Constitution declares has_logo = false. Ghost Logo poisoning eliminated!");
            return false;
        }

        // Hardware Ground-Truth Logo Routing:
        // If this machine does NOT have physical USB Lettering Logo hardware, route to Motherboard EC PWM
        if !Self::probe_usb_logo_supported() && Self::is_ec_logo_supported() {
            return Self::sync_ec_logo(enabled, effect, brightness_idx, color_hex);
        }

        self.refresh_devices();

        // Defensive fallback: if the cached handle is still missing even though the firmware
        // reports a USB lettering logo, open the device on demand instead of silently
        // turning the request into a no-op (which used to be reported to the UI as success).
        if Self::probe_usb_logo_supported() {
            let mut lb = self.lb_dev.lock();
            if lb.is_none() {
                crate::log_warn!(
                    "LIGHTING_ARBITER",
                    "apply_logo: cached 7001 handle missing, opening device on demand"
                );
                *lb = Self::open_hid_device(ITE_VID, ITE_PID_LIGHTBAR_BASE, USAGE_LIGHTBAR);
            }
        }

        let light = LIGHTBAR_BRIGHTNESS_MAP
            .get(brightness_idx as usize)
            .copied()
            .unwrap_or(75);
        let speed = 5;

        let actual_effect = match effect {
            20 => 2,          // Rainbow Breathing (彩虹呼吸 -> 固件呼吸动效码 2)
            13 | 4 => 4,      // Conflict
            12 | 10 | 6 => 6, // Meteor
            9 => 9,           // Marquee
            e => e,
        };

        let is_circular = effect == 20
            || actual_effect == 3
            || actual_effect == 4
            || actual_effect == 9
            || (actual_effect == 2 && color_hex.eq_ignore_ascii_case("rainbow"));
        let color_index = if is_circular { 8 } else { 1 };
        let (r, g, b) = parse_hex_color(color_hex);

        let effect_pkt: [u8; 9] = [
            0,
            8,
            CONTROL_LOGO,
            actual_effect,
            speed,
            light,
            color_index,
            1,
            0, // Save = 0 (RAM preview, protects eFlash against brown-out bricking)
        ];

        let mut usb_success = false;

        // Branch A: USB 048D:7001 (Base Lightbar & Lettering Logo)
        {
            let lb = self.lb_dev.lock();
            if let Some(dev) = lb.as_ref() {
                if !enabled || brightness_idx == 0 {
                    // Save = 0 strictly to avoid touching eFlash during sleep/off
                    let off_pkt: [u8; 9] = [0, 8, CONTROL_LOGO, 255, 1, 50, 0, 0, 0];
                    usb_success = dev.write_feature(&off_pkt) || usb_success;
                } else {
                    // For USB 7001, circular rainbow is generated internally by hardware MCU.
                    // DO NOT flood 7 color-slot packets without delay; write single color slot for static modes only.
                    if !is_circular {
                        let color_pkt: [u8; 9] = [0, 20, BLANK_LOGO, 1, r, g, b, 0, 0];
                        dev.write_feature(&color_pkt);
                    }
                    usb_success = dev.write_feature(&effect_pkt) || usb_success;
                }
            }
        }

        // Branch B: USB 048D:7000 (Hinge & Lettering Logo)
        {
            let hg = self.hinge_dev.lock();
            if let Some(dev) = hg.as_ref() {
                if !enabled || brightness_idx == 0 {
                    // Save = 0 strictly to avoid touching eFlash during sleep/off
                    let off_pkt: [u8; 9] = [0, 8, CONTROL_LOGO, 255, 1, 50, 0, 0, 0];
                    usb_success = dev.write_feature(&off_pkt) || usb_success;
                } else {
                    if !is_circular {
                        let color_pkt: [u8; 9] = [0, 20, BLANK_LOGO, 1, r, g, b, 0, 0];
                        dev.write_feature(&color_pkt);
                    }
                    usb_success = dev.write_feature(&effect_pkt) || usb_success;
                }
            }
        }

        usb_success
    }

    pub fn start_idle_watcher(self: &Arc<Self>) {
        sleep_watcher::start_idle_watcher(self);
    }

    /// Turn off all lighting zones during sleep
    pub fn turn_off_all(&self) {
        crate::driver::ite_streamer::get_global_streamer().turn_off();
        if self.has_four_zone_keyboard_device() {
            let kb = self.kb_dev.lock();
            if let Some(dev) = kb.as_ref() {
                let off_pkt: [u8; 9] = [0, 8, 1, 0, 0, 0, 0, 0, 0];
                dev.write_feature(&off_pkt);
            }
        }
        let contract = self.active_contract();
        if contract.backend == "EC" {
            self.apply_keyboard(false, 1, 0, 1, 1, "#000000");
        }
        self.refresh_devices();
        self.lb_strategy.turn_off();
        if !self.lb_strategy.is_connected() && Self::is_ec_lightbar_supported() {
            lightbar::EcDirectLightbarStrategy.turn_off();
        }
        self.hinge_strategy.turn_off();
        self.strip_strategy.turn_off();

        let has_ec_logo = Self::is_ec_logo_supported();
        if !has_ec_logo {
            let lb = self.lb_dev.lock();
            if let Some(dev) = lb.as_ref() {
                // Save = 0 strictly to protect eFlash against brown-out corruption during sleep
                let off_logo: [u8; 9] = [0, 8, CONTROL_LOGO, 255, 1, 50, 0, 0, 0];
                dev.write_feature(&off_logo);
            }
            drop(lb);
        } else {
            Self::sync_ec_logo(false, 1, 0, "#000000");
        }

        let hg = self.hinge_dev.lock();
        let has_usb_hg = hg.is_some();
        if let Some(dev) = hg.as_ref() {
            // Save = 0 strictly to protect eFlash against brown-out corruption during sleep
            let off_logo: [u8; 9] = [0, 8, CONTROL_LOGO, 255, 1, 50, 0, 0, 0];
            dev.write_feature(&off_logo);
        }
        drop(hg);

        if !has_usb_hg && Self::probe_ec_hinge_supported() {
            Self::sync_ec_hinge(false, 1, 0, "#000000");
        }
        if has_ec_logo {
            Self::sync_ec_logo(false, 1, 0, "#000000");
        }
    }

    /// Restore all lighting zones to their saved preset states
    pub fn restore_all(&self) {
        // Clear EC ADDR_BACKLIGHT_WHITE Bit 1 so hardware rail is active
        if let Some(cur_bl) = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE) {
            if (cur_bl & 0x02) != 0 {
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, cur_bl & !0x02);
            }
        }
        if let Some(cur_trigger) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, cur_trigger | 0x20);
        }
        self.refresh_devices();
        let st = self.state.lock().clone();
        if self.has_keyboard_device() {
            if self.has_perkey_keyboard_device() {
                if st.kb_engine == "hardware" {
                    let hw_effect = if st.kb_effect > 0 && st.kb_effect < 100 {
                        st.kb_effect
                    } else {
                        3
                    };
                    crate::driver::ite_streamer::get_global_streamer().apply_hardware(
                        st.kb_enabled,
                        hw_effect,
                        st.kb_brightness,
                        st.kb_speed,
                        st.kb_direction,
                        &st.kb_color,
                    );
                } else {
                    crate::driver::ite_streamer::get_global_streamer().resume_after_wake();
                }
            } else if self.has_four_zone_keyboard_device() {
                self.apply_four_zone_keyboard(
                    st.kb_enabled,
                    st.kb_effect,
                    st.kb_brightness,
                    st.kb_speed,
                    st.kb_direction,
                    &st.kb_color,
                    Some(&st.kb_zones),
                );
            }
        } else {
            // Restore EC direct keyboard backlight state (SingleZone or MonochromeWhite)
            self.apply_keyboard(
                st.kb_enabled,
                st.kb_effect,
                st.kb_brightness,
                st.kb_speed,
                st.kb_direction,
                &st.kb_color,
            );
        }

        if self.has_base_lightbar_device() {
            if st.lb_enabled {
                self.apply_lightbar(
                    st.lb_enabled,
                    st.lb_effect,
                    st.lb_brightness,
                    st.lb_speed,
                    &st.lb_color,
                );
            } else {
                self.apply_lightbar(
                    false,
                    st.lb_effect,
                    0,
                    st.lb_speed,
                    &st.lb_color,
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        if self.has_hinge_device() {
            if st.hinge_enabled {
                self.apply_hinge(
                    st.hinge_enabled,
                    st.hinge_effect,
                    st.hinge_brightness,
                    st.hinge_speed,
                    &st.hinge_color,
                );
            } else {
                self.apply_hinge(
                    false,
                    st.hinge_effect,
                    0,
                    st.hinge_speed,
                    &st.hinge_color,
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        let has_logo = Self::probe_usb_logo_supported() || Self::is_ec_logo_supported();
        if has_logo {
            if st.logo_enabled {
                self.apply_logo(
                    st.logo_enabled,
                    st.logo_effect,
                    st.logo_brightness,
                    &st.logo_color,
                );
            } else {
                self.apply_logo(
                    false,
                    st.logo_effect,
                    0,
                    &st.logo_color,
                );
            }
        }
    }

    /// Set Lighting Sleep Timeout
    pub fn set_timeout(&self, minutes: u8) -> bool {
        self.refresh_devices();
        {
            let mut st = self.state.lock();
            st.sleep_minutes = minutes;
        }

        self.is_sleeping.store(false, std::sync::atomic::Ordering::SeqCst);
        self.restore_all();
        true
    }

    /// Trigger Full Matrix Startup & Wake Ceremony across all hardware tiers with automatic state restore
    pub fn trigger_welcome(&self) -> bool {
        if IS_WELCOME_ACTIVE.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::Relaxed).is_err() {
            crate::log_info!("LIGHTING", "trigger_welcome: ceremony already in progress, ignoring duplicate call");
            return false;
        }

        self.refresh_devices();

        // 1. 机身前沿海岸灯 (Base Lightbar) & A面全彩文字 LOGO 激活全彩仪式流光 (USB 7001)
        {
            let lb = self.lb_dev.lock();
            if let Some(dev) = lb.as_ref() {
                // 海岸灯下发 8 槽全彩调色盘与波浪流光 (Wave, 3, direction=1, Save=0)
                for (idx, &(cr, cg, cb)) in RAINBOW_COLORS.iter().enumerate() {
                    let slot = (idx + 1) as u8;
                    let color_pkt: [u8; 9] = [0, 20, BLANK_LIGHTBAR, slot, cr, cg, cb, 0, 0];
                    dev.write_feature(&color_pkt);
                }
                let pkt_lb: [u8; 9] = [0, 8, CONTROL_LIGHTBAR, 3, 5, 75, 8, 1, 0];
                dev.write_feature(&pkt_lb);

                // A面文字 LOGO 下发 8 槽全彩调色盘与全彩跑马流水 (Marquee, 9, Save=0)
                for (idx, &(cr, cg, cb)) in RAINBOW_COLORS.iter().enumerate() {
                    let slot = (idx + 1) as u8;
                    let color_pkt: [u8; 9] = [0, 20, BLANK_LOGO, slot, cr, cg, cb, 0, 0];
                    dev.write_feature(&color_pkt);
                }
                let pkt_logo: [u8; 9] = [0, 8, CONTROL_LOGO, 9, 5, 75, 8, 1, 0];
                dev.write_feature(&pkt_logo);
            } else if Self::is_ec_lightbar_supported() {
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_LIGHTBAR_CONTROL_BYTE, 0x80);
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_REDBAR, 0);
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_GREENBAR, 0);
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_BLUEBAR, 0);
                if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
                    let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, v | crate::driver::wmi_ec::TRIGGER_LIGHTBAR);
                }
            }
        }

        // 2. 尾部出风口 (Hinge) & 7000 LOGO (若存在, Save=0)
        {
            let hg = self.hinge_dev.lock();
            if let Some(dev) = hg.as_ref() {
                for (idx, &(cr, cg, cb)) in RAINBOW_COLORS.iter().enumerate() {
                    let slot = (idx + 1) as u8;
                    let color_pkt: [u8; 9] = [0, 20, BLANK_HINGE, slot, cr, cg, cb, 0, 0];
                    dev.write_feature(&color_pkt);
                }
                let pkt_hg: [u8; 9] = [0, 8, CONTROL_HINGE, 3, 5, 75, 8, 1, 0];
                dev.write_feature(&pkt_hg);
            }
        }

        let has_perkey = self.has_perkey_keyboard_device();
        let has_fourzone = self.has_four_zone_keyboard_device();
        let raw_kb_support = crate::driver::wmi_ec::wmi_read_ec(ADDR_SUPPORT_BYTE2).unwrap_or(0);
        let is_single_zone = (raw_kb_support & 0x04) != 0;
        let cur_st = self.state.lock().clone();
        let is_streamer = cur_st.kb_engine == "streamer";

        std::thread::spawn(move || {
            crate::log_info!("LIGHTING", "Startup Ceremony launched across detected hardware tier");

            if has_perkey {
                // T1 旗舰全彩逐键：若为推流引擎，注入 60FPS 绚丽星光烟花流光 (Sparkle 111)；若为硬件 MCU，下发全彩波浪 (Wave 3)
                if is_streamer {
                    let streamer = crate::driver::ite_streamer::get_global_streamer();
                    let welcome_sp = crate::driver::ite_streamer::StreamerParams {
                        enabled: true,
                        effect: 111, // Sparkle / 璀璨星火烟花流光
                        fps: 60,
                        brightness_pct: 100,
                        speed: 5,
                        direction: 0,
                        color_hex: "#6366F1".to_string(),
                    };
                    streamer.update_params(welcome_sp);
                } else {
                    let streamer = crate::driver::ite_streamer::get_global_streamer();
                    streamer.apply_hardware(true, 3, 4, 3, 1, "#6366F1");
                }
                std::thread::sleep(Duration::from_millis(3200));
            } else if has_fourzone {
                // T2 4区 RGB 竞技款：下发 4 区平滑彩虹波浪 (Wave 3)
                let global_mgr = get_global_lighting();
                global_mgr.apply_four_zone_keyboard(true, 3, 4, 3, 1, "#6366F1", None);
                std::thread::sleep(Duration::from_millis(3200));
            } else if is_single_zone {
                // T3 单区 RGB：七彩星芒动态彩虹流光 (触发锁存 Bit 7)
                let cur_bl_white = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, (cur_bl_white & !0x02 & 0x0F) | 0x90);
                let cur_trigger_byte = crate::driver::wmi_ec::wmi_read_ec(ADDR_TRIGGER_BYTE).unwrap_or(0);
                let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, cur_trigger_byte | 0xA0); // Bit 7 彩虹模式 + Bit 5 更新使能
                std::thread::sleep(Duration::from_millis(3000));
            } else {
                // T4 纯单色白光轻薄本：优雅微光渐醒呼吸阶梯 (0 关 -> 1 档微光 450ms -> 2 档柔和 450ms -> 3 档极亮 800ms)
                let cur_bl_white = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);
                let base_bl = cur_bl_white & !0x02 & 0x0F;
                let steps = [
                    (0x30, 450), // 1 档微光
                    (0x50, 450), // 2 档柔和
                    (0x90, 800), // 3 档极亮盛放
                ];
                for &(step, sleep_ms) in &steps {
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, base_bl | step);
                    if let Some(cur_trig) = crate::driver::wmi_ec::wmi_read_ec(ADDR_TRIGGER_BYTE) {
                        let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_TRIGGER_BYTE, cur_trig | 0x20);
                    }
                    std::thread::sleep(Duration::from_millis(sleep_ms));
                }
            }

            // 仪式结束：优雅平滑恢复全域用户日常预设
            let global_mgr = get_global_lighting();
            global_mgr.restore_all();
            IS_WELCOME_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
            crate::log_info!("LIGHTING", "Startup Ceremony completed successfully, user lighting states seamlessly restored");
        });

        true
    }

    /// Query the real-time physical/driver runtime state of all lighting zones (Keyboard, Lightbar, Hinge, Logo)
    pub fn apply_hardware_override_and_reinit(&self, override_cfg: LightingHardwareOverride) -> Result<(), String> {
        // 1. Save new override into config.json
        let mut cfg = crate::system::config::load_config();
        cfg.lighting.hardware_override = override_cfg.clone();
        crate::system::config::save_config(&cfg);

        // 2. Synchronize in-memory shared manager state immediately
        self.state.lock().hardware_override = override_cfg.clone();

        // 3. Teardown: Stop Streamer if running, and invalidate cached devices
        let streamer = crate::driver::ite_streamer::get_global_streamer();
        if streamer.is_running() {
            streamer.stop();
        }
        self.invalidate_devices();

        // 3. Reset conflicting EC registers
        if override_cfg.enabled {
            if override_cfg.keyboard == "single_white" {
                if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE) {
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, val | 0x01);
                }
            } else if override_cfg.keyboard == "single_zone" {
                if let Some(val) = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE) {
                    let _ = crate::driver::wmi_ec::wmi_write_ec(ADDR_BACKLIGHT_WHITE, val & !0x01);
                }
            }
        }

        // 4. Re-refresh devices based on new contract and restore preset effects
        self.refresh_devices();
        self.restore_all();

        crate::log_info!("LIGHTING", "Hardware topology override applied and re-initialized: {:?}", override_cfg);
        Ok(())
    }

    pub fn get_runtime_status(&self) -> LightingRuntimeStatus {
        let is_sleep = self.is_sleeping.load(std::sync::atomic::Ordering::Relaxed);
        let cfg = crate::system::config::load_config();
        let streamer = crate::driver::ite_streamer::get_global_streamer();
        let streamer_running = streamer.is_running();
        let streamer_params = streamer.get_params();

        let contract = self.active_contract();
        let hw_override = cfg.lighting.hardware_override.clone();

        // 1. Keyboard Zone Detection
        let raw_kb_support = crate::driver::wmi_ec::wmi_read_ec(ADDR_SUPPORT_BYTE2).unwrap_or(0);
        let is_single_zone = (raw_kb_support & 0x04) != 0;
        let raw_bl_white = crate::driver::wmi_ec::wmi_read_ec(ADDR_BACKLIGHT_WHITE).unwrap_or(0);
        let is_single_color = (raw_bl_white & 0x01) != 0;
        let is_ec_disabled = (raw_bl_white & 0x02) != 0;

        // Baseline auto detection
        let auto_kb = if Self::probe_four_zone_keyboard_exists() {
            "four_zone".to_string()
        } else if crate::driver::ite_streamer::IteStreamerController::probe_exists() {
            "per_key".to_string()
        } else if is_single_zone {
            "single_zone".to_string()
        } else if is_single_color {
            "single_white".to_string()
        } else {
            "none".to_string()
        };

        let auto_front = if Self::probe_lightbar_device_exists() { "present" } else { "none" }.to_string();
        let auto_amb = if Self::probe_hinge_device_exists() { "vent" } else { "none" }.to_string();
        let auto_logo = if Self::probe_usb_logo_supported() {
            "letter".to_string()
        } else if Self::is_ec_logo_supported() {
            "shield".to_string()
        } else {
            "none".to_string()
        };

        let detected_baseline = LightingHardwareOverride {
            enabled: false,
            keyboard: auto_kb,
            front_bar: auto_front,
            ambient: auto_amb,
            logo: auto_logo,
        };

        let (kb_engine, kb_active, kb_fps) = match contract.keyboard_type.as_str() {
            "PerKey" => {
                if cfg.lighting.kb_engine == "streamer" {
                    let active = !is_sleep && streamer_running && streamer_params.enabled && streamer_params.brightness_pct > 0;
                    ("streamer".to_string(), active, if streamer_running { streamer_params.fps } else { 0 })
                } else {
                    let active = !is_sleep && cfg.lighting.kb_enabled && cfg.lighting.kb_brightness > 0;
                    ("hardware".to_string(), active, 0)
                }
            }
            "4Zone" => {
                let active = !is_sleep && cfg.lighting.kb_enabled && cfg.lighting.kb_brightness > 0;
                ("four_zone".to_string(), active, 0)
            }
            "SingleZone" => {
                let active = !is_sleep && cfg.lighting.kb_enabled && !is_ec_disabled;
                ("single_zone".to_string(), active, 0)
            }
            "MonochromeWhite" | "White" => {
                let active = !is_sleep && cfg.lighting.kb_enabled && !is_ec_disabled;
                ("single_color".to_string(), active, 0)
            }
            _ => ("none".to_string(), false, 0),
        };

        // 2. Base Lightbar Zone Detection
        let lb_online = contract.has_coastline;
        let lb_active = !is_sleep && lb_online && cfg.lighting.lb_enabled && cfg.lighting.lb_brightness > 0;

        // 3. Air Vent / Hinge Lightbar Zone Detection
        let hinge_online = contract.has_air_vent;
        let hinge_active = !is_sleep && hinge_online && cfg.lighting.hinge_enabled && cfg.lighting.hinge_brightness > 0;

        // 4. Logo Zone Detection
        let logo_online = contract.has_logo;
        let logo_active = !is_sleep && logo_online && cfg.lighting.logo_enabled && cfg.lighting.logo_brightness > 0;

        LightingRuntimeStatus {
            keyboard_engine: kb_engine,
            keyboard_online: contract.keyboard_type != "None" && contract.backend != "None",
            keyboard_active: kb_active,
            keyboard_fps: kb_fps,
            lightbar_online: lb_online,
            lightbar_active: lb_active,
            hinge_online,
            hinge_active,
            logo_online,
            logo_active,
            is_sleeping: is_sleep,
            hardware_override: hw_override,
            detected_baseline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lighting_contract_assertions() {
        let mgr = IteLightingManager::new();
        let contract = mgr.active_contract();
        assert_eq!(mgr.is_coastline_allowed(), contract.has_coastline);
        assert_eq!(mgr.is_air_vent_allowed(), contract.has_air_vent);
        assert_eq!(mgr.is_logo_allowed(), contract.has_logo);
        assert_eq!(mgr.is_keyboard_allowed(), contract.keyboard_type != "None" && contract.backend != "None");
    }

    #[test]
    fn test_ghost_hardware_physical_isolation() {
        let mgr = IteLightingManager::new();
        let contract = mgr.active_contract();
        if !contract.has_logo {
            assert!(!mgr.apply_logo(true, 1, 4, "#FFFFFF"));
        }
        if !contract.has_coastline {
            assert!(!mgr.apply_lightbar(true, 1, 4, 3, "#00FF00"));
        }
        if !contract.has_air_vent {
            assert!(!mgr.apply_hinge(true, 1, 4, 3, "#0000FF"));
        }
    }

    #[test]
    fn test_single_zone_and_perkey_contract_routing() {
        // Verify probe_lighting normalizes SingleZone_RGB to SingleZone (or live PerKey streamer if physically present)
        let contract_sz = crate::system::model_engine::TruthReconstructor::probe_lighting("SingleZone_RGB");
        assert!(contract_sz.keyboard_type == "SingleZone" || contract_sz.keyboard_type == "PerKey");

        let contract_sz2 = crate::system::model_engine::TruthReconstructor::probe_lighting("SingleZone");
        assert!(contract_sz2.keyboard_type == "SingleZone" || contract_sz2.keyboard_type == "PerKey");
    }
}

