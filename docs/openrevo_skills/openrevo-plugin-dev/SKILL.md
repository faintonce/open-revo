---
name: openrevo-plugin-dev
description: Official OpenRevo expert agent skill for developing third-party plugins, custom themes, wallpapers, desktop widgets, and slot extensions with zero hallucinations.
---

# OpenRevo 官方插件与主题扩展开发 Agent 指南 (OpenRevo Plugin Dev Skill)

> **设计宗旨**：本 Skill 是 OpenRevo 官方权威 Agentic 规范，专供 AI Agent 与高阶极客使用。旨在指导任何代码助手（Antigravity、Claude Code、Cursor、Copilot 等）以**零假幻觉、零二手失真、100% 贴合宿主真值**的方式开发和审计 OpenRevo 的第三方插件。

---

## 0. 核心心智模型与架构本质 (Mental Model)

1. **OpenRevo 的本质是「笔记本硬件微内核平台」**：
   - 底座是 Rust 原生微内核，管理 Ring0 ACPI、EC 物理钥匙库、NVAPI、ITE 60FPS 流光和 BLE 外置水冷；
   - 官方自带的 React 仪表盘仅仅是其中一个“默认客户端”；
   - 开发者可以通过插件**深度换肤、重排布局、外挂独立桌面挂件、甚至完全自建控制台**。
2. **双通道样式解耦 (Dual-Channel Style Decoupling)**：
   - **主题皮肤通道 (`active_skin`)**：注入 `<style id="openrevo-theme-skin">`，负责整体色调、圆角、插槽排版、窗口最佳宽高；
   - **背景壁纸通道 (`active_background`)**：注入 `<style id="openrevo-custom-background">`，负责大面板与迷你抽屉的背景贴图与遮罩；
   - **两者完全正交解耦，可同时生效、任意组合！**
3. **零 Base64 原生支持**：
   - 宿主 Rust 后端在读取插件 CSS 时，**会自动将 `url('assets/bg.png')` 等相对路径在内存中解析并转为 Data URI 注入**；
   - **严禁编写任何 Python / Node.js 脚本强行把图片编码为长串 Base64**，直接写正常相对路径即可！
4. **Widget 悬浮挂件自由渲染与内存资产打包 (In-Memory Asset Bundler)**：
   - 挂件不再受任何固定 Demo 样式限制！只要插件根目录下包含 `index.html`（或 `manifest.entry`），挂件视窗将完全由开发者自由掌控；
   - **多文件与相对路径零心智负担**：插件目录内的 `<link rel="stylesheet">`、`<script src="...">`、`<img src="...">` 以及音视频标签，Rust 后端在内存中自动内联解析并转换为 Data URI，零 404、零跨域问题；
   - **Universal Tauri Bridge & 原生 Import Map**：宿主自动向挂件注入 `@tauri-apps/api/*` 模块映射与 `window.OpenRevo` / `window.__TAURI__` 全局对象，支持原生物理拖拽与毫秒级硬件遥测。
5. **绝对禁止创建“隐藏透明挂件偷跑轮询”（Anti-Hidden-Widget Anti-Pattern）**：
   - **历史灾难与误区**：部分旧版 Agent 或开发者为实现“监控结温或自动切模式”，错误地声明一个全透明隐藏的 `widget` 并在其 Webview 内部跑 `setInterval` 轮询死循环，白白浪费 25MB 内存，且会被 Chromium 激进节流冻结导致切模失效；
   - **核心军规**：**必须且只能使用微内核内建的 `watchers` 声明式规则总线**！宿主内核统一调度，0 MB 视窗、0.0% CPU 占用、100% 精准唤醒。
6. **全域双向广播总线 (Universal Broadcast Bus)**：
   - 彻底打破传统桌面端与嵌入式沙盒的事件隔离壁垒。无论运行在 Tauri Webview、超轻量 Slint 原生端 (Native Wry) 还是沙盒 iframe 中，微内核均提供统一的双向事件通道；
   - 支持跨插件直接互通，支持标准 `@tauri-apps/api/event`、`window.postMessage` 与 DOM 原生 `CustomEvent` 无缝互联互通。
7. **按需硬件遥测消费驱动 (Consumer Lifecycle Driving)**：
   - 遥测引擎坚决杜绝无脑高频轮询。**插件必须在 `manifest.json` 中显式声明 `telemetry` 传感器清单**；
   - 宿主自动为激活的 Tab 或挂件注册 Consumer，并在切走或关闭时注销。当活跃消费者为 0 时，采样引擎彻底停跳休眠，确保独显 D3Cold 与 CPU 深睡零功耗！
8. **伴随提权服务 (Companion Service) 全运行时统一调度**：
   - Slint 与 Tauri 双端统一管理，解决后台原生进程（如 Python、C#、Node 或守护程序）的生命周期；
   - 支持宿主开机自愈静默就绪 (`bootstrap_enabled_plugins`)、端口探活、以及级联安全杀进程树防孤儿。

---

## 1. 硬件安全与沙盒防火墙 (Inviolable Guardrails)

在任何插件开发与代码生成中，必须无条件遵循以下安全铁律：
1. **绝对禁止裸物理寄存器交互（Rule 4 硬件防火墙）**：
   - 严禁在插件中请求类似 `ec_write(0x1844, 0x01)` 或 dump 整块 EC RAM；
   - 必须且只能调用宿主提供的**语义化 IPC 硬件指令**（如 `invoke('set_fan_boost', { enable: true })`）。
2. **严禁无节制唤醒独显（Anti-dGPU Wake-Lock）**：
   - 在双显卡笔记本轻度办公时，NVIDIA 独立显卡处于 **D3Cold 零功耗深睡 (0W)**；
   - 编写挂件或插件时，若不需要显示显卡温度或显存，**清单中严禁声明 `gpu_*` 字段**！只有当所有运行插件都没声明 GPU 时，宿主才保证独显持续安详深睡。
3. **删除操作必须走回收站**：
   - 插件管理或测试脚本如需删除临时文件，严禁使用带转义引号的 CMD 通配符删除，必须优先使用系统回收站 API。

---

## 2. 插件能力声明大一统矩阵 (Capability-Driven Architecture)

