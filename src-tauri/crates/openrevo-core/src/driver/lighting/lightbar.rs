// OpenRevo - Coastline Lightbar Hardware Abstraction & Strategy Pattern
// Isolates USB 7001 (Front/Base), USB 7000 (Air Vent/Hinge), and Motherboard EC backends
// to prevent cross-model hardware packet collisions and eliminate hot-enumeration I/O jitter.

use parking_lot::Mutex;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use super::device::IteLightingDevice;
use super::types::{parse_hex_color, LIGHTBAR_BRIGHTNESS_MAP, SPEED_MAP};
use crate::hal::vault::ite::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightbarBackendType {
    Usb7001,
    Usb7000,
    EcDirect,
    Ite8291Strip,
    None,
}

pub trait LightbarStrategy: Send + Sync {
    fn apply_lightbar(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        color_hex: &str,
    ) -> bool;

    fn turn_off(&self) -> bool;
    fn is_connected(&self) -> bool;
    fn backend_type(&self) -> LightbarBackendType;
}

// --- USB 7001 Implementation (Base / Front Lightbar, e.g. Yaoshi 16 Super) ---

pub struct Usb7001LightbarStrategy {
    device: Arc<Mutex<Option<IteLightingDevice>>>,
}

impl Usb7001LightbarStrategy {
    pub fn new(device: Arc<Mutex<Option<IteLightingDevice>>>) -> Self {
        Self { device }
    }
}

impl LightbarStrategy for Usb7001LightbarStrategy {
    fn apply_lightbar(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        color_hex: &str,
    ) -> bool {
        let dev_guard = self.device.lock();
        let dev = match dev_guard.as_ref() {
            Some(d) => d,
            None => return false,
        };

        if !enabled || brightness_idx == 0 {
            // Save = 0 to prevent brown-out eFlash corruption during power state transitions
            let off_pkt: [u8; 9] = [0, 8, CONTROL_LIGHTBAR, 255, 1, 50, 0, 0, 0];
            return dev.write_feature(&off_pkt);
        }

        let speed = SPEED_MAP.get(speed_idx as usize).copied().unwrap_or(5);
        let light = LIGHTBAR_BRIGHTNESS_MAP
            .get(brightness_idx as usize)
            .copied()
            .unwrap_or(75);

        let actual_effect = match effect {
            20 => 2,          // Rainbow Breathing (彩虹呼吸 -> 固件呼吸动效码 2)
            13 | 4 => 4,      // Conflict / Collision / Devour
            12 | 6 => 6,      // Meteor
            10 => 6,          // Raindrop / 璀璨闪烁 -> 7001 灯条硬件无点阵，官方映射为流星 (6)，传 10 导致固件熄灭
            9 => 3,           // Marquee / 跑马流水 -> 7001 仅 A面Logo支持9，灯条通道平滑转为全彩波浪 (3) 防止黑屏
            14 => 3,          // Aurora is 8291 Per-Key only. Fallback to Wave to prevent blackout on 7001!
            e => e,
        };

        let is_circular = effect == 20
            || actual_effect == 3
            || actual_effect == 4
            || actual_effect == 14
            || (actual_effect == 2 && color_hex.eq_ignore_ascii_case("rainbow"));
        let color_index = if is_circular { 8 } else { 1 };
        let (r, g, b) = parse_hex_color(color_hex);

        let direction: u8 = match actual_effect {
            3 => 1,
            _ => 0,
        };

        // Official LM_ITE_RGB.cs:3485: For MEZone_Lighbar4 (7001), do NOT send 7 color-slot packets
        // during circular modes. 7001 hardware has internal rainbow hues.
        // Flooding 7 USB packets without delay overruns the 7001 USB FIFO and causes blackouts!
        if !is_circular {
            let color_pkt: [u8; 9] = [0, 20, BLANK_LIGHTBAR, 1, r, g, b, 0, 0];
            dev.write_feature(&color_pkt);
            sleep(Duration::from_millis(15));
        }

        let effect_pkt: [u8; 9] = [
            0,
            8,
            CONTROL_LIGHTBAR,
            actual_effect,
            speed,
            light,
            color_index,
            direction,
            0,
        ];
        let success = dev.write_feature(&effect_pkt);
        if actual_effect == 1 {
            sleep(Duration::from_millis(100));
            dev.write_feature(&effect_pkt);
        }
        success
    }

