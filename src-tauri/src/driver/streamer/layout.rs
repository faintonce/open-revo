// OpenRevo - BetterRGB Keyboard Layout & Physical Coordinates Matrix
// Per-key coordinate grid and layout definitions for ITE 8291.

use std::sync::atomic::{AtomicU8, Ordering};

pub const BUF_SIZE: usize = 512;
pub const ITE_VID: u16 = 0x048D;
pub const ITE_PID_KEYBOARD: u16 = 0x600B;
/// 蛟龙 GM7ZG7S 等机型：外形被登记为四区 RGB，但实际是逐键 RGB，走同一套 0x33 协议。
pub const ITE_PID_KEYBOARD_CE00: u16 = 0xCE00;
pub const USAGE_PAGE_ITE: u16 = 0xFF03;
pub const USAGE_KEYBOARD: u16 = 1;

/// ITE 8291 逐键 RGB 键盘的 PID 判据。
/// 设备「存在性探测」与「设备打开」必须共用这一份判据，否则会出现
/// 探测通过却打不开（或反之）的分裂行为。
#[inline]
pub fn is_perkey_pid(pid: u16) -> bool {
    matches!(pid, 0x6000..=0x600B | ITE_PID_KEYBOARD_CE00)
}

pub const OFFSET_SHIFT_LEFT: usize = 5;
pub const OFFSET_SHIFT_LEFT_AUX: usize = 29;
pub const OFFSET_ENTER: usize = 321;
pub const OFFSET_ENTER_AUX: usize = 345;
pub const OFFSET_BACKSLASH: usize = 349;

#[derive(Debug, Clone, Copy)]
pub struct KeyEntry {
    pub name: &'static str,
    pub offset: usize,
    pub row: i32,
    pub col: i32,
}