从 v0.9.7 起，插件彻底告别排他式的单一类型限制，拥抱**能力声明积木模型**：
- **`tab`** 与 **`widget`** 是同一个插件的可选界面出口（Surfaces）。复合插件（如 `mro-hud`）可同时拥有桌面悬浮条与控制台设置页；
- 未声明任何界面的插件自动作为 **0 GUI / 0 MB 纯无头服务（Headless Service）** 在后台静默运行；
- 老插件格式 100% 自动无感归一化向下兼容。

| 能力通道与形态 | 声明方式 | 运行时载体 | 视窗与样式特征 |
| :--- | :--- | :--- | :--- |
| **`tab`** | `"tab": { "entry": "...", "title": "..." }` 或根 `entry` | 宿主沙盒 / Slint 子窗口 | 嵌入在官方主控制台工作区中，可选 WebView2 或 Slint 原生渲染 |
| **`widget`** | `"widget": { "entry": "...", "window": { ... } }` | 独立 Edge WebView2 / Slint 原生 | 独立悬浮条、全透明、置顶，可选 WebView2 或 0MB Chromium 原生 Slint |
| **`hybrid` (正交混搭)** | `tab.type: "webview2"` + `widget.type: "slint"` | 双引擎正交协同 | 大面板采用现代 Web 生态（React/Vue/ECharts），常驻悬浮挂件采用 Slint 极速光栅化 |
| **`app` (复合体)** | 同时声明 `tab` 与 `widget` (如 `mro-hud`) | 双出口协同运行 | 既有桌面 HUD 悬浮条，又有控制台设置面板，单包闭环 |
| **`skin`** | `"theme_css": "theme.css"` | 宿主主窗口 Webview | 注入 `<style id="openrevo-theme-skin">`，挂载全局 `data-skin`，可平滑重设窗口宽高 |
| **`background`** | `"image": "bg.png"` 或 `"background_css": "..."` | 宿主主窗口 Webview | 注入 `<style id="openrevo-custom-background">`，挂载全局 `data-background`，双通道正交 |
| **`slot`** | `"slot": "power.hardware_limit"`, `"entry": "..."` | 宿主预留微插槽 | 嵌入官方界面对应卡片微区域，严丝合缝继承主题 Tokens |
| **`service` / `headless`** | 仅声明 `service` / `watchers` / `shortcuts` | Rust 内置微任务 / Watcher Hub | **完全无界面 (Zero GUI / 0 MB)**，坚决不弹窗，静默极低功耗调度 |

---

## 2.1 正交混搭架构规范 (Orthogonal Hybrid Architecture)

OpenRevo 率先实现了 **WebView2 与 Slint 原生渲染引擎的正交混搭与自由组合**：
- **核心痛点解决**：以往开发者必须在“丰富的 Web 生态大控制台”和“低内存常驻桌面挂件”之间二选一；
- **正交自由组合**：同一个插件内，`tab`（嵌入主界面面板）与 `widget`（桌面常驻悬浮视窗）彼此独立，各自可选 `"webview2"` 或 `"slint"`：
  1. **模式 A：`tab (webview2)` + `widget (slint)`（最强黄金组合）**
     - 控制台主页面采用 Vue/React/HTML 富生态，绘制丰富图表、电池历史曲线与配置树；
     - 桌面挂件采用 Slint 原生编译动态执行，极速启动、GPU 光栅化、0MB Chromium 内存开销！
  2. **模式 B：`tab (slint)` + `widget (slint)`**：全原生纯净模式，0 额外 Chromium 依赖。
  3. **模式 C：`tab (slint)` + `widget (webview2)`**：主面板极速直出，挂件使用现有 Web 样式。
  4. **模式 D：`tab (webview2)` + `widget (webview2)`**：经典传统 Web 插件。

### 清单声明范例 (`manifest.json`)：
```json
{
  "id": "hybrid-battery",
  "name": "电池卫士 (混搭示范)",
  "version": "1.0.0",
  "tab": {
    "type": "webview2",
    "entry": "dist/index.html",
    "title": "电池管理控制台"
  },
  "widget": {
    "type": "slint",
    "entry": "widget.slint",
    "title": "电池常驻挂件",
    "window": { "width": 260, "height": 80 }
  }
}
```
> 宿主自动根据声明的 `type`（或入口后缀 `.slint` / `.html`）完成底层 Win32 进程与 Wry 视窗的正交分流调度。

---

## 3. 插件清单权威规范 (`manifest.json`)

每个插件根目录下必须包含 `manifest.json`：

```json
{
  "id": "my-cool-plugin",
  "name": "插件展示名称",
  "version": "1.0.0",
  "author": "Your Name",
  "description": "插件详细功能描述",
  "plugin_type": "skin",
  "theme_css": "theme.css",
  "background_css": "background.css",
  "image": "assets/wallpaper.jpg",
  "entry": "index.html",
  "window": {
    "width": 1080,
    "height": 780,
    "transparent": false,
    "always_on_top": false,
    "resizable": true
  },
  "telemetry": {
    "enabled": true,
    "desiredIntervalMs": 1500,
    "sensors": [
      "cpu_temp",
      "cpu_fan_rpm",
      "fan_boost",
      "battery_level",
      "is_charging",
      "is_ac_online"
    ]
  },
  "permissions": ["telemetry:read", "fan:control", "power:control"],
  "min_host_version": "0.9.3"
}
```

### 关键字段兼容性指南：
* **`telemetry`（声明式硬件遥测按需订阅）**：
  若插件需要实时状态（如电池、温度、风扇），**必须显式声明 `telemetry` 对象**。
  - 核心字段：`cpu_temp`, `cpu_fan_rpm`, `cpu_power_w`, `power_mode`, `fan_boost`, `battery_level` (0~100), `is_charging` (bool), `is_ac_online` (bool)。
  - **防独显唤醒原则**：只有需要展示显卡温度或显存时才声明 `gpu_*`，严禁无脑索取 GPU 传感器导致独显 D3Cold 被打断。
