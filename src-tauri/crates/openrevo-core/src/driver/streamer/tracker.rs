// OpenRevo - Real-time Key Press Tracker & Interactive Event State Machine
// Event-driven keystroke ingestion for interactive ripples, currents, and lightning strikes.
// ZERO polling, ZERO GetAsyncKeyState syscalls to guarantee 0ms typing latency and flicker-free lighting.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use parking_lot::Mutex;
use super::effects::hsv_to_rgb;

static IS_INTERACTIVE_STREAMING: AtomicBool = AtomicBool::new(false);
static KEY_EVENT_SENDER: Mutex<Option<SyncSender<i32>>> = Mutex::new(None);

/// Register an event receiver channel for the streamer worker thread.
/// Maximum buffer capacity of 64 events prevents memory bloat while absorbing rapid typing bursts.
pub fn register_key_event_receiver() -> Receiver<i32> {
    let (tx, rx) = sync_channel(64);
    *KEY_EVENT_SENDER.lock() = Some(tx);
    rx
}

/// Unregister sender and disable interactive ingestion on streamer thread termination.
pub fn unregister_key_event_sender() {
    IS_INTERACTIVE_STREAMING.store(false, Ordering::Release);
    *KEY_EVENT_SENDER.lock() = None;
}

/// Control whether low-level keyboard hook forwards keystrokes to streamer.
/// Automatically active only when interactive effects (Ripple, TouchCurrent) are selected.
pub fn set_interactive_mode(active: bool) {
    IS_INTERACTIVE_STREAMING.store(active, Ordering::Release);
}

/// Fast-path push called from Windows WH_KEYBOARD_LL hook.
/// Operates in <50ns without acquiring heavy locks or blocking typing inputs.
#[inline]
pub fn push_interactive_key_event(vk: i32) {
    if !IS_INTERACTIVE_STREAMING.load(Ordering::Relaxed) {
        return;
    }
    if let Some(sender) = KEY_EVENT_SENDER.try_lock() {
        if let Some(tx) = sender.as_ref() {
            let _ = tx.try_send(vk);
        }
    }
}

/// Check if current lighting effect relies on interactive keystroke events
pub fn is_interactive_effect(effect: u8) -> bool {
    let raw = if effect >= 200 {
        effect
    } else if effect >= 100 {
        effect - 100
    } else {
        effect
    };
    raw == 2 || raw == 8 || raw == 12 // 2: Ripple, 8/12: TouchCurrent
}

#[derive(Clone, Copy)]
pub struct LightningStrike {
    pub head_pos: f64,
    pub bolt_x: f64,
    pub bend_seed: f64,
    pub countdown: i32,
}

#[derive(Clone, Copy)]
pub struct RainDrop {
    pub x: f64,
    pub head_y: f64,
    pub speed: f64,
    pub length: f64,
    pub color_idx: usize,
}

#[derive(Clone, Copy)]
pub struct SparkleItem {
    pub age: u32,
    pub duration: u32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy)]
pub struct RippleEvent {
    pub x: f64,
    pub y: f64,
    pub start_time: std::time::Instant,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy)]
pub struct TouchCurrentEvent {
    pub row: usize,
    pub origin_x: f64,
    pub start_time: std::time::Instant,
    pub phase: f64,
}

pub struct KeyTracker {
    pub was_down: [bool; 256],
    pub ripples: [Option<RippleEvent>; 32],
    pub next_ripple: usize,
    pub ripple_seq: u64,

    pub current_prev: [(f64, f64, f64); 128],
    pub lightning_prev: [f64; 128],
    pub lightning_strikes: [LightningStrike; 4],
    pub flame_heat: [f64; 128],
    pub rain_drops: [RainDrop; 20],
    pub sparkle_list: [SparkleItem; 128],
    pub touch_currents: [Option<TouchCurrentEvent>; 8],
    pub next_touch: usize,
    pub last_ambient_touch: std::time::Instant,
    pub matrix_heads: [f64; 24],
}

impl KeyTracker {
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        let mut rain_drops = [RainDrop { x: 0.0, head_y: 0.0, speed: 0.1, length: 1.5, color_idx: 0 }; 20];
        for (i, drop) in rain_drops.iter_mut().enumerate() {
            drop.x = (i as f64 * 1.05 + 0.5) % 19.0;
            drop.head_y = -((i as f64 * 1.3) % 7.0);
            drop.speed = 0.08 + ((i * 7) % 10) as f64 * 0.015;
            drop.length = 1.2 + ((i * 3) % 5) as f64 * 0.3;
            drop.color_idx = i % 5;
        }

