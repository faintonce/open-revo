use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlintThemeTokens {
    pub theme_mode: u8,

    // 1. Surfaces & Backgrounds
    pub bg_root: String,
    pub surface_base: String,
    pub surface_card: String,
    pub surface_card_hover: String,
    pub surface_pill: String,
    pub surface_elevated: String,
    pub surface_sunken: String,

    // 2. Borders & Dividers
    pub border_subtle: String,
    pub border_medium: String,
    pub border_highlight: String,

    // 3. Typography
    pub text_primary: String,
    pub text_secondary: String,
    pub text_muted: String,
    pub font_family: String,

    // 4. Semantic Accents
    pub accent_brand: String,
    pub mode_office: String,
    pub mode_balance: String,
    pub mode_beast: String,
    pub accent_fan: String,
    pub accent_gold: String,

    // 5. Interactive & Navigation States
    pub accent_active: String,
    pub accent_active_bg: String,
    pub accent_active_border: String,
    pub accent_heart: String,
    pub accent_heart_bg: String,
    pub accent_heart_border: String,
    pub tag_brand_text: String,

    // 6. Geometry
    pub radius_xs: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,

    // 7. Background Image
    pub bg_image_path: String,
    pub bg_image_opacity: f32,
    pub drawer_bg_image_path: String,
    pub drawer_bg_image_opacity: f32,
}

impl SlintThemeTokens {
    pub fn from_builtin_mode(theme_mode: u8) -> Self {
        match theme_mode {
            // 1: Light
            1 => Self {
                theme_mode: 1,
                bg_root: "#f3f4f6".into(),
                surface_base: "#ffffff".into(),
                surface_card: "#ffffff".into(),
                surface_card_hover: "#f9fafb".into(),
                surface_pill: "#e5e7eb".into(),
                surface_elevated: "#ffffff".into(),
                surface_sunken: "#e5e7eb".into(),
                border_subtle: "#e5e7eb".into(),
                border_medium: "#d1d5db".into(),
                border_highlight: "#9ca3af".into(),
                text_primary: "#111827".into(),
                text_secondary: "#4b5563".into(),
                text_muted: "#9ca3af".into(),
                font_family: String::new(),
                accent_brand: "#6366f1".into(),
                mode_office: "#10b981".into(),
                mode_balance: "#3b82f6".into(),
                mode_beast: "#f43f5e".into(),
                accent_fan: "#06b6d4".into(),
                accent_gold: "#f59e0b".into(),
                accent_active: "#0284c7".into(),
                accent_active_bg: "#0284c718".into(),
                accent_active_border: "#0284c740".into(),
                accent_heart: "#f43f5e".into(),
                accent_heart_bg: "#f43f5e18".into(),
                accent_heart_border: "#f43f5e40".into(),
                tag_brand_text: "#ffffff".into(),
                radius_xs: 4.0,
                radius_sm: 6.0,
                radius_md: 8.0,
                radius_lg: 12.0,
                bg_image_path: String::new(),
                bg_image_opacity: 0.30,
                drawer_bg_image_path: String::new(),
                drawer_bg_image_opacity: 0.30,
            },
            // 2: Mono (Geek High-Contrast)
            2 => Self {
                theme_mode: 2,
                bg_root: "#000000".into(),
                surface_base: "#0d0d0d".into(),
                surface_card: "#161616".into(),
                surface_card_hover: "#222222".into(),
                surface_pill: "#262626".into(),
                surface_elevated: "#1f1f1f".into(),
                surface_sunken: "#080808".into(),
                border_subtle: "#262626".into(),
                border_medium: "#3d3d3d".into(),
                border_highlight: "#666666".into(),
                text_primary: "#ffffff".into(),
                text_secondary: "#cccccc".into(),
                text_muted: "#777777".into(),
                font_family: String::new(),
                accent_brand: "#ffffff".into(),
                mode_office: "#ffffff".into(),
                mode_balance: "#ffffff".into(),
                mode_beast: "#ffffff".into(),
                accent_fan: "#ffffff".into(),
                accent_gold: "#ffffff".into(),
                accent_active: "#ffffff".into(),
                accent_active_bg: "#ffffff22".into(),
                accent_active_border: "#ffffff60".into(),
                accent_heart: "#ffffff".into(),
                accent_heart_bg: "#ffffff22".into(),
                accent_heart_border: "#ffffff60".into(),
                tag_brand_text: "#000000".into(),
                radius_xs: 4.0,
                radius_sm: 6.0,
                radius_md: 8.0,
                radius_lg: 12.0,
                bg_image_path: String::new(),
                bg_image_opacity: 0.25,
                drawer_bg_image_path: String::new(),
                drawer_bg_image_opacity: 0.25,
            },
            // 0: Dark (Default)
            _ => Self {
                theme_mode: 0,
                bg_root: "#0b0e14".into(),
                surface_base: "#10141d".into(),
                surface_card: "#161b24".into(),
                surface_card_hover: "#1f2633".into(),
                surface_pill: "#202736".into(),
                surface_elevated: "#1b212c".into(),
                surface_sunken: "#080a0f".into(),
                border_subtle: "#1f2736".into(),
                border_medium: "#2e384d".into(),
                border_highlight: "#475569".into(),
                text_primary: "#f8fafc".into(),
                text_secondary: "#cbd5e1".into(),
                text_muted: "#64748b".into(),
                font_family: String::new(),
                accent_brand: "#6366f1".into(),
                mode_office: "#10b981".into(),
                mode_balance: "#3b82f6".into(),
                mode_beast: "#f43f5e".into(),
                accent_fan: "#06b6d4".into(),
                accent_gold: "#f59e0b".into(),
                accent_active: "#38bdf8".into(),
                accent_active_bg: "#38bdf81a".into(),
                accent_active_border: "#38bdf860".into(),
                accent_heart: "#f43f5e".into(),
                accent_heart_bg: "#f43f5e18".into(),
                accent_heart_border: "#f43f5e60".into(),
                tag_brand_text: "#ffffff".into(),
                radius_xs: 4.0,
                radius_sm: 6.0,
                radius_md: 8.0,
                radius_lg: 12.0,
                bg_image_path: String::new(),
                bg_image_opacity: 0.30,
                drawer_bg_image_path: String::new(),
                drawer_bg_image_opacity: 0.30,
            },
        }
    }
}