* **`min_host_version`（宿主最低兼容版本 / 别名 `min_version` / `openrevo_version`）**：
  推荐使用了新架构特性（如 `watchers` 规则中心、快捷键总线、提权服务等）的插件明确声明（如 `"min_host_version": "0.9.3"`）。宿主基于标准 SemVer 语义化规范进行版本断言；若用户的 OpenRevo 版本低于该要求，管理器会自动呈现醒目的 `⚠️ 需 OpenRevo v0.9.3+` 告警徽章并安全拦截启闭，彻底告别“用户拿旧版二进制到处搜不到新功能”的痛点；
* **`plugin_type` 与 `type`**：宿主 Rust 后端已配置 alias，**两者完全等价**；
* **`theme_css`**：仅供 `skin` 插件使用，默认读取 `"theme.css"`；
* **`background_css`**：仅供 `background` 插件使用，默认读取 `"background.css"`（回退读取 `"theme.css"`）；
* **`image`（零代码壁纸）**：仅供 `background` 插件使用。如果声明了此字段且无 CSS 文件，宿主会自动生成全套铺满与磨砂适配样式！
* **`window`（全场景视窗与复杂工具窗口）**：
  * `decorations: true`：启用 Windows 原生标题栏（最小化/最大化/关闭按钮），XTU/SMU 复杂工具推荐开启；
  * `skip_taskbar: false`：在 Windows 系统任务栏显示独立图标，支持 `Alt+Tab` 秒切；
  * `transparent: false`：实色窗口背景，适用于非异形工具视窗；
  * **Tab 双模弹出机制**：无论插件是 `tab` 还是 `widget`，主界面选项卡右上角原生提供【独立视窗运行】按钮，零代码改动即可脱离主控台独立弹出！
* **`service`（伴随后台提权服务 / Sidecar 声明 - Slint/Tauri 双端打通）**：
  专为 `ec-config`、`cpu-guardian`、`mro-hud` 等需要调用原生 `.exe` 的硬件级插件设计，由宿主微内核统一监督管理，彻底解决双运行时差异与引导用户点黑框的割裂：
  ```json
  "service": {
    "executable": "agent/ec_agent.exe",
    "port": 18945,
    "health_url": "http://127.0.0.1:18945/api/ping",
    "elevated": true,
    "auto_start": false,
    "auto_stop": true
  }
  ```
  * **全平台与双界面统一**：无论用户使用传统的 Tauri 界面还是超轻量 Slint 界面，微内核均统一拉起和维护伴生服务；
  * **启动自愈 (`bootstrap_enabled_plugins`)**：若插件处于启用状态且设置 `auto_start: true`，宿主启动时自动在后台静默就绪；
  * **宿主顶栏直管**：只要声明了 `service`，宿主顶栏工具条自动生成状态指示灯（🟢在线/🟡启动中/⚪离线/🔴异常）与【启动/停止】按钮；
  * **进程防孤儿**：宿主退出或关闭 Tab 时自动级联终止 `auto_stop: true` 的服务进程树；
  * **双向沙盒消息桥**：
    * 插件请求拉起服务：`window.parent.postMessage({ action: 'START_PLUGIN_SERVICE' }, '*')`；
    * 插件请求关闭服务：`window.parent.postMessage({ action: 'STOP_PLUGIN_SERVICE' }, '*')`；
    * 监听状态广播：`window.addEventListener('message', e => { if (e.data?.type === 'OPENREVO_PLUGIN_SERVICE_STATUS') ... })`。
* **`shortcuts`（声明式全局快捷键微内核总线 / Universal Shortcut Bus）**：
  专为游戏电竞按键映射、快捷工具箱、硬件瞬切设计。由宿主 `WH_KEYBOARD_LL` 底层钩子直接接管，**零增量进程、零报毒、支持全屏独占游戏穿透并自动联动炫酷 OSD 屏幕弹窗**：
  ```json
  "shortcuts": [
    {
      "id": "turbo_fan",
      "name": "一键全速强冷",
      "combo": "Ctrl+Shift+F",
      "action": { "type": "ToggleFanBoost" },
      "osd": true
    },
    {
      "id": "power_cycle",
      "name": "循环切换性能模式",
      "combo": "Ctrl+Shift+M",
      "action": { "type": "CyclePowerMode" },
      "osd": true
    },
    {
      "id": "hz_toggle",
      "name": "刷新率瞬切 (高刷/省电)",
      "combo": "Ctrl+Alt+R",
      "action": { "type": "ToggleRefreshRate" },
      "osd": true
    }
  ]
  ```
  * **生命周期完全托管**：插件启用时宿主自动注入总线；禁用或卸载时自动注销，零心智负担。
* **`watchers` / `rules`（声明式自动化监控规则总线 / Universal Watcher & Trigger Hub）**：
  专为硬件状态联动、自动化保护、进程感知设计。**彻底根除“开隐藏桌宠偷跑轮询”的荒诞魔幻做法，零代码、0 MB 内存、0 额外视窗**：
  ```json
  "watchers": [
    {
      "id": "mode4_temp_guard",
      "name": "55度模式刷新看门狗",
      "enabled": true,
      "cooldown_ms": 15000,
      "when": {
        "type": "telemetry",
        "power_mode": 4,
        "cpu_temp_gte": 55
      },
      "actions": [
        { "action": "set_power_mode", "mode": 2 },
        { "action": "delay_ms", "duration": 500 },
        { "action": "set_power_mode", "mode": 4 },
        { "action": "raw_ec_modify_mask", "address": "0x0751", "mask": "0x10", "value": true }
      ]
    }
  ]
  ```
  * **4 种监听条件 (`type`)**：`"telemetry"` (结温/模式/转速复合阈值)、`"foreground_process"` (前台游戏进出感知)、`"power_event"` (`"ac_connected"`/`"ac_disconnected"` 充电器插拔)、`"raw_ec_diff"` (受控 EC 物理寄存器变动)。
  * **常用动作 (`action`)**：`set_power_mode`, `set_fan_boost`, `delay_ms`, `raw_ec_write`, `raw_ec_modify_mask`, `notify_osd`, `plugin_event`。

---

## 4. 全界面标准插槽矩阵 (Slot Matrix - 绝无虚构类名)

> ⚠️ **历史大坑预警**：严禁在生成的 CSS 中使用 `.overview-main-grid`、`.sensor-gauge-cluster`、`.cooling-fan-card`、`.power-mode-selector` 等旧版文档虚构的假类名！必须且只能使用以下官方落地的实体插槽：

