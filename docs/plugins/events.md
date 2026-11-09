> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 7. 全域双向广播通信与跨插件事件总线 (Universal Broadcast Bus)

为消除旧版中“Tauri 界面与 Slint 原生界面事件割裂”、“主窗口无法与弹窗/iframe 双向对等广播”的痛点，OpenRevo 建立了**全域双向广播总线（Universal Broadcast Bus）**：

```
                    ┌──────────────────────────┐
                    │  Rust Microkernel Bus    │
                    │  (system/broadcast.rs)   │
                    └────────────┬─────────────┘
                                 │
         ┌───────────────────────┼────────────────────────┐
         ▼                       ▼                        ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│  Tauri Webviews  │    │  Slint Native    │    │  Sandbox Iframes │
│  & Popout Tabs   │    │  (Native Wry)    │    │  & Sidecars      │
│  (app.emit)      │    │  (postMessage)   │    │  (Event Bridge)  │
└──────────────────┘    └──────────────────┘    └──────────────────┘
```

## 1. 发射广播（Broadcast Emitter）

无论当前插件运行在何种宿主外壳中，均可向全域发射自定义事件：

* **方式 A：使用 Tauri 标准 `@tauri-apps/api/event`**：
  ```javascript
  import { emit } from '@tauri-apps/api/event';
  // 广播自定义业务事件
  await emit('my-plugin:state-changed', { status: 'running', count: 42 });
  ```
* **方式 B：调用宿主 IPC 发射塔**：
  ```javascript
  import { invoke } from '@tauri-apps/api/core';
  await invoke('emit_plugin_broadcast', {
    event: 'my-plugin:state-changed',
    payload: { status: 'running', count: 42 }
  });
  ```
* **方式 C：在 Slint Native Wry 或沙盒 iframe 中使用 `postMessage` / `__NATIVE_WRY_EMIT__`**：
  ```javascript
  // 方式 C-1: 宿主自动注入的 Native Wry 发射接口
  if (window.__NATIVE_WRY_EMIT__) {
    window.__NATIVE_WRY_EMIT__('my-plugin:state-changed', { status: 'running' });
  }
  // 方式 C-2: 标准跨文档消息
  window.parent.postMessage({
    action: 'EMIT',
    event: 'my-plugin:state-changed',
    payload: { status: 'running' }
  }, '*');
  ```

## 2. 接收广播（Broadcast Listener）

宿主保证所有发送到广播总线的事件，都会自动解构并投递到各前端环境：

* **方式 A：Tauri 模块监听**：
  ```javascript
  import { listen } from '@tauri-apps/api/event';
  const unlisten = await listen('my-plugin:state-changed', (event) => {
    console.log('收到总线广播:', event.payload);
  });
  ```
* **方式 B：DOM 原生 CustomEvent 监听**（Native Wry 自动桥接）：
  ```javascript
  window.addEventListener('my-plugin:state-changed', (event) => {
    console.log('收到 CustomEvent 广播:', event.detail);
  });
  ```
* **方式 C：标准 `window.message` 监听**（同时兼容 `OPENREVO_EVENT` 封包与直接事件）：
  ```javascript
  window.addEventListener('message', (event) => {
    // 兼容格式 1: 直接匹配事件名称
    if (event.data?.type === 'my-plugin:state-changed') {
      console.log('收到直投广播:', event.data.payload);
    }
    // 兼容格式 2: OPENREVO_EVENT 封包
    if (event.data?.type === 'OPENREVO_EVENT' && event.data.event === 'my-plugin:state-changed') {
      console.log('收到封包广播:', event.data.payload);
    }
  });
  ```

---

## 相关页面

- 上一页：[自适应遥测与动态心跳总线](telemetry.md)
- 下一页：[经典实战用例集](use-cases/README.md)
- 返回目录：[插件与扩展导览](README.md)