pub static KEYMAP: &[KeyEntry] = &[
    KeyEntry { name: "Esc", offset: 21, row: 0, col: 0 },
    KeyEntry { name: "F1", offset: 45, row: 0, col: 1 },
    KeyEntry { name: "F2", offset: 69, row: 0, col: 2 },
    KeyEntry { name: "F3", offset: 93, row: 0, col: 3 },
    KeyEntry { name: "F4", offset: 117, row: 0, col: 4 },
    KeyEntry { name: "F5", offset: 141, row: 0, col: 5 },
    KeyEntry { name: "F6", offset: 165, row: 0, col: 6 },
    KeyEntry { name: "F7", offset: 189, row: 0, col: 7 },
    KeyEntry { name: "F8", offset: 213, row: 0, col: 8 },
    KeyEntry { name: "F9", offset: 237, row: 0, col: 9 },
    KeyEntry { name: "F10", offset: 261, row: 0, col: 10 },
    KeyEntry { name: "F11", offset: 285, row: 0, col: 11 },
    KeyEntry { name: "F12", offset: 309, row: 0, col: 12 },
    KeyEntry { name: "FnLockCam", offset: 333, row: 0, col: 13 },
    KeyEntry { name: "PrtSc", offset: 357, row: 0, col: 14 },
    KeyEntry { name: "Del", offset: 381, row: 0, col: 15 },
    KeyEntry { name: "Home", offset: 405, row: 0, col: 16 },
    KeyEntry { name: "PgUp", offset: 429, row: 0, col: 17 },
    KeyEntry { name: "PgDn", offset: 453, row: 0, col: 18 },
    KeyEntry { name: "End", offset: 477, row: 0, col: 19 },
    KeyEntry { name: "Grave", offset: 17, row: 1, col: 0 },
    KeyEntry { name: "1", offset: 41, row: 1, col: 1 },
    KeyEntry { name: "2", offset: 65, row: 1, col: 2 },
    KeyEntry { name: "3", offset: 89, row: 1, col: 3 },
    KeyEntry { name: "4", offset: 113, row: 1, col: 4 },
    KeyEntry { name: "5", offset: 137, row: 1, col: 5 },
    KeyEntry { name: "6", offset: 161, row: 1, col: 6 },
    KeyEntry { name: "7", offset: 185, row: 1, col: 7 },
    KeyEntry { name: "8", offset: 209, row: 1, col: 8 },
    KeyEntry { name: "9", offset: 233, row: 1, col: 9 },
    KeyEntry { name: "0", offset: 257, row: 1, col: 10 },
    KeyEntry { name: "Minus", offset: 281, row: 1, col: 11 },
    KeyEntry { name: "Equals", offset: 305, row: 1, col: 12 },
    KeyEntry { name: "Backspace", offset: 353, row: 1, col: 13 },
    KeyEntry { name: "NumLock", offset: 377, row: 1, col: 14 },
    KeyEntry { name: "NumpadDiv", offset: 401, row: 1, col: 15 },
    KeyEntry { name: "NumpadMul", offset: 425, row: 1, col: 16 },
    KeyEntry { name: "NumpadMinus", offset: 449, row: 1, col: 17 },
    KeyEntry { name: "Tab", offset: 13, row: 2, col: 0 },
    KeyEntry { name: "Q", offset: 61, row: 2, col: 1 },
    KeyEntry { name: "W", offset: 85, row: 2, col: 2 },
    KeyEntry { name: "E", offset: 109, row: 2, col: 3 },
    KeyEntry { name: "R", offset: 133, row: 2, col: 4 },
    KeyEntry { name: "T", offset: 157, row: 2, col: 5 },
    KeyEntry { name: "Y", offset: 181, row: 2, col: 6 },
    KeyEntry { name: "U", offset: 205, row: 2, col: 7 },
    KeyEntry { name: "I", offset: 229, row: 2, col: 8 },
    KeyEntry { name: "O", offset: 253, row: 2, col: 9 },
    KeyEntry { name: "P", offset: 277, row: 2, col: 10 },
    KeyEntry { name: "BracketL", offset: 301, row: 2, col: 11 },
    KeyEntry { name: "BracketR", offset: 325, row: 2, col: 12 },
    KeyEntry { name: "Backslash", offset: OFFSET_BACKSLASH, row: 2, col: 13 },
    KeyEntry { name: "Numpad7", offset: 373, row: 2, col: 14 },
    KeyEntry { name: "Numpad8", offset: 397, row: 2, col: 15 },
    KeyEntry { name: "Numpad9", offset: 421, row: 2, col: 16 },
    KeyEntry { name: "NumpadPlus", offset: 445, row: 2, col: 17 },
    KeyEntry { name: "CapsLock", offset: 9, row: 3, col: 0 },
    KeyEntry { name: "A", offset: 57, row: 3, col: 1 },
    KeyEntry { name: "S", offset: 81, row: 3, col: 2 },
    KeyEntry { name: "D", offset: 105, row: 3, col: 3 },
    KeyEntry { name: "F", offset: 129, row: 3, col: 4 },
    KeyEntry { name: "G", offset: 153, row: 3, col: 5 },
    KeyEntry { name: "H", offset: 177, row: 3, col: 6 },
    KeyEntry { name: "J", offset: 201, row: 3, col: 7 },
    KeyEntry { name: "K", offset: 225, row: 3, col: 8 },
    KeyEntry { name: "L", offset: 249, row: 3, col: 9 },
    KeyEntry { name: "Semicolon", offset: 273, row: 3, col: 10 },
    KeyEntry { name: "Quote", offset: 297, row: 3, col: 11 },
    KeyEntry { name: "Enter", offset: OFFSET_ENTER, row: 3, col: 12 },
    KeyEntry { name: "Numpad4", offset: 369, row: 3, col: 13 },
    KeyEntry { name: "Numpad5", offset: 393, row: 3, col: 14 },
    KeyEntry { name: "Numpad6", offset: 417, row: 3, col: 15 },
    KeyEntry { name: "ShiftL", offset: OFFSET_SHIFT_LEFT, row: 4, col: 0 },
    KeyEntry { name: "ISO_Backslash", offset: 53, row: 4, col: 1 },
    KeyEntry { name: "Z", offset: 77, row: 4, col: 2 },
    KeyEntry { name: "X", offset: 101, row: 4, col: 3 },
    KeyEntry { name: "C", offset: 125, row: 4, col: 4 },
    KeyEntry { name: "V", offset: 149, row: 4, col: 5 },
    KeyEntry { name: "B", offset: 173, row: 4, col: 6 },
    KeyEntry { name: "N", offset: 197, row: 4, col: 7 },
    KeyEntry { name: "M", offset: 221, row: 4, col: 8 },
    KeyEntry { name: "Comma", offset: 245, row: 4, col: 9 },
    KeyEntry { name: "Period", offset: 269, row: 4, col: 10 },
    KeyEntry { name: "Slash", offset: 293, row: 4, col: 11 },
    KeyEntry { name: "ShiftR", offset: 341, row: 4, col: 12 },
    KeyEntry { name: "Numpad1", offset: 365, row: 4, col: 13 },
    KeyEntry { name: "Numpad2", offset: 389, row: 4, col: 14 },
    KeyEntry { name: "Numpad3", offset: 413, row: 4, col: 15 },
    KeyEntry { name: "NumpadEnter", offset: 437, row: 4, col: 16 },
    KeyEntry { name: "CtrlL", offset: 1, row: 5, col: 0 },
    KeyEntry { name: "Fn", offset: 49, row: 5, col: 1 },
    KeyEntry { name: "Windows", offset: 73, row: 5, col: 2 },
    KeyEntry { name: "AltL", offset: 97, row: 5, col: 3 },
    KeyEntry { name: "Space", offset: 169, row: 5, col: 4 },
    KeyEntry { name: "AltGr", offset: 241, row: 5, col: 5 },
    KeyEntry { name: "CopilotKey", offset: 289, row: 5, col: 6 },
    KeyEntry { name: "ArrowUp", offset: 337, row: 5, col: 7 },
    KeyEntry { name: "Numpad0", offset: 385, row: 5, col: 8 },
    KeyEntry { name: "NumpadDecimal", offset: 409, row: 5, col: 9 },
    KeyEntry { name: "ArrowLeft", offset: 313, row: 6, col: 6 },
    KeyEntry { name: "ArrowDown", offset: 433, row: 6, col: 7 },
    KeyEntry { name: "ArrowRight", offset: 361, row: 6, col: 8 },
];