### 4.1 主窗体 Shell 骨架插槽
* **`.openrevo-shell`**（`[data-slot="shell"]`）：主窗体总外壳容器，100vw × 100vh 满高；
* **`.shell-titlebar`**（`[data-slot="titlebar"]`）：顶栏标题栏、Logo 与窗口操作区（高度约 44px，`flex-shrink: 0`）；
* **`.shell-navbar`**（`[data-slot="navbar"]`）：Sub-Nav 横向滚动胶囊页签栏（`flex-shrink: 0`）；
* **`.shell-viewport`**（`[data-slot="viewport"]`）：核心视口容器，`flex: 1 1 0%`，纵向平滑滚动并吞满所有剩余窗口高度。

### 4.2 状态概览页插槽 (`OverviewView`)
* **`.overview-slot-grid`**（`[data-slot="overview-grid"]`）：概览页总容器；
* **`.overview-slot-telemetry`**（`[data-slot="telemetry"]`）：顶部遥测监控区插槽；
* **`.telemetry-cards-grid`**（`[data-slot="telemetry-grid"]`）：
  - 默认布局：`grid-template-columns: repeat(4, minmax(0, 1fr));`
  - 默认网格区域：`grid-template-areas: "cpu gpu fan vram";`
  - **4 联监控卡片**：
    - `.telemetry-card-cpu`（`[data-slot="cpu"]`，`grid-area: cpu`）：CPU 状态与结温卡片
    - `.telemetry-card-gpu`（`[data-slot="gpu"]`，`grid-area: gpu`）：独立显卡状态卡片
    - `.telemetry-card-fan`（`[data-slot="fan"]`，`grid-area: fan`）：散热双风扇与强冷卡片
    - `.telemetry-card-vram`（`[data-slot="vram"]`，`grid-area: vram`）：显存容量与 GDDR 负载卡片
* **`.overview-slot-power`**（`[data-slot="power"]`）：性能释放模式（办公/均衡/狂暴/自定义）卡片；
* **`.overview-slot-quickactions`**（`[data-slot="quickactions"]`）：中部高频功能微调（一键强冷/离电低刷/刷新率）；
* **`.overview-slot-switches`**（`[data-slot="switches"]`）：底部 9 联电竞级外设开关网格。

### 4.3 迷你抽屉面板插槽 (`MiniDrawer`)
* **`.mini-drawer-root`**（`[data-slot="mini-root"]`）：迷你面板外层根容器；
* **8 联语义插槽**：
  - `.mini-slot-header`：顶栏拖拽与关闭区；
  - `.mini-slot-power`：性能模式切换行；
  - `.mini-slot-gpu`：显卡直连切换行；
  - `.mini-slot-display`：屏幕刷新率切换行；
  - `.mini-slot-lighting`：键盘背光快捷行；
  - `.mini-slot-battery`：电池养护开关行；
  - `.mini-slot-switches`：快捷外设硬件开关行；
  - `.mini-slot-footer`：底栏状态行。

### 4.4 屏幕快捷弹窗插槽 (`OsdOverlay`)
宿主已实现跨窗口主题热重载，OSD 窗口会自动同步当前激活皮肤并挂载 `html[data-skin]`：
* **`.osd-root`**（`[data-slot="osd-root"]`）：最外层全屏透明容器（包含 `data-osd-pos="top-left / bottom-center"`）；
* **`.osd-container`**（`[data-slot="osd-container"]`）：弹窗卡片本体（包含 `data-osd-theme="cinema / fade / pill"`）；
* **`.osd-slot-icon`**（`[data-slot="icon"]`）：左侧快捷功能图标与胶囊底盘；
* **`.osd-slot-content`**（`[data-slot="content"]`）：右侧文字与进度槽外层容器；
* **`.osd-slot-title`**（`[data-slot="title"]`）：标题大字；
* **`.osd-slot-dot`**（`[data-slot="status-dot"]`）：状态呼吸指示点（包含 `data-state="on / off / neutral"`）；
* **`.osd-slot-meter`**（`[data-slot="meter"]`）：键盘背光等分级进度条；
* **`.osd-slot-meter-bar`**（`[data-slot="meter-bar"]`）：进度条分级单元（包含 `data-level="1~4"` 与 `data-active="true/false"`）。

---

## 5. 官方 UI Design Tokens 对照表 (严禁硬编码裸数值)

在编写插件样式时，**必须且只能引用以下真实生效的 CSS 变量**，确保自动适配深色、浅色及极简 Mono 黑白主题：

```css
/* --- 表面与背景 --- */
--surface-base: rgba(11, 14, 20, 0.94);     /* 最底层窗口底板 */
--surface-card: rgba(22, 27, 34, 0.75);     /* 悬浮功能卡片 */
--surface-card-hover: rgba(33, 38, 45, 0.85);/* 悬停卡片表面 */
--surface-overlay: rgba(15, 23, 42, 0.85);  /* 折叠子面板、弹窗表面 */
--surface-sunken: rgba(0, 0, 0, 0.35);      /* 凹陷数据底盘 */

/* --- 描边与轮廓 --- */
--border-subtle: rgba(255, 255, 255, 0.08); /* 微弱分割线 */
--border-medium: rgba(255, 255, 255, 0.15); /* 可交互控件边框 */
--border-highlight: rgba(255, 255, 255, 0.30);/* 焦点描边 */

/* --- 文字排印 --- */
--text-primary: #f8fafc;                    /* 主文字、高强调数值 */
--text-secondary: #cbd5e1;                  /* 次级描述 */
--text-muted: #94a3b8;                      /* 功能辅助说明 */
--text-dim: #64748b;                        /* 单位、未激活微字 */

/* --- 状态语义 --- */
--status-ok: #10b981;                       /* 正常运行、办公模式 (绿) */
--status-warn: #f59e0b;                     /* 告警、原厂避让 (黄) */
--status-error: #ef4444;                    /* 高危、超温、熔断 (红) */

/* --- 圆角与动效 --- */
--radius-md: 8px;                           /* 标准按钮/卡片圆角 */
--radius-lg: 12px;                          /* 大面板圆角 */
--transition-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
```

