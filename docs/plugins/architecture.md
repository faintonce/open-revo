> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 架构与插件形态

## 1. 微内核架构宣言：OpenRevo 的本质是什么？

很多开发者初次接触 OpenRevo 时，以为它仅仅是一个“机械革命第三方笔记本控制中心软件”。**这是对 OpenRevo 最大的低估**。

### 核心真相：OpenRevo 的本质是「笔记本硬件微内核平台（Hardware Microkernel）」
* **底座是真正的微内核 (Rust Core Engine)**：
  它掌控着 Ring0 ACPI 驱动通信、同方与宝龙达模具的 EC 物理寄存器钥匙库（Vault）、NVIDIA 显卡驱动交互、ITE 独立芯片的高性能 60 FPS 键盘流光推流（Better RGB）、低功耗蓝牙外置水冷魔盒通信以及底层键盘钩子。
* **官方自带的 `main` 主控制台，仅仅是一个“默认客户端”**：
  官方的 React 亚克力界面只是运行在微内核上的一个 Webview 展现层。
* **一切皆可插拔，前端完全可以自定义**：
  如果用户愿意，**完全可以关掉官方自带的 `main` 界面**，桌面上只挂一个极简的透明机甲桌宠；或者使用 Vue、Svelte、原生 HTML5、甚至是纯 Python 脚本，自建一套完全属于自己的赛博朋克控制台！微内核都会默默在后台提供坚如磐石的硬件驱动和毫秒级总线调度。

