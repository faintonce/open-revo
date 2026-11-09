// OpenRevo - Native Wry & Tao Runtime Manager
// Provides direct, unmanaged Wry WebView2 popout windows and embedded child webviews.
// Decoupled from Tauri's WebviewWindowBuilder, guaranteeing clean instantiation and destruction
// with 0 residual background WebView2 processes when closed.

pub mod ipc;

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Once, OnceLock};
use parking_lot::{Mutex, RwLock};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder};
use wry::{Rect, WebContext, WebView, WebViewBuilder};

#[derive(Debug, Clone)]
pub enum ViewTarget {
    Popout(String),
    Embedded,
}

pub enum NativeWryMessage {
    OpenPopout {
        plugin_id: String,
    },
    ClosePopout {
        plugin_id: String,
    },
    DragWindow {
        plugin_id: String,
    },
    SetWindowSize {
        plugin_id: String,
        width: f64,
        height: f64,
    },
    BroadcastEvent {
        event: String,
        payload_json: String,
    },
    AttachEmbedded {
        parent_hwnd: isize,
        plugin_id: String,
        rect: (i32, i32, i32, i32),
    },
    UpdateEmbeddedBounds {
        rect: (i32, i32, i32, i32),
    },
    HideEmbedded,
    EvalJs {
        target: ViewTarget,
        script: String,
    },
    Shutdown,
}

pub struct HwndWrapper(pub isize);

impl raw_window_handle::HasWindowHandle for HwndWrapper {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let non_zero = std::num::NonZeroIsize::new(self.0)
            .ok_or(raw_window_handle::HandleError::Unavailable)?;
        let win32 = raw_window_handle::Win32WindowHandle::new(non_zero);
        let raw = raw_window_handle::RawWindowHandle::Win32(win32);
        unsafe { Ok(raw_window_handle::WindowHandle::borrow_raw(raw)) }
    }
}

static INIT_ONCE: Once = Once::new();
static PROXY_HOLDER: Mutex<Option<EventLoopProxy<NativeWryMessage>>> = Mutex::new(None);
static ACTIVE_EMBEDDED_PLUGIN: RwLock<Option<String>> = RwLock::new(None);
static IS_SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

fn get_running_popouts_set() -> &'static RwLock<HashSet<String>> {
    static SET: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| RwLock::new(HashSet::new()))
}

/// Ensure the background Tao EventLoop runtime thread is initialized
pub fn ensure_native_wry_loop() -> Option<EventLoopProxy<NativeWryMessage>> {
    if IS_SHUTTING_DOWN.load(Ordering::SeqCst) {
        return None;
    }

    INIT_ONCE.call_once(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        let spawn_res = std::thread::Builder::new()
            .name("native-wry-runtime".into())
            .spawn(move || {
                run_native_wry_event_loop(tx);
            });

        match spawn_res {
            Ok(_) => {
                if let Ok(proxy) = rx.recv() {
                    *PROXY_HOLDER.lock() = Some(proxy);
                    crate::log_info!("NATIVE_WRY", "Native Wry event loop thread initialized successfully.");
                } else {
                    crate::log_error!("NATIVE_WRY", "Failed to receive EventLoopProxy from native wry thread.");
                }
            }
            Err(e) => {
                crate::log_error!("NATIVE_WRY", "Failed to spawn native-wry-runtime thread: {:?}", e);
            }
        }
    });

    PROXY_HOLDER.lock().clone()
}