### 5.1 原生表单与暗黑视窗避坑铁律 (WebView2 Dark Mode Gotchas)
* **`<select>` 原生下拉菜单暗黑适配**：
  在 Windows WebView2 下，若页面为暗黑背景，原生 `<select>` 弹出的系统下拉菜单默认按浅色渲染，导致**白底白字无法看清**！
  **必须声明**：
  ```css
  select {
    color-scheme: dark;
    background-color: var(--surface-overlay, #0f172a);
    color: var(--text-primary, #f8fafc);
  }
  select option {
    background-color: #0b0f17;
    color: #f8fafc;
  }
  ```
* **展开排版与防溢出**：
  严禁对可展开的容器写死绝对像素高度（如 `height: 500px`），必须使用 Flex 响应式排版配合 `overflow-y: auto;` 防止展开时突出窗口边界被截断。

---

## 6. 宿主 IPC 控制能力军火库速查 (Host IPC Cheat Sheet)

在独立 Webview 或通过 Tauri API 可直接调用微内核暴露的指令：

### 6.1 硬件控制指令
* `invoke('set_fan_boost', { enable: boolean })`：一键全速强冷 (5400+ RPM)
* `invoke('set_power_mode', { mode: 1 | 2 | 3 | 4 })`：1-办公, 2-均衡, 3-狂暴, 4-自定义
* `invoke('set_battery_limit', { limit: number })`：电池养护限额 (60 ~ 100)
* `invoke('switch_refresh_rate', { hz: number })`：屏幕刷新率瞬切 (60 / 165 / 240)
* `invoke('set_gpu_mode', { mode: 'discrete' | 'hybrid' })`：显卡模式直连切换
* `invoke('apply_four_zone_colors', { zone1: [r,g,b], zone2: [r,g,b], zone3: [r,g,b], zone4: [r,g,b] })`：键盘背光
* `invoke('set_water_cooler_speed', { level: number })`：第二代外置水冷魔盒调控
* `invoke('set_device_switch', { device: 'camera'|'mic'|'touchpad'|'winlock', enabled: boolean })`：外设软开关

### 6.2 伴随提权服务管理指令 (Sidecar Supervisor)
* `invoke('start_plugin_service', { pluginId: string })`：无黑框拉起提权原生微服务并启动 TCP 端口探活
* `invoke('stop_plugin_service', { pluginId: string })`：优雅终止服务进程
* `invoke('get_plugin_service_status', { pluginId: string })`：查询 PID、端口、健康状态与运行耗时
* `invoke('restart_plugin_service', { pluginId: string })`：一键重启服务
* **沙盒 iframe postMessage 桥**：
  * 请求拉起：`window.parent.postMessage({ action: 'START_PLUGIN_SERVICE' }, '*')`
  * 请求停止：`window.parent.postMessage({ action: 'STOP_PLUGIN_SERVICE' }, '*')`
  * 状态监听：`window.addEventListener('message', e => { if (e.data?.type === 'OPENREVO_PLUGIN_SERVICE_STATUS') ... })`

### 6.3 插件市场与生命周期管理 IPC 军火库 (Market & Lifecycle)
* `invoke('get_plugin_host_capabilities') -> HostCapabilities`：运行时版本、系统架构与能力标签探测
* `invoke('install_plugin_from_url', { url: string, expectedSha256?: string }) -> InstallResult`：强制 HTTPS、防 Zip Slip 路径穿越、自动解压安装并热重载
* `invoke('uninstall_plugin', { pluginId: string }) -> boolean`：停服务、清理配置并**调用 Windows Shell API 移入回收站**
* `invoke('reload_plugins') -> CustomPlugin[]`：免重启热重载全量插件列表
* `invoke('open_external_url', { url: string }) -> boolean`：在默认浏览器打开白名单 HTTPS 外部链接（突破沙盒 iframe 限制）
* `invoke('write_clipboard', { text: string }) -> boolean`：安全写入系统剪贴板（突破沙盒 iframe 限制）
* `invoke('get_plugin_manifest', { pluginId: string }) -> any`：获取指定插件原始 manifest.json
* `invoke('set_plugin_enabled_unified', { pluginId: string, enabled: boolean }) -> boolean`：全形态插件统一启停分派

### 6.4 全局快捷键微内核总线指令 (Universal Shortcut Bus)
* `invoke('get_registered_shortcuts') -> ShortcutBinding[]`：获取当前微内核已注册生效的所有快捷键清单
* `invoke('register_custom_shortcut', { binding: ShortcutBinding }) -> boolean`：动态注册快捷键（如 `Ctrl+Alt+F`、`Win+Shift+R`），自动联动机甲 OSD 弹窗
* `invoke('unregister_custom_shortcut', { id: string }) -> boolean`：注销指定快捷键
* `invoke('trigger_shortcut_action', { action: ShortcutAction, osd?: boolean }) -> void`：手动触发动作（供界面按键模拟触发）
* **沙盒 iframe postMessage 桥**：
  * 查询快捷键：`window.parent.postMessage({ action: 'GET_SHORTCUTS' }, '*')`
  * 注册快捷键：`window.parent.postMessage({ action: 'REGISTER_SHORTCUT', payload: binding }, '*')`
  * 注销快捷键：`window.parent.postMessage({ action: 'UNREGISTER_SHORTCUT', payload: { id } }, '*')`
  * 状态监听：`window.addEventListener('message', e => { if (e.data?.type === 'OPENREVO_SHORTCUTS') ... })`

### 6.5 多模具底层驱动网关指令 (Raw Hardware Driver Gateway)
赋能高阶极客根据模具协议直接读写主板 EC 或向邮箱下发原生报文，带物理防爆盾保护：
* `invoke('get_raw_ec_capabilities') -> RawEcCapabilities`：嗅探当前机型可用的底层协议（同方 IOCTL、宝龙达 Mailbox、通用 WMI、ITE HID）
* `invoke('raw_ec_read', { request: { protocol: string, address: number } }) -> number`：读取单字节 EC 物理寄存器数值
* `invoke('raw_ec_write', { request: { protocol: string, address: number, value: number } }) -> boolean`：写入单字节 EC 物理寄存器数值（受物理防爆盾拦截保护）
* `invoke('raw_mailbox_invoke', { request: { in_bytes: number[] } }) -> number[]`：向宝龙达等邮箱下发原始多字节报文
* **沙盒 iframe postMessage 桥**：
  * 请求读：`window.parent.postMessage({ action: 'RAW_EC_READ', payload: { protocol, address } }, '*')`
  * 请求写：`window.parent.postMessage({ action: 'RAW_EC_WRITE', payload: { protocol, address, value } }, '*')`
  * 请求邮箱：`window.parent.postMessage({ action: 'RAW_MAILBOX_INVOKE', payload: { in_bytes } }, '*')`
  * 监听结果：`window.addEventListener('message', e => { if (e.data?.type === 'OPENREVO_RAW_EC_READ_RESULT') ... })`
