// OpenRevo - Universal Shortcut Bus Types & Key Combo Parser
// Defines shortcut bindings, parsed virtual key codes, modifier masks, and serializable actions.

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum ShortcutAction {
    // 1. Hardware Actions
    ToggleFanBoost,
    CyclePowerMode,
    SetPowerMode(u8),
    ToggleRefreshRate,
    ToggleTouchpad,
    ToggleWifi,
    ToggleWinLock,
    ToggleFnLock,
    CycleKeyboardBrightness,
    TriggerOsdPreview,

    // 2. Windows System Actions
    VolumeMute,
    VolumeUp,
    VolumeDown,
    MonitorOff,
    LockScreen,
    LaunchApp {
        path: String,
        #[serde(default)]
        args: Vec<String>,
    },

    // 3. Custom Cross-Plugin Event Bus
    PluginEvent {
        event_name: String,
        payload: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub id: String,
    pub name: String,
    pub combo: String,
    pub action: ShortcutAction,
    #[serde(default = "default_true")]
    pub osd: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub swallow: bool,
    #[serde(default)]
    pub plugin_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParsedKeyCombo {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vk_code: u16,
}

impl ParsedKeyCombo {
    /// Parse a human-readable combo string such as "Ctrl+Alt+F", "Win+Shift+R", "F11", "Ctrl+1"
    pub fn parse(combo: &str) -> Option<Self> {
        let parts: Vec<&str> = combo.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return None;
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut win = false;
        let mut primary_key_vk: Option<u16> = None;

        for part in parts {
            let p_upper = part.to_ascii_uppercase();
            match p_upper.as_str() {
                "CTRL" | "CONTROL" => ctrl = true,
                "ALT" | "MENU" => alt = true,
                "SHIFT" => shift = true,
                "WIN" | "WINDOWS" | "SUPER" => win = true,
                _ => {
                    if let Some(vk) = map_key_to_vk(&p_upper) {
                        primary_key_vk = Some(vk);
                    }
                }
            }
        }

        primary_key_vk.map(|vk_code| Self {
            ctrl,
            alt,
            shift,
            win,
            vk_code,
        })
    }

    /// Check if the parsed combo matches the active keydown event and modifier states
    pub fn matches(&self, vk: u16, is_ctrl_down: bool, is_alt_down: bool, is_shift_down: bool, is_win_down: bool) -> bool {
        self.vk_code == vk
            && self.ctrl == is_ctrl_down
            && self.alt == is_alt_down
            && self.shift == is_shift_down
            && self.win == is_win_down
    }
}

/// Map human-readable key name to Win32 Virtual-Key Code
fn map_key_to_vk(key: &str) -> Option<u16> {
    // Single character A-Z, 0-9
    if key.len() == 1 {
        let c = key.chars().next()?;
        if c.is_ascii_uppercase() {
            return Some(c as u16);
        }
        if c.is_ascii_digit() {
            return Some(c as u16);
        }
    }

    // Function keys F1-F24
    if key.starts_with('F') {
        if let Ok(num) = key[1..].parse::<u16>() {
            if (1..=24).contains(&num) {
                // VK_F1 is 0x70
                return Some(0x70 + (num - 1));
            }
        }
    }

    // Special keys
    match key {
        "SPACE" => Some(0x20),
        "TAB" => Some(0x09),
        "RETURN" | "ENTER" => Some(0x0D),
        "ESCAPE" | "ESC" => Some(0x1B),
        "BACK" | "BACKSPACE" => Some(0x08),
        "DELETE" | "DEL" => Some(0x2E),
        "INSERT" | "INS" => Some(0x2D),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGEUP" | "PRIOR" => Some(0x21),
        "PAGEDOWN" | "NEXT" => Some(0x22),
        "LEFT" => Some(0x25),
        "UP" => Some(0x26),
        "RIGHT" => Some(0x27),
        "DOWN" => Some(0x28),
        "CAPS" | "CAPSLOCK" => Some(0x14),
        "NUMLOCK" => Some(0x90),
        "SCROLL" | "SCROLLLOCK" => Some(0x91),
        "PRINTSCREEN" | "SNAPSHOT" => Some(0x2C),
        "PAUSE" => Some(0x13),
        // OEM punctuation
        "-" | "MINUS" => Some(0xBD),
        "=" | "EQUAL" => Some(0xBB),
        "[" | "LBRACKET" => Some(0xDB),
        "]" | "RBRACKET" => Some(0xDD),
        ";" | "SEMICOLON" => Some(0xBA),
        "'" | "QUOTE" => Some(0xDE),
        "," | "COMMA" => Some(0xBC),
        "." | "PERIOD" => Some(0xBE),
        "/" | "SLASH" => Some(0xBF),
        "\\" | "BACKSLASH" => Some(0xDC),
        "`" | "GRAVE" | "TILDE" => Some(0xC0),
        // Media Keys
        "VOLUMEMUTE" | "MUTE" => Some(0xAD),
        "VOLUMEDOWN" => Some(0xAE),
        "VOLUMEUP" => Some(0xAF),
        "MEDIANEXT" => Some(0xB0),
        "MEDIAPREV" => Some(0xB1),
        "MEDIASTOP" => Some(0xB2),
        "MEDIAPLAYPAUSE" => Some(0xB3),
        _ => None,
    }
}