/// Injected bridge polyfill for Wry webviews to connect plugins directly to OpenRevo RPC
pub fn get_bridge_script(plugin_id: &str) -> String {
    let script = r#"
(function() {
  const pluginId = "__PLUGIN_ID__";
  let reqSeq = 0;
  const pendingRequests = new Map();

  window.__NATIVE_WRY_RESOLVE__ = function(reqId, ok, data, err) {
    const cb = pendingRequests.get(reqId);
    if (cb) {
      pendingRequests.delete(reqId);
      if (ok) {
        cb.resolve(data);
      } else {
        cb.reject(new Error(err || "Native IPC call failed"));
      }
    }
  };

  function callRpc(cmd, args) {
    return new Promise((resolve, reject) => {
      reqSeq += 1;
      const reqId = "wry_rpc_" + reqSeq + "_" + Date.now();
      pendingRequests.set(reqId, { resolve, reject });
      try {
        window.ipc.postMessage(JSON.stringify({
          reqId: reqId,
          cmd: cmd,
          args: args || {}
        }));
      } catch (e) {
        pendingRequests.delete(reqId);
        reject(e);
      }
    });
  }

  const listeners = new Map();
  window.__NATIVE_WRY_EMIT__ = function(event, payload) {
    const cbs = listeners.get(event) || [];
    for (const cb of cbs) {
      try { cb({ event: event, payload: payload }); } catch (e) { console.error(e); }
    }
    window.dispatchEvent(new CustomEvent("openrevo-" + event, { detail: payload }));
    if (event === "hardware-state-updated" || event === "openrevo-telemetry-update") {
      window.dispatchEvent(new CustomEvent("openrevo-telemetry", { detail: payload }));
      try {
        window.postMessage({ type: "OPENREVO_TELEMETRY", payload: payload }, "*");
      } catch (_) {}
    }
    try {
      window.postMessage({ type: "OPENREVO_EVENT", event: event, payload: payload }, "*");
    } catch (_) {}
  };

  let lastDragTime = 0;
  function safeStartDrag() {
    const now = Date.now();
    if (now - lastDragTime < 300) return;
    lastDragTime = now;
    callRpc("start_dragging", { pluginId: pluginId });
  }

  const bridge = {
    invoke: callRpc,
    pluginId: pluginId,
    listen: function(event, callback) {
      if (!listeners.has(event)) {
        listeners.set(event, []);
      }
      listeners.get(event).push(callback);
      return Promise.resolve(function unlisten() {
        const arr = listeners.get(event) || [];
        const idx = arr.indexOf(callback);
        if (idx !== -1) arr.splice(idx, 1);
      });
    },
    emit: function(event, payload) {
      return callRpc("emit", { event: event, payload: payload });
    },
    getCurrentWebviewWindow: function() {
      return {
        startDragging: safeStartDrag,
        close: function() { return callRpc("close_custom_widget", { pluginId: pluginId }); },
        hide: function() { return callRpc("close_custom_widget", { pluginId: pluginId }); },
        show: function() {},
        setSize: function(sz) { return callRpc("set_window_size", { pluginId: pluginId, width: sz.width, height: sz.height }); },
        setFocus: function() {}
      };
    },
    getCurrentWindow: function() {
      return this.getCurrentWebviewWindow();
    },
    startDragging: safeStartDrag,
    close: function() {
      return callRpc("close_custom_widget", { pluginId: pluginId });
    },
    LogicalSize: function(width, height) {
      this.width = width;
      this.height = height;
    },
    storage: {
      get: function() { return callRpc("get_plugin_storage", { pluginId: pluginId }); },
      set: function(data) { return callRpc("set_plugin_storage", { pluginId: pluginId, data: data }); },
      getItem: async function(key) {
        const all = await callRpc("get_plugin_storage", { pluginId: pluginId });
        return all ? all[key] : undefined;
      },
      setItem: async function(key, val) {
        const all = (await callRpc("get_plugin_storage", { pluginId: pluginId })) || {};
        all[key] = val;
        return await callRpc("set_plugin_storage", { pluginId: pluginId, data: all });
      }
    }
  };

  window.__OPENREVO_BRIDGE__ = bridge;
  window.__TAURI__ = {
    core: {
      invoke: callRpc
    },
    event: {
      listen: function(event, callback) {
        return bridge.listen(event, callback);
      },
      emit: function(event, payload) {
        return callRpc("emit", { event: event, payload: payload });
      }
    },
    webviewWindow: {
      getCurrentWebviewWindow: function() {
        return bridge.getCurrentWebviewWindow();
      }
    },
    window: {
      getCurrentWindow: function() {
        return bridge.getCurrentWebviewWindow();
      }
    },
    dpi: {
      LogicalSize: function(width, height) {
        this.width = width;
        this.height = height;
      }
    }
  };
  window.__TAURI_INTERNALS__ = {
    invoke: callRpc,
    plugins: {}
  };
  if (window.parent) {
    try {
      window.parent.__OPENREVO_BRIDGE__ = bridge;
      window.parent.__TAURI__ = window.__TAURI__;
      window.parent.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__;
    } catch (e) {}
  }
  window.OpenRevo = bridge;

  // Global mouse drag handler for [data-tauri-drag-region]
  document.addEventListener("pointerdown", function(e) {
    if (e.target && e.target.closest) {
      if (e.target.closest("[data-no-drag]")) return;
      if (e.target.closest("button, [role=\"button\"], input, select, textarea, .tool-btn, .clickable, .status-card, .core-disc")) return;
      if (e.target.closest("[data-tauri-drag-region]")) {
        if (e.button === 0) {
          safeStartDrag();
        }
      }
    }
  });

  // postMessage fallback listener for older iframe/widget plugins
  window.addEventListener("message", async function(e) {
    if (!e.data || !e.data.action || !e.data.payload) return;
    const { action, payload } = e.data;
    const reqId = payload.reqId;
    let cmd = action.toLowerCase();
    try {
      const res = await callRpc(cmd, payload);
      if (reqId) {
        window.postMessage({ reqId: reqId, success: true, payload: res }, "*");
      }
    } catch (err) {
      if (reqId) {
        window.postMessage({ reqId: reqId, success: false, error: err && err.message ? err.message : String(err) }, "*");
      }
    }
  });
})();
"#;
    script.replace("__PLUGIN_ID__", plugin_id)
}