* **Slint 原生挂件直调硬件 Callbacks (脱离 WebView2 / Tauri IPC)**：
  在 `.slint` 声明式组件中，微内核执行器（`openrevo-widget`）在动态编译实例化时自动注入以下微内核 Callbacks：
  * `callback raw_ec_read(int) -> int` / `callback raw_ec_read(string, int) -> int`
  * `callback raw_ec_write(int, int) -> bool` / `callback raw_ec_write(string, int, int) -> bool`
  * `callback set_fan_boost(bool) -> bool`
  * `callback set_power_mode(int) -> bool`
  * `callback set_battery_health(int) -> bool`
  * `callback get_raw_ec_capabilities() -> string`

### 6.6 公共监听与自动化规则总线指令 (Universal Watcher & Trigger Hub)
让插件充分复用宿主原生无锁心跳与 Win32 前台窗口探测，彻底杜绝插件自建 `setInterval`：
* `invoke('get_registered_watchers') -> WatcherInfo[]`：查询微内核当前所有活跃的监听器状态与触发历史；
* `invoke('register_custom_watcher', { watcher: WatcherDefinition }) -> boolean`：动态注册复杂条件监听器与动作流；
* `invoke('unregister_custom_watcher', { id: string }) -> boolean`：注销指定监听器；
* `invoke('trigger_watcher_manual', { id: string }) -> boolean`：手动测试触发指定监听器；
* **微插槽 / Tab 沙盒桥 (`__OPENREVO_BRIDGE__`)**：
  * `window.__OPENREVO_BRIDGE__.registerWatcher(def)`：动态挂载并与插件视窗生命周期自动绑定；
  * `window.__OPENREVO_BRIDGE__.unregisterWatcher(id)`：动态注销；
  * `window.__OPENREVO_BRIDGE__.getRegisteredWatchers()`：查询列表。

### 6.7 全域双向广播与动态遥测订阅指令 (Broadcast & Telemetry Dynamic IPC)
* `invoke('emit_plugin_broadcast', { event: string, payload: any }) -> boolean`：
  向全域总线发射事件（Tauri、Slint Native Wry 与 iframe 全量同步接收）；
* `invoke('register_telemetry_consumer', { consumerId: string, sensors: string[], desiredIntervalMs?: number }) -> boolean`：
  动态注册遥测消费者，请求特定传感器采样并指定期望心跳间隔；
* `invoke('unregister_telemetry_consumer', { consumerId: string }) -> boolean`：
  动态注销消费者。全系统活跃消费者清空时自动降频休眠。
* **双向事件消费桥（跨环境统一）**：
  * **Tauri**：`await emit('openrevo://broadcast', { event: 'name', payload: ... })` / `listen('name', cb)`
  * **Native Wry (Slint)**：`window.__NATIVE_WRY_EMIT__('name', payload)` 或 `postMessage({ action: 'EMIT', event: 'name', payload })`，接收端支持 `CustomEvent` 与 `window.addEventListener('message')`（支持直接匹配 `event.data.type === 'name'` 或 `OPENREVO_EVENT` 包装）。

---

## 7. 标准开发模版与实战范式 (Boilerplates)

### 模版 1：机甲主题皮肤插件 (`skin`)

目录结构：
```text
%APPDATA%\OpenRevo\plugins\skin-cyber-mecha\
├── manifest.json
├── theme.css
└── assets\
    └── mecha_pattern.png
```

`manifest.json`:
```json
{
  "id": "skin-cyber-mecha",
  "name": "赛博机甲暗夜皮肤",
  "version": "1.0.0",
  "author": "OpenRevo Designer",
  "description": "极客战术 HUD 布局，颠倒监控卡片顺序，自适应 1080 宽屏。",
  "plugin_type": "skin",
  "theme_css": "theme.css",
  "window": {
    "width": 1080,
    "height": 780,
    "resizable": true
  }
}
```

`theme.css`:
```css
/* 1. 覆写 Design Tokens */
html[data-skin="skin-cyber-mecha"] {
  --surface-base: rgba(9, 12, 18, 0.95);
  --border-subtle: rgba(99, 102, 241, 0.2);
  --border-medium: rgba(99, 102, 241, 0.4);
  --radius-md: 4px;
}

/* 2. 背景贴图 (直接使用相对路径，Rust 内存自动转 Data URI) */
html[data-skin="skin-cyber-mecha"] .openrevo-shell {
  background-image: 
    radial-gradient(circle at 85% 15%, rgba(99, 102, 241, 0.15) 0%, transparent 40%),
    url("assets/mecha_pattern.png") !important;
  background-size: cover !important;
}

/* 3. 颠倒监控卡片次序 (CSS Grid Areas 实战) */
html[data-skin="skin-cyber-mecha"] .telemetry-cards-grid {
  grid-template-areas: "gpu vram cpu fan" !important;
}

/* 4. 全局弹窗穿透适配 */
html[data-skin="skin-cyber-mecha"] .sys-modal-card {
  border-color: rgba(99, 102, 241, 0.5) !important;
  box-shadow: 0 0 30px rgba(99, 102, 241, 0.25) !important;
}
```

---

### 模版 2：独立背景壁纸插件 (`background`)

#### 方式 A：标准 CSS 带遮罩壁纸
`manifest.json`:
```json
{
  "id": "bg-waifu-night",
  "name": "暗夜星空同人壁纸",
  "version": "1.0.0",
  "author": "AnimeGeek",
  "plugin_type": "background",
  "background_css": "background.css"
}
```

