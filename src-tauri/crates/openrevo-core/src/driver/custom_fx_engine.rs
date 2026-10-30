use std::sync::Arc;
use parking_lot::RwLock;
use super::ite_streamer::{BUF_SIZE, set_key, keymap, get_key_x, get_key_y};

pub struct CustomFxEngine {
    engine: rhai::Engine,
    active_ast: RwLock<Option<Arc<rhai::AST>>>,
    active_name: RwLock<String>,
    active_text: RwLock<String>,
}

pub static GLOBAL_CUSTOM_FX_ENGINE: std::sync::OnceLock<CustomFxEngine> = std::sync::OnceLock::new();

pub fn get_custom_fx_engine() -> &'static CustomFxEngine {
    GLOBAL_CUSTOM_FX_ENGINE.get_or_init(CustomFxEngine::new)
}

impl CustomFxEngine {
    pub fn new() -> Self {
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(50_000);

        // Register standard 2D math & geometry utilities
        engine.register_fn("sin", |x: f64| x.sin());
        engine.register_fn("sin", |x: i64| (x as f64).sin());
        engine.register_fn("cos", |x: f64| x.cos());
        engine.register_fn("cos", |x: i64| (x as f64).cos());
        engine.register_fn("tan", |x: f64| x.tan());
        engine.register_fn("tan", |x: i64| (x as f64).tan());
        engine.register_fn("sqrt", |x: f64| if x >= 0.0 { x.sqrt() } else { 0.0 });
        engine.register_fn("sqrt", |x: i64| if x >= 0 { (x as f64).sqrt() } else { 0.0 });
        engine.register_fn("abs", |x: f64| x.abs());
        engine.register_fn("abs", |x: i64| x.abs() as f64);
        engine.register_fn("pow", |x: f64, p: f64| x.powf(p));
        engine.register_fn("pow", |x: f64, p: i64| x.powf(p as f64));
        engine.register_fn("min", |a: f64, b: f64| a.min(b));
        engine.register_fn("min", |a: i64, b: i64| a.min(b) as f64);
        engine.register_fn("max", |a: f64, b: f64| a.max(b));
        engine.register_fn("max", |a: i64, b: i64| a.max(b) as f64);
        engine.register_fn("clamp", |v: f64, min: f64, max: f64| v.clamp(min, max));
        engine.register_fn("dist", |x1: f64, y1: f64, x2: f64, y2: f64| {
            ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
        });
        engine.register_fn("dist", |x1: f64, y1: f64, x2: i64, y2: i64| {
            ((x1 - x2 as f64).powi(2) + (y1 - y2 as f64).powi(2)).sqrt()
        });
        engine.register_fn("atan2", |y: f64, x: f64| y.atan2(x));
        engine.register_fn("floor", |x: f64| x.floor());
        engine.register_fn("ceil", |x: f64| x.ceil());
        engine.register_fn("smoothstep", |edge0: f64, edge1: f64, x: f64| {
            if (edge1 - edge0).abs() < 1e-9 {
                return if x >= edge1 { 1.0 } else { 0.0 };
            }
            let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        });
        engine.register_fn("hsv", |h: f64, s: f64, v: f64| {
            let (r, g, b) = super::ite_streamer::hsv_to_rgb(h, s, v);
            let mut arr = rhai::Array::new();
            arr.push(rhai::Dynamic::from_float(r as f64));
            arr.push(rhai::Dynamic::from_float(g as f64));
            arr.push(rhai::Dynamic::from_float(b as f64));
            arr
        });
        engine.register_fn("sample_text", |text: &str, x: f64, y: f64| -> f64 {
            sample_text_pixel(text, x, y)
        });
        engine.register_fn("sample_text", |text: rhai::ImmutableString, x: f64, y: f64| -> f64 {
            sample_text_pixel(&text, x, y)
        });
        engine.register_fn("get_text_width", |text: &str| -> f64 {
            get_text_pixel_width(text)
        });
        engine.register_fn("get_text_width", |text: rhai::ImmutableString| -> f64 {
            get_text_pixel_width(&text)
        });

        Self {
            engine,
            active_ast: RwLock::new(None),
            active_name: RwLock::new(String::new()),
            active_text: RwLock::new("I LOVE OPEN-REVO".to_string()),
        }
    }

    pub fn has_active_script(&self) -> bool {
        self.active_ast.read().is_some()
    }

    pub fn get_active_name(&self) -> String {
        self.active_name.read().clone()
    }

    pub fn set_active_text(&self, text: &str) {
        *self.active_text.write() = text.to_string();
        crate::log_info!("CUSTOM_FX", "Set active custom FX text: {}", text);
    }

    pub fn get_active_text(&self) -> String {
        self.active_text.read().clone()
    }

    pub fn load_script(&self, name: &str, content: &str) -> Result<(), String> {
        match self.engine.compile(content) {
            Ok(ast) => {
                let mut guard = self.active_ast.write();
                *guard = Some(Arc::new(ast));
                *self.active_name.write() = name.to_string();
                crate::log_info!("CUSTOM_FX", "Successfully compiled and hot-reloaded BRFX script: {}", name);
                Ok(())
            }
            Err(err) => {
                crate::log_error!("CUSTOM_FX", "Failed to compile BRFX script {}: {}", name, err);
                Err(format!("编译脚本错误: {}", err))
            }
        }
    }