    fn turn_off(&self) -> bool {
        let dev_guard = self.device.lock();
        if let Some(dev) = dev_guard.as_ref() {
            // Save = 0 strictly to avoid touching eFlash during sleep/shutdown
            let off_lb: [u8; 9] = [0, 8, CONTROL_LIGHTBAR, 255, 1, 50, 0, 0, 0];
            dev.write_feature(&off_lb)
        } else {
            false
        }
    }

    fn is_connected(&self) -> bool {
        self.device.lock().is_some()
    }

    fn backend_type(&self) -> LightbarBackendType {
        LightbarBackendType::Usb7001
    }
}

// --- USB 7000 Implementation (Air Vent / Hinge Coastline Lightbar, e.g. Kuangshi 16) ---

pub struct Usb7000LightbarStrategy {
    device: Arc<Mutex<Option<IteLightingDevice>>>,
}

impl Usb7000LightbarStrategy {
    pub fn new(device: Arc<Mutex<Option<IteLightingDevice>>>) -> Self {
        Self { device }
    }
}

impl LightbarStrategy for Usb7000LightbarStrategy {
    fn apply_lightbar(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        color_hex: &str,
    ) -> bool {
        let dev_guard = self.device.lock();
        let dev = match dev_guard.as_ref() {
            Some(d) => d,
            None => return false,
        };

        if !enabled || brightness_idx == 0 {
            // Save = 0 to prevent brown-out eFlash corruption
            let off_pkt: [u8; 9] = [0, 8, CONTROL_HINGE, 255, 1, 50, 0, 0, 0];
            return dev.write_feature(&off_pkt);
        }

        // Send Power-On wake report (Command 1, Status 0 = ON) to ensure MCU is awake
        let pwr_on: [u8; 9] = [0, 8, 1, 0, 0, 0, 0, 0, 0];
        let _ = dev.write_feature(&pwr_on);

        let speed = SPEED_MAP.get(speed_idx as usize).copied().unwrap_or(5);
        let light = LIGHTBAR_BRIGHTNESS_MAP
            .get(brightness_idx as usize)
            .copied()
            .unwrap_or(75);

        let actual_effect = match effect {
            20 => 2,          // Rainbow Breathing
            13 | 4 => 4,      // Conflict / Devour
            12 | 6 => 6,      // Meteor
            10 => 6,          // Raindrop / 璀璨闪烁 -> 映射为流星 (6)
            9 => 3,           // Marquee / 跑马流水 -> 平滑转为波浪 (3)
            14 => 3,          // Aurora fallback to Wave
            e => e,
        };

        let is_circular = effect == 20
            || actual_effect == 3
            || actual_effect == 4
            || actual_effect == 14
            || (actual_effect == 2 && color_hex.eq_ignore_ascii_case("rainbow"));
        let color_index = if is_circular { 8 } else { 1 };
        let (r, g, b) = parse_hex_color(color_hex);

        if !is_circular {
            // Save = 0 for RAM preview
            let color_pkt: [u8; 9] = [0, 20, BLANK_HINGE, 1, r, g, b, 0, 0];
            dev.write_feature(&color_pkt);
            sleep(Duration::from_millis(15));
        }

        let effect_pkt: [u8; 9] = [
            0,
            8,
            CONTROL_HINGE,
            actual_effect,
            speed,
            light,
            color_index,
            1, // direction = 1
            0, // save = 0 (RAM execution, protects eFlash)
        ];

        let success = dev.write_feature(&effect_pkt);

        if actual_effect == 1 {
            sleep(Duration::from_millis(200));
            dev.write_feature(&effect_pkt);
        }

        success
    }