`background.css`:
```css
/* 同时覆盖大面板和迷你抽屉，叠一层半透明黑色保证文字可读性 */
.openrevo-shell,
.mini-drawer-root {
  background-image: 
    linear-gradient(rgba(11, 14, 20, 0.55), rgba(11, 14, 20, 0.75)),
    url("assets/starry_sky.jpg") !important;
  background-size: cover !important;
  background-position: center !important;
  background-repeat: no-repeat !important;
}
```

#### 方式 B：极简零代码壁纸包 (连 CSS 都不用写！)
目录结构：
```text
%APPDATA%\OpenRevo\plugins\bg-minimal\
├── manifest.json
└── wallpaper.jpg
```

`manifest.json`:
```json
{
  "id": "bg-minimal",
  "name": "极简极客原厂壁纸",
  "version": "1.0.0",
  "author": "Geek",
  "plugin_type": "background",
  "image": "wallpaper.jpg"
}
```
* **内核自动合成**：宿主自动生成全套居中、铺满与高 DPI 适配 CSS，直接生效！

---

### 模版 3：桌面透明异形桌宠挂件 (`widget`)

`manifest.json`:
```json
{
  "id": "chibi-core",
  "name": "能量核心异形桌宠",
  "version": "1.0.0",
  "author": "OpenRevo Community",
  "plugin_type": "widget",
  "entry": "index.html",
  "window": {
    "width": 240,
    "height": 180,
    "transparent": true,
    "always_on_top": true,
    "resizable": false
  },
  "telemetry": {
    "enabled": true,
    "desiredIntervalMs": 1500,
    "sensors": ["cpu_temp", "cpu_fan_rpm", "fan_boost"]
  },
  "permissions": ["telemetry:read", "fan:control"]
}
```

`index.html`:
```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    body {
      margin: 0;
      background: transparent;
      overflow: hidden;
      font-family: sans-serif;
      user-select: none;
      -webkit-user-select: none;
    }
    .widget-card {
      width: 216px;
      height: 156px;
      margin: 12px;
      background: rgba(15, 23, 42, 0.85);
      backdrop-filter: blur(16px);
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 16px;
      box-shadow: 0 10px 30px rgba(0,0,0,0.5);
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: #fff;
    }
    .drag-handle {
      width: 100%;
      height: 24px;
      cursor: grab;
    }
    .temp-val {
      font-size: 32px;
      font-weight: 800;
      color: #818cf8;
    }
    .boost-btn {
      margin-top: 10px;
      padding: 6px 14px;
      background: #4f46e5;
      color: white;
      border: none;
      border-radius: 8px;
      cursor: pointer;
      font-weight: 600;
    }
  </style>
</head>
<body>
  <div class="widget-card">
    <div class="drag-handle" data-tauri-drag-region></div>
    <div>CPU 结温</div>
    <div id="temp" class="temp-val">-- °C</div>
    <button id="boost" class="boost-btn">一键强冷</button>
  </div>

  <script type="module">
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';

    const tempEl = document.getElementById('temp');
    const boostBtn = document.getElementById('boost');
    let isBoost = false;

    // 监听硬件遥测数据流
    await listen('hardware-state-updated', (event) => {
      const data = event.payload;
      if (data.cpu_temp) {
        tempEl.textContent = `${data.cpu_temp} °C`;
      }
      isBoost = !!data.fan_boost;
      boostBtn.textContent = isBoost ? '关闭强冷' : '一键强冷';
      boostBtn.style.background = isBoost ? '#ef4444' : '#4f46e5';
    });

    // 控制强冷
    boostBtn.addEventListener('click', async () => {
      await invoke('set_fan_boost', { enable: !isBoost });
    });
  </script>
</body>
</html>
```

#### 挂件开发进阶指南：
* **窗口原生物理拖拽**：
  * **方法 1（声明式）**：在任意希望作为拖拽把手的元素上添加属性 `data-tauri-drag-region`；
  * **方法 2（编程式）**：调用 `getCurrentWebviewWindow().startDragging()` 或 `window.OpenRevo.startDragging()`。
* **API 调用方式（双模支持）**：
  * **ES Module 模式**：通过原生 Import Map 直接 `import { invoke } from '@tauri-apps/api/core'`、`import { listen } from '@tauri-apps/api/event'` 等；
  * **全局对象模式**：直接调用 `window.OpenRevo.invoke(...)`、`window.OpenRevo.listen(...)` 或 `window.OpenRevo.close()`。
* **多文件组织与资产内嵌**：
  * 支持自由引入 `<link rel="stylesheet" href="css/style.css">` 与 `<script src="js/widget.js">`；
  * 支持相对路径图片 `<img src="assets/avatar.png">` 或音视频 `<audio src="sounds/beep.mp3">`，宿主在内存中自动将其转换为 Data URI，零 404 隐患。
* **开发者自愈防护栏**：
  * 鼠标悬停在挂件右上角时，会自动显现极小半透明控制按钮（可一键刷新代码或关闭挂件），便于开发阶段随时热重载调试。

---

### 模版 4：伴随提权服务插件 (`tab` + `service`)

`manifest.json`:
```json
{
  "id": "my-hardware-tool",
  "name": "硬件高级调控台",
  "version": "1.0.0",
  "plugin_type": "tab",
  "entry": "index.html",
  "service": {
    "executable": "bin/service.exe",
    "args": ["--port", "18945"],
    "port": 18945,
    "elevated": true,
    "auto_start": false,
    "auto_stop": true,
    "description": "本地桥接驱动代理"
  }
}
```

沙盒 iframe 界面控制代码 (`index.html`):
```html
<button onclick="window.parent.postMessage({ action: 'START_PLUGIN_SERVICE' }, '*')">启动服务</button>
<button onclick="window.parent.postMessage({ action: 'STOP_PLUGIN_SERVICE' }, '*')">停止服务</button>
<div id="status">⚪ 未连接</div>

<script>
window.addEventListener('message', (e) => {
  if (e.data?.type === 'OPENREVO_PLUGIN_SERVICE_STATUS') {
    const s = e.data.status;
    document.getElementById('status').innerText = s.running 
      ? (s.healthy ? `🟢 端口 ${s.port} 在线` : `🟡 探活中...`)
      : `⚪ 已离线`;
  }
});
window.parent.postMessage({ action: 'GET_PLUGIN_SERVICE_STATUS' }, '*');
</script>
```

