// OpenRevo - Central Webview Orchestrator (视窗统一调度中枢)
// System-level atomic controller for all secondary Webview2 instances (OSD & Widgets).
// Guarantees strictly serialized, staggered window instantiation with circuit breaker timeouts.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};

/// Specific window creation task payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowTaskType {
    /// OSD HUD overlay window
    Osd,
    /// Desktop organic widget sandbox window
    Widget { plugin_id: String },
}

/// Central orchestrator state and queue keeper
pub struct WebviewOrchestrator {
    app: OnceLock<AppHandle>,
    queue: Mutex<VecDeque<WindowTaskType>>,
    is_main_ready: AtomicBool,
    is_worker_running: AtomicBool,
}

impl WebviewOrchestrator {
    /// Retrieve global singleton instance
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<WebviewOrchestrator> = OnceLock::new();
        INSTANCE.get_or_init(|| WebviewOrchestrator {
            app: OnceLock::new(),
            queue: Mutex::new(VecDeque::new()),
            is_main_ready: AtomicBool::new(false),
            is_worker_running: AtomicBool::new(false),
        })
    }

    /// Bind global AppHandle during early startup and spawn worker loop
    pub fn bind_app_handle(&self, app: AppHandle) {
        if self.app.set(app.clone()).is_ok() {
            crate::log_info!("ORCHESTRATOR", "AppHandle successfully bound to WebviewOrchestrator");
            let cfg = crate::system::config::load_config();
            if !cfg.use_classic_webview {
                self.is_main_ready.store(true, Ordering::SeqCst);
                crate::log_info!(
                    "ORCHESTRATOR",
                    "Native Slint mode active: Sub-webview FIFO pipeline unlocked immediately by host microkernel."
                );
            }
            self.ensure_worker();
        }
    }

    /// Retrieve stored AppHandle
    pub fn app(&self) -> Option<AppHandle> {
        self.app.get().cloned()
    }

    /// Signal from frontend that the primary 'main' Webview DOM mounted and IPC loop is 100% active.
    /// This is the inviolable gatekeeper: unlocks the FIFO queue for secondary webview creation.
    pub fn notify_main_ready(&self) {
        if !self.is_main_ready.swap(true, Ordering::SeqCst) {
            crate::log_info!(
                "ORCHESTRATOR",
                "Main window handshake verified! Sub-webview FIFO pipeline unlocked."
            );
            self.ensure_worker();
        }
    }

    /// Check if primary window is ready
    pub fn is_main_ready(&self) -> bool {
        self.is_main_ready.load(Ordering::Relaxed)
    }

    /// Enqueue a secondary window creation task
    pub fn enqueue(&self, task: WindowTaskType) {
        crate::log_info!("ORCHESTRATOR", "Enqueue requested for task: {:?}", task);
        {
            let mut q = self.queue.lock();
            // Prevent duplicate tasks in queue
            if !q.contains(&task) {
                crate::log_info!("ORCHESTRATOR", "Task enqueued: {:?}", task);
                q.push_back(task);
            } else {
                crate::log_info!("ORCHESTRATOR", "Task already in queue, skipping duplicate: {:?}", task);
                return;
            }
        }
        self.ensure_worker();
    }

    /// Enqueue OSD overlay window creation
    pub fn enqueue_osd(&self) {
        self.enqueue(WindowTaskType::Osd);
    }

    /// Enqueue a desktop widget creation
    pub fn enqueue_widget(&self, plugin_id: String) {
        self.enqueue(WindowTaskType::Widget { plugin_id });
    }

    /// Ensure the background queue processor worker is alive
    fn ensure_worker(&self) {
        if self.app.get().is_none() {
            return;
        }

        if !self.is_worker_running.swap(true, Ordering::SeqCst) {
            tauri::async_runtime::spawn(async move {
                Self::worker_loop().await;
            });
        }
    }

    /// Background queue processing loop
    async fn worker_loop() {
        crate::log_info!("ORCHESTRATOR", "WebviewOrchestrator worker loop started.");

        loop {
            let orchestrator = Self::global();

            // Gatekeeper 1: Wait until primary window has confirmed responsiveness
            if !orchestrator.is_main_ready.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }

            // Pop next task from FIFO queue
            let task_opt = {
                let mut q = orchestrator.queue.lock();
                q.pop_front()
            };

            let Some(task) = task_opt else {
                // Queue is empty: sleep and wait for next task
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };

            let Some(app) = orchestrator.app() else {
                crate::log_error!("ORCHESTRATOR", "AppHandle missing during task processing!");
                break;
            };

            // Gatekeeper 2: Execute task with deterministic 2500ms circuit-breaker watchdog
            crate::log_info!("ORCHESTRATOR", "Processing dequeued task: {:?}", task);
            match task {
                WindowTaskType::Osd => {
                    let app_clone = app.clone();
                    let build_future = tokio::time::timeout(
                        Duration::from_millis(2500),
                        async move {
                            crate::system::osd_window::build_osd_window_async(&app_clone).await
                        },
                    );

                    match build_future.await {
                        Ok(Ok(_)) => {
                            crate::log_info!("ORCHESTRATOR", "OSD window built successfully.");
                        }
                        Ok(Err(e)) => {
                            crate::log_error!("ORCHESTRATOR", "OSD window build returned error: {}", e);
                        }
                        Err(_) => {
                            crate::log_warn!(
                                "ORCHESTRATOR",
                                "OSD window build timed out (2500ms)! Terminating hanging instance to safeguard host."
                            );
                            if let Some(win) = app.get_webview_window("osd") {
                                let _ = win.destroy();
                            }
                        }
                    }
                }
                WindowTaskType::Widget { plugin_id } => {
                    let app_clone = app.clone();
                    let pid = plugin_id.clone();
                    crate::log_info!("ORCHESTRATOR", "Spawning widget window for '{}'...", pid);
                    let build_future = tokio::time::timeout(
                        Duration::from_millis(2500),
                        async move {
                            crate::system::widget_manager::spawn_widget_window_internal(&app_clone, &pid).await
                        },
                    );

                    match build_future.await {
                        Ok(Ok(label)) => {
                            crate::log_info!(
                                "ORCHESTRATOR",
                                "Widget '{}' ({}) built and mounted smoothly.",
                                plugin_id,
                                label
                            );
                        }
                        Ok(Err(e)) => {
                            crate::log_error!(
                                "ORCHESTRATOR",
                                "Widget '{}' build returned error: {}",
                                plugin_id,
                                e
                            );
                        }
                        Err(_) => {
                            crate::log_warn!(
                                "ORCHESTRATOR",
                                "Widget '{}' build timed out (2500ms)! Circuit breaker triggered: destroying and quarantining.",
                                plugin_id
                            );
                            let label = crate::system::widget_manager::format_widget_label(&plugin_id);
                            if let Some(win) = app.get_webview_window(&label) {
                                let _ = win.destroy();
                            }
                            // Auto-quarantine: disable plugin in config to prevent subsequent boot freezes
                            let mut cfg = crate::system::config::load_config();
                            cfg.custom_plugin_toggles.insert(plugin_id.clone(), false);
                            crate::system::config::save_config(&cfg);
                            crate::system::telemetry::unregister_consumer(&plugin_id);
                        }
                    }
                }
            }

            // Gatekeeper 3: Staggered cooldown period (150ms) to smooth out Chromium GPU composition spikes
            tokio::time::sleep(Duration::from_millis(150)).await;
        }

        Self::global().is_worker_running.store(false, Ordering::SeqCst);
    }

    /// Graceful shutdown: safely close and destroy all secondary webviews to release Edge EBWebView file locks
    pub fn safe_shutdown_all_subviews(&self) {
        if let Some(app) = self.app() {
            crate::log_info!("ORCHESTRATOR", "Executing clean shutdown of all secondary Webview windows...");

            // 1. OSD overlay
            if let Some(win) = app.get_webview_window("osd") {
                let _ = win.destroy();
                crate::log_info!("ORCHESTRATOR", "OSD window destroyed cleanly.");
            }

            // 2. All running widgets
            if let Ok(widgets) = crate::system::widget_manager::get_running_widgets() {
                for pid in widgets {
                    let label = crate::system::widget_manager::format_widget_label(&pid);
                    if let Some(win) = app.get_webview_window(&label) {
                        let _ = win.destroy();
                        crate::log_info!("ORCHESTRATOR", "Widget window '{}' destroyed cleanly.", label);
                    }
                }
            }
        }
    }
}