    fn turn_off(&self) -> bool {
        let dev_guard = self.device.lock();
        if let Some(dev) = dev_guard.as_ref() {
            // Save = 0 strictly to avoid touching eFlash during sleep/shutdown
            let off_hg: [u8; 9] = [0, 8, CONTROL_HINGE, 255, 1, 50, 0, 0, 0];
            dev.write_feature(&off_hg)
        } else {
            false
        }
    }

    fn is_connected(&self) -> bool {
        self.device.lock().is_some()
    }

    fn backend_type(&self) -> LightbarBackendType {
        LightbarBackendType::Usb7000
    }
}

// --- Motherboard EC Direct Strategy (when no USB MCU is present) ---

pub struct EcDirectLightbarStrategy;

impl EcDirectLightbarStrategy {
    pub fn probe_supported() -> bool {
        // 1. If physical USB ITE controllers (7001 Base or 7000 Hinge) are present,
        // USB ITE takes absolute precedence. Legacy motherboard EC lightbar registers are inactive stubs.
        if crate::driver::ite_lighting::IteLightingManager::open_hid_device(
            crate::hal::vault::ite::ITE_VID,
            crate::hal::vault::ite::ITE_PID_LIGHTBAR_BASE,
            0,
        ).is_some() || crate::driver::ite_lighting::IteLightingManager::open_hid_device(
            crate::hal::vault::ite::ITE_VID,
            crate::hal::vault::ite::ITE_PID_LIGHTBAR_HINGE,
            0,
        ).is_some() {
            return false;
        }

        // 2. Motherboard EC RAM Ground-Truth capability flags (only for legacy pre-USB platforms):
        let support_byte1 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_SUPPORT_BYTE1).unwrap_or(0);
        let support_byte3 = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_SUPPORT_BYTE3).unwrap_or(0);

        (support_byte3 & crate::driver::wmi_ec::MASK_LIGHTBAR_EC_SUPPORT) != 0
            || (support_byte1 & crate::driver::wmi_ec::MASK_LIGHTBAR_SUPPORT) != 0
    }
}

impl LightbarStrategy for EcDirectLightbarStrategy {
    fn apply_lightbar(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        _speed_idx: u8,
        color_hex: &str,
    ) -> bool {
        if !Self::probe_supported() {
            return false;
        }

        if !enabled || brightness_idx == 0 {
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_LIGHTBAR_CONTROL_BYTE, 0x08);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_REDBAR, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_GREENBAR, 0);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_BLUEBAR, 0);
            if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, v | crate::driver::wmi_ec::TRIGGER_LIGHTBAR);
            }
            crate::log_info!("ITE_LIGHTING", "Air Vent EC Lightbar synchronized: OFF");
        } else {
            let (r, g, b) = parse_hex_color(color_hex);
            let ctrl_byte = match effect {
                1 => 0x08, // Static
                2 => 0x00, // Breathing
                4 => 0x88, // Collision / Conflict
                _ => 0x80, // Dynamic stream / Rainbow / Wave / Meteor
            };

            let (r_val, g_val, b_val) = if effect >= 3 {
                (0, 0, 0)
            } else {
                let max_pwm = match brightness_idx {
                    1 => 15u32,
                    2 => 32u32,
                    _ => 50u32,
                };
                let r_v = ((r as u32 * max_pwm) / 255).min(max_pwm) as u8;
                let g_v = ((g as u32 * max_pwm) / 255).min(max_pwm) as u8;
                let b_v = ((b as u32 * max_pwm) / 255).min(max_pwm) as u8;
                (r_v, g_v, b_v)
            };

            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_LIGHTBAR_CONTROL_BYTE, ctrl_byte);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_REDBAR, r_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_GREENBAR, g_val);
            let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_BLUEBAR, b_val);
            if let Some(v) = crate::driver::wmi_ec::wmi_read_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE) {
                let _ = crate::driver::wmi_ec::wmi_write_ec(crate::driver::wmi_ec::ADDR_TRIGGER_BYTE, v | crate::driver::wmi_ec::TRIGGER_LIGHTBAR);
            }
            crate::log_info!("ITE_LIGHTING", "Air Vent EC Lightbar synchronized: ctrl=0x{:02X}, rgb=({},{},{})", ctrl_byte, r_val, g_val, b_val);
        }
        true
    }

    fn turn_off(&self) -> bool {
        if Self::probe_supported() {
            self.apply_lightbar(false, 1, 0, 0, "#000000")
        } else {
            false
        }
    }

    fn is_connected(&self) -> bool {
        Self::probe_supported()
    }

    fn backend_type(&self) -> LightbarBackendType {
        LightbarBackendType::EcDirect
    }
}