---

### 模版 5：第三方插件市场对接范式 (`tab` + 市场 IPC)

`manifest.json`:
```json
{
  "id": "plugin-market",
  "name": "OpenRevo 插件市场",
  "version": "1.0.0",
  "plugin_type": "tab",
  "entry": "dist/index.html",
  "permissions": ["plugins:install", "plugins:uninstall"]
}
```

市场核心业务逻辑速查：
```javascript
import { invoke } from '@tauri-apps/api/core';

// 1. 嗅探宿主能力
const caps = await invoke('get_plugin_host_capabilities');

// 2. 一键静默安装远端插件包
const installed = await invoke('install_plugin_from_url', {
  url: 'https://cdn.openrevo.top/plugins/cyber-mecha.zip',
  expectedSha256: '...'
});

// 3. 安全卸载送入回收站
await invoke('uninstall_plugin', { pluginId: 'old-plugin' });

// 4. 统一开关分派 (自动识别 skin / background / slot / tab)
await invoke('set_plugin_enabled_unified', { pluginId: 'cyber-mecha', enabled: true });

// 5. 突破沙盒 iframe 限制打开外链与写剪贴板
await invoke('open_external_url', { url: 'https://github.com/OpenRevo' });
await invoke('write_clipboard', { text: 'https://openrevo.top' });
```

---

### 模版 6：电竞全局快捷键与按键宏映射插件 (`tab` / `slot`)

专为需要快速瞬切强冷、切换刷新率、或设置按键宏的游戏极客打造，**零增量进程、零报毒、支持全屏独占游戏穿透**：

`manifest.json`:
```json
{
  "id": "pro-gamer-keys",
  "name": "极客电竞按键工坊",
  "version": "1.0.0",
  "plugin_type": "tab",
  "entry": "dist/index.html",
  "shortcuts": [
    {
      "id": "turbo_boost",
      "name": "一键全速暴走强冷",
      "combo": "Ctrl+Shift+F",
      "action": { "type": "ToggleFanBoost" },
      "osd": true
    },
    {
      "id": "power_cycle",
      "name": "循环性能释放模式",
      "combo": "Ctrl+Shift+M",
      "action": { "type": "CyclePowerMode" },
      "osd": true
    },
    {
      "id": "hz_toggle",
      "name": "刷新率瞬切 (高刷/省电)",
      "combo": "Ctrl+Alt+R",
      "action": { "type": "ToggleRefreshRate" },
      "osd": true
    },
    {
      "id": "screen_off",
      "name": "挂机一键息屏",
      "combo": "Ctrl+Shift+L",
      "action": { "type": "MonitorOff" },
      "osd": false
    }
  ]
}
```

前端可视化动态改键速查：
```javascript
import { invoke } from '@tauri-apps/api/core';

// 1. 查询当前已生效的快捷键
const list = await invoke('get_registered_shortcuts');

// 2. 动态注册用户按键宏 (如老板键: 锁屏并拦截按键)
await invoke('register_custom_shortcut', {
  binding: {
    id: 'user_boss_key',
    name: '老板键：静音并锁屏',
    combo: 'Ctrl+Alt+B',
    action: { type: 'LockScreen' },
    osd: false,
    enabled: true,
    swallow: true
  }
});

// 3. 注销快捷键
await invoke('unregister_custom_shortcut', { id: 'user_boss_key' });
```

---

### 模版 7：零代码自动化监控规则插件 (`service` / 纯声明式 Watcher)

专为**硬件状态联动、自动化温度刷新、前台游戏自适应切模式**打造。**零 HTML、零 JS、零 Webview 视窗、0 MB 内存占用**，彻底杜绝创建隐藏桌宠偷跑轮询的恶习！

目录结构：
```text
%APPDATA%\OpenRevo\plugins\mode4-fan-unblocker\
└── manifest.json
```

`manifest.json`:
```json
{
  "id": "mode4-fan-unblocker",
  "name": "自定义模式55度解锁补丁",
  "version": "1.0.0",
  "author": "OpenRevo Community",
  "description": "监控结温并在进入自定义模式且>=55℃时自动执行安全切模与EC状态字刷新。",
  "plugin_type": "service",
  "min_host_version": "0.9.3",
  "watchers": [
    {
      "id": "unblock_751_bit4",
      "name": "55度模式刷新看门狗",
      "enabled": true,
      "cooldown_ms": 15000,
      "when": {
        "type": "telemetry",
        "power_mode": 4,
        "cpu_temp_gte": 55
      },
      "actions": [
        { "action": "set_power_mode", "mode": 2 },
        { "action": "delay_ms", "duration": 500 },
        { "action": "set_power_mode", "mode": 4 },
        { "action": "raw_ec_modify_mask", "address": "0x0751", "mask": "0x10", "value": true }
      ]
    },
    {
      "id": "game_auto_beast",
      "name": "黑神话启动自动切狂暴",
      "enabled": true,
      "cooldown_ms": 5000,
      "when": {
        "type": "foreground_process",
        "targets": ["b1.exe", "b1-win64-shipping.exe"],
        "trigger_on": "enter"
      },
      "actions": [
        { "action": "set_power_mode", "mode": 3 },
        { "action": "notify_osd", "title": "游戏感知联动", "message": "已自动切换至狂暴模式" }
      ]
    }
  ]
}
```

---

## 8. 调试、安装与排错工作流 (Troubleshooting)

1. **安装路径**：
   - 目标路径：`%APPDATA%\OpenRevo\plugins\<plugin_id>\`
2. **免重启热刷新**：
   - 打开 OpenRevo 主界面，进入【插件】页签；
   - 点击右上角【刷新】按钮，宿主调用 `invoke('get_custom_plugins')` 立即重新扫描并挂载新插件；
3. **紧急故障自愈 (Safe Mode)**：
   - 若某插件语法错误导致崩溃：打开 `%APPDATA%\OpenRevo\config.json`；
   - 找到 `custom_plugin_toggles` 节点，将该插件 ID 设为 `false`；或将 `active_skin` 设为 `null`；
   - 重新打开 OpenRevo，微内核直接跳过问题插件，安全运行！