fn handle_custom_protocol(
    request: wry::http::Request<Vec<u8>>,
) -> wry::http::Response<std::borrow::Cow<'static, [u8]>> {
    use wry::http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE};
    use wry::http::Response;

    let uri_path = request.uri().path();
    let clean_path = uri_path.trim_start_matches('/').replace("%20", " ");

    let (plugin_id, subpath) = match clean_path.split_once('/') {
        Some((id, sub)) => (id, if sub.is_empty() { "index.html" } else { sub }),
        None => (clean_path.as_str(), "index.html"),
    };

    let target_dir = crate::commands::system_cmd::find_plugin_dir(plugin_id);
    let target_file = target_dir.join(subpath);

    if !target_file.exists() {
        crate::log_warn!("NATIVE_WRY", "Custom protocol 404 for plugin '{}': {:?}", plugin_id, target_file);
        return Response::builder()
            .status(404)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(std::borrow::Cow::Borrowed(b"404 Not Found" as &'static [u8]))
            .unwrap();
    }

    match std::fs::read(&target_file) {
        Ok(bytes) => {
            let mime = if subpath.ends_with(".html") || subpath.ends_with(".htm") {
                "text/html; charset=utf-8"
            } else if subpath.ends_with(".css") {
                "text/css; charset=utf-8"
            } else if subpath.ends_with(".js") || subpath.ends_with(".mjs") {
                "application/javascript; charset=utf-8"
            } else if subpath.ends_with(".json") {
                "application/json; charset=utf-8"
            } else if subpath.ends_with(".svg") {
                "image/svg+xml"
            } else if subpath.ends_with(".png") {
                "image/png"
            } else if subpath.ends_with(".jpg") || subpath.ends_with(".jpeg") {
                "image/jpeg"
            } else if subpath.ends_with(".webp") {
                "image/webp"
            } else if subpath.ends_with(".woff2") {
                "font/woff2"
            } else if subpath.ends_with(".woff") {
                "font/woff"
            } else if subpath.ends_with(".ttf") {
                "font/ttf"
            } else {
                "application/octet-stream"
            };

            Response::builder()
                .status(200)
                .header(CONTENT_TYPE, mime)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(std::borrow::Cow::Owned(bytes))
                .unwrap()
        }
        Err(e) => {
            crate::log_error!("NATIVE_WRY", "Failed to read plugin file {:?}: {:?}", target_file, e);
            Response::builder()
                .status(500)
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(std::borrow::Cow::Owned(format!("Read error: {}", e).into_bytes()))
                .unwrap()
        }
    }
}

fn handle_ipc_payload(
    plugin_id: &str,
    raw_payload: &str,
    proxy: EventLoopProxy<NativeWryMessage>,
    target: ViewTarget,
) {
    #[derive(serde::Deserialize)]
    struct WryIpcEnvelope {
        #[serde(alias = "reqId")]
        req_id: Option<String>,
        cmd: String,
        #[serde(default)]
        args: serde_json::Value,
    }

    let parsed: Result<WryIpcEnvelope, _> = serde_json::from_str(raw_payload);
    match parsed {
        Ok(env) => {
            if env.cmd == "start_dragging" {
                let _ = proxy.send_event(NativeWryMessage::DragWindow {
                    plugin_id: plugin_id.to_string(),
                });
                return;
            }
            if env.cmd == "set_window_size" {
                let w = env.args.get("width").and_then(|v| v.as_f64()).unwrap_or(260.0);
                let h = env.args.get("height").and_then(|v| v.as_f64()).unwrap_or(190.0);
                let _ = proxy.send_event(NativeWryMessage::SetWindowSize {
                    plugin_id: plugin_id.to_string(),
                    width: w,
                    height: h,
                });
                return;
            }

            let pid = plugin_id.to_string();
            let req_id = env.req_id.unwrap_or_default();
            tauri::async_runtime::spawn(async move {
                let result = ipc::dispatch_plugin_ipc(&pid, &env.cmd, env.args).await;
                let script = match result {
                    Ok(val) => format!(
                        "window.__NATIVE_WRY_RESOLVE__('{}', true, {}, null);",
                        req_id,
                        serde_json::to_string(&val).unwrap_or_else(|_| "null".into())
                    ),
                    Err(err) => format!(
                        "window.__NATIVE_WRY_RESOLVE__('{}', false, null, {});",
                        req_id,
                        serde_json::to_string(&err).unwrap_or_else(|_| "\"IPC error\"".into())
                    ),
                };
                let _ = proxy.send_event(NativeWryMessage::EvalJs { target, script });
            });
        }
        Err(e) => {
            crate::log_error!("NATIVE_WRY", "Failed to parse IPC message '{}': {:?}", raw_payload, e);
        }
    }
}

fn run_native_wry_event_loop(tx: std::sync::mpsc::Sender<EventLoopProxy<NativeWryMessage>>) {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Ole::OleInitialize;
        let _ = OleInitialize(None);
    }

    let mut builder = EventLoopBuilder::<NativeWryMessage>::with_user_event();
    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }

    let event_loop = builder.build();
    let proxy = event_loop.create_proxy();
    let _ = tx.send(proxy.clone());

    let data_dir = crate::system::config::get_config_dir().join("webview2_runtime");
    let _ = std::fs::create_dir_all(&data_dir);
    let mut web_context = WebContext::new(Some(data_dir));

    let mut popouts: HashMap<String, (Window, WebView)> = HashMap::new();
    let mut embedded: Option<(String, WebView, isize)> = None;

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            tao::event::Event::WindowEvent {
                window_id,
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } => {
                crate::log_info!("NATIVE_WRY", "Popout window close requested for WindowId: {:?}", window_id);
                popouts.retain(|pid, (win, _)| {
                    if win.id() == window_id {
                        crate::log_info!("NATIVE_WRY", "Destroying popout window for plugin '{}'", pid);
                        get_running_popouts_set().write().remove(pid);
                        crate::system::telemetry::unregister_consumer(pid);
                        let mut cfg = crate::system::config::load_config();
                        cfg.custom_plugin_toggles.insert(pid.clone(), false);
                        let _ = crate::system::config::save_config(&cfg);
                        crate::system::slint_window::sync::sync_slint_plugins_state();
                        false
                    } else {
                        true
                    }
                });
            }

            tao::event::Event::WindowEvent {
                window_id,
                event: tao::event::WindowEvent::Resized(size),
                ..
            } => {
                for (_, (win, view)) in popouts.iter() {
                    if win.id() == window_id {
                        let _ = view.set_bounds(Rect {
                            position: wry::dpi::Position::Physical(tao::dpi::PhysicalPosition::new(0, 0)),
                            size: wry::dpi::Size::Physical(tao::dpi::PhysicalSize::new(size.width, size.height)),
                        });
                        break;
                    }
                }
            }

            tao::event::Event::WindowEvent {
                window_id,
                event: tao::event::WindowEvent::Moved(pos),
                ..
            } => {
                for (pid, (win, _)) in popouts.iter() {
                    if win.id() == window_id {
                        crate::system::widget_manager::queue_widget_position(pid.clone(), pos.x, pos.y);
                        break;
                    }
                }
            }

            tao::event::Event::UserEvent(msg) => match msg {
                NativeWryMessage::DragWindow { plugin_id } => {
                    static LAST_DRAG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let prev = LAST_DRAG.load(std::sync::atomic::Ordering::Relaxed);
                    if now.saturating_sub(prev) > 250 {
                        LAST_DRAG.store(now, std::sync::atomic::Ordering::Relaxed);
                        if let Some((win, _)) = popouts.get(&plugin_id) {
                            let _ = win.drag_window();
                        }
                    }
                }

                NativeWryMessage::SetWindowSize { plugin_id, width, height } => {
                    if let Some((win, view)) = popouts.get(&plugin_id) {
                        win.set_inner_size(tao::dpi::LogicalSize::new(width, height));
                        let phys = win.inner_size();
                        let _ = view.set_bounds(Rect {
                            position: wry::dpi::Position::Physical(tao::dpi::PhysicalPosition::new(0, 0)),
                            size: wry::dpi::Size::Physical(tao::dpi::PhysicalSize::new(phys.width, phys.height)),
                        });
                    }
                }

                NativeWryMessage::BroadcastEvent { event, payload_json } => {
                    let script = format!(
                        "window.__NATIVE_WRY_EMIT__ && window.__NATIVE_WRY_EMIT__('{}', {});",
                        event, payload_json
                    );
                    for (_, (_, view)) in popouts.iter() {
                        let _ = view.evaluate_script(&script);
                    }
                    if let Some((_, ref view, _)) = embedded {
                        let _ = view.evaluate_script(&script);
                    }
                }
                NativeWryMessage::OpenPopout { plugin_id } => {
                    if let Some((win, _)) = popouts.get(&plugin_id) {
                        win.set_visible(true);
                        win.set_focus();
                        #[cfg(target_os = "windows")]
                        {
                            use tao::platform::windows::WindowExtWindows;
                            use windows::Win32::Foundation::HWND;
                            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SetForegroundWindow, SW_SHOW};
                            let hwnd = HWND(win.hwnd() as *mut _);
                            unsafe {
                                let _ = ShowWindow(hwnd, SW_SHOW);
                                let _ = SetForegroundWindow(hwnd);
                            }
                        }
                        return;
                    }

                    match spawn_popout_window(target, &proxy, &plugin_id, &mut web_context) {
                        Ok((win, view)) => {
                            get_running_popouts_set().write().insert(plugin_id.clone());
                            popouts.insert(plugin_id.clone(), (win, view));
                            crate::log_info!("NATIVE_WRY", "Popout window opened successfully for '{}'", plugin_id);
                        }
                        Err(e) => {
                            crate::log_error!("NATIVE_WRY", "Failed to open popout window for '{}': {:?}", plugin_id, e);
                        }
                    }
                }

                NativeWryMessage::ClosePopout { plugin_id } => {
                    get_running_popouts_set().write().remove(&plugin_id);
                    crate::system::telemetry::unregister_consumer(&plugin_id);
                    if let Some((win, _)) = popouts.remove(&plugin_id) {
                        win.set_visible(false);
                        drop(win);
                        crate::log_info!("NATIVE_WRY", "Popout '{}' closed and destroyed.", plugin_id);
                    }
                }

                NativeWryMessage::AttachEmbedded {
                    parent_hwnd,
                    plugin_id,
                    rect,
                } => {
                    if let Some((curr_id, view, _)) = embedded.as_ref() {
                        if curr_id == &plugin_id {
                            let _ = view.set_bounds(Rect {
                                position: wry::dpi::Position::Physical(tao::dpi::PhysicalPosition::new(rect.0, rect.1)),
                                size: wry::dpi::Size::Physical(tao::dpi::PhysicalSize::new(rect.2 as u32, rect.3 as u32)),
                            });
                            let _ = view.set_visible(true);
                            return;
                        }
                    }

                    // Drop previous embedded webview if switching plugins
                    if let Some((prev_pid, _, _)) = embedded.take() {
                        crate::system::telemetry::unregister_consumer(&prev_pid);
                    }
                    *ACTIVE_EMBEDDED_PLUGIN.write() = None;

                    match spawn_embedded_child(&proxy, parent_hwnd, &plugin_id, rect, &mut web_context) {
                        Ok(view) => {
                            *ACTIVE_EMBEDDED_PLUGIN.write() = Some(plugin_id.clone());
                            embedded = Some((plugin_id.clone(), view, parent_hwnd));
                            crate::log_info!("NATIVE_WRY", "Embedded child webview attached for '{}'", plugin_id);
                        }
                        Err(e) => {
                            crate::log_error!("NATIVE_WRY", "Failed to attach embedded child webview for '{}': {:?}", plugin_id, e);
                            // 1. Reset Slint UI state immediately so interface doesn't hang/freeze on blank container
                            crate::system::slint_window::send_slint_ipc_message(serde_json::json!({
                                "action": "set_active_plugin_id",
                                "plugin_id": ""
                            }));
                            // 2. Automatically fallback to independent popout window so the plugin can still run
                            crate::log_info!("NATIVE_WRY", "Falling back to standalone popout window for '{}'...", plugin_id);
                            match spawn_popout_window(target, &proxy, &plugin_id, &mut web_context) {
                                Ok((win, view)) => {
                                    get_running_popouts_set().write().insert(plugin_id.clone());
                                    popouts.insert(plugin_id.clone(), (win, view));
                                    crate::log_info!("NATIVE_WRY", "Fallback popout window opened successfully for '{}'", plugin_id);
                                }
                                Err(pop_err) => {
                                    crate::log_error!("NATIVE_WRY", "Fallback popout window also failed for '{}': {:?}", plugin_id, pop_err);
                                }
                            }
                        }
                    }
                }

                NativeWryMessage::UpdateEmbeddedBounds { rect } => {
                    if let Some((_, ref view, _)) = embedded {
                        let _ = view.set_bounds(Rect {
                            position: wry::dpi::Position::Physical(tao::dpi::PhysicalPosition::new(rect.0, rect.1)),
                            size: wry::dpi::Size::Physical(tao::dpi::PhysicalSize::new(rect.2 as u32, rect.3 as u32)),
                        });
                    }
                }

                NativeWryMessage::HideEmbedded => {
                    *ACTIVE_EMBEDDED_PLUGIN.write() = None;
                    if let Some((pid, view, parent_hwnd)) = embedded.take() {
                        crate::system::telemetry::unregister_consumer(&pid);
                        crate::log_info!("NATIVE_WRY", "Dropping embedded child webview for '{}'", pid);
                        drop(view);

                        #[cfg(target_os = "windows")]
                        unsafe {
                            use windows::Win32::Foundation::HWND;
                            use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};
                            let h = HWND(parent_hwnd as *mut _);
                            let _ = InvalidateRect(h, None, true);
                            let _ = UpdateWindow(h);
                        }
                    }
                }

                NativeWryMessage::EvalJs { target, script } => {
                    match target {
                        ViewTarget::Popout(pid) => {
                            if let Some((_, view)) = popouts.get(&pid) {
                                if let Err(e) = view.evaluate_script(&script) {
                                    crate::log_warn!("NATIVE_WRY", "Eval JS error on popout '{}': {:?}", pid, e);
                                }
                            }
                        }
                        ViewTarget::Embedded => {
                            if let Some((_, ref view, _)) = embedded {
                                if let Err(e) = view.evaluate_script(&script) {
                                    crate::log_warn!("NATIVE_WRY", "Eval JS error on embedded view: {:?}", e);
                                }
                            }
                        }
                    }
                }

                NativeWryMessage::Shutdown => {
                    crate::log_info!("NATIVE_WRY", "Shutting down all native Wry views...");
                    get_running_popouts_set().write().clear();
                    *ACTIVE_EMBEDDED_PLUGIN.write() = None;
                    popouts.clear();
                    embedded = None;
                    *control_flow = ControlFlow::Exit;
                }
            },

            _ => {}
        }
    });
}