/// Normalize a CSS color (`#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(...)`, `rgba(...)`) to `#rrggbb` or `#rrggbbaa`.
fn normalize_css_color(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.starts_with('#') {
        let hex = &s[1..];
        if hex.len() == 3 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            let mut out = String::from("#");
            for ch in hex.chars() {
                out.push(ch);
                out.push(ch);
            }
            return Some(out);
        }
        if (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(format!("#{}", hex.to_lowercase()));
        }
        return None;
    }

    let lower = s.to_ascii_lowercase();
    if (lower.starts_with("rgba(") || lower.starts_with("rgb(")) && lower.ends_with(')') {
        let inner = lower
            .trim_start_matches("rgba(")
            .trim_start_matches("rgb(")
            .trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<f32>().ok()?.clamp(0.0, 255.0).round() as u8;
            let g = parts[1].parse::<f32>().ok()?.clamp(0.0, 255.0).round() as u8;
            let b = parts[2].parse::<f32>().ok()?.clamp(0.0, 255.0).round() as u8;
            if parts.len() >= 4 {
                let a_val = parts[3].parse::<f32>().ok()?.clamp(0.0, 1.0);
                let a = (a_val * 255.0).round() as u8;
                return Some(format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a));
            }
            return Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
        }
    }

    None
}

fn parse_css_px(raw: &str) -> Option<f32> {
    let s = raw.trim().trim_end_matches("px").trim();
    s.parse::<f32>().ok()
}

fn parse_css_font(raw: &str) -> Option<String> {
    let first = raw.split(',').next()?.trim().trim_matches(|c| c == '\'' || c == '"');
    let lower = first.to_ascii_lowercase();
    if first.is_empty() || matches!(lower.as_str(), "sans-serif" | "serif" | "monospace" | "system-ui") {
        return None;
    }
    Some(first.to_string())
}