    pub fn clear_script(&self) {
        let mut guard = self.active_ast.write();
        *guard = None;
        *self.active_name.write() = String::new();
    }

    pub fn render_frame(
        &self,
        t: f64,
        speed: f64,
        scale: f64,
        direction: i64,
        reactive: bool,
        color: &str,
        buf: &mut [u8; BUF_SIZE],
    ) -> bool {
        let ast = match self.active_ast.read().clone() {
            Some(a) => a,
            None => return false,
        };

        let cur_text = self.active_text.read().clone();
        let mut map = rhai::Map::new();
        map.insert("t".into(), rhai::Dynamic::from_float(t));
        map.insert("speed".into(), rhai::Dynamic::from_float(speed));
        map.insert("scale".into(), rhai::Dynamic::from_float(scale));
        map.insert("direction".into(), rhai::Dynamic::from_int(direction));
        map.insert("reactive".into(), rhai::Dynamic::from_bool(reactive));
        map.insert("color".into(), rhai::Dynamic::from(color.to_string()));
        map.insert("text".into(), rhai::Dynamic::from(cur_text));
        let ctx = rhai::Dynamic::from_map(map);

        for key in keymap().iter() {
            let kx = get_key_x(key);
            let ky = get_key_y(key);
            let mut scope = rhai::Scope::new();
            match self.engine.call_fn::<rhai::Array>(&mut scope, &ast, "render_key", (kx, ky, ctx.clone())) {
                Ok(arr) => {
                    if arr.len() >= 3 {
                        let r = arr[0].as_float().or_else(|_| arr[0].as_int().map(|v| v as f64)).unwrap_or(0.0);
                        let g = arr[1].as_float().or_else(|_| arr[1].as_int().map(|v| v as f64)).unwrap_or(0.0);
                        let b = arr[2].as_float().or_else(|_| arr[2].as_int().map(|v| v as f64)).unwrap_or(0.0);
                        set_key(buf, key.offset, r.clamp(0.0, 255.0) as u8, g.clamp(0.0, 255.0) as u8, b.clamp(0.0, 255.0) as u8);
                    }
                }
                Err(e) => {
                    crate::log_warn!("CUSTOM_FX", "Runtime error in render_key: {}", e);
                    return false;
                }
            }
        }
        true
    }
}