fn spawn_popout_window(
    target: &EventLoopWindowTarget<NativeWryMessage>,
    proxy: &EventLoopProxy<NativeWryMessage>,
    plugin_id: &str,
    web_context: &mut WebContext,
) -> Result<(Window, WebView), String> {
    let p_dir = crate::commands::system_cmd::find_plugin_dir(plugin_id);
    let manifest_path = p_dir.join("manifest.json");
    let entry_file = if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                manifest.tab.and_then(|t| t.entry).or(manifest.entry).unwrap_or_else(|| "index.html".into())
            } else {
                "index.html".into()
            }
        } else {
            "index.html".into()
        }
    } else {
        "index.html".into()
    };
    let target_file = p_dir.join(&entry_file);
    if !target_file.exists() {
        return Err(format!("Plugin entry file not found: {:?}", target_file));
    }

    let manifest_path = p_dir.join("manifest.json");
    let mut title = format!("OpenRevo - {}", plugin_id);
    let mut width = 1024.0;
    let mut height = 720.0;
    let mut resizable = true;
    let mut transparent = false;
    let mut decorations = true;
    let mut always_on_top = false;
    let mut skip_taskbar = false;
    let mut is_widget = false;
    let mut telemetry_config: Option<crate::commands::system_cmd::PluginTelemetryConfig> = None;
    let mut entry_file = "index.html".to_string();

    if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                let has_explicit_widget = manifest.widget.is_some();
                let p_type = manifest.plugin_type.as_deref().unwrap_or_default();
                is_widget = has_explicit_widget || p_type.eq_ignore_ascii_case("widget") || p_type.eq_ignore_ascii_case("skin");
                telemetry_config = manifest.telemetry;

                entry_file = if is_widget {
                    manifest.widget.as_ref().and_then(|w| w.entry.clone()).or(manifest.entry.clone()).unwrap_or_else(|| "index.html".into())
                } else {
                    manifest.tab.as_ref().and_then(|t| t.entry.clone()).or(manifest.entry.clone()).unwrap_or_else(|| "index.html".into())
                };

                if let Some(ref name) = manifest.name {
                    title = format!("OpenRevo - {}", name);
                }
                if is_widget {
                    width = 260.0;
                    height = 200.0;
                    resizable = false;
                    decorations = false;
                    transparent = true;
                    always_on_top = true;
                    skip_taskbar = true;
                }
                let win_cfg_opt = manifest.widget.as_ref().and_then(|w| w.window.clone()).or(manifest.window);
                if let Some(win_cfg) = win_cfg_opt {
                    width = win_cfg.width;
                    height = win_cfg.height;
                    resizable = win_cfg.resizable;
                    transparent = win_cfg.transparent;
                    always_on_top = win_cfg.always_on_top;
                    decorations = win_cfg.decorations;
                    skip_taskbar = win_cfg.skip_taskbar;
                }
            }
        }
    }

    let cfg = crate::system::config::load_config();
    let saved_pos = if is_widget {
        cfg.widget_positions.get(plugin_id).copied()
    } else {
        None
    };

    #[cfg(target_os = "windows")]
    use tao::platform::windows::WindowBuilderExtWindows;

    let mut win_builder = WindowBuilder::new()
        .with_title(&title)
        .with_inner_size(tao::dpi::LogicalSize::new(width, height))
        .with_resizable(resizable)
        .with_decorations(decorations)
        .with_transparent(transparent)
        .with_always_on_top(always_on_top)
        .with_visible(true);

    #[cfg(target_os = "windows")]
    {
        if skip_taskbar {
            win_builder = win_builder.with_skip_taskbar(true);
        }
    }

    if let Some((x, y)) = saved_pos {
        win_builder = win_builder.with_position(tao::dpi::PhysicalPosition::new(x, y));
    }

    let window = win_builder
        .build(target)
        .map_err(|e| format!("Failed to create tao window: {:?}", e))?;

    let proxy_for_ipc = proxy.clone();
    let plugin_id_for_ipc = plugin_id.to_string();
    let target_for_ipc = ViewTarget::Popout(plugin_id.to_string());

    let webview = WebViewBuilder::new_with_web_context(web_context)
        .with_transparent(transparent)
        .with_custom_protocol("revo".into(), move |_id, request| {
            handle_custom_protocol(request)
        })
        .with_url(&format!("revo://localhost/{}/{}", plugin_id, entry_file))
        .with_initialization_script(&get_bridge_script(plugin_id))
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body().clone();
            let pid = plugin_id_for_ipc.clone();
            let px = proxy_for_ipc.clone();
            let tg = target_for_ipc.clone();
            handle_ipc_payload(&pid, &body, px, tg);
        })
        .build(&window)
        .map_err(|e| format!("Failed to build wry WebView: {:?}", e))?;

    window.set_visible(true);
    if !is_widget {
        window.set_focus();
    }

    let default_sensors = vec![
        "cpu_temp".into(), "gpu_temp".into(), "cpu_power".into(), "gpu_power".into(),
        "cpu_fan_rpm".into(), "gpu_fan_rpm".into(), "fps".into(), "frametime_ms".into(),
        "power_mode".into(), "fan_boost".into(), "battery_level".into(), "is_charging".into()
    ];
    let should_register_telemetry = if let Some(ref tel) = telemetry_config {
        tel.enabled
    } else {
        is_widget
    };

    if should_register_telemetry {
        let sensors = telemetry_config
            .map(|t| t.sensors)
            .filter(|s| !s.is_empty())
            .unwrap_or(default_sensors);
        crate::system::telemetry::register_consumer(plugin_id, &sensors);
    }

    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowExtWindows;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SetForegroundWindow, SW_SHOW};
        let hwnd = HWND(window.hwnd() as *mut _);
        unsafe {
            if transparent {
                use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
                use windows::Win32::UI::Controls::MARGINS;
                let margins = MARGINS {
                    cxLeftWidth: -1,
                    cxRightWidth: -1,
                    cyTopHeight: -1,
                    cyBottomHeight: -1,
                };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
            }
            let _ = ShowWindow(hwnd, SW_SHOW);
            if !is_widget {
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }

    Ok((window, webview))
}