fn extract_css_variables_scoped(css: &str, theme_mode: u8) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    let mut cleaned = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(nc) = chars.next() {
                if nc == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else {
            cleaned.push(c);
        }
    }

    // Split into CSS rules: selector { declarations }
    let mut remaining = cleaned.as_str();
    while let Some(brace_open) = remaining.find('{') {
        let selector = remaining[..brace_open].trim();
        let after_open = &remaining[brace_open + 1..];
        let brace_close = match after_open.find('}') {
            Some(i) => i,
            None => break,
        };
        let body = &after_open[..brace_close];
        remaining = &after_open[brace_close + 1..];

        let actual_selector = selector.rsplit('}').next().unwrap_or(selector).trim();
        let sel_lower = actual_selector.to_ascii_lowercase();
        let is_cyber = sel_lower.contains("cyber") || sel_lower.contains("dark");
        let is_mono = sel_lower.contains("mono");
        let is_light = sel_lower.contains("light");

        let matches = match theme_mode {
            0 => is_cyber,
            1 => is_light || (!is_cyber && !is_mono && (sel_lower.contains(":root") || sel_lower.contains("[data-skin]"))),
            2 => is_mono,
            _ => true,
        };

        if matches {
            for stmt in body.split(';') {
                if let Some(idx) = stmt.find("--") {
                    let decl = &stmt[idx..];
                    if let Some(colon) = decl.find(':') {
                        let key = decl[..colon].trim().to_ascii_lowercase();
                        let mut val = decl[colon + 1..].trim();
                        if let Some(imp) = val.find("!important") {
                            val = val[..imp].trim();
                        }
                        if !key.is_empty() && !val.is_empty() {
                            vars.insert(key, val.to_string());
                        }
                    }
                }
            }
        }
    }

    vars
}

fn resolve_var_value<'a>(vars: &'a HashMap<String, String>, val: &'a str, depth: u8) -> &'a str {
    if depth > 4 {
        return val;
    }
    let trimmed = val.trim();
    if trimmed.starts_with("var(") && trimmed.ends_with(')') {
        let inner = &trimmed[4..trimmed.len() - 1];
        let key = inner.split(',').next().unwrap_or("").trim().to_ascii_lowercase();
        if let Some(target) = vars.get(&key) {
            return resolve_var_value(vars, target, depth + 1);
        }
    }
    trimmed
}

fn apply_token_map(tokens: &mut SlintThemeTokens, key: &str, val: &str) {
    match key {
        "bg_root" | "--bg-root" => {
            if let Some(c) = normalize_css_color(val) { tokens.bg_root = c; }
        }
        "surface_base" | "--surface-base" => {
            if let Some(c) = normalize_css_color(val) { tokens.surface_base = c; }
        }
        "surface_card" | "--surface-card" => {
            if let Some(c) = normalize_css_color(val) { tokens.surface_card = c; }
        }
        "surface_card_hover" | "--surface-card-hover" => {
            if let Some(c) = normalize_css_color(val) { tokens.surface_card_hover = c; }
        }
        "surface_pill" | "--surface-pill" => {
            if let Some(c) = normalize_css_color(val) { tokens.surface_pill = c; }
        }
        "surface_elevated" | "--surface-elevated" | "--surface-overlay" => {
            if let Some(c) = normalize_css_color(val) { tokens.surface_elevated = c; }
        }
        "surface_sunken" | "--surface-sunken" => {
            if let Some(c) = normalize_css_color(val) { tokens.surface_sunken = c; }
        }
        "border_subtle" | "--border-subtle" => {
            if let Some(c) = normalize_css_color(val) { tokens.border_subtle = c; }
        }
        "border_medium" | "--border-medium" => {
            if let Some(c) = normalize_css_color(val) { tokens.border_medium = c; }
        }
        "border_highlight" | "--border-highlight" => {
            if let Some(c) = normalize_css_color(val) { tokens.border_highlight = c; }
        }
        "text_primary" | "--text-primary" => {
            if let Some(c) = normalize_css_color(val) { tokens.text_primary = c; }
        }
        "text_secondary" | "--text-secondary" => {
            if let Some(c) = normalize_css_color(val) { tokens.text_secondary = c; }
        }
        "text_muted" | "--text-muted" => {
            if let Some(c) = normalize_css_color(val) { tokens.text_muted = c; }
        }
        "font_family" | "--font-family" | "--font-mech" => {
            if let Some(f) = parse_css_font(val) { tokens.font_family = f; }
        }
        "accent_brand" | "--accent-brand" | "--color-brand" | "--accent-pwr" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_brand = c; }
        }
        "mode_office" | "--mode-office" | "--color-office" | "--accent-gpu" => {
            if let Some(c) = normalize_css_color(val) { tokens.mode_office = c; }
        }
        "mode_balance" | "--mode-balance" | "--color-balance" => {
            if let Some(c) = normalize_css_color(val) { tokens.mode_balance = c; }
        }
        "mode_beast" | "--mode-beast" | "--color-beast" | "--accent-beast" => {
            if let Some(c) = normalize_css_color(val) { tokens.mode_beast = c; }
        }
        "accent_fan" | "--accent-fan" | "--color-fan" | "--accent-cool" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_fan = c; }
        }
        "accent_gold" | "--accent-gold" | "--color-gold" | "--accent-lux" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_gold = c; }
        }
        "accent_active" | "--accent-active" | "--status-info" => {
            if let Some(c) = normalize_css_color(val) {
                tokens.accent_active = c;
            }
        }
        "accent_active_bg" | "--accent-active-bg" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_active_bg = c; }
        }
        "accent_active_border" | "--accent-active-border" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_active_border = c; }
        }
        "accent_heart" | "--accent-heart" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_heart = c; }
        }
        "accent_heart_bg" | "--accent-heart-bg" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_heart_bg = c; }
        }
        "accent_heart_border" | "--accent-heart-border" => {
            if let Some(c) = normalize_css_color(val) { tokens.accent_heart_border = c; }
        }
        "tag_brand_text" | "--tag-brand-text" => {
            if let Some(c) = normalize_css_color(val) { tokens.tag_brand_text = c; }
        }
        "radius_xs" | "--radius-xs" => {
            if let Some(r) = parse_css_px(val) { tokens.radius_xs = r.clamp(0.0, 24.0); }
        }
        "radius_sm" | "--radius-sm" => {
            if let Some(r) = parse_css_px(val) { tokens.radius_sm = r.clamp(0.0, 24.0); }
        }
        "radius_md" | "--radius-md" => {
            if let Some(r) = parse_css_px(val) { tokens.radius_md = r.clamp(0.0, 32.0); }
        }
        "radius_lg" | "--radius-lg" => {
            if let Some(r) = parse_css_px(val) { tokens.radius_lg = r.clamp(0.0, 32.0); }
        }
        "bg_image_opacity" => {
            if let Ok(op) = val.trim().parse::<f32>() {
                tokens.bg_image_opacity = op.clamp(0.05, 1.0);
            }
        }
        "drawer_bg_image_opacity" | "drawer_opacity" => {
            if let Ok(op) = val.trim().parse::<f32>() {
                tokens.drawer_bg_image_opacity = op.clamp(0.05, 1.0);
            }
        }
        _ => {}
    }
}