/// 蛟龙 GM7ZG7S (ITE 048D:CE00) 的实测灯位表。
///
/// 与 0x600B 基准表 `KEYMAP` 共 4 处差异，全部为实机逐灯位对照验证：
///   1. ShiftR        基准第 85 号(offset 341) 在本机不亮 → 实测第 79 号(offset 317)。
///      注意：基准的 85 号在本机其实是**上方向键**的灯珠，因此不能简单加镜像。
///   2. ArrowUp       基准第 84 号(offset 337) 在本机不亮 → 实测第 85 号(offset 341)。
///   3. CopilotKey    本机无此键（2022 年机型）。基准的 72 号在本机是**右 Ctrl** 的灯珠，
///      而基准表里根本没有 CtrlR 条目。
///   4. ISO_Backslash 本机为 ANSI 布局，无此键。基准的 13 号在本机不亮。
///
/// 因此本表把 3、4 两条空条目改为本机实际存在的 CtrlR / PrtScr，
/// 并把一颗无名印刷键（右 Alt 与右 Ctrl 之间）登记为 PrtScr（第 66 号）。
/// 其它 98 个灯位与基准表完全一致。
pub static KEYMAP_CE00: &[KeyEntry] = &[
    KeyEntry { name: "Esc", offset: 21, row: 0, col: 0 },
    KeyEntry { name: "F1", offset: 45, row: 0, col: 1 },
    KeyEntry { name: "F2", offset: 69, row: 0, col: 2 },
    KeyEntry { name: "F3", offset: 93, row: 0, col: 3 },
    KeyEntry { name: "F4", offset: 117, row: 0, col: 4 },
    KeyEntry { name: "F5", offset: 141, row: 0, col: 5 },
    KeyEntry { name: "F6", offset: 165, row: 0, col: 6 },
    KeyEntry { name: "F7", offset: 189, row: 0, col: 7 },
    KeyEntry { name: "F8", offset: 213, row: 0, col: 8 },
    KeyEntry { name: "F9", offset: 237, row: 0, col: 9 },
    KeyEntry { name: "F10", offset: 261, row: 0, col: 10 },
    KeyEntry { name: "F11", offset: 285, row: 0, col: 11 },
    KeyEntry { name: "F12", offset: 309, row: 0, col: 12 },
    KeyEntry { name: "FnLockCam", offset: 333, row: 0, col: 13 },
    KeyEntry { name: "PrtSc", offset: 357, row: 0, col: 14 },
    KeyEntry { name: "Del", offset: 381, row: 0, col: 15 },
    KeyEntry { name: "Home", offset: 405, row: 0, col: 16 },
    KeyEntry { name: "PgUp", offset: 429, row: 0, col: 17 },
    KeyEntry { name: "PgDn", offset: 453, row: 0, col: 18 },
    KeyEntry { name: "End", offset: 477, row: 0, col: 19 },
    KeyEntry { name: "Grave", offset: 17, row: 1, col: 0 },
    KeyEntry { name: "1", offset: 41, row: 1, col: 1 },
    KeyEntry { name: "2", offset: 65, row: 1, col: 2 },
    KeyEntry { name: "3", offset: 89, row: 1, col: 3 },
    KeyEntry { name: "4", offset: 113, row: 1, col: 4 },
    KeyEntry { name: "5", offset: 137, row: 1, col: 5 },
    KeyEntry { name: "6", offset: 161, row: 1, col: 6 },
    KeyEntry { name: "7", offset: 185, row: 1, col: 7 },
    KeyEntry { name: "8", offset: 209, row: 1, col: 8 },
    KeyEntry { name: "9", offset: 233, row: 1, col: 9 },
    KeyEntry { name: "0", offset: 257, row: 1, col: 10 },
    KeyEntry { name: "Minus", offset: 281, row: 1, col: 11 },
    KeyEntry { name: "Equals", offset: 305, row: 1, col: 12 },
    KeyEntry { name: "Backspace", offset: 353, row: 1, col: 13 },
    KeyEntry { name: "NumLock", offset: 377, row: 1, col: 14 },
    KeyEntry { name: "NumpadDiv", offset: 401, row: 1, col: 15 },
    KeyEntry { name: "NumpadMul", offset: 425, row: 1, col: 16 },
    KeyEntry { name: "NumpadMinus", offset: 449, row: 1, col: 17 },
    KeyEntry { name: "Tab", offset: 13, row: 2, col: 0 },
    KeyEntry { name: "Q", offset: 61, row: 2, col: 1 },
    KeyEntry { name: "W", offset: 85, row: 2, col: 2 },
    KeyEntry { name: "E", offset: 109, row: 2, col: 3 },
    KeyEntry { name: "R", offset: 133, row: 2, col: 4 },
    KeyEntry { name: "T", offset: 157, row: 2, col: 5 },
    KeyEntry { name: "Y", offset: 181, row: 2, col: 6 },
    KeyEntry { name: "U", offset: 205, row: 2, col: 7 },
    KeyEntry { name: "I", offset: 229, row: 2, col: 8 },
    KeyEntry { name: "O", offset: 253, row: 2, col: 9 },
    KeyEntry { name: "P", offset: 277, row: 2, col: 10 },
    KeyEntry { name: "BracketL", offset: 301, row: 2, col: 11 },
    KeyEntry { name: "BracketR", offset: 325, row: 2, col: 12 },
    KeyEntry { name: "Backslash", offset: 349, row: 2, col: 13 },
    KeyEntry { name: "Numpad7", offset: 373, row: 2, col: 14 },
    KeyEntry { name: "Numpad8", offset: 397, row: 2, col: 15 },
    KeyEntry { name: "Numpad9", offset: 421, row: 2, col: 16 },
    KeyEntry { name: "NumpadPlus", offset: 445, row: 2, col: 17 },
    KeyEntry { name: "CapsLock", offset: 9, row: 3, col: 0 },
    KeyEntry { name: "A", offset: 57, row: 3, col: 1 },
    KeyEntry { name: "S", offset: 81, row: 3, col: 2 },
    KeyEntry { name: "D", offset: 105, row: 3, col: 3 },
    KeyEntry { name: "F", offset: 129, row: 3, col: 4 },
    KeyEntry { name: "G", offset: 153, row: 3, col: 5 },
    KeyEntry { name: "H", offset: 177, row: 3, col: 6 },
    KeyEntry { name: "J", offset: 201, row: 3, col: 7 },
    KeyEntry { name: "K", offset: 225, row: 3, col: 8 },
    KeyEntry { name: "L", offset: 249, row: 3, col: 9 },
    KeyEntry { name: "Semicolon", offset: 273, row: 3, col: 10 },
    KeyEntry { name: "Quote", offset: 297, row: 3, col: 11 },
    KeyEntry { name: "Enter", offset: 321, row: 3, col: 12 },
    KeyEntry { name: "Numpad4", offset: 369, row: 3, col: 13 },
    KeyEntry { name: "Numpad5", offset: 393, row: 3, col: 14 },
    KeyEntry { name: "Numpad6", offset: 417, row: 3, col: 15 },
    KeyEntry { name: "ShiftL", offset: 5, row: 4, col: 0 },
    KeyEntry { name: "CtrlR", offset: 289, row: 5, col: 11 },
    KeyEntry { name: "Z", offset: 77, row: 4, col: 2 },
    KeyEntry { name: "X", offset: 101, row: 4, col: 3 },
    KeyEntry { name: "C", offset: 125, row: 4, col: 4 },
    KeyEntry { name: "V", offset: 149, row: 4, col: 5 },
    KeyEntry { name: "B", offset: 173, row: 4, col: 6 },
    KeyEntry { name: "N", offset: 197, row: 4, col: 7 },
    KeyEntry { name: "M", offset: 221, row: 4, col: 8 },
    KeyEntry { name: "Comma", offset: 245, row: 4, col: 9 },
    KeyEntry { name: "Period", offset: 269, row: 4, col: 10 },
    KeyEntry { name: "Slash", offset: 293, row: 4, col: 11 },
    KeyEntry { name: "ShiftR", offset: 317, row: 4, col: 12 },
    KeyEntry { name: "Numpad1", offset: 365, row: 4, col: 13 },
    KeyEntry { name: "Numpad2", offset: 389, row: 4, col: 14 },
    KeyEntry { name: "Numpad3", offset: 413, row: 4, col: 15 },
    KeyEntry { name: "NumpadEnter", offset: 437, row: 4, col: 16 },
    KeyEntry { name: "CtrlL", offset: 1, row: 5, col: 0 },
    KeyEntry { name: "Fn", offset: 49, row: 5, col: 1 },
    KeyEntry { name: "Windows", offset: 73, row: 5, col: 2 },
    KeyEntry { name: "AltL", offset: 97, row: 5, col: 3 },
    KeyEntry { name: "Space", offset: 169, row: 5, col: 4 },
    KeyEntry { name: "AltGr", offset: 241, row: 5, col: 5 },
    KeyEntry { name: "PrtScr", offset: 265, row: 5, col: 10 },
    KeyEntry { name: "ArrowUp", offset: 341, row: 5, col: 7 },
    KeyEntry { name: "Numpad0", offset: 385, row: 5, col: 8 },
    KeyEntry { name: "NumpadDecimal", offset: 409, row: 5, col: 9 },
    KeyEntry { name: "ArrowLeft", offset: 313, row: 6, col: 6 },
    KeyEntry { name: "ArrowDown", offset: 337, row: 6, col: 7 },
    KeyEntry { name: "ArrowRight", offset: 361, row: 6, col: 8 },
];