fn spawn_embedded_child(
    proxy: &EventLoopProxy<NativeWryMessage>,
    parent_hwnd: isize,
    plugin_id: &str,
    rect: (i32, i32, i32, i32),
    web_context: &mut WebContext,
) -> Result<WebView, String> {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::IsWindow;
        if !IsWindow(HWND(parent_hwnd as *mut _)).as_bool() {
            return Err(format!("Parent HWND 0x{:X} is not a valid Win32 window", parent_hwnd));
        }
    }

    let p_dir = crate::commands::system_cmd::find_plugin_dir(plugin_id);
    let manifest_path = p_dir.join("manifest.json");
    let entry_file = if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                manifest.tab.and_then(|t| t.entry).or(manifest.entry).unwrap_or_else(|| "index.html".into())
            } else {
                "index.html".into()
            }
        } else {
            "index.html".into()
        }
    } else {
        "index.html".into()
    };
    let target_file = p_dir.join(&entry_file);
    if !target_file.exists() {
        return Err(format!("Plugin entry file not found: {:?}", target_file));
    }

    let proxy_for_ipc = proxy.clone();
    let plugin_id_for_ipc = plugin_id.to_string();
    let target_for_ipc = ViewTarget::Embedded;

    let safe_w = rect.2.max(100) as u32;
    let safe_h = rect.3.max(100) as u32;
    let wry_rect = Rect {
        position: wry::dpi::Position::Physical(tao::dpi::PhysicalPosition::new(rect.0.max(0), rect.1.max(0))),
        size: wry::dpi::Size::Physical(tao::dpi::PhysicalSize::new(safe_w, safe_h)),
    };

    let parent_wrapper = HwndWrapper(parent_hwnd);

    let webview = WebViewBuilder::new_with_web_context(web_context)
        .with_bounds(wry_rect)
        .with_custom_protocol("revo".into(), move |_id, request| {
            handle_custom_protocol(request)
        })
        .with_url(&format!("revo://localhost/{}/{}", plugin_id, entry_file))
        .with_initialization_script(&get_bridge_script(plugin_id))
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body().clone();
            let pid = plugin_id_for_ipc.clone();
            let px = proxy_for_ipc.clone();
            let tg = target_for_ipc.clone();
            handle_ipc_payload(&pid, &body, px, tg);
        })
        .build_as_child(&parent_wrapper)
        .map_err(|e| format!("Failed to build child wry WebView: {:?}", e))?;

    let manifest_path = p_dir.join("manifest.json");
    if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<crate::commands::system_cmd::RawPluginManifest>(&content) {
                if let Some(ref tel) = manifest.telemetry {
                    if tel.enabled {
                        let default_sensors = vec![
                            "cpu_temp".into(), "gpu_temp".into(), "cpu_power".into(), "gpu_power".into(), "fps".into()
                        ];
                        let sensors = if tel.sensors.is_empty() {
                            default_sensors
                        } else {
                            tel.sensors.clone()
                        };
                        crate::system::telemetry::register_consumer(plugin_id, &sensors);
                    }
                }
            }
        }
    }

    Ok(webview)
}