fn discover_plugin_image(plugin_dir: &Path, explicit_rel: Option<&str>) -> Option<PathBuf> {
    if let Some(rel) = explicit_rel {
        let p = plugin_dir.join(rel.trim_start_matches("./").trim_start_matches('/'));
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }
    let candidates = [
        "background.jpg", "background.jpeg", "background.png", "background.webp",
        "wallpaper.jpg", "wallpaper.jpeg", "wallpaper.png", "wallpaper.webp",
        "bg.jpg", "bg.jpeg", "bg.png", "bg.webp",
    ];
    for name in candidates {
        let p = plugin_dir.join(name);
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }
    None
}

fn discover_plugin_drawer_image(plugin_dir: &Path, explicit_rel: Option<&str>) -> Option<PathBuf> {
    if let Some(rel) = explicit_rel {
        let p = plugin_dir.join(rel.trim_start_matches("./").trim_start_matches('/'));
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }
    let candidates = [
        "drawer.jpg", "drawer.jpeg", "drawer.png", "drawer.webp",
        "drawer_bg.jpg", "drawer_bg.jpeg", "drawer_bg.png", "drawer_bg.webp",
        "drawer-bg.jpg", "drawer-bg.jpeg", "drawer-bg.png", "drawer-bg.webp",
        "drawer_background.jpg", "drawer_background.jpeg", "drawer_background.png", "drawer_background.webp",
        "drawer_wallpaper.jpg", "drawer_wallpaper.jpeg", "drawer_wallpaper.png", "drawer_wallpaper.webp",
    ];
    for name in candidates {
        let p = plugin_dir.join(name);
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }
    None
}