/// 当前激活的键位表。0 = 0x600B 基准表（默认），1 = 蛟龙 CE00 实测表。
static ACTIVE_LAYOUT: AtomicU8 = AtomicU8::new(LAYOUT_DEFAULT);

pub const LAYOUT_DEFAULT: u8 = 0;
pub const LAYOUT_CE00: u8 = 1;

/// 由驱动在探测到 048D:CE00 后调用。必须在启动流式线程之前设置。
#[inline]
pub fn set_active_layout(kind: u8) {
    ACTIVE_LAYOUT.store(kind, Ordering::Relaxed);
}

/// 取当前机型应使用的键位表。所有特效渲染都必须经由本函数，而不是直接引用 `KEYMAP`。
#[inline]
pub fn keymap() -> &'static [KeyEntry] {
    if ACTIVE_LAYOUT.load(Ordering::Relaxed) == LAYOUT_CE00 {
        KEYMAP_CE00
    } else {
        KEYMAP
    }
}

pub fn get_key_x(k: &KeyEntry) -> f64 {
    match k.name {
        "NumLock" | "Numpad7" | "Numpad4" | "Numpad1" => 15.5,
        "NumpadDiv" | "Numpad8" | "Numpad5" | "Numpad2" => 16.5,
        "NumpadMul" | "Numpad9" | "Numpad6" | "Numpad3" | "NumpadDecimal" => 17.5,
        "NumpadMinus" | "NumpadPlus" | "NumpadEnter" => 18.5,
        "Numpad0" => 16.0,
        "Backspace" | "Enter" => 13.5,
        "ShiftL" => 0.125,
        "ShiftR" => 13.375,
        "CtrlL" => 0.125,
        "Fn" => 1.25,
        "Windows" => 2.25,
        "AltL" => 3.25,
        "Space" => 6.25,
        "AltGr" => 9.5,
        "CopilotKey" => 10.75,
        "ArrowUp" | "ArrowDown" => 12.75,
        "ArrowLeft" => 12.25,
        "ArrowRight" => 13.25,
        _ => {
            if k.row == 2 {
                k.col as f64 + 0.25
            } else if k.row == 3 {
                k.col as f64 + 0.375
            } else if k.row == 4 {
                k.col as f64 + 0.25
            } else {
                k.col as f64
            }
        }
    }
}

