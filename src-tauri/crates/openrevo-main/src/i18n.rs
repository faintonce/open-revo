use crate::{AppI18n, MainWindow};
use slint::{ComponentHandle, SharedString};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

static I18N_MAP: OnceLock<RwLock<HashMap<String, SharedString>>> = OnceLock::new();

fn flatten_bundle(val: &serde_json::Value, prefix: &str, out: &mut HashMap<String, SharedString>) {
    if let Some(obj) = val.as_object() {
        for (k, v) in obj {
            if prefix.is_empty() && k == "_meta" {
                continue;
            }
            let next = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{prefix}.{k}")
            };
            if v.is_object() {
                flatten_bundle(v, &next, out);
            } else if let Some(s) = v.as_str() {
                out.insert(next, SharedString::from(s));
            }
        }
    }
}

fn get_map() -> &'static RwLock<HashMap<String, SharedString>> {
    I18N_MAP.get_or_init(|| {
        let mut map = HashMap::new();
        let bundle = openrevo_core::system::i18n::get_active_locale_bundle();
        flatten_bundle(&bundle, "", &mut map);
        RwLock::new(map)
    })
}

pub fn translate(key: &str) -> SharedString {
    if let Ok(guard) = get_map().read() {
        if let Some(val) = guard.get(key) {
            return val.clone();
        }
    }
    SharedString::from(key)
}

pub fn register(window: &MainWindow) {
    let _ = get_map();
    window
        .global::<AppI18n>()
        .on_tr(|_rev, key| translate(key.as_str()));
}

pub fn apply_bundle(ui: &MainWindow, bundle: &serde_json::Value) {
    let mut next_map = HashMap::new();
    flatten_bundle(bundle, "", &mut next_map);
    if let Ok(mut guard) = get_map().write() {
        *guard = next_map;
    }
    let i18n = ui.global::<AppI18n>();
    i18n.set_rev(i18n.get_rev().wrapping_add(1));
}