// --- Null Strategy (Fallback when no lightbar is detected) ---

// --- ITE 8291 Ambient Strip Implementation (Companion 048D:6008, e.g. 30-Series Moulds) ---
// Ground-Truth Protocol verified from official GamingCenter2 registry & hardware logs:
// Effect codes: 1=Static, 2=Breath, 3=Wave, 5=Rainbow, 13=Ripple, 19=Colorful, 32=Raindrop, 33=Wings
// Dynamic multi-color modes drive official 7-color wheel into slots 1..=7.

const OFFICIAL_PALETTE_7: [(u8, u8, u8); 7] = [
    (255, 0, 0),     // #FF0000 Red
    (255, 165, 0),   // #FFA500 Orange
    (255, 255, 0),   // #FFFF00 Yellow
    (0, 255, 0),     // #00FF00 Green
    (0, 0, 255),     // #0000FF Blue
    (0, 255, 255),   // #00FFFF Cyan
    (139, 0, 255),   // #8B00FF Purple
];

pub struct Ite8291StripLightbarStrategy {
    device: Arc<Mutex<Option<IteLightingDevice>>>,
}

impl Ite8291StripLightbarStrategy {
    pub fn new(device: Arc<Mutex<Option<IteLightingDevice>>>) -> Self {
        Self { device }
    }
}

impl LightbarStrategy for Ite8291StripLightbarStrategy {
    fn apply_lightbar(
        &self,
        enabled: bool,
        effect: u8,
        brightness_idx: u8,
        speed_idx: u8,
        color_hex: &str,
    ) -> bool {
        let dev_guard = self.device.lock();
        let dev = match dev_guard.as_ref() {
            Some(d) => d,
            None => return false,
        };

        if !enabled || brightness_idx == 0 {
            // Save = 0 strictly to avoid brown-out eFlash corruption
            // Flush all 7 color slots to black before shutdown to eliminate residual ghost frames
            for slot in 1..=7 {
                let black_pkt: [u8; 9] = [0, 20, BLANK_KEYBOARD, slot, 0, 0, 0, 0, 0];
                dev.write_feature(&black_pkt);
            }
            let off_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 255, 1, 50, 0, 0, 0];
            return dev.write_feature(&off_pkt);
        }

        let speed = SPEED_MAP.get(speed_idx as usize).copied().unwrap_or(5);
        let light = LIGHTBAR_BRIGHTNESS_MAP
            .get(brightness_idx as usize)
            .copied()
            .unwrap_or(75);

        // Hardware-grounded effect ID mapping for 8291 companion MCU (derived from GamingCenter2 reg):
        let actual_effect = match effect {
            1 => 1,           // Static
            2 => 2,           // Breath
            3 => 3,           // Wave
            20 | 5 => 5,      // Rainbow Cycle
            10 | 32 => 32,    // Raindrop / Sparkle
            13 => 13,         // Ripple / Flow
            19 => 19,         // Colorful
            4 | 33 => 33,     // Wings / Collision
            6 | 12 => 13,     // Meteor -> Ripple
            9 => 5,           // Marquee -> Rainbow cycle
            e => e,
        };

        let is_circular = matches!(actual_effect, 3 | 5 | 13 | 19 | 32 | 33)
            || color_hex.eq_ignore_ascii_case("rainbow");
        let color_index = if is_circular { 8 } else { 1 };
        let (r, g, b) = parse_hex_color(color_hex);
        let direction: u8 = match actual_effect {
            3 | 5 => 1,
            _ => 0,
        };

