---
name: openrevo-plugin-dev
description: Official OpenRevo expert agent skill for developing third-party plugins, custom themes, wallpapers, desktop widgets, and slot extensions with zero hallucinations.
---

# OpenRevo AI Agent 文档入口 (Agent Guide)

> **定位**：本文件是 OpenRevo 仓库**唯一面向 AI Agent 的入口**。任何 AI 代码助手在修改、生成或审计 OpenRevo 插件、主题、壁纸、挂件与插槽扩展之前，必须先读完本页。
>
> **权威优先级**：本页只做路由与约束，不重复技术细节。权威技术事实位于 [`docs/plugins/`](plugins/README.md) 与 [`docs/core/`](core/README.md)。若本页与链接页面冲突，**以链接的权威文档为准**，并在回复中说明冲突点。
>
> **用法**：`差不多逻辑就是用这一个 skill.md 去引导 AI 读取文档`——先定位任务所属章节，再按需阅读对应页面，不要凭记忆编造 API。

---

## 1. 任务分流表 (Scope Routing)

| 你的任务 | 先读 |
| :--- | :--- |
| 插件整体架构、能力矩阵、WebView2 / Slint 混搭 | [plugins/architecture.md](plugins/architecture.md) |
| `manifest.json` 字段、别名、`window` / `service` / `shortcuts` / `watchers` | [plugins/manifest-reference.md](plugins/manifest-reference.md) |
| 硬件 IPC 指令、沙盒 `postMessage` 桥、能力清单 | [plugins/capabilities.md](plugins/capabilities.md) |
| 遥测传感器声明、消费者生命周期、采样降频 | [plugins/telemetry.md](plugins/telemetry.md) |
| 跨插件事件广播、`postMessage` / `CustomEvent` 互通 | [plugins/events.md](plugins/events.md) |
| UI 插槽类名、Design Tokens、WebView2 暗色表单 | [plugins/ui-design.md](plugins/ui-design.md) |
| 安装、热刷新、安全模式排错 | [plugins/troubleshooting.md](plugins/troubleshooting.md) |
| 13 个可运行实战用例 | [plugins/use-cases/README.md](plugins/use-cases/README.md) |
| Core SDK（C# / C++ / C-ABI / Linux）接入 | [core/README.md](core/README.md) |
| 仓库整体文档导航 | [docs/README.md](README.md) |

---

## 2. Agent 工作规则 (Working Rules)

1. **不虚构**：不得编造 manifest 字段、CSS 选择器、IPC 命令名、事件名、EC 物理地址或运行时 API。只使用被链接文档明确记录的内容。
2. **优先复用示例**：写代码前先看 [`plugins_sample/`](../plugins_sample/README.md) 中对应形态的现成插件，照抄结构与命名。
3. **保持安全边界**：硬件安全铁律见 [plugins/security.md](plugins/security.md)；涉及专属硬件写入时保守处理，不扩大改动范围。
4. **裸 EC 访问需授权**：除非目标文档明确记录且用户明确要求，不得生成 `raw_ec_write` / `raw_ec_modify_mask` / 直接寄存器读写代码。
5. **禁止隐藏轮询挂件**：不得创建全透明隐藏 `widget` 用 `setInterval` 偷跑轮询；必须使用声明式 `watchers` 或遥测消费者机制（见 [plugins/telemetry.md](plugins/telemetry.md)、[plugins/capabilities.md](plugins/capabilities.md)）。
6. **先取证再编码**：动手前先请求或查阅相关页面；回复时报告你的假设与不确定项，不要用“应该可以”代替文档依据。

---

## 3. 运行时契约 (Runtime Contract)

适用于独立 Webview、Tab 与 Widget 运行环境：

- **Import Map**：宿主向视窗注入 `@tauri-apps/api/*` 模块映射，可直接
  `import { invoke } from '@tauri-apps/api/core'`、`import { listen } from '@tauri-apps/api/event'`。
