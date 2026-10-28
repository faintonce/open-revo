use std::io::BufRead;
use std::sync::{Arc, Mutex};
use crate::{handle_ipc_message, OsdWindow, SafeHwnd};

pub fn start_ipc_server(ui_weak: slint::Weak<OsdWindow>, hwnd_shared: Arc<Mutex<Option<SafeHwnd>>>) {
    std::thread::Builder::new()
        .name("openrevo-osd-stdin-listener".into())
        .spawn(move || {
            let stdin = std::io::stdin();
            for line_res in stdin.lock().lines() {
                match line_res {
                    Ok(line) => {
                        let line = line.trim();
                        if !line.is_empty() {
                            crate::osd_log(&format!("Received IPC line: {}", line));
                            handle_ipc_message(line, &ui_weak, &hwnd_shared);
                        }
                    }
                    Err(e) => {
                        crate::osd_log(&format!("IPC stdin error / EOF: {}", e));
                        break;
                    }
                }
            }
            crate::osd_log("Host process terminated, exiting");
            std::process::exit(0);
        })
        .expect("Failed to spawn OSD stdin listener");
}