        if !is_circular {
            // Static / Breath: Fill all 7 hardware slots with active color to ensure uniform illumination
            for slot in 1..=7 {
                let color_pkt: [u8; 9] = [0, 20, BLANK_KEYBOARD, slot, r, g, b, 0, 0];
                dev.write_feature(&color_pkt);
            }
            sleep(Duration::from_millis(15));
        } else {
            // Dynamic multi-color: Program official 7-color spectrum palette into slots 1..=7
            for (i, &(pr, pg, pb)) in OFFICIAL_PALETTE_7.iter().enumerate() {
                let slot = (i + 1) as u8;
                let color_pkt: [u8; 9] = [0, 20, BLANK_KEYBOARD, slot, pr, pg, pb, 0, 0];
                dev.write_feature(&color_pkt);
            }
            sleep(Duration::from_millis(15));
        }

        let effect_pkt: [u8; 9] = [
            0,
            8,
            CONTROL_KEYBOARD,
            actual_effect,
            speed,
            light,
            color_index,
            direction,
            0,
        ];
        dev.write_feature(&effect_pkt)
    }

    fn turn_off(&self) -> bool {
        let dev_guard = self.device.lock();
        if let Some(dev) = dev_guard.as_ref() {
            for slot in 1..=7 {
                let black_pkt: [u8; 9] = [0, 20, BLANK_KEYBOARD, slot, 0, 0, 0, 0, 0];
                dev.write_feature(&black_pkt);
            }
            let off_pkt: [u8; 9] = [0, 8, CONTROL_KEYBOARD, 255, 1, 50, 0, 0, 0];
            dev.write_feature(&off_pkt)
        } else {
            false
        }
    }

    fn is_connected(&self) -> bool {
        self.device.lock().is_some()
    }

    fn backend_type(&self) -> LightbarBackendType {
        LightbarBackendType::Ite8291Strip
    }
}

pub struct NullLightbarStrategy;

impl LightbarStrategy for NullLightbarStrategy {
    fn apply_lightbar(
        &self,
        _enabled: bool,
        _effect: u8,
        _brightness_idx: u8,
        _speed_idx: u8,
        _color_hex: &str,
    ) -> bool {
        false
    }

    fn turn_off(&self) -> bool {
        false
    }

    fn is_connected(&self) -> bool {
        false
    }

    fn backend_type(&self) -> LightbarBackendType {
        LightbarBackendType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lightbar_strategy_types() {
        let null_strat = NullLightbarStrategy;
        assert_eq!(null_strat.backend_type(), LightbarBackendType::None);
        assert!(!null_strat.is_connected());
        assert!(!null_strat.apply_lightbar(true, 1, 3, 2, "#00FFFF"));
        assert!(!null_strat.turn_off());

        let dev_slot = Arc::new(Mutex::new(None));
        let usb7001_strat = Usb7001LightbarStrategy::new(Arc::clone(&dev_slot));
        assert_eq!(usb7001_strat.backend_type(), LightbarBackendType::Usb7001);
        assert!(!usb7001_strat.is_connected());
        assert!(!usb7001_strat.apply_lightbar(true, 1, 3, 2, "#00FFFF"));

        let usb7000_strat = Usb7000LightbarStrategy::new(Arc::clone(&dev_slot));
        assert_eq!(usb7000_strat.backend_type(), LightbarBackendType::Usb7000);
        assert!(!usb7000_strat.is_connected());
        assert!(!usb7000_strat.apply_lightbar(true, 1, 3, 2, "#00FFFF"));

        let ec_strat = EcDirectLightbarStrategy;
        assert_eq!(ec_strat.backend_type(), LightbarBackendType::EcDirect);

        let strip_strat = Ite8291StripLightbarStrategy::new(Arc::clone(&dev_slot));
        assert_eq!(strip_strat.backend_type(), LightbarBackendType::Ite8291Strip);
        assert!(!strip_strat.is_connected());
        assert!(!strip_strat.apply_lightbar(true, 1, 3, 2, "#00FFFF"));
        assert!(!strip_strat.turn_off());
    }
}
