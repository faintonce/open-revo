fn main() {
    let args: Vec<String> = std::env::args().collect();
    let plugin_id = args.get(1).map(|s| s.as_str()).unwrap_or_default();
    if plugin_id.is_empty() {
        eprintln!("Usage: openrevo-widget <plugin_id> [--embed <parent_hwnd> <x> <y> <w> <h>]");
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--embed") {
        if args.len() >= pos + 6 {
            let parent_hwnd: isize = args[pos + 1].parse().unwrap_or(0);
            let x: i32 = args[pos + 2].parse().unwrap_or(0);
            let y: i32 = args[pos + 3].parse().unwrap_or(0);
            let w: i32 = args[pos + 4].parse().unwrap_or(600);
            let h: i32 = args[pos + 5].parse().unwrap_or(400);
            openrevo_widget::run_widget_embedded(plugin_id, parent_hwnd, (x, y, w, h));
            return;
        }
    }

    openrevo_widget::run_widget_standalone(plugin_id);
}