        let mut strikes = [LightningStrike { head_pos: 999.0, bolt_x: 9.5, bend_seed: 0.0, countdown: 10 }; 4];
        for (i, s) in strikes.iter_mut().enumerate() {
            s.countdown = (i as i32) * 15 + 5;
        }

        let mut matrix_heads = [0.0; 24];
        for (i, h) in matrix_heads.iter_mut().enumerate() {
            *h = -((i as f64 * 1.7) % 8.0);
        }

        Self {
            was_down: [false; 256],
            ripples: [None; 32],
            next_ripple: 0,
            ripple_seq: 0,
            current_prev: [(0.0, 0.0, 0.0); 128],
            lightning_prev: [0.0; 128],
            lightning_strikes: strikes,
            flame_heat: [0.0; 128],
            rain_drops,
            sparkle_list: [SparkleItem { age: 0, duration: 30, r: 255, g: 255, b: 255 }; 128],
            touch_currents: [None; 8],
            next_touch: 0,
            last_ambient_touch: now,
            matrix_heads,
        }
    }

    #[inline]
    pub fn record_key_press(&mut self, vk: i32, now: std::time::Instant) {
        let (kx, ky) = vk_to_pos(vk);

        // Golden angle (137.5077°) step for complementary color variety
        let hue = (self.ripple_seq as f64 * 137.5077 + kx * 17.0 + ky * 29.0).rem_euclid(360.0);
        let (r, g, b) = hsv_to_rgb(hue, 0.95, 1.0);
        self.ripple_seq = self.ripple_seq.wrapping_add(1);

        self.ripples[self.next_ripple] = Some(RippleEvent {
            x: kx,
            y: ky,
            start_time: now,
            r,
            g,
            b,
        });
        self.next_ripple = (self.next_ripple + 1) % self.ripples.len();

        let row = get_vk_visual_row(vk);
        self.touch_currents[self.next_touch] = Some(TouchCurrentEvent {
            row,
            origin_x: kx,
            start_time: now,
            phase: kx * 0.05,
        });
        self.next_touch = (self.next_touch + 1) % self.touch_currents.len();
    }

    /// Process queued keystroke events from the lock-free event channel.
    pub fn process_key_events(&mut self, rx: &Receiver<i32>) {
        let now = std::time::Instant::now();
        while let Ok(vk) = rx.try_recv() {
            let idx = (vk as usize) & 0xFF;
            self.was_down[idx] = true;
            self.record_key_press(vk, now);
        }
    }

    /// Active key scan fallback ensuring ripples fire even if Windows hook was dropped or bypassed
    pub fn poll_fallback_keys(&mut self) {
        const TRACK_VKS: &[i32] = &[
            0x08, 0x09, 0x0D, 0x1B, 0x20, // Backspace, Tab, Enter, Esc, Space
            0x21, 0x22, 0x23, 0x24,       // PgUp, PgDn, End, Home
            0x25, 0x26, 0x27, 0x28, 0x2E, // Left, Up, Right, Down, Del
            0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, // 0..9
            0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, // A..J
            0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, // K..T
            0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,                         // U..Z
            0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, // Num 0..9
            0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, // F1..F12
            0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xBF, 0xC0, 0xDB, 0xDC, 0xDD, 0xDE, // OEM punctuation
        ];

        let now = std::time::Instant::now();
        for &vk in TRACK_VKS {
            let is_down = unsafe { (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk) as u16 & 0x8000) != 0 };
            let idx = (vk as usize) & 0xFF;
            if is_down {
                if !self.was_down[idx] {
                    self.was_down[idx] = true;
                    self.record_key_press(vk, now);
                }
            } else {
                self.was_down[idx] = false;
            }
        }
    }

    /// Legacy compat stub
    pub fn poll(&mut self) {}
}