pub fn get_glyph_bitmap(ch: char) -> (usize, [u8; 6]) {
    let upper = ch.to_ascii_uppercase();
    match upper {
        'A' => (4, [0b0110, 0b1001, 0b1111, 0b1001, 0b1001, 0b1001]),
        'B' => (4, [0b1110, 0b1001, 0b1110, 0b1001, 0b1001, 0b1110]),
        'C' => (4, [0b0111, 0b1000, 0b1000, 0b1000, 0b1000, 0b0111]),
        'D' => (4, [0b1110, 0b1001, 0b1001, 0b1001, 0b1001, 0b1110]),
        'E' => (4, [0b1111, 0b1000, 0b1110, 0b1110, 0b1000, 0b1111]),
        'F' => (4, [0b1111, 0b1000, 0b1110, 0b1000, 0b1000, 0b1000]),
        'G' => (4, [0b0111, 0b1000, 0b1011, 0b1001, 0b1001, 0b0111]),
        'H' => (4, [0b1001, 0b1001, 0b1111, 0b1111, 0b1001, 0b1001]),
        'I' => (3, [0b111, 0b010, 0b010, 0b010, 0b010, 0b111]),
        'J' => (4, [0b0011, 0b0001, 0b0001, 0b0001, 0b1001, 0b0110]),
        'K' => (4, [0b1001, 0b1010, 0b1100, 0b1100, 0b1010, 0b1001]),
        'L' => (4, [0b1000, 0b1000, 0b1000, 0b1000, 0b1000, 0b1111]),
        'M' => (5, [0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001]),
        'N' => (4, [0b1001, 0b1101, 0b1101, 0b1011, 0b1011, 0b1001]),
        'O' => (4, [0b0110, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110]),
        'P' => (4, [0b1110, 0b1001, 0b1110, 0b1000, 0b1000, 0b1000]),
        'Q' => (4, [0b0110, 0b1001, 0b1001, 0b1011, 0b1001, 0b0111]),
        'R' => (4, [0b1110, 0b1001, 0b1110, 0b1100, 0b1010, 0b1001]),
        'S' => (4, [0b0111, 0b1000, 0b0110, 0b0001, 0b0001, 0b1110]),
        'T' => (5, [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        'U' => (4, [0b1001, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110]),
        'V' => (5, [0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100]),
        'W' => (5, [0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
        'X' => (4, [0b1001, 0b1001, 0b0110, 0b0110, 0b1001, 0b1001]),
        'Y' => (5, [0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        'Z' => (4, [0b1111, 0b0001, 0b0010, 0b0100, 0b1000, 0b1111]),
        '0' => (4, [0b0110, 0b1011, 0b1101, 0b1001, 0b1001, 0b0110]),
        '1' => (3, [0b010, 0b110, 0b010, 0b010, 0b010, 0b111]),
        '2' => (4, [0b0110, 0b1001, 0b0010, 0b0100, 0b1000, 0b1111]),
        '3' => (4, [0b1110, 0b0001, 0b0110, 0b0001, 0b0001, 0b1110]),
        '4' => (4, [0b1001, 0b1001, 0b1111, 0b0001, 0b0001, 0b0001]),
        '5' => (4, [0b1111, 0b1000, 0b1110, 0b0001, 0b0001, 0b1110]),
        '6' => (4, [0b0111, 0b1000, 0b1110, 0b1001, 0b1001, 0b0110]),
        '7' => (4, [0b1111, 0b0001, 0b0010, 0b0100, 0b0100, 0b0100]),
        '8' => (4, [0b0110, 0b1001, 0b0110, 0b1001, 0b1001, 0b0110]),
        '9' => (4, [0b0110, 0b1001, 0b1001, 0b0111, 0b0001, 0b1110]),
        '-' => (3, [0b000, 0b000, 0b111, 0b111, 0b000, 0b000]),
        '_' => (4, [0b0000, 0b0000, 0b0000, 0b0000, 0b0000, 0b1111]),
        '!' => (1, [0b1, 0b1, 0b1, 0b1, 0b0, 0b1]),
        '.' => (1, [0b0, 0b0, 0b0, 0b0, 0b0, 0b1]),
        ':' => (1, [0b0, 0b1, 0b0, 0b1, 0b0, 0b0]),
        '+' => (3, [0b000, 0b010, 0b111, 0b010, 0b000, 0b000]),
        '/' => (3, [0b001, 0b001, 0b010, 0b010, 0b100, 0b100]),
        '♥' => (5, [0b01010, 0b11111, 0b11111, 0b01110, 0b00100, 0b00000]),
        ' ' => (3, [0, 0, 0, 0, 0, 0]),
        _ => (3, [0b111, 0b101, 0b101, 0b101, 0b101, 0b111]),
    }
}

pub fn get_text_pixel_width(text: &str) -> f64 {
    let effective = if text.is_empty() { "I LOVE OPEN-REVO" } else { text };
    let mut total = 0.0;
    for ch in effective.chars() {
        let (w, _) = get_glyph_bitmap(ch);
        total += w as f64 + 1.0;
    }
    total
}

pub fn sample_text_pixel(text: &str, x: f64, y: f64) -> f64 {
    if y < -0.45 || y > 5.45 {
        return 0.0;
    }
    let row = (y.round().clamp(0.0, 5.0)) as usize;

    let chars: Vec<char> = if text.is_empty() {
        "I LOVE OPEN-REVO".chars().collect()
    } else {
        text.chars().collect()
    };

    let mut total_width = 0.0;
    let mut glyph_ranges = Vec::with_capacity(chars.len());
    for &ch in &chars {
        let (w, mask) = get_glyph_bitmap(ch);
        glyph_ranges.push((total_width, w, mask));
        total_width += w as f64 + 1.0;
    }

    if total_width <= 0.0 {
        return 0.0;
    }

    let loop_len = total_width + 20.0;
    let banner_x = x.rem_euclid(loop_len);

    for (start_x, w, mask) in glyph_ranges {
        if banner_x >= start_x && banner_x < start_x + (w as f64) {
            let col = (banner_x - start_x).floor() as usize;
            if col < w {
                let bit = (mask[row] >> (w - 1 - col)) & 1;
                return if bit == 1 { 1.0 } else { 0.0 };
            }
        }
    }

    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marquee_script() {
        let engine = CustomFxEngine::new();
        let script = r#"
fn render_key(kx, ky, ctx) {
    if ky > 5.05 {
        return [0.0, 0.0, 0.0];
    }

    let text = if ctx.text != "" { ctx.text } else { "I LOVE OPEN-REVO" };
    let speed = ctx.speed * 6.5;
    
    let offset_x = kx - 19.0 + ctx.t * speed;
    let hit = sample_text(text, offset_x, ky);

    let r = 3.0;
    let g = 8.0;
    let b = 18.0;

    if hit > 0.05 {
        let hue = ((kx * 12.0 - ctx.t * 30.0) % 360.0);
        if hue < 0.0 { hue += 360.0; }
        
        let c = hsv(hue, 0.95, hit);
        
        let white_core = hit * hit * 60.0;
        r = c[0] + white_core;
        g = c[1] + white_core;
        b = c[2] + white_core;
    }

    return [clamp(r, 0.0, 255.0), clamp(g, 0.0, 255.0), clamp(b, 0.0, 255.0)];
}
"#;
        let res = engine.load_script("marquee", script);
        assert!(res.is_ok(), "Compile error: {:?}", res.err());
        let mut buf = [0u8; 512];
        let render_res = engine.render_frame(3.0, 1.0, 1.0, 0, false, "#00F0FF", &mut buf);
        assert!(render_res, "render_frame returned false!");
    }
}