fn apply_skin_plugin(tokens: &mut SlintThemeTokens, plugin_id: &str, theme_mode: u8) {
    let plugin_dir = crate::commands::system_cmd::find_plugin_dir(plugin_id);
    if !plugin_dir.exists() {
        return;
    }

    let manifest_path = plugin_dir.join("manifest.json");
    let manifest_val = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());

    let theme_css_name = manifest_val
        .as_ref()
        .and_then(|m| m.get("theme_css").and_then(|v| v.as_str()))
        .unwrap_or("theme.css");

    // 1. Parse theme.css scoped to theme_mode (Dark: cyber/:root, Light: :root, Mono: mono/:root)
    let css_path = plugin_dir.join(theme_css_name);
    if css_path.exists() {
        if let Ok(raw_css) = std::fs::read_to_string(&css_path) {
            let vars = extract_css_variables_scoped(&raw_css, theme_mode);
            for (k, v) in &vars {
                let resolved = resolve_var_value(&vars, v, 0);
                apply_token_map(tokens, k, resolved);
            }
        }
    }

    // 2. Apply multi-mode or generic JSON tokens
    let mode_key = match theme_mode {
        1 => "tokens_light",
        2 => "tokens_mono",
        _ => "tokens_dark",
    };

    let apply_json_obj = |toks: &mut SlintThemeTokens, obj: &serde_json::Map<String, serde_json::Value>| {
        // Mode-specific overrides (tokens_light / tokens_dark / tokens_mono) takes highest precedence
        if let Some(mode_obj) = obj.get(mode_key).and_then(|v| v.as_object()) {
            for (k, v) in mode_obj {
                if let Some(s) = v.as_str() {
                    apply_token_map(toks, k, s);
                } else if let Some(n) = v.as_f64() {
                    apply_token_map(toks, k, &n.to_string());
                }
            }
        } else if let Some(gen_obj) = obj.get("tokens").or_else(|| obj.get("slint_tokens")).and_then(|v| v.as_object()) {
            // Generic tokens
            for (k, v) in gen_obj {
                if let Some(s) = v.as_str() {
                    apply_token_map(toks, k, s);
                } else if let Some(n) = v.as_f64() {
                    apply_token_map(toks, k, &n.to_string());
                }
            }
        } else if !obj.contains_key("tokens")
            && !obj.contains_key("slint_tokens")
            && !obj.contains_key("tokens_light")
            && !obj.contains_key("tokens_dark")
            && !obj.contains_key("tokens_mono")
        {
            // Raw flat token map (only for standalone token json files that have no mode keys)
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    apply_token_map(toks, k, s);
                } else if let Some(n) = v.as_f64() {
                    apply_token_map(toks, k, &n.to_string());
                }
            }
        }
    };

    for json_name in ["slint_tokens.json", "tokens.json"] {
        let p = plugin_dir.join(json_name);
        if p.exists() {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&content) {
                    apply_json_obj(tokens, &map);
                }
            }
        }
    }

    if let Some(obj) = manifest_val.as_ref().and_then(|m| m.as_object()) {
        apply_json_obj(tokens, obj);
    }

    // 3. Optional background image declared in skin manifest
    let img_rel = manifest_val
        .as_ref()
        .and_then(|m| m.get("image").or_else(|| m.get("wallpaper")).or_else(|| m.get("main_image")).and_then(|v| v.as_str()));
    if let Some(img_path) = discover_plugin_image(&plugin_dir, img_rel) {
        tokens.bg_image_path = img_path.to_string_lossy().to_string();
    }
    if let Some(op) = manifest_val
        .as_ref()
        .and_then(|m| m.get("opacity").or_else(|| m.get("bg_image_opacity")).and_then(|v| v.as_f64()))
    {
        tokens.bg_image_opacity = (op as f32).clamp(0.05, 1.0);
    }

    // 4. Optional drawer-specific background image declared in skin manifest
    let drawer_img_rel = manifest_val
        .as_ref()
        .and_then(|m| m.get("drawer_image")
            .or_else(|| m.get("drawer_wallpaper"))
            .or_else(|| m.get("drawer_bg"))
            .and_then(|v| v.as_str()));
    if let Some(drawer_path) = discover_plugin_drawer_image(&plugin_dir, drawer_img_rel) {
        tokens.drawer_bg_image_path = drawer_path.to_string_lossy().to_string();
    }
    if let Some(op) = manifest_val
        .as_ref()
        .and_then(|m| m.get("drawer_opacity")
            .or_else(|| m.get("drawer_bg_image_opacity"))
            .and_then(|v| v.as_f64()))
    {
        tokens.drawer_bg_image_opacity = (op as f32).clamp(0.05, 1.0);
    }
}