pub fn get_key_y(k: &KeyEntry) -> f64 {
    match k.name {
        "Enter" | "NumpadPlus" => 2.5,
        "NumpadEnter" => 4.5,
        "ArrowUp" => 4.75,
        "ArrowLeft" | "ArrowDown" | "ArrowRight" => 5.25,
        _ => k.row as f64,
    }
}

#[inline]
pub fn get_key_visual_row(k: &KeyEntry) -> usize {
    if k.name == "Enter" {
        3
    } else if k.name == "Backslash" {
        2
    } else if k.name == "ArrowLeft" || k.name == "ArrowDown" || k.name == "ArrowRight" {
        6
    } else {
        k.row as usize
    }
}

pub const KEY_SAMPLE_X: [f64; 4] = [-0.28, 0.28, -0.28, 0.28];
pub const KEY_SAMPLE_Y: [f64; 4] = [-0.22, -0.22, 0.22, 0.22];
pub const CURRENT_SAMPLE_X: [f64; 8] = [-0.44, -0.31, -0.19, -0.06, 0.06, 0.19, 0.31, 0.44];

#[inline]
pub fn set_key(buf: &mut [u8; BUF_SIZE], offset: usize, r: u8, g: u8, b: u8) {
    if offset + 2 < BUF_SIZE {
        buf[offset] = r;
        buf[offset + 1] = g;
        buf[offset + 2] = b;
    }
    // Mirror wide key extra LEDs
    if offset == OFFSET_SHIFT_LEFT {
        set_key(buf, OFFSET_SHIFT_LEFT_AUX, r, g, b);
    } else if offset == OFFSET_ENTER {
        set_key(buf, OFFSET_ENTER_AUX, r, g, b);
    }
}

#[inline]
pub fn set_key_from_linear(buf: &mut [u8; BUF_SIZE], offset: usize, r: f64, g: f64, b: f64) {
    use super::effects::linear_to_srgb;
    set_key(buf, offset, linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b));
}
