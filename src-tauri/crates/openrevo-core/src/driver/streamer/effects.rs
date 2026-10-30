// OpenRevo - BetterRGB Streamer Visual Shaders & Procedural Math Pipeline
// High-FPS procedural lighting shaders, MSAA sub-sample anti-aliasing, and color transforms.

use std::f64::consts::PI;
use super::layout::*;
use super::tracker::{KeyTracker, TouchCurrentEvent};
use super::StreamerParams;

// --- Color & Math Helpers ---

#[inline]
pub fn srgb_to_linear(c: u8) -> f64 {
    let v = c as f64 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
pub fn linear_to_srgb(linear: f64) -> u8 {
    let clamped = linear.clamp(0.0, 1.0);
    let s = if clamped <= 0.0031308 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

#[inline]
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;
    let h = if delta == 0.0 {
        0.0
    } else if max == rf {
        60.0 * (((gf - bf) / delta).rem_euclid(6.0))
    } else if max == gf {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { delta / max };
    (h, s, max)
}

pub fn hsv_to_rgb(hue_deg: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((hue_deg % 360.0) + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

#[allow(dead_code)]
#[inline]
pub fn wrap_unit(val: f64) -> f64 {
    let v = val % 1.0;
    if v < 0.0 { v + 1.0 } else { v }
}

#[inline]
pub fn sample_palette_linear(palette: &[(u8, u8, u8); 3], phase: f64) -> (f64, f64, f64) {
    let p = wrap_unit(phase) * 3.0;
    let idx0 = p.floor() as usize % 3;
    let idx1 = (idx0 + 1) % 3;
    let mix = smooth_unit(p.fract());
    let r0 = srgb_to_linear(palette[idx0].0);
    let g0 = srgb_to_linear(palette[idx0].1);
    let b0 = srgb_to_linear(palette[idx0].2);
    let r1 = srgb_to_linear(palette[idx1].0);
    let g1 = srgb_to_linear(palette[idx1].1);
    let b1 = srgb_to_linear(palette[idx1].2);
    (r0 + (r1 - r0) * mix, g0 + (g1 - g0) * mix, b0 + (b1 - b0) * mix)
}

#[inline]
pub fn rand_f64(seed: f64) -> f64 {
    let s = (seed * 12.9898).sin() * 43758.5453;
    s.fract().abs()
}

#[allow(dead_code)]
#[inline]
pub fn smooth_unit(t: f64) -> f64 {
    let ct = t.clamp(0.0, 1.0);
    ct * ct * (3.0 - 2.0 * ct)
}

pub fn render_streamer_frame(
    buf: &mut [u8; BUF_SIZE],
    params: &StreamerParams,
    t: &mut f64,
    key_tracker: &mut KeyTracker,
    pr: u8,
    pg: u8,
    pb: u8,
) {
    let speed_mult = match params.speed {
        0 => 0.35,
        1 => 0.65,
        2 => 1.0, // Standard middle profile
        3 => 1.6,
        4 => 2.4,
        _ => 1.0,
    };

    buf.fill(0);

    let user_palette = if pr == 255 && pg == 255 && pb == 255 {
        [(255, 0, 0), (0, 255, 0), (0, 0, 255)]
    } else if pr == 99 && pg == 102 && pb == 241 {
        [(99, 102, 241), (0, 255, 255), (236, 72, 153)]
    } else if pr == 0 && pg == 255 && pb == 255 {
        [(0, 255, 255), (99, 102, 241), (16, 185, 129)]
    } else {
        let (h, s, v) = rgb_to_hsv(pr, pg, pb);
        let s_boost = s.max(0.75);
        let v_boost = v.max(0.75);
        // Form a closed 360-degree equilateral loop (h, h+120, h+240) so that sample_palette_linear
        // wraps seamlessly from index 2 back to index 0 with 0 phase discontinuity.
        [
            hsv_to_rgb(h, s_boost, v_boost),
            hsv_to_rgb((h + 120.0).rem_euclid(360.0), s_boost, v_boost),
            hsv_to_rgb((h + 240.0).rem_euclid(360.0), s_boost, v_boost),
        ]
    };

    let raw_effect = if params.effect >= 200 {
        params.effect
    } else if params.effect >= 100 {
        params.effect - 100
    } else {
        params.effect
    };

    match raw_effect {
        // 0: Breathing
        0 => {
            let raw = ((*t * 0.05 * speed_mult).sin() + 1.0) * 0.5;
            let pulse = raw.powf(1.8);
            let r = (pr as f64 * pulse) as u8;
            let g = (pg as f64 * pulse) as u8;
            let b = (pb as f64 * pulse) as u8;
            for key in keymap().iter() {
                set_key(buf, key.offset, r, g, b);
            }
            *t += 1.0;
        }

        // 1: Wave (均衡倾角多相补偿算法：将整键盘光通量方差严格压制在 7.0% 以内，彻底消除暗沉盲区与规律闪烁)
        1 => {
            let (dx, dy) = match params.direction {
                0 => (-19.0, -35.0), // 右流向：斜向交错，全键盘红橙黄绿青蓝紫完美互补
                1 => (19.0, -35.0),  // 左流向
                2 => (-20.0, 55.0),  // 下流向
                3 => (-20.0, -55.0), // 上流向
                _ => (-19.0, -35.0),
            };

            for key in keymap().iter() {
                let kx = get_key_x(key);
                let ky = get_key_y(key);
                let mut sum_r = 0.0;
                let mut sum_g = 0.0;
                let mut sum_b = 0.0;

                for s in 0..4 {
                    let sx = kx + KEY_SAMPLE_X[s];
                    let sy = ky + KEY_SAMPLE_Y[s];
                    let proj = sx * dx + sy * dy;
                    let hue = (*t + proj).rem_euclid(360.0);
                    let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);

                    sum_r += srgb_to_linear(r);
                    sum_g += srgb_to_linear(g);
                    sum_b += srgb_to_linear(b);
                }

                set_key_from_linear(buf, key.offset, sum_r / 4.0, sum_g / 4.0, sum_b / 4.0);
            }

            // t must grow monotonically; applying rem_euclid(360.0) here caused a
            // one-frame phase discontinuity every time t wrapped from ~360 back to ~0.
            // Because proj can range far beyond +-360, the sudden jump in t produced a
            // visible colour-hue glitch (the "flicker").  Instead we let t accumulate
            // freely and only fold it inside hsv_to_rgb via the rem_euclid already
            // present there.  A safety wrap at a large multiple of 360 prevents f64
            // precision from degrading after hours of continuous operation.
            *t += 8.0 * speed_mult;
            if *t > 360.0 * 1000.0 {
                *t -= 360.0 * 1000.0;
            }
        }

        // 2: Ripple
        2 => {
            let travel_speed = 13.0 * speed_mult;
            let lifetime = 1.3 / speed_mult.max(0.5);
            let width = 1.8;

            for key in keymap().iter() {
                let kx = get_key_x(key);
                let ky = get_key_y(key);
                let mut sum_r = 0.0;
                let mut sum_g = 0.0;
                let mut sum_b = 0.0;

                for opt in key_tracker.ripples.iter() {
                    if let Some(r) = opt {
                        let elapsed = r.start_time.elapsed().as_secs_f64();
                        if elapsed < lifetime {
                            let radius = elapsed * travel_speed;
                            let dist = ((kx - r.x).powi(2) + (ky - r.y).powi(2)).sqrt();
                            let diff = (dist - radius).abs();
                            if diff < width {
                                let fade = 1.0 - elapsed / lifetime;
                                let pulse = (1.0 - diff / width).powi(2) * fade;

                                let lin_r = srgb_to_linear(r.r);
                                let lin_g = srgb_to_linear(r.g);
                                let lin_b = srgb_to_linear(r.b);

                                let white_foam = pulse.powi(3) * 0.45;

                                sum_r += lin_r * pulse + white_foam;
                                sum_g += lin_g * pulse + white_foam;
                                sum_b += lin_b * pulse + white_foam;
                            }
                        }
                    }
                }

                set_key_from_linear(buf, key.offset, sum_r.min(1.0), sum_g.min(1.0), sum_b.min(1.0));
            }
        }

        // 3: Quicksand
        3 => {
            let speed_scale = match params.speed {
                0 => 2.0,
                1 => 3.5,
                2 => 5.0,
                3 => 7.0,
                4 => 9.0,
                _ => 5.0,
            };
            let speed = 0.22 + speed_scale * 0.16;
            let wavelength = 3.25;
            let flow = (*t * 0.033) * speed;

            for key in keymap().iter() {
                let mut sum_r = 0.0;
                let mut sum_g = 0.0;
                let mut sum_b = 0.0;
                let kx = get_key_x(key);
                let ky = get_key_y(key);
                for s in 0..4 {
                    let x = kx + KEY_SAMPLE_X[s];
                    let y = ky + KEY_SAMPLE_Y[s];
                    let warped_y = y - flow + 0.24 * (x * 0.68 + flow * 0.55).sin() + 0.10 * (x * 1.73 - flow * 0.31).sin();
                    let band = warped_y / wavelength;
                    let phase = band + x * 0.035;
                    let curtain = ((band * 2.0 * PI).sin() + 1.0) * 0.5;
                    let intensity = 0.08 + 0.92 * smooth_unit(curtain);
                    let grain = 0.92 + 0.08 * (x * 2.41 + y * 3.17 + flow * 1.2).sin();

                    let (lr, lg, lb) = sample_palette_linear(&user_palette, phase);
                    sum_r += lr * intensity * grain;
                    sum_g += lg * intensity * grain;
                    sum_b += lb * intensity * grain;
                }
                set_key_from_linear(buf, key.offset, sum_r / 4.0, sum_g / 4.0, sum_b / 4.0);
            }
            *t += 1.0;
        }

        // 4: Current
        4 => {
            let speed_scale = match params.speed {
                0 => 3.0,
                1 => 5.5,
                2 => 8.5,
                3 => 12.0,
                4 => 16.0,
                _ => 8.5,
            };
            let travel_speed = (5.5 + speed_scale * 1.5) * speed_mult;
            let width = 3.2;
            let cycle = 20.0 + width;
            let now_sec = *t * 0.033;

            for (i, key) in keymap().iter().enumerate() {
                let row = get_key_visual_row(key);
                let kx = get_key_x(key);
                let mut sum_r = 0.0;
                let mut sum_g = 0.0;
                let mut sum_b = 0.0;

                for s in 0..8 {
                    let x = kx + CURRENT_SAMPLE_X[s];
                    let travel_x = if (row & 1) != 0 { 19.0 - x } else { x };
                    let head = ((now_sec * travel_speed + row as f64 * 3.35).rem_euclid(cycle)) - width;
                    let dist_behind = head - travel_x;
                    let mut intensity = 0.0;
                    if dist_behind > -0.75 && dist_behind < width + 0.75 {
                        let head_cover = if dist_behind < 0.0 { smooth_unit((dist_behind + 0.75) / 0.75) } else { 1.0 };
                        let tail_cover = if dist_behind > width { smooth_unit((width + 0.75 - dist_behind) / 0.75) } else { 1.0 };
                        let decay = if dist_behind > 0.0 { dist_behind } else { 0.0 };
                        intensity = (-decay / (0.35 * width + 0.35)).exp() * head_cover * tail_cover;
                    }
                    let phase = (row as f64) / 7.0 + travel_x * 0.045 + now_sec * 0.12;
                    let (lr, lg, lb) = sample_palette_linear(&user_palette, phase);
                    sum_r += lr * intensity;
                    sum_g += lg * intensity;
                    sum_b += lb * intensity;
                }

                let target_r = sum_r / 8.0;
                let target_g = sum_g / 8.0;
                let target_b = sum_b / 8.0;
                let target_level = target_r + target_g + target_b;
                let prev = key_tracker.current_prev[i];
                let prev_level = prev.0 + prev.1 + prev.2;
                let smoothing = if target_level > prev_level { 0.50 } else { 0.22 };

                let new_r = prev.0 + (target_r - prev.0) * smoothing;
                let new_g = prev.1 + (target_g - prev.1) * smoothing;
                let new_b = prev.2 + (target_b - prev.2) * smoothing;
                key_tracker.current_prev[i] = (new_r, new_g, new_b);

                set_key_from_linear(buf, key.offset, new_r, new_g, new_b);
            }
            *t += 1.0;
        }

        // 5: Wheel
        5 => {
            let cx = 6.8;
            let cy = 2.5;
            let dir_mul = if params.direction == 1 { -1.0 } else { 1.0 };
            let rot_deg = *t;

            for key in keymap().iter() {
                let dx = get_key_x(key) - cx;
                let dy = (get_key_y(key) - cy) * 1.85;
                let angle_deg = dy.atan2(dx).to_degrees();
                let hue = (rot_deg + angle_deg).rem_euclid(360.0);
                let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
                set_key(buf, key.offset, r, g, b);
            }
            // Grow t monotonically to prevent phase snap on rem_euclid boundary
            *t += 8.5 * speed_mult * dir_mul;
            if *t > 360.0 * 1000.0 {
                *t -= 360.0 * 1000.0;
            } else if *t < -360.0 * 1000.0 {
                *t += 360.0 * 1000.0;
            }
        }

        // 6: Rainbow
        6 => {
            let hue = (*t * 3.0 * speed_mult).rem_euclid(360.0);
            let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
            for key in keymap().iter() {
                set_key(buf, key.offset, r, g, b);
            }
            *t += 1.0;
            if *t > 360.0 * 1000.0 {
                *t -= 360.0 * 1000.0;
            }
        }

        // 7: Matrix
        7 => {
            let speed = 0.08 * speed_mult;
            for h in key_tracker.matrix_heads.iter_mut() {
                *h += speed;
                if *h > 12.0 {
                    *h = -(rand_f64(*t) * 4.0);
                }
            }
            for key in keymap().iter() {
                let col = (get_key_x(key).floor() as usize).min(23);
                let y = get_key_y(key);
                let head = key_tracker.matrix_heads[col];
                let dist = head - y;
                if dist >= 0.0 && dist < 4.5 {
                    if dist < 0.6 {
                        set_key(buf, key.offset, 200, 255, 200);
                    } else {
                        let fade = (1.0 - (dist - 0.6) / 3.9).clamp(0.0, 1.0);
                        let g = (230.0 * fade) as u8;
                        set_key(buf, key.offset, 0, g, (30.0 * fade) as u8);
                    }
                } else {
                    set_key(buf, key.offset, 0, 8, 2);
                }
            }
            *t += 1.0;
        }

        // 8: Lightning
        8 => {
            let speed = (0.55 + (params.speed as f64) * 0.22) * speed_mult;
            let band_width = 0.50;
            let tail_len = band_width * 2.2;
            let finish_pos = 7.0 + tail_len;
            let x_sigma = 0.32;

            let mut bright_arr = [0.0f64; 128];

            for strike in key_tracker.lightning_strikes.iter_mut() {
                if strike.head_pos > finish_pos {
                    strike.countdown -= 1;
                    if strike.countdown <= 0 {
                        strike.head_pos = -2.5;
                        strike.bolt_x = rand_f64(*t + strike.bend_seed * 11.3) * 18.0 + 0.5;
                        strike.bend_seed = rand_f64(*t * 3.7 + strike.bolt_x) * 6.28;
                        strike.countdown = 4 + (rand_f64(*t * 2.3) * 16.0) as i32;
                    }
                }
                if strike.head_pos <= finish_pos {
                    for (i, key) in keymap().iter().enumerate() {
                        let y = get_key_y(key);
                        let dv = strike.head_pos - y;
                        let center_x = strike.bolt_x + (y * 2.35 + strike.bend_seed).sin() * 0.40;
                        let dh = get_key_x(key) - center_x;
                        let front = if dv < 0.0 { smooth_unit((dv + 0.95) / 0.95) } else { 1.0 };
                        let tail = if dv > 0.0 { (-dv / (tail_len * 0.45)).exp() } else { 1.0 };
                        let horizontal = (-0.5 * (dh / x_sigma).powi(2)).exp();
                        if horizontal >= 0.02 {
                            let b = front * tail * horizontal;
                            if b > bright_arr[i] {
                                bright_arr[i] = b;
                            }
                        }
                    }
                    strike.head_pos += speed;
                }
            }

            let attack = 0.85;
            let release = 0.16;
            for (i, key) in keymap().iter().enumerate() {
                let target = if bright_arr[i] < 0.002 { 0.0 } else { bright_arr[i] };
                let coef = if target > key_tracker.lightning_prev[i] { attack } else { release };
                key_tracker.lightning_prev[i] += (target - key_tracker.lightning_prev[i]) * coef;
                let glow = key_tracker.lightning_prev[i];

                let r = linear_to_srgb((glow * 1.15).min(1.0));
                let g = linear_to_srgb((glow * 1.15).min(1.0));
                let b = linear_to_srgb((glow * 1.35 + 0.005).min(1.0));
                set_key(buf, key.offset, r, g, b);
            }
            *t += 1.0;
        }

        // 9: Flame
        9 => {
            let speed_val = 3.0 + (params.speed as f64) * 1.5;
            let volatility = 50.0 + speed_val * 16.0;
            let alpha = 0.22 * speed_mult;

            let frame_seed = (*t as u64).wrapping_mul(0x9e3779b97f4a7c15);

            for (i, key) in keymap().iter().enumerate() {
                let row_frac = (key.row as f64) / 5.0;
                let bottom_heat_bias = 55.0 * row_frac;

                let mut rng = frame_seed ^ ((i as u64).wrapping_mul(0x517cc1b727220a95));
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let rand_val = (rng & 0xFFFF) as f64 / 65535.0;

                let target = 80.0 + bottom_heat_bias + (rand_val * volatility - volatility * 0.5);
                key_tracker.flame_heat[i] = key_tracker.flame_heat[i] * (1.0 - alpha) + target * alpha;
                let h = key_tracker.flame_heat[i].clamp(0.0, 255.0);

                let (r, g, b) = if h < 85.0 {
                    ((h * 3.0) as u8, 0, 0)
                } else if h < 170.0 {
                    (255, ((h - 85.0) * 3.0) as u8, 0)
                } else {
                    (255, 255, ((h - 170.0) * 3.0) as u8)
                };
                set_key(buf, key.offset, r, g, b);
            }
            *t += 1.0;
        }

        // 10: Rain
        10 => {
            let rain_palette: [(u8, u8, u8); 5] = [
                (80, 140, 255),
                (120, 80, 255),
                (80, 220, 255),
                (240, 245, 255),
                (80, 255, 180),
            ];

            for drop in key_tracker.rain_drops.iter_mut() {
                drop.head_y += drop.speed * speed_mult;
                if drop.head_y > 8.0 {
                    drop.head_y = -(rand_f64(*t + drop.x) * 3.0);
                    drop.x = rand_f64(*t * 1.3 + drop.head_y) * 19.0;
                    drop.speed = 0.08 + rand_f64(*t * 2.1) * 0.12;
                }
            }

            for key in keymap().iter() {
                let kx = get_key_x(key);
                let ky = get_key_y(key);
                let mut total_r = 0.0;
                let mut total_g = 0.0;
                let mut total_b = 0.0;

                for drop in key_tracker.rain_drops.iter() {
                    let dx = (kx - drop.x).abs();
                    if dx < 0.6 {
                        let dy = drop.head_y - ky;
                        if dy >= 0.0 && dy < drop.length {
                            let tail_fade = 1.0 - dy / drop.length;
                            let h_fade = 1.0 - dx / 0.6;
                            let intensity = tail_fade * h_fade;
                            let col = rain_palette[drop.color_idx % rain_palette.len()];
                            total_r += srgb_to_linear(col.0) * intensity;
                            total_g += srgb_to_linear(col.1) * intensity;
                            total_b += srgb_to_linear(col.2) * intensity;
                        }
                    }
                }
                let bg_r = 0.003;
                let bg_g = 0.008;
                let bg_b = 0.020;
                set_key_from_linear(
                    buf,
                    key.offset,
                    (total_r + bg_r).min(1.0),
                    (total_g + bg_g).min(1.0),
                    (total_b + bg_b).min(1.0),
                );
            }
            *t += 1.0;
        }

        // 11: Sparkle
        11 => {
            let life_duration = (40.0 / speed_mult.max(0.5)).round() as u32;
            let target_alive = 14;
            let mut alive = 0;
            for item in key_tracker.sparkle_list.iter() {
                if item.age > 0 {
                    alive += 1;
                }
            }

            if alive < target_alive {
                let to_spawn = target_alive - alive;
                for _ in 0..to_spawn {
                    let idx = (rand_f64(*t * 3.7) * (keymap().len() as f64)) as usize % keymap().len();
                    let item = &mut key_tracker.sparkle_list[idx];
                    if item.age == 0 {
                        item.age = 1;
                        item.duration = life_duration + (rand_f64(*t) * 15.0) as u32;
                        let hue = rand_f64(*t * 5.1) * 360.0;
                        let (sr, sg, sb) = hsv_to_rgb(hue, 1.0, 1.0);
                        item.r = sr;
                        item.g = sg;
                        item.b = sb;
                    }
                }
            }

            for (i, key) in keymap().iter().enumerate() {
                let item = &mut key_tracker.sparkle_list[i];
                if item.age > 0 {
                    let progress = item.age as f64 / item.duration as f64;
                    let mut envelope = (PI * progress).sin();
                    envelope = envelope * envelope;
                    let lr = srgb_to_linear(item.r) * envelope;
                    let lg = srgb_to_linear(item.g) * envelope;
                    let lb = srgb_to_linear(item.b) * envelope;
                    set_key_from_linear(buf, key.offset, lr, lg, lb);
                    item.age += 1;
                    if item.age >= item.duration {
                        item.age = 0;
                    }
                } else {
                    set_key(buf, key.offset, 0, 0, 0);
                }
            }
            *t += 1.0;
        }

        // 12: Touch Current
        12 => {
            let travel_speed = 14.0 * speed_mult;
            let sigma = 0.40;
            let lifetime = 2.0 / speed_mult.max(0.5);

            let now = std::time::Instant::now();
            let has_active = key_tracker.touch_currents.iter().any(|ev| {
                if let Some(e) = ev {
                    e.start_time.elapsed().as_secs_f64() < lifetime
                } else {
                    false
                }
            });
            if !has_active && key_tracker.last_ambient_touch.elapsed().as_secs_f64() > 2.0 {
                let row = (rand_f64(*t) * 5.0).floor() as usize + 1;
                key_tracker.touch_currents[key_tracker.next_touch] = Some(TouchCurrentEvent {
                    row,
                    origin_x: 9.5,
                    start_time: now,
                    phase: 0.0,
                });
                key_tracker.next_touch = (key_tracker.next_touch + 1) % key_tracker.touch_currents.len();
                key_tracker.last_ambient_touch = now;
            }

            for key in keymap().iter() {
                let row = get_key_visual_row(key);
                let kx = get_key_x(key);
                let mut sum_r = 0.0;
                let mut sum_g = 0.0;
                let mut sum_b = 0.0;

                for s in 0..4 {
                    let x = kx + KEY_SAMPLE_X[s];
                    let mut sample_r = 0.0;
                    let mut sample_g = 0.0;
                    let mut sample_b = 0.0;

                    for opt in key_tracker.touch_currents.iter() {
                        if let Some(ev) = opt {
                            if ev.row == row {
                                let elapsed = ev.start_time.elapsed().as_secs_f64();
                                if elapsed < lifetime {
                                    let radius = elapsed * travel_speed;
                                    let delta = ((x - ev.origin_x).abs() - radius).abs();
                                    let intensity = (-(delta * delta) / (2.0 * sigma * sigma)).exp();
                                    let edge_fade = 1.0 - smooth_unit(elapsed / lifetime);
                                    let phase = ev.phase + (x - ev.origin_x) * 0.045 + elapsed * 0.12;
                                    let (lr, lg, lb) = sample_palette_linear(&user_palette, phase);
                                    sample_r += lr * intensity * edge_fade;
                                    sample_g += lg * intensity * edge_fade;
                                    sample_b += lb * intensity * edge_fade;
                                }
                            }
                        }
                    }
                    sum_r += sample_r.min(1.0);
                    sum_g += sample_g.min(1.0);
                    sum_b += sample_b.min(1.0);
                }
                set_key_from_linear(buf, key.offset, sum_r / 4.0, sum_g / 4.0, sum_b / 4.0);
            }
            *t += 1.0;
        }

        // 13: Static
        13 => {
            for key in keymap().iter() {
                set_key(buf, key.offset, pr, pg, pb);
            }
        }

        // BetterRGB Custom BRFX Engine Presets (200+)
        200..=255 => {
            let custom_engine = crate::driver::custom_fx_engine::get_custom_fx_engine();
            if custom_engine.has_active_script() {
                if custom_engine.render_frame(
                    *t,
                    speed_mult,
                    1.0,
                    params.direction as i64,
                    false,
                    &params.color_hex,
                    buf,
                ) {
                    *t += 0.033;
                    return;
                }
            }

            match params.effect {
                // 200: Kamehameha vs Final Flash
                200 => {
                    let t_cycle = (*t * speed_mult * 1.0).rem_euclid(6.5);
                    let y0 = 2.5;
                    let clash_x = 9.0 + 1.1 * (*t * 12.0).sin() * (0.7 + 0.3 * (*t * 23.0).cos());

                    for key in keymap().iter() {
                        let kx = get_key_x(key);
                        let ky = get_key_y(key);
                        let dist_y = (ky - y0).abs();
                        let mut r: f64 = 0.0;
                        let mut g: f64 = 0.0;
                        let mut b: f64 = 0.0;

                        if t_cycle < 1.2 {
                            let head_x = -1.0 + (t_cycle / 1.0) * 23.0;
                            let tail_x = 0.0;

                            if kx <= head_x && kx >= tail_x {
                                let d_head = ((kx - head_x).powi(2) + dist_y.powi(2)).sqrt();
                                let head_radius = 2.5;
                                if d_head < head_radius {
                                    if d_head < 1.1 {
                                        r += 255.0;
                                        g += 255.0;
                                        b += 255.0;
                                    } else {
                                        let outer = 1.0 - (d_head - 1.1) / (head_radius - 1.1);
                                        r += 170.0 * outer;
                                        g += 235.0 * outer;
                                        b += 255.0 * outer;
                                    }
                                }

                                let core_h = 0.60;
                                if dist_y < core_h {
                                    let core = (1.0 - dist_y / core_h).powi(2);
                                    r += 240.0 * core;
                                    g += 250.0 * core;
                                    b += 255.0 * core;
                                }
                                let halo_h = 1.25;
                                if dist_y < halo_h {
                                    let halo = 1.0 - dist_y / halo_h;
                                    let ripple = (kx * 4.0 - *t * 30.0).sin() * 0.15;
                                    r += 20.0 * halo;
                                    g += (140.0 * halo + 40.0 * ripple).clamp(0.0, 255.0);
                                    b += 255.0 * halo;
                                }

                                if kx < 2.5 {
                                    let flare = (1.0 - kx / 2.5).max(0.0) * (1.0 - dist_y / 1.8).max(0.0);
                                    r += 60.0 * flare;
                                    g += 180.0 * flare;
                                    b += 255.0 * flare;
                                }
                            }
                        } else if t_cycle < 2.4 {
                            let p2_t = t_cycle - 1.2;
                            let head_x = 19.5 - (p2_t / 1.0) * 23.0;
                            let tail_x = 18.5;

                            if kx >= head_x && kx <= tail_x {
                                let d_head = ((kx - head_x).powi(2) + dist_y.powi(2)).sqrt();
                                let head_radius = 2.5;
                                if d_head < head_radius {
                                    if d_head < 1.1 {
                                        r += 255.0;
                                        g += 255.0;
                                        b += 220.0;
                                    } else {
                                        let outer = 1.0 - (d_head - 1.1) / (head_radius - 1.1);
                                        r += 255.0 * outer;
                                        g += 215.0 * outer;
                                        b += 70.0 * outer;
                                    }
                                }

                                let core_h = 0.60;
                                if dist_y < core_h {
                                    let core = (1.0 - dist_y / core_h).powi(2);
                                    r += 255.0 * core;
                                    g += 250.0 * core;
                                    b += 160.0 * core;
                                }
                                let halo_h = 1.25;
                                if dist_y < halo_h {
                                    let halo = 1.0 - dist_y / halo_h;
                                    let ripple = (kx * 4.0 + *t * 30.0).sin() * 0.15;
                                    r += (255.0 * halo + 40.0 * ripple).clamp(0.0, 255.0);
                                    g += 160.0 * halo;
                                    b += 20.0 * halo;
                                }

                                if kx > 16.0 {
                                    let flare = ((kx - 16.0) / 2.5).max(0.0) * (1.0 - dist_y / 1.8).max(0.0);
                                    r += 255.0 * flare;
                                    g += 180.0 * flare;
                                    b += 40.0 * flare;
                                }
                            }
                        } else if t_cycle < 5.4 {
                            let p3_t = t_cycle - 2.4;
                            let (left_tip, right_tip) = if p3_t < 0.4 {
                                let l = -1.0 + (p3_t / 0.4) * (clash_x + 1.0);
                                let r_pos = 19.5 - (p3_t / 0.4) * (19.5 - clash_x);
                                (l.min(clash_x), r_pos.max(clash_x))
                            } else {
                                (clash_x, clash_x)
                            };

                            if kx <= left_tip {
                                let d_head_l = ((kx - left_tip).powi(2) + dist_y.powi(2)).sqrt();
                                let head_radius = 2.5;
                                if d_head_l < head_radius {
                                    if d_head_l < 1.1 {
                                        r += 255.0;
                                        g += 255.0;
                                        b += 255.0;
                                    } else {
                                        let outer = 1.0 - (d_head_l - 1.1) / (head_radius - 1.1);
                                        r += 170.0 * outer;
                                        g += 235.0 * outer;
                                        b += 255.0 * outer;
                                    }
                                }

                                let core_h = 0.50;
                                if dist_y < core_h {
                                    let core = (1.0 - dist_y / core_h).powi(2);
                                    r += 240.0 * core;
                                    g += 255.0 * core;
                                    b += 255.0 * core;
                                }
                                let halo_h = 1.05;
                                if dist_y < halo_h {
                                    let halo = 1.0 - dist_y / halo_h;
                                    r += 10.0 * halo;
                                    g += 130.0 * halo;
                                    b += 255.0 * halo;
                                }
                            }

                            if kx >= right_tip {
                                let d_head_r = ((kx - right_tip).powi(2) + dist_y.powi(2)).sqrt();
                                let head_radius = 2.5;
                                if d_head_r < head_radius {
                                    if d_head_r < 1.1 {
                                        r += 255.0;
                                        g += 255.0;
                                        b += 220.0;
                                    } else {
                                        let outer = 1.0 - (d_head_r - 1.1) / (head_radius - 1.1);
                                        r += 255.0 * outer;
                                        g += 215.0 * outer;
                                        b += 70.0 * outer;
                                    }
                                }

                                let core_h = 0.50;
                                if dist_y < core_h {
                                    let core = (1.0 - dist_y / core_h).powi(2);
                                    r += 255.0 * core;
                                    g += 245.0 * core;
                                    b += 180.0 * core;
                                }
                                let halo_h = 1.05;
                                if dist_y < halo_h {
                                    let halo = 1.0 - dist_y / halo_h;
                                    r += 255.0 * halo;
                                    g += 150.0 * halo;
                                    b += 10.0 * halo;
                                }
                            }

                            let d_clash = ((kx - clash_x).powi(2) + (dist_y * 1.3).powi(2)).sqrt();
                            if p3_t >= 0.4 {
                                let vibe = 0.85 + 0.15 * (*t * 45.0).sin();
                                let r_clash = 2.5 + 0.3 * (*t * 18.0).sin();
                                if d_clash < r_clash {
                                    let clash_glow = (1.0 - d_clash / r_clash) * vibe;
                                    r += 255.0 * clash_glow * 1.3;
                                    g += 255.0 * clash_glow * 1.3;
                                    b += 255.0 * clash_glow * 1.3;
                                }

                                let dx_clash = (kx - clash_x).abs();
                                if dx_clash < 1.6 {
                                    let vert = (1.0 - dx_clash / 1.6) * (1.0 - dist_y / 2.8).max(0.0) * vibe;
                                    if kx <= clash_x {
                                        r += 120.0 * vert;
                                        g += 220.0 * vert;
                                        b += 255.0 * vert;
                                    } else {
                                        r += 255.0 * vert;
                                        g += 210.0 * vert;
                                        b += 70.0 * vert;
                                    }
                                }

                                let spark_seed = (kx * 19.37 + ky * 41.29 + *t * 55.0).sin();
                                if spark_seed > 0.82 && d_clash < 7.5 {
                                    let arc_pwr = ((spark_seed - 0.82) / 0.18) * (1.0 - d_clash / 7.5);
                                    if kx <= clash_x {
                                        r += 190.0 * arc_pwr;
                                        g += 235.0 * arc_pwr;
                                        b += 255.0 * arc_pwr;
                                    } else {
                                        r += 255.0 * arc_pwr;
                                        g += 220.0 * arc_pwr;
                                        b += 90.0 * arc_pwr;
                                    }
                                }
                            }
                        } else {
                            let p4_t = t_cycle - 5.4;
                            let ring_r = p4_t * 16.0;
                            let d_center = ((kx - 9.0).powi(2) + dist_y.powi(2)).sqrt();
                            let ring_dist = (d_center - ring_r).abs();
                            let fade = (1.0 - p4_t / 1.1).max(0.0);

                            if ring_dist < 1.6 {
                                let burst = (1.0 - ring_dist / 1.6) * fade;
                                r += 255.0 * burst;
                                g += 240.0 * burst;
                                b += 255.0 * burst;
                            }
                            if d_center < 3.0 {
                                let ember = (1.0 - d_center / 3.0) * fade * fade;
                                r += 160.0 * ember;
                                g += 190.0 * ember;
                                b += 255.0 * ember;
                            }
                        }

                        let final_r = (r.clamp(0.0, 255.0) * (pr as f64 / 255.0).max(0.4)).min(255.0) as u8;
                        let final_g = (g.clamp(0.0, 255.0) * (pg as f64 / 255.0).max(0.4)).min(255.0) as u8;
                        let final_b = (b.clamp(0.0, 255.0) * (pb as f64 / 255.0).max(0.4)).min(255.0) as u8;
                        set_key(buf, key.offset, final_r, final_g, final_b);
                    }
                    *t += 0.033 * speed_mult;
                }

                // 201: Cyber Lotus
                201 => {
                    let time_val = *t * speed_mult;
                    let spin = time_val * 0.6;
                    for key in keymap().iter() {
                        let kx = get_key_x(key);
                        let ky = get_key_y(key);
                        let dx = kx - 7.0;
                        let dy = (ky - 2.5) * 1.5;
                        let r_dist = (dx * dx + dy * dy).sqrt();
                        let angle = dy.atan2(dx);

                        let breathe = (time_val * 1.5).sin() * 0.4;
                        let petal_lobe = (6.0 * (angle + spin)).cos();
                        let petal_r = 3.6 + breathe + 1.6 * petal_lobe;
                        let inner_lobe = (3.0 * (angle - spin * 1.3)).cos();
                        let inner_r = 1.8 + 0.5 * inner_lobe;

                        let mut r: f64 = 4.0;
                        let mut g: f64 = 14.0;
                        let mut b: f64 = 26.0;

                        if r_dist < inner_r {
                            let core_w = 1.0 - r_dist / inner_r;
                            let pulse = 0.8 + 0.2 * (time_val * 4.0).sin();
                            r += 240.0 * core_w * pulse;
                            g += 210.0 * core_w * pulse;
                            b += 60.0 * core_w;
                        } else if r_dist < petal_r {
                            let petal_pos = (r_dist - inner_r) / (petal_r - inner_r);
                            let edge_dist = (r_dist - petal_r).abs();
                            let edge_glow = (1.0 - edge_dist / 1.2).clamp(0.0, 1.0);
                            r += 10.0 + 80.0 * petal_pos + 120.0 * edge_glow;
                            g += 160.0 + 95.0 * (1.0 - petal_pos);
                            b += 255.0 * (1.0 - petal_pos * 0.3);
                        } else {
                            let ripple = (r_dist * 1.6 - time_val * 3.5).sin();
                            if ripple > 0.6 {
                                let rip_pwr = ((ripple - 0.6) / 0.4) * (1.0 - r_dist / 12.0).max(0.0) * 0.4;
                                r += 10.0 * rip_pwr;
                                g += 150.0 * rip_pwr;
                                b += 240.0 * rip_pwr;
                            }
                        }

                        let star_freq = 1.1 + (kx * 17.31 + ky * 31.17).sin() * 0.5;
                        let star_phase = kx * 4.83 + ky * 9.27;
                        let p1 = (time_val * star_freq + star_phase).sin();
                        let p2 = (time_val * (star_freq * 0.47) + star_phase * 1.618).sin();
                        let spark = p1 * p2;

                        if spark > 0.60 {
                            let star_intensity = ((spark - 0.60) / 0.40).powf(2.2);
                            let star_seed = (kx * 37.19 + ky * 53.81).sin();
                            if star_seed > 0.3 {
                                r += 140.0 * star_intensity;
                                g += 240.0 * star_intensity;
                                b += 255.0 * star_intensity;
                            } else if star_seed > -0.3 {
                                r += 30.0 * star_intensity;
                                g += 255.0 * star_intensity;
                                b += 190.0 * star_intensity;
                            } else {
                                r += 255.0 * star_intensity;
                                g += 225.0 * star_intensity;
                                b += 90.0 * star_intensity;
                            }
                        }

                        let final_r = ((r.clamp(0.0, 255.0) * (pr as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        let final_g = ((g.clamp(0.0, 255.0) * (pg as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        let final_b = ((b.clamp(0.0, 255.0) * (pb as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        set_key(buf, key.offset, final_r, final_g, final_b);
                    }
                    *t += 0.033 * speed_mult;
                }

                // 202: Singularity
                202 => {
                    let time_val = *t * speed_mult;
                    for key in keymap().iter() {
                        let kx = get_key_x(key);
                        let ky = get_key_y(key);
                        let dx = kx - 8.5;
                        let dy = (ky - 2.5) * 1.5;
                        let r_dist = (dx * dx + dy * dy).sqrt();
                        let angle = dy.atan2(dx);

                        let event_horizon = 1.2;
                        if r_dist < event_horizon {
                            set_key(buf, key.offset, 0, 0, 0);
                            continue;
                        }

                        let spin = time_val * 2.5 - r_dist * 1.2;
                        let spiral = (angle * 3.0 + spin).sin();

                        let mut r: f64 = 0.0;
                        let mut g: f64 = 0.0;
                        let mut b: f64 = 0.0;

                        if r_dist >= event_horizon && r_dist < 2.0 {
                            let ring_intensity = 1.0 - (r_dist - event_horizon) / 0.8;
                            r = 255.0 * ring_intensity;
                            g = 220.0 * ring_intensity;
                            b = 255.0 * ring_intensity;
                        } else if r_dist < 7.5 {
                            let disk_intensity = (1.0 - (r_dist - 2.0) / 5.5) * (0.6 + 0.4 * spiral);
                            let doppler = angle.cos();
                            if doppler > 0.0 {
                                r = (180.0 + 75.0 * doppler) * disk_intensity;
                                g = (100.0 + 100.0 * doppler) * disk_intensity;
                                b = 255.0 * disk_intensity;
                            } else {
                                r = 255.0 * disk_intensity;
                                g = (120.0 + 80.0 * doppler) * disk_intensity;
                                b = 40.0 * disk_intensity;
                            }
                        }

                        let final_r = ((r * (pr as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        let final_g = ((g * (pg as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        let final_b = ((b * (pb as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        set_key(buf, key.offset, final_r, final_g, final_b);
                    }
                    *t += 0.033 * speed_mult;
                }

                // 203: Cyber Marquee
                203 => {
                    let custom_engine = crate::driver::custom_fx_engine::get_custom_fx_engine();
                    let text = custom_engine.get_active_text();
                    let speed = speed_mult * 6.5;
                    let scroll_offset = *t * speed;

                    for key in keymap().iter() {
                        let kx = get_key_x(key);
                        let ky = get_key_y(key);

                        if ky > 5.05 {
                            set_key(buf, key.offset, 0, 0, 0);
                            continue;
                        }

                        let offset_x = kx - 19.0 + scroll_offset;
                        let hit = crate::driver::custom_fx_engine::sample_text_pixel(&text, offset_x, ky);

                        let mut r = 3.0;
                        let mut g = 8.0;
                        let mut b = 18.0;

                        if hit > 0.05 {
                            let hue = (kx * 12.0 - *t * 30.0).rem_euclid(360.0);
                            let (hr, hg, hb) = hsv_to_rgb(hue, 0.95, hit);
                            let white_core = hit * hit * 60.0;
                            r = hr as f64 + white_core;
                            g = hg as f64 + white_core;
                            b = hb as f64 + white_core;
                        }

                        let final_r = ((r.clamp(0.0, 255.0) * (pr as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        let final_g = ((g.clamp(0.0, 255.0) * (pg as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        let final_b = ((b.clamp(0.0, 255.0) * (pb as f64 / 255.0).max(0.4)).min(255.0)) as u8;
                        set_key(buf, key.offset, final_r, final_g, final_b);
                    }
                    *t += 0.033 * speed_mult;
                }

                _ => {}
            }
        }

        // Fallback default: Wave
        _ => {
            for key in keymap().iter() {
                let kx = get_key_x(key);
                let ky = get_key_y(key);
                let hue = (*t + kx * 15.0 + ky * 20.0).rem_euclid(360.0);
                let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
                set_key(buf, key.offset, r, g, b);
            }
            *t = (*t + 8.0 * speed_mult).rem_euclid(360.0);
        }
    }
}
