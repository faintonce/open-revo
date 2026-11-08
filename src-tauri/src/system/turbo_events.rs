// OpenRevo - 狂暴子模式事件发布通道
//
// 命令路径（UI 切换）与驱动路径（物理 Q-key / 托盘 / 模式 3 重放）必须使用
// 同一条通道，否则某个 surface 会漏掉事件。
//
// 统一策略：
//   1) `openrevo_core::emit_global` -> 当前运行时的全局 sink。Tauri 运行时下
//      TauriEventSink 会把事件 `app.emit` 给所有 WebViewWindow。
//   2) `native_wry::broadcast_event` -> 独立 wry 弹出/内嵌 surface。
//
// 因此这里不再直接持有 `AppHandle`：命令路径与驱动路径调用同一函数，
// 每个目标只收到一次事件，且事件通道不依赖调用者是否持有 UI 句柄。

pub const TURBO_SUBMODE_CHANGED: &str = "turbo-submode-changed";

/// 广播已经由硬件回读确认的子模式状态。
///
/// 调用者必须只在硬件状态确认后调用；读取失败时不要用 `silent=false` 兜底，
/// 应跳过本事件（`physical-mode-switched` / 立即刷新查询会兜底）。
pub fn publish_turbo_submode_changed(supported: bool, silent: bool) {
    let payload = serde_json::json!({
        "supported": supported,
        "silent": silent,
    });
    crate::openrevo_core::emit_global(TURBO_SUBMODE_CHANGED, payload.clone());
    crate::system::native_wry::broadcast_event(TURBO_SUBMODE_CHANGED, &payload);
}