- **全局对象**：`window.OpenRevo.invoke(...)`、`window.OpenRevo.listen(...)`、`window.OpenRevo.close()`、`window.OpenRevo.startDragging()`。
- **Tauri 全局**：`window.__TAURI__` 在 Tauri 运行时可用；Sandbox/原生 Wry 场景不保证存在，使用前先探测。
- **窗口拖拽**：声明式用 `data-tauri-drag-region`；编程式用 `getCurrentWebviewWindow().startDragging()` 或 `window.OpenRevo.startDragging()`。**同一元素不要同时使用 `data-tauri-drag-region` 与 `pointerdown` 手写 `startDragging()`**，否则会触发重复拖拽抖动。
- **多文件与资源**：`<link>`、`<script src>`、`<img>`、音视频等路径保持**相对于插件根目录**，由宿主运行时在内存中内联解析（含 CSS `url()`），不要手工编码 Base64。
- 细节与示例：[plugins/use-cases/01-desktop-widget.md](plugins/use-cases/01-desktop-widget.md)、[plugins/architecture.md](plugins/architecture.md)

---

## 4. 沙盒桥合约 (Sandbox Bridge Contract)

沙盒 iframe 通过 `window.parent.postMessage({ action, payload }, '*')` 请求宿主，通过 `window.addEventListener('message', ...)` 接收结果。动作名与事件名是**契约**，不得改写或臆造。

| 域 | 发送动作 (`action`) | 载荷 | 回传事件 (`e.data.type`) |
| :--- | :--- | :--- | :--- |
| 快捷键总线 | `GET_SHORTCUTS` | 无 | `OPENREVO_SHORTCUTS` |
| 快捷键总线 | `REGISTER_SHORTCUT` | `binding` | `OPENREVO_SHORTCUTS` |
| 快捷键总线 | `UNREGISTER_SHORTCUT` | `{ id }` | `OPENREVO_SHORTCUTS` |
| 裸 EC 网关 | `RAW_EC_READ` | `{ protocol, address }` | `OPENREVO_RAW_EC_READ_RESULT` |
| 裸 EC 网关 | `RAW_EC_WRITE` | `{ protocol, address, value }` | `OPENREVO_RAW_EC_READ_RESULT` |
| 裸 EC 网关 | `RAW_MAILBOX_INVOKE` | `{ in_bytes }` | `OPENREVO_RAW_EC_READ_RESULT` |

**Watcher 桥**：在微插槽 / Tab 沙盒内使用 `window.__OPENREVO_BRIDGE__`：

- `registerWatcher(definition)`：注册监听器，随视窗生命周期自动清理；
- `unregisterWatcher(id)`：注销指定监听器；
- `getRegisteredWatchers()`：查询当前注册列表。

完整签名、权限声明与安全约束见 [plugins/capabilities.md](plugins/capabilities.md)。裸 EC 相关动作仅在文档明确记录且用户授权时使用。

---

## 5. 安装与刷新流程 (Install & Refresh)

1. 将插件目录放到 `%APPDATA%\OpenRevo\plugins\<plugin_id>\`，根目录必须含 `manifest.json`（与入口文件）。
2. 打开 OpenRevo 主界面 →【插件】页签 → 右上角【刷新】，宿主调用 `invoke('get_custom_plugins')` 重新扫描并挂载；需要全量重载时用 `invoke('reload_plugins')`。
3. 若插件导致崩溃：编辑 `%APPDATA%\OpenRevo\config.json` 的 `custom_plugin_toggles`，将插件 ID 置为 `false`（或 `active_skin` 置 `null`），重开宿主即跳过。
4. 详见 [plugins/troubleshooting.md](plugins/troubleshooting.md)；目录与热刷新说明亦见 [plugins/capabilities.md](plugins/capabilities.md)。

---

## 6. 回复前自检清单 (Anti-Hallucination Checklist)

在向用户交付结论或代码前逐条确认：

- [ ] 我是否已指明本次改动依据的目标文档页面？
- [ ] 我引用的 `manifest.json` 字段是否在该页面中出现过（拼写与别名一致）？
- [ ] 我引用的 IPC 命令 / `postMessage` action / 事件名是否与文档逐字一致？
- [ ] 我是否越过了安全边界（裸 EC 写、GPU 无谓唤醒、隐藏轮询挂件）？
- [ ] 我是否给出了可点击的文档证据链接？
- [ ] 我是否明确说明了不确定项与假设？

---

## 7. 文档地图 (Document Map)

- [文档总入口 docs/README.md](README.md)：全仓库文档导航
- [插件与扩展手册 docs/plugins/README.md](plugins/README.md)：章节地图与 13 个用例
- [核心引擎与 SDK docs/core/README.md](core/README.md)：C# / C++ / C-ABI / Linux 接入
- [本页](skill.md)：AI Agent 唯一入口（先读这里，再按任务分流）