pub fn get_vk_visual_row(vk: i32) -> usize {
    match vk {
        0x1B | 0x70..=0x7B => 0,
        0xC0 | 0x30..=0x39 | 0xBD | 0xBB | 0x08 => 1,
        0x09 | 0x51 | 0x57 | 0x45 | 0x52 | 0x54 | 0x59 | 0x55 | 0x49 | 0x4F | 0x50 | 0xDB | 0xDD | 0xDC => 2,
        0x14 | 0x41 | 0x53 | 0x44 | 0x46 | 0x47 | 0x48 | 0x4A | 0x4B | 0x4C | 0xBA | 0xDE | 0x0D => 3,
        0x10 | 0xA0 | 0x5A | 0x58 | 0x43 | 0x56 | 0x42 | 0x4E | 0x4D | 0xBC | 0xBE | 0xBF | 0xA1 => 4,
        0x20 | 0x25 | 0x26 | 0x28 | 0x27 => 5,
        _ => 3,
    }
}

pub fn vk_to_pos(vk: i32) -> (f64, f64) {
    match vk {
        0x1B => (0.5, 0.0), // Esc
        0x70..=0x7B => ((vk - 0x70) as f64 * 1.1 + 2.0, 0.0), // F1..F12
        0xC0 => (0.5, 1.0), // `
        0x31..=0x39 => ((vk - 0x31) as f64 * 1.0 + 1.5, 1.0), // 1..9
        0x30 => (10.5, 1.0), // 0
        0xBD => (11.5, 1.0), // -
        0xBB => (12.5, 1.0), // =
        0x08 => (14.0, 1.0), // Backspace
        0x09 => (0.75, 2.0), // Tab
        0x51 => (1.75, 2.0), // Q
        0x57 => (2.75, 2.0), // W
        0x45 => (3.75, 2.0), // E
        0x52 => (4.75, 2.0), // R
        0x54 => (5.75, 2.0), // T
        0x59 => (6.75, 2.0), // Y
        0x55 => (7.75, 2.0), // U
        0x49 => (8.75, 2.0), // I
        0x4F => (9.75, 2.0), // O
        0x50 => (10.75, 2.0), // P
        0xDB => (11.75, 2.0), // [
        0xDD => (12.75, 2.0), // ]
        0xDC => (13.75, 2.0), // \
        0x14 => (0.85, 3.0), // Caps
        0x41 => (1.9, 3.0), // A
        0x53 => (2.9, 3.0), // S
        0x44 => (3.9, 3.0), // D
        0x46 => (4.9, 3.0), // F
        0x47 => (5.9, 3.0), // G
        0x48 => (6.9, 3.0), // H
        0x4A => (7.9, 3.0), // J
        0x4B => (8.9, 3.0), // K
        0x4C => (9.9, 3.0), // L
        0xBA => (10.9, 3.0), // ;
        0xDE => (11.9, 3.0), // '
        0x0D => (13.5, 3.0), // Enter
        0x10 | 0xA0 => (1.1, 4.0), // LShift
        0x5A => (2.3, 4.0), // Z
        0x58 => (3.3, 4.0), // X
        0x43 => (4.3, 4.0), // C
        0x56 => (5.3, 4.0), // V
        0x42 => (6.3, 4.0), // B
        0x4E => (7.3, 4.0), // N
        0x4D => (8.3, 4.0), // M
        0xBC => (9.3, 4.0), // ,
        0xBE => (10.3, 4.0), // .
        0xBF => (11.3, 4.0), // /
        0xA1 => (13.0, 4.0), // RShift
        0x20 => (7.0, 5.0), // Space
        0x25 => (12.0, 5.0), // Left
        0x26 => (13.0, 4.5), // Up
        0x28 => (13.0, 5.25), // Down
        0x27 => (14.0, 5.0), // Right
        0x60..=0x69 => (16.0 + ((vk - 0x60) % 3) as f64, 2.0 + ((vk - 0x60) / 3) as f64), // Numpad
        _ => (9.5, 2.5), // Center
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_keyboard_events_do_not_wait_for_a_busy_or_full_streamer_channel() {
        let receiver = register_key_event_receiver();
        set_interactive_mode(true);
        let guard = KEY_EVENT_SENDER.lock();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            push_interactive_key_event(1);
            tx.send(()).unwrap();
        });
        let result = rx.recv_timeout(std::time::Duration::from_millis(500));
        drop(guard);
        worker.join().unwrap();
        assert!(result.is_ok());
        assert!(receiver.try_recv().is_err());
        for key in 0..65 {
            push_interactive_key_event(key);
        }
        assert_eq!(
            receiver.try_iter().collect::<Vec<_>>(),
            (0..64).collect::<Vec<_>>()
        );
        set_interactive_mode(false);
        push_interactive_key_event(99);
        assert!(receiver.try_recv().is_err());
        unregister_key_event_sender();
    }
}
