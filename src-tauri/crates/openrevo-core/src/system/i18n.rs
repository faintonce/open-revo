use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub const BUILTIN_ZH_CN: &str = include_str!("../../../../../locales/zh-CN.json");
pub const BUILTIN_EN_US: &str = include_str!("../../../../../locales/en-US.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleMeta {
    pub code: String,
    pub name: String,
    pub font_priority: String,
    pub is_builtin: bool,
    pub is_external: bool,
}

#[cfg(target_os = "windows")]
pub fn detect_system_locale() -> String {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    let lang_id = unsafe { GetUserDefaultUILanguage() };
    if (lang_id & 0x03FF) == 0x0004 {
        "zh-CN".to_string()
    } else {
        "en-US".to_string()
    }
}

#[cfg(not(target_os = "windows"))]
pub fn detect_system_locale() -> String {
    "en-US".to_string()
}

pub fn get_locales_dir() -> PathBuf {
    super::config::get_config_dir().join("locales")
}

pub fn ensure_locales_dir() -> Result<PathBuf, String> {
    let dir = get_locales_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create locales dir: {e}"))?;
    Ok(dir)
}

fn parse_meta(val: &Value, fallback_code: &str, is_builtin: bool, is_external: bool) -> LocaleMeta {
    let meta = val.get("_meta");
    let code = meta
        .and_then(|m| m.get("code"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(fallback_code)
        .to_string();
    let name = meta
        .and_then(|m| m.get("name"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&code)
        .to_string();
    let font_priority = meta
        .and_then(|m| m.get("font_priority"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(if code.starts_with("zh") { "cjk" } else { "segoe" })
        .to_string();

    LocaleMeta {
        code,
        name,
        font_priority,
        is_builtin,
        is_external,
    }
}

pub fn merge_json_overlay(base: &mut Value, overlay: &Value) {
    if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
        for (k, v) in overlay_obj {
            if let Some(existing) = base_obj.get_mut(k) {
                if existing.is_object() && v.is_object() {
                    merge_json_overlay(existing, v);
                } else if existing.is_string() && v.is_string() {
                    *existing = v.clone();
                }
            }
        }
    }
}

fn scan_external_locale_files() -> Vec<(PathBuf, Value)> {
    let dir = get_locales_dir();
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('_') || name.starts_with("template.") || !name.ends_with(".json") {
                continue;
            }
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<Value>(&raw) {
                    if val.is_object() {
                        results.push((path, val));
                    }
                }
            }
        }
    }
    results
}

pub fn list_available_locales() -> Vec<LocaleMeta> {
    let zh: Value = serde_json::from_str(BUILTIN_ZH_CN).unwrap_or_default();
    let en: Value = serde_json::from_str(BUILTIN_EN_US).unwrap_or_default();

    let mut list = vec![
        parse_meta(&zh, "zh-CN", true, false),
        parse_meta(&en, "en-US", true, false),
    ];

    for (path, val) in scan_external_locale_files() {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("custom");
        let meta = parse_meta(&val, stem, false, true);
        if let Some(existing) = list.iter_mut().find(|m| m.code.eq_ignore_ascii_case(&meta.code)) {
            existing.is_external = true;
            existing.name = meta.name;
            existing.font_priority = meta.font_priority;
        } else {
            list.push(meta);
        }
    }
    list
}

pub fn load_locale_bundle(code: &str) -> Value {
    let zh: Value = serde_json::from_str(BUILTIN_ZH_CN).unwrap_or_default();
    let en: Value = serde_json::from_str(BUILTIN_EN_US).unwrap_or_default();

    let mut bundle = zh.clone();
    if !code.eq_ignore_ascii_case("zh-CN") {
        merge_json_overlay(&mut bundle, &en);
    }

    for (path, val) in scan_external_locale_files() {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let ext_code = val
            .get("_meta")
            .and_then(|m| m.get("code"))
            .and_then(|v| v.as_str())
            .unwrap_or(stem);
        if ext_code.eq_ignore_ascii_case(code) || stem.eq_ignore_ascii_case(code) {
            merge_json_overlay(&mut bundle, &val);
            break;
        }
    }

    bundle
}

pub fn lookup_str<'a>(bundle: &'a Value, dot_path: &str) -> Option<&'a str> {
    let mut cur = bundle;
    for part in dot_path.split('.') {
        cur = cur.get(part)?;
    }
    cur.as_str()
}

pub fn lookup_or(bundle: &Value, dot_path: &str, fallback: &str) -> String {
    lookup_str(bundle, dot_path)
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback.to_string())
}

pub fn get_active_locale_bundle() -> Value {
    let cfg = super::config::load_config();
    load_locale_bundle(&cfg.locale)
}

pub fn export_i18n_template() -> Result<PathBuf, String> {
    let dir = ensure_locales_dir()?;
    let target = dir.join("template.en-US.json");
    super::config_io::atomic_write(&target, BUILTIN_EN_US.as_bytes())?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn collect_leaf_keys(val: &Value, prefix: &str, out: &mut Vec<String>) {
        if let Some(obj) = val.as_object() {
            for (k, v) in obj {
                let next = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                if v.is_object() {
                    collect_leaf_keys(v, &next, out);
                } else {
                    out.push(next);
                }
            }
        }
    }

    #[test]
    fn builtin_locales_have_matching_schema() {
        let zh: Value = serde_json::from_str(BUILTIN_ZH_CN).expect("valid zh-CN.json");
        let en: Value = serde_json::from_str(BUILTIN_EN_US).expect("valid en-US.json");
        let mut zh_keys = Vec::new();
        let mut en_keys = Vec::new();
        collect_leaf_keys(&zh, "", &mut zh_keys);
        collect_leaf_keys(&en, "", &mut en_keys);
        zh_keys.sort();
        en_keys.sort();
        assert_eq!(zh_keys, en_keys);
    }

    #[test]
    fn overlay_falls_back_for_missing_or_wrong_type_keys() {
        let mut base = json!({
            "_meta": { "code": "en-US", "name": "English", "font_priority": "segoe" },
            "common": { "confirm": "Confirm", "cancel": "Cancel" }
        });
        let overlay = json!({
            "_meta": { "code": "ru-RU", "name": "Русский" },
            "common": { "confirm": "Подтвердить", "cancel": 123, "extra": "ignored" }
        });
        merge_json_overlay(&mut base, &overlay);
        assert_eq!(lookup_str(&base, "common.confirm"), Some("Подтвердить"));
        assert_eq!(lookup_str(&base, "common.cancel"), Some("Cancel"));
        assert_eq!(lookup_str(&base, "_meta.font_priority"), Some("segoe"));
        assert_eq!(lookup_str(&base, "common.extra"), None);
    }
}