```
┌────────────────────────────────────────────────────────────────────────┐
│                        OpenRevo 微内核平台架构图                       │
├────────────────────────────────────────────────────────────────────────┤
│  [官方默认控制台]    [第三方异形桌宠]    [AI语音助手]    [极客自建客户端]   │
│  (React Full GUI)    (Chibi Widget)     (Desktop Agent) (Custom Webview) │
├────────────────────────────────────────────────────────────────────────┤
│                       Tauri 2 跨窗口事件与 IPC 通信总线                 │
│         ├─ hardware-state-updated (按需自适应毫秒级遥测流)              │
│         ├─ fan-boost-changed / physical-mode-switched (硬件状态广播)   │
│         └─ invoke('set_fan_boost') / invoke('set_power_mode') (控制API) │
├────────────────────────────────────────────────────────────────────────┤
│                   HAL Capability Broker 语义化能力网关                  │
│             (权限防呆 ➔ 边界钳位 ➔ 阻断裸 EC 寄存器 ➔ 自动析构)         │
├────────────────────────────────────────────────────────────────────────┤
│                 Hardware Vault (硬件钥匙库与原生内核驱动)               │
│      [Tongfang ACPI IOCTL]   [ITE Lighting HID]   [WinRT Bluetooth]    │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. 插件形态与能力声明大一统矩阵 (Capability-Driven Architecture)

OpenRevo 坚决反对“无脑开 Webview”。我们确立了**按需分配运行时原则**与**“一切皆插件”能力驱动哲学**：
> 💡 **架构大一统升级**：从 v0.9.7 起，插件彻底摆脱了排他式的单一类型限制！插件是一个功能实体，而 **`tab`（控制台视窗）与 `widget`（桌面独立视窗）只是该插件可自由选配的界面出口（Surfaces）**。
> 一个插件可以既有桌面悬浮条，又自带控制台设置面板；若未声明任何视窗，则自动作为 **0 GUI / 0 MB 纯无头服务（Headless Service）** 在后台静默调度。

| 能力通道与形态 | 声明方式 | 运行时载体 | 增量内存开销 | 视窗与交互特性 |
| :--- | :--- | :--- | :--- | :--- |
| **`tab` (控制台专属页签)** | `"tab": { "entry": "...", "title": "..." }` 或根 `entry` | **宿主工作区 WebView2 或 Slint 原生贴片** | **< 5 MB** (Webview)<br>或 **0 MB Webview** (Slint) | 嵌入在官方主控制台工作区中，可选 WebView2 富生态或 Slint 原生 GPU 动态渲染 |
| **`widget` (桌面悬浮挂件)** | `"widget": { "entry": "...", "window": { ... } }` | **独立 Edge WebView2 或 Slint 原生视窗** | **15MB ~ 25MB** (Webview)<br>或 **< 8MB** (Slint) | 桌面独立挂件、全透明、置顶，可选 WebView2 或 0MB Chromium 原生 Slint 极速光栅化 |
| **`hybrid` (正交混搭体系)** | `tab.type: "webview2"` + `widget.type: "slint"` | **双引擎正交分流** | **按需最佳组合** | **大面板走前端 Web 生态 (React/Vue/ECharts)，常驻桌面挂件走 Slint 原生极低内存常驻** |
| **`app` (复合全功能插件)** | 同时声明 `tab` 与 `widget`（如 `mro-hud`） | **双出口并存调度** | 按需组合 | 既在桌面上常驻悬浮条，又在主控制台提供完整设置页，**单包全生命周期闭环** |
| **`skin` (主窗口皮肤外观)** | `"theme_css": "theme.css"` | **宿主主窗口 Webview** | **0 MB**<br>(纯样式热装卸) | 动态注入 `<style id="openrevo-theme-skin">` + 全局 `data-skin`，可平滑重置窗口几何尺寸 |
| **`background` (独立背景壁纸)** | `"image": "bg.png"` 或 `"background_css": "..."` | **宿主主窗口 Webview** | **0 MB**<br>(正交独立通道) | 覆盖主窗体 (`.openrevo-shell`) 与抽屉 (`.mini-drawer-root`)，与 `skin` 自由组合并存 |
| **`slot` (微插槽组件插件)** | `"slot": "power.hardware_limit"`, `"entry": "..."` | **宿主界面预留插槽** | **< 1 MB** (微型 iframe) | 精准接管宿主预留位（如电池卡片内部），严丝合缝无缝继承 Design Tokens |
| **`service` / `headless` (无头后台服务)** | 仅声明 `watchers` / `shortcuts` / `service` | **Rust 内置微任务 / Watcher Hub** | **0 ~ 100 KB** (极致轻量) | **完全无界面 (Zero GUI / 0 MB)**，坚决不弹窗，由内核心跳与底层键盘钩子接管 |

> 💡 **双通道样式解耦优势**：
> `skin`（主题骨架）与 `background`（背景壁纸）拥有完全独立的样式通道与持久化配置键（`active_skin` 与 `active_background`）。用户可以选一套沉稳的极简工业风主题，同时配上一张自己心仪的动漫背景，两类插件可以同时启用、自由搭配！

---

### 2.1 正交混搭渲染架构 (Orthogonal Hybrid Architecture: WebView2 + Slint)

随着 OpenRevo 架构演进，微内核率先实现了 **WebView2 (Chromium/Edge) 与 Slint 原生动态编译渲染引擎的正交混搭与自由组合**：

#### 核心痛点解决：生态丰富度与桌面轻量常驻的完美统一
- **痛点**：传统插件开发中，开发者常常陷入两难——Web 技术（React/Vue/ECharts/Tailwind）开发大面板极为高效、生态极其丰富，但如果将其作为桌面常驻悬浮条，Chromium 即使再优化也需要占用 20MB~30MB 内存并产生额外的渲染开销；而如果全量改用 C++/原生 GUI 开发，又失去了 Web 生态庞大的组件库和敏捷性。
- **解法**：OpenRevo 提出**正交混搭架构（Orthogonal Hybrid Architecture）**。在同一个插件清单内，**`tab` 与 `widget` 各自独立指定渲染引擎**：
  - **`tab` 采用 `webview2`**：嵌入宿主工作区的大面板使用富交互 Web 前端开发，尽情享受 ECharts 电池健康历史折线图、复杂表格和炫酷 CSS 动效；
  - **`widget` 采用 `slint`**：常驻 Windows 桌面的轻量悬浮条使用声明式 `.slint` 编写，由 Rust 内置的 `slint-interpreter` 动态 JIT 编译并以 FemtoVG/Skia GPU 原生光栅化执行，**0MB Chromium 运行时占用、冷启动仅需数十毫秒、内存极低常驻**！

#### 四大组合矩阵：
1. **模式 A：`tab (webview2)` + `widget (slint)`（推荐黄金搭档）**：兼顾复杂大控制台与极致极速桌面挂件；
2. **模式 B：`tab (slint)` + `widget (slint)`（全原生模式）**：纯净无 Webview 依赖，全部由 Slint 驱动；
3. **模式 C：`tab (slint)` + `widget (webview2)`**：主面板极速直出，挂件使用现有 HTML 动画挂件；
4. **模式 D：`tab (webview2)` + `widget (webview2)`**：经典传统 Web 插件。

#### 引擎分流判定规则：
宿主微内核在调度 `select_plugin_tab`（进入面板）与 `open_custom_widget` / `toggle_custom_plugin`（拉起挂件）时，按照以下优先级严格分流：
- **显式声明优先**：`tab.type` / `widget.type`（支持 `"slint"` 与 `"webview2"` / `"web"`，别名 `engine`/`plugin_type`）；
- **文件后缀推断**：入口文件以 `.slint` 结尾自动路由到 Slint JIT 执行器；以 `.html` / `.htm` 结尾自动路由到 WebView2 视窗；
- **单体插件回退**：若插件根目录下仅存在 `widget.slint` 或 `tab.slint`，宿主亦能智能探测并正交分流。

---

## 相关页面

- 上一页：[插件与扩展导览](README.md)
- 下一页：[核心安全军规与沙盒边界](security.md)
- 返回目录：[插件与扩展导览](README.md)