// --- Public Facade APIs ---

fn get_active_proxy() -> Option<EventLoopProxy<NativeWryMessage>> {
    if IS_SHUTTING_DOWN.load(Ordering::SeqCst) {
        return None;
    }
    PROXY_HOLDER.lock().clone()
}

pub fn open_popout(plugin_id: &str) {
    if let Some(proxy) = ensure_native_wry_loop() {
        let _ = proxy.send_event(NativeWryMessage::OpenPopout {
            plugin_id: plugin_id.to_string(),
        });
    }
}

pub fn close_popout(plugin_id: &str) {
    if let Some(proxy) = get_active_proxy() {
        let _ = proxy.send_event(NativeWryMessage::ClosePopout {
            plugin_id: plugin_id.to_string(),
        });
    }
}

pub fn attach_or_switch_embedded(parent_hwnd: isize, plugin_id: &str, rect: (i32, i32, i32, i32)) {
    if let Some(proxy) = ensure_native_wry_loop() {
        let _ = proxy.send_event(NativeWryMessage::AttachEmbedded {
            parent_hwnd,
            plugin_id: plugin_id.to_string(),
            rect,
        });
    }
}

pub fn update_embedded_bounds(rect: (i32, i32, i32, i32)) {
    if let Some(proxy) = get_active_proxy() {
        let _ = proxy.send_event(NativeWryMessage::UpdateEmbeddedBounds { rect });
    }
}

pub fn hide_embedded() {
    if let Some(proxy) = get_active_proxy() {
        let _ = proxy.send_event(NativeWryMessage::HideEmbedded);
    }
}

pub fn reload_active_embedded() {
    if let Some(proxy) = get_active_proxy() {
        let _ = proxy.send_event(NativeWryMessage::EvalJs {
            target: ViewTarget::Embedded,
            script: "window.location.reload();".to_string(),
        });
    }
}

pub fn get_running_popouts() -> Vec<String> {
    get_running_popouts_set().read().iter().cloned().collect()
}

pub fn broadcast_event(event: &str, payload: &serde_json::Value) {
    if let Some(proxy) = get_active_proxy() {
        if ACTIVE_EMBEDDED_PLUGIN.read().is_none() && get_running_popouts_set().read().is_empty() {
            return;
        }
        let _ = proxy.send_event(NativeWryMessage::BroadcastEvent {
            event: event.to_string(),
            payload_json: payload.to_string(),
        });
    }
}

pub fn shutdown_all() {
    IS_SHUTTING_DOWN.store(true, Ordering::SeqCst);
    if let Some(proxy) = PROXY_HOLDER.lock().take() {
        let _ = proxy.send_event(NativeWryMessage::Shutdown);
    }
}