/// Resolve the active Slint theme tokens from built-in theme_mode + active skin/background plugins.
pub fn resolve_slint_theme(cfg: &crate::system::config::AppConfig) -> SlintThemeTokens {
    let mut tokens = SlintThemeTokens::from_builtin_mode(cfg.theme_mode);

    // 1. Resolve active skin plugin (prefer cfg.active_skin, or fallback to enabled "skin" plugin)
    let mut active_skin_id = cfg.active_skin.clone();
    let mut active_bg_id: Option<String> = None;

    if let Ok(plugins) = crate::commands::system_cmd::get_custom_plugins() {
        if active_skin_id.is_none() {
            active_skin_id = plugins
                .iter()
                .find(|p| p.enabled && p.plugin_type.eq_ignore_ascii_case("skin"))
                .map(|p| p.id.clone());
        }
        active_bg_id = plugins
            .iter()
            .find(|p| p.enabled && p.plugin_type.eq_ignore_ascii_case("background"))
            .map(|p| p.id.clone());
    }

    if let Some(skin_id) = active_skin_id {
        apply_skin_plugin(&mut tokens, &skin_id, cfg.theme_mode);
    }

    // 2. Apply active background plugin (overrides wallpaper image if enabled)
    if let Some(bg_id) = active_bg_id {
        let bg_dir = crate::commands::system_cmd::find_plugin_dir(&bg_id);
        let manifest_path = bg_dir.join("manifest.json");
        let manifest_val = std::fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());
        let img_rel = manifest_val
            .as_ref()
            .and_then(|m| m.get("image").or_else(|| m.get("wallpaper")).or_else(|| m.get("main_image")).and_then(|v| v.as_str()));
        if let Some(img_path) = discover_plugin_image(&bg_dir, img_rel) {
            tokens.bg_image_path = img_path.to_string_lossy().to_string();
        }
        if let Some(op) = manifest_val
            .as_ref()
            .and_then(|m| m.get("opacity").or_else(|| m.get("bg_image_opacity")).and_then(|v| v.as_f64()))
        {
            tokens.bg_image_opacity = (op as f32).clamp(0.05, 1.0);
        }

        let drawer_img_rel = manifest_val
            .as_ref()
            .and_then(|m| m.get("drawer_image")
                .or_else(|| m.get("drawer_wallpaper"))
                .or_else(|| m.get("drawer_bg"))
                .and_then(|v| v.as_str()));
        if let Some(drawer_path) = discover_plugin_drawer_image(&bg_dir, drawer_img_rel) {
            tokens.drawer_bg_image_path = drawer_path.to_string_lossy().to_string();
        }
        if let Some(op) = manifest_val
            .as_ref()
            .and_then(|m| m.get("drawer_opacity")
                .or_else(|| m.get("drawer_bg_image_opacity"))
                .and_then(|v| v.as_f64()))
        {
            tokens.drawer_bg_image_opacity = (op as f32).clamp(0.05, 1.0);
        }
    }

    // 3. Fallback: If no dedicated drawer background image was specified, share the main background image
    if tokens.drawer_bg_image_path.is_empty() && !tokens.bg_image_path.is_empty() {
        tokens.drawer_bg_image_path = tokens.bg_image_path.clone();
        tokens.drawer_bg_image_opacity = tokens.bg_image_opacity;
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cream_cup_theme_resolution() {
        let mut cfg = crate::system::config::AppConfig::default();
        cfg.active_skin = Some("theme-cream-cup".into());

        // 1. Light mode (theme_mode = 1) -> 浅色奶油
        cfg.theme_mode = 1;
        let tokens_light = resolve_slint_theme(&cfg);
        assert_eq!(tokens_light.bg_root, "#f5eeda");
        assert_eq!(tokens_light.surface_card, "#fffcf3");
        assert_eq!(tokens_light.text_primary, "#35291c");
        assert_eq!(tokens_light.accent_brand, "#d9772f");

        // 2. Dark mode (theme_mode = 0) -> 深焙奶茶
        cfg.theme_mode = 0;
        let tokens_dark = resolve_slint_theme(&cfg);
        assert_eq!(tokens_dark.bg_root, "#332b24");
        assert_eq!(tokens_dark.surface_card, "#42382e");
        assert_eq!(tokens_dark.text_primary, "#f7f0e5");
        assert_eq!(tokens_dark.accent_brand, "#e79a50");

        // 3. Mono mode (theme_mode = 2) -> 复古版画黑白
        cfg.theme_mode = 2;
        let tokens_mono = resolve_slint_theme(&cfg);
        assert_eq!(tokens_mono.bg_root, "#1f1d1a");
        assert_eq!(tokens_mono.surface_card, "#2e2a27");
        assert_eq!(tokens_mono.accent_brand, "#faf6f0");
    }
}


