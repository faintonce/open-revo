# OpenRevo 开发者圣经：全场景插件与扩展开发手册 (Plugin Developer Bible)

> **版本**：`v1.0 Official Specification`  
> **面向对象**：OpenRevo 社区极客、二次元/机甲皮肤设计师、独立客户端重构者、AI Agent 开发者、硬件自动化极客  
> **设计哲学**：基于微内核与能力网关，实现“按需分配运行时（Runtime on Demand）、物理沙盒隔离、可逆副作用、零污染与即插即用”。

---

## 目录
1. [微内核架构宣言：OpenRevo 的本质是什么？](#1-微内核架构宣言openrevo-的本质是什么)
2. [7 种插件形态全景对照表 (一切皆插件生态矩阵)](#2-7-种插件形态全景对照表-一切皆插件生态矩阵)
3. [核心安全军规与沙盒边界 (Rule 4 硬件防火墙)](#3-核心安全军规与沙盒边界-rule-4-硬件防火墙)
4. [插件清单权威规范 (Plugin Manifest Reference)](#4-插件清单权威规范-plugin-manifest-reference)
5. [硬件 IPC 控制能力军火库速查 (Capabilities Cheat Sheet)](#5-硬件-ipc-控制能力军火库速查-capabilities-cheat-sheet)
6. [自适应遥测与动态心跳总线 (Telemetry Bus)](#6-自适应遥测与动态心跳总线-telemetry-bus)
7. [经典实战用例集 (从桌面桌宠到自动化看门狗)](#7-经典实战用例集-从桌面桌宠到无头脚本)
   - [用例 1: 50 行代码开发一个桌面异形机甲桌宠 (Widget)](#用例-1-50-行代码开发一个桌面异形机甲桌宠-widget)
   - [用例 2: 桌面“一键强冷”大开关挂件 (Widget)](#用例-2-桌面一键强冷大开关挂件-widget)
   - [用例 3: 纯文本脚本自定义开机仪式灯效 (Headless Service - 零 Webview)](#用例-3-纯文本脚本自定义开机仪式灯效-headless-service---零-webview)
   - [用例 4: 桌面 AI 助手 Function Calling 联动控制](#用例-4-桌面-ai-助手-function-calling-联动控制)
   - [用例 5: 完全重写前端！自建独立替代控制台 (Alternative Shell)](#用例-5-完全重写前端自建独立替代控制台-alternative-shell)
   - [用例 6: 进阶机甲桌宠工程规范 —— 边缘吸附灵动小球、动态换肤与跨插件事件总线 (Advanced Widget)](#用例-6-进阶机甲桌宠工程规范--边缘吸附灵动小球动态换肤与跨插件事件总线-advanced-widget)
   - [用例 7: 主窗口主题皮肤与插槽化布局重排 (Skin Plugin —— 一切皆插件)](#用例-7-主窗口主题皮肤与插槽化布局重排-skin-plugin--一切皆插件)
   - [用例 8: 独立背景壁纸插件与零代码壁纸包 (Background Plugin)](#用例-8-独立背景壁纸插件与零代码壁纸包-background-plugin)
   - [用例 9: 硬件级伴随提权服务开发范式 (Sidecar Service Integration)](#用例-9-硬件级伴随提权服务开发范式-sidecar-service-integration---以-ec-config-为例)
   - [用例 10: 原生体验第三方插件市场开发对接范式 (Plugin Market Integration)](#用例-10-原生体验第三方插件市场开发对接范式-plugin-market-integration---以-plugin-market-为例)
   - [用例 11: 声明式零代码全局快捷键与按键宏映射插件 (Universal Shortcut Bus)](#用例-11-声明式零代码全局快捷键与按键宏映射插件-universal-shortcut-bus)
   - [用例 12: 声明式自动化监听与规则看门狗 (Universal Watcher & Trigger Hub)](#用例-12-声明式自动化监听与规则看门狗-universal-watcher--trigger-hub)
8. [UI 设计语言与 Design Tokens 复用指南](#8-ui-设计语言与-design-tokens-复用指南)
9. [调试与安全模式排错 (Safe Mode 二分法)](#9-调试与安全模式排错-safe-mode-二分法)

---

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

## 2. 7 种插件形态全景对照表 (一切皆插件生态矩阵)

OpenRevo 坚决反对“无脑开 Webview”。我们确立了**按需分配运行时原则**与**“一切皆插件”大一统哲学**：无论桌面悬浮挂件、主控制台皮肤、功能插槽页、独立背景壁纸还是无头服务，统一作为插件收拢在 `%APPDATA%\OpenRevo\plugins\<plugin_id>\` 下管理！

| 插件形态 (`plugin_type` / `type`) | 典型应用场景 | 运行时载体 | 增量内存开销 | 视窗特性 |
| :--- | :--- | :--- | :--- | :--- |
| **`skin` (主窗口皮肤外观)** | 机甲二次元主题换肤、模块插槽网格重排、自定义窗口尺寸与全局色系 | **宿主主窗口 Webview**<br>(动态注入 `<style id="openrevo-theme-skin">` + 全局 `data-skin`) | **0 MB**<br>(纯样式与布局重排，毫秒级热装卸) | 自动平滑重置主窗口几何尺寸 (`manifest.window`)，自适应锁定或自由拉伸 |
| **`background` (独立背景壁纸)** | 窗口背景图片更换、透明度/磨砂遮罩微调、二次元同人壁纸 | **宿主主窗口 Webview**<br>(动态注入 `<style id="openrevo-custom-background">` + 全局 `data-background`) | **0 MB**<br>(与 `skin` 双通道正交并存，互不冲突) | 覆盖主窗体 (`.openrevo-shell`) 与迷你面板 (`.mini-drawer-root`)，支持相对路径与零代码放图 |
| **`slot` (微插槽组件插件)** | 小众机型硬件开关、锁定充电上限、实验功能解耦、特定模具卡片 | **宿主界面预留插槽**<br>(`<ExtensionSlot name="..." />` 安全沙盒) | **< 1 MB**<br>(极低开销微型 iframe) | 精准接管宿主预留位（如 `power.hardware_limit`），严丝合缝无缝继承 Design Tokens |
| **`tab` (主界面插槽选项卡)** | 显卡高级调试页、风扇实验室、社区扩展页面 | **宿主主窗口沙盒 iframe**<br>(安全隔离 + 跨域推流) | **< 5 MB**<br>(复用 Chromium 进程上下文) | 嵌入在官方主控制台顶栏 Sub-Nav 胶囊栏与内容视口中 |
| **`widget` (桌面悬浮挂件)** | 机甲能量核心桌宠、HUD 状态条、桌面看板娘、大开关 | **独立 Edge WebView2**<br>(物理沙盒隔离) | **15MB ~ 25MB**<br>(共享 Chromium 渲染段) | 全透明、无边框、置顶、跳过任务栏、可拖拽 |
| **`service` (无头后台服务)** | 开机欢迎灯效覆盖、自动温控曲线、按电量切模式、**纯声明式看门狗与自动化规则总线** | **Rust 内置微任务 / Watcher Hub / Rhai** | **0 ~ 100 KB**<br>(极致轻量) | **完全无界面 (Zero GUI / 0 MB)**，坚决不弹窗 |
| **`shell` (替代型客户端)** | 用户嫌官方界面不好看，自建全新极简/赛博控制台 | **独立全尺寸 Webview** | ~35MB | 拥有主控权，可彻底取代官方 `main` 界面 |
| **`driver` (模具驱动适配)** | 适配宝龙达新机型、机械革命未来新架构 | **原生 Rust 动态库 (.dll)** | 0 运行时开销 | 直接与 ACPI/WMI 驱动对接 |

> 💡 **双通道样式解耦优势**：
> `skin`（主题骨架）与 `background`（背景壁纸）拥有完全独立的样式通道与持久化配置键（`active_skin` 与 `active_background`）。用户可以选一套沉稳的极简工业风主题，同时配上一张自己心仪的动漫背景，两类插件可以同时启用、自由搭配！

---

## 3. 核心安全军规与沙盒边界 (Rule 4 硬件防火墙)

笔记本硬件涉及高压供电、功耗墙释放与风扇转速。任何代码错误或越权写入都可能导致主板断电甚至硬件损坏。所有插件开发必须遵守以下铁律：

1. **绝对禁止裸物理寄存器交互（Rule 4 硬件资产钥匙库守则）**：
   * 严禁在插件中请求类似 `ec_write(0x1844, 0x01)` 或 dump 整块 EC RAM。
   * OpenRevo 仅对外暴露**“语义化硬件控制指令”**（如 `set_fan_boost(true)`）。
2. **硬件物理边界防呆（Guardrails）**：
   * 功耗墙调节范围由机型固件严格钳位（例如 PL4 必须 `<= 255W`，防 0xFF 翻转单字节溢出为 45W 慢充）。
   * 电池养护限额严格限制在 `60% ~ 100%` 之间。
3. **非侵入式执行（Non-blocking）**：
   * 严禁在主线程执行耗时同步阻塞；网络请求与大模型调用必须采用异步流式处理。

---

## 4. 插件清单权威规范 (Plugin Manifest Reference)

每个插件为一个独立文件夹，存放于 `%APPDATA%\OpenRevo\plugins\<plugin_id>\`。根目录下必须包含 `manifest.json`：

```json
{
  "id": "eva01-core",
  "name": "EVA-01 初号机 S2 能量核心",
  "version": "1.0.0",
  "author": "OpenRevo Community",
  "description": "异形拟态机甲桌宠挂件，支持结温脉冲呼吸、风扇转速联动与一键强冷暴走切换。",
  "plugin_type": "widget",
  "entry": "index.html",
  "window": {
    "width": 260,
    "height": 180,
    "transparent": true,
    "always_on_top": true,
    "resizable": false
  },
  "telemetry": {
    "enabled": true,
    "desiredIntervalMs": 1500,
    "sensors": [
      "cpu_temp",
      "cpu_fan_rpm",
      "cpu_power_w",
      "power_mode",
      "fan_boost"
    ]
  },
  "permissions": [
    "telemetry:read",
    "power:control",
    "fan:control"
  ],
  "min_host_version": "0.9.3"
}
```

### 清单字段详细说明

| 字段名 | 类型 | 是否必填 | 适用类型 | 描述 |
| :--- | :--- | :--- | :--- | :--- |
| `id` | `string` | **必填** | 全类型 | 插件全局唯一标识符（纯英文、数字、下划线、中划线） |
| `name` | `string` | **必填** | 全类型 | 人类可读的插件名称，展示于插件管理器 |
| `version` | `string` | **必填** | 全类型 | 语义化版本号（如 `1.0.0`） |
| `min_host_version` / `min_version` / `openrevo_version` | `string` | 可选 | 全类型 | **【宿主最低兼容版本】**采用标准 SemVer（如 `"0.9.3"`）。若宿主版本低于该值，插件管理器自动显示警告徽章并安全阻止运行，杜绝新特性在旧二进制中静默失效 |
| `plugin_type` / `type` | `string` | **必填** | 全类型 | 插件形态：`"skin"` \| `"background"` \| `"slot"` \| `"tab"` \| `"widget"` \| `"service"` \| `"shell"`（双向别名兼容） |
| `target_slot` / `slot` | `string` | 条件必填 | `slot` | 挂载的目标宿主插槽代号（如 `"power.hardware_limit"`） |
| `slot_height` / `slotHeight` | `string` | 可选 | `slot` | 微插槽高度约束（如 `"36px"` 或 `"54px"`，默认 `"54px"`） |
| `theme_css` | `string` | `skin`可选 | `skin` | 皮肤样式表入口相对路径（默认 `"theme.css"`） |
| `background_css` | `string` | `background`可选 | `background` | 背景样式表入口相对路径（默认 `"background.css"`，回退 `"theme.css"`） |
| `image` / `wallpaper` | `string` | 可选 | `background` | **【零代码壁纸】**直接指定图片路径（如 `"bg.png"`），无需编写一行 CSS，宿主自动合成全套适配样式 |
| `entry` | `string` | 可选 | `widget`/`tab`/`service` | 入口相对路径，如 `index.html` 或 `welcome.rhai` |
| `window` | `object` | 可选 | `widget`/`tab`/`skin`/`shell` | 视窗高级几何与外廓属性（详见下方 `window` 子对象详解） |
| `telemetry` | `object` | 可选 | 全类型 | 声明硬件遥测需求（见下文防独显唤醒机制） |
| `permissions` | `string[]`| 可选 | 全类型 | 声明所需权限（未声明将被安全网关拦截） |
| `watchers` / `rules` | `object[]` | 可选 | 全类型 | **【声明式自动化规则总线】**无需 Webview 视窗，宿主内核统一调度温度联动、游戏切模与硬件保护规则 |
| `shortcuts` | `object[]` | 可选 | 全类型 | **【声明式全局快捷键总线】**Ring3 穿透级热键绑定，自动联动机甲 OSD 弹窗与动作流 |
| `service` | `object` | 可选 | `tab`/`widget`/`slot` | **【声明式伴随提权服务 / Sidecar】**声明插件依赖的后端本地原生进程服务（见下方 `service` 子对象详解） |

#### `window` 视窗子对象详细配置表 (全场景悬浮与专业大窗口)
| 子属性 | 类型 | 默认值 | 适用场景与说明 |
| :--- | :--- | :--- | :--- |
| `width` | `number` | `240` | 初始逻辑宽度（如 XTU/SMU 建议设为 `920`） |
| `height` | `number` | `320` | 初始逻辑高度（如 XTU/SMU 建议设为 `650`） |
| `resizable` | `boolean` | `false` | 是否允许鼠标拖拽窗口边缘缩放 |
| `decorations` | `boolean` | `false` | **【原生标题栏】**：设为 `true` 即可显示标准 Windows 标题栏（包含最小化、最大化、关闭按钮）。大型复杂工具页（XTU/SMU/风扇曲线）强烈建议开启！ |
| `transparent` | `boolean` | `true` | 是否启用全透明 Alpha 通道（异形桌宠设为 `true`，复杂实色工具面板设为 `false`） |
| `always_on_top` | `boolean` | `true` | 是否保持在其他软件窗口上方置顶（大型复杂工具通常设为 `false`） |
| `skip_taskbar` | `boolean` | `true` | 是否在 Windows 系统任务栏跳过显示（设为 `false` 可在任务栏显示独立图标，支持 `Alt+Tab` 秒切） |

#### `service` 伴随提权服务子对象详细配置表 (Sidecar / Companion Service)

针对需要借助原生可执行程序（如 Go、Rust、Python、Node.js 编译的 `.exe` 或 EC/驱动桥接代理）的硬件级扩展插件（如 `ec-config`、`sdt-pbo`），OpenRevo 提供了内置的**伴随服务监督器（Companion Service Supervisor）**。

开发者**严禁让用户手动双击 `.bat` 启动黑框脚本**，只需在 `manifest.json` 中声明 `service`，宿主即会自动接管提权静默拉起、端口探活、顶栏胶囊状态展示与退出清理：

```json
"service": {
  "executable": "agent/ec_agent.exe",
  "args": ["--port", "18945", "--mode", "daemon"],
  "port": 18945,
  "health_url": "http://127.0.0.1:18945/api/ping",
  "elevated": true,
  "auto_start": false,
  "auto_stop": true,
  "description": "EC 物理读写调试微服务"
}
```

| 子属性 | 类型 | 默认值 | 说明与安全约束 |
| :--- | :--- | :--- | :--- |
| `executable` | `string` | **必填** | 可执行文件相对路径（如 `"server.exe"` 或 `"bin/bridge.exe"`）。**宿主内核强制执行沙盒防穿越断言**，严禁使用 `../` 跳出插件目录！ |
| `args` | `string[]` | `[]` | 传递给可执行文件的命令行启动参数数组 |
| `port` | `number` | 可选 | 服务监听的本机回环 TCP 端口（如 `18945`），宿主后台自动发起 TCP 连接探活 |
| `health_url` | `string` | 可选 | 专用 HTTP 探活 URL（如 `"http://127.0.0.1:18945/health"`），支持自定义就绪检测 |
| `elevated` | `boolean` | `true` | 是否以管理员提权模式运行。由于 OpenRevo 本身具备管理员权限，拉起子进程自动继承最高权限且默认采用 `CREATE_NO_WINDOW (0x08000000)` 静默无黑框运行 |
| `auto_start` | `boolean` | `false` | 打开插件界面时是否自动静默拉起伴随服务（建议设为 `false`，由用户点击或前端按需拉起） |
| `auto_stop` | `boolean` | `true` | 宿主退出或关闭插件选项卡时，是否自动级联终止该服务进程，彻底防止残留孤儿进程 |
| `description` | `string` | 可选 | 服务的简短中文/英文描述，在顶栏服务状态面板悬停时展示 |

> 💡 **Tab 插件一键弹出独立视窗 (Detachable Pop-out Tab)**：
> 无论插件被定义为 `widget` 还是 `tab`，宿主均在主界面选项卡右上角原生提供**【独立视窗运行】**按钮。用户既能在控制中心中内嵌使用，也能随时一键弹出为独立多屏调优窗口，零代码改动即可双模共存！

---

## 5. 硬件 IPC 控制能力军火库速查 (Capabilities Cheat Sheet)

在独立 Webview 或插件前端中，只要引入 `@tauri-apps/api/core` 的 `invoke`，即可直接调用微内核暴露的 8 大硬件控制指令：

```javascript
import { invoke } from '@tauri-apps/api/core';
```

### 1. 一键全速强冷 (Fan Boost)
* **指令**：`invoke('set_fan_boost', { enable: boolean })`
* **硬件动作**：向 EC 写入强冷标志位，风扇瞬间以 5400+ RPM 暴走排气；托盘图标加风扇角标，广播 `fan-boost-changed`。

### 2. 性能模式切换 (Power Mode)
* **指令**：`invoke('set_power_mode', { mode: 1 | 2 | 3 | 4 })`
* **模式映射**：`1-办公(Office)`, `2-均衡(Balance)`, `3-狂暴(Turbo/Beast)`, `4-自定义(Custom)`。
* **硬件动作**：刷新 CPU PL1/PL2/PL4 功耗偏置，调整风扇曲线与动态背光，广播 `physical-mode-switched`。

### 3. 电池健康养护限额 (Battery Protect)
* **指令**：`invoke('set_battery_limit', { limit: number })`
* **参数范围**：`60 ~ 100`（通常设定为 60%、80% 或 100%）。
* **硬件动作**：写入 EC 电池控制寄存器。达到指定电量后硬件直接断充，避免电芯持续高压鼓包。

### 4. 屏幕刷新率瞬切 (Display Refresh Rate)
* **指令**：`invoke('switch_refresh_rate', { hz: number })`
* **常用参数**：`60`（离电省电）或 `165` / `240`（游戏电竞）。
* **硬件动作**：调用 Win32 底层显示配置无黑屏快速切换。

### 5. 显卡模式直连切换 (GPU MUX)
* **指令**：`invoke('set_gpu_mode', { mode: string })`
* **模式选项**：`"discrete"` (独显直连性能最大化), `"hybrid"` (混合输出兼顾续航)。

### 6. 四分区全彩键盘背光 (RGB Lighting)
* **指令**：`invoke('apply_four_zone_colors', { zone1: [r,g,b], zone2: [r,g,b], zone3: [r,g,b], zone4: [r,g,b] })`
* **硬件动作**：调用 ITE 键盘灯控接口下发色彩。

### 7. 第二代外置水冷魔盒调控 (BLE Cooler)
* **指令**：`invoke('set_water_cooler_speed', { level: number })`
* **硬件动作**：WinRT 蓝牙低功耗向水冷盒下发泵速与转速挡位。

### 8. 硬件外设软开关 (Device Switches)
* **指令**：`invoke('set_device_switch', { device: string, enabled: boolean })`
* **设备标识**：`"camera"` (摄像头), `"mic"` (麦克风), `"touchpad"` (触控板), `"winlock"` (Win键锁定)。

### 9. 皮肤与背景样式管理 IPC 军火库 (Plugin Styles & Assets)
OpenRevo 现已全面提供原生双通道样式加载与持久化命令，插件或外部脚本可直接调用：
* **`invoke('load_plugin_theme_css', { pluginId: string })`**：读取指定皮肤插件的 CSS，自动将 CSS 中的相对路径图片转为内存 Data URI 并返回，供注入 `<style id="openrevo-theme-skin">`。
* **`invoke('load_plugin_background_css', { pluginId: string })`**：读取指定背景插件的 CSS（或直接读取图片资产自动合成全套背景规则），供注入 `<style id="openrevo-custom-background">`。
* **`invoke('get_active_skin')` / `invoke('set_active_skin', { skinId: string | null })`**：获取或设置当前全局激活的主题皮肤 ID。
* **`invoke('get_active_background')` / `invoke('set_active_background', { backgroundId: string | null })`**：获取或设置当前全局激活的背景壁纸 ID。
* **`invoke('get_custom_plugins')`**：重新枚举 `%APPDATA%\OpenRevo\plugins` 目录，即时返回所有插件元数据（支持前端热刷新，无需重启宿主）。
* **`invoke('open_plugins_dir')`**：唤起系统文件管理器直接定位并打开插件根目录。

### 10. 伴随提权服务生命周期控制 (Companion Service Supervisor)
OpenRevo 为伴随原生后台微服务提供了完整的主机控制指令（同时原生支持在选项卡顶栏通过胶囊按钮直接操作）：
* **`invoke('start_plugin_service', { pluginId: string }) -> PluginServiceStatus`**：拉起指定插件声明的原生可执行文件（继承管理员提权无黑框运行，自动启动 TCP 探活轮询）。
* **`invoke('stop_plugin_service', { pluginId: string }) -> PluginServiceStatus`**：优雅终止指定插件的伴随服务进程。
* **`invoke('get_plugin_service_status', { pluginId: string }) -> PluginServiceStatus`**：实时查询指定插件的服务运行状态、PID、回环端口与端口连通性。
* **`invoke('restart_plugin_service', { pluginId: string }) -> PluginServiceStatus`**：一键重启服务。

`PluginServiceStatus` 强类型状态对象定义：
```typescript
interface PluginServiceStatus {
  running: boolean;        // 进程是否处于活动状态
  pid: number | null;      // Windows 系统进程 ID
  port: number | null;     // 声明的本地回环 TCP 端口
  healthy: boolean;        // TCP 端口是否已成功连通探活
  last_error: string | null; // 最近一次启动或运行异常报错
  uptime_secs: number;     // 持续运行秒数
}
```

#### 沙盒 iframe 双向事件桥 (PostMessage Bridge)
对于运行在沙盒 `iframe` 中的插件前端（如 `ec-config`），无需直接调用 Tauri 原生模块，直接通过跨文档通信与宿主对话：
```javascript
// 1. 向宿主发起控制指令
window.parent.postMessage({ action: 'START_PLUGIN_SERVICE' }, '*');
window.parent.postMessage({ action: 'STOP_PLUGIN_SERVICE' }, '*');
window.parent.postMessage({ action: 'RESTART_PLUGIN_SERVICE' }, '*');
window.parent.postMessage({ action: 'GET_PLUGIN_SERVICE_STATUS' }, '*');

// 2. 监听宿主下发的状态变更广播
window.addEventListener('message', (event) => {
  if (event.data?.type === 'OPENREVO_PLUGIN_SERVICE_STATUS') {
    const { status, pluginId } = event.data;
    console.log(`[服务状态] 进程运行: ${status.running}, 端口健康: ${status.healthy}`);
  }
});
```

### 11. 插件市场与生命周期管理 IPC 军火库 (Plugin Market & Lifecycle API)
为支持官方及第三方社区开发者构建流畅的原生“插件市场”（如 `plugin-market` Tab 插件），OpenRevo 宿主提供了完整的生命周期管理 IPC，告别让用户手动下载解压或手动改配置的原始体验：

* **`invoke('get_plugin_host_capabilities') -> HostCapabilities`**：
  获取宿主版本号、操作系统、芯片架构与支持的能力特性标签列表，供市场做运行时兼容性检查。
  ```typescript
  interface HostCapabilities {
    version: string;
    tauri_version: string;
    os: string;
    arch: string;
    capabilities: string[]; // ["service_supervisor", "market_installer", "dual_channel_themes", ...]
  }
  ```
* **`invoke('install_plugin_from_url', { url: string, expectedSha256?: string }) -> InstallResult`**：
  **【一键安全拉取并静默安装】**：
  * 强制限制 `https://` 协议，杜绝中间人劫持；
  * 可选校验 `expectedSha256` 完整性指纹；
  * 自动解压至临时目录并严格断言路径边界（防 Zip Slip 穿越）；
  * 解析验证 `manifest.json` 合法性（限制 ID 为纯字母数字下划线中划线，长度 `<= 64`）；
  * 安全原子化移动至 `%APPDATA%\OpenRevo\plugins\<plugin_id>\`；
  * 自动触发内部重载并广播 `plugins-changed`。
* **`invoke('uninstall_plugin', { pluginId: string }) -> boolean`**：
  **【安全无痕卸载】**：
  * 若该插件正运行伴随服务，先自动停机；
  * 自动在 `config.json` 中将其从 `custom_plugin_toggles`、`active_skin`、`active_background` 中清理剔除；
  * **严格遵守安全守则，调用 Windows Shell API 将插件目录移入系统回收站**，物理保留可恢复性，绝不盲目物理抹除；
  * 触发重载并广播 `plugins-changed`。
* **`invoke('reload_plugins') -> CustomPlugin[]`**：
  免重启强制刷新并重新扫描插件目录，返回最新插件列表。
* **`invoke('open_external_url', { url: string }) -> boolean`**：
  在系统默认浏览器中打开白名单 URL（限制 `https://` / `http://`），突破沙盒 iframe 无法 `window.open` 的限制，杜绝高危协议提权。
* **`invoke('write_clipboard', { text: string }) -> boolean`**：
  突破沙盒 iframe 无法直接调用 `navigator.clipboard` 的限制，由宿主代为安全写入系统剪贴板。
* **`invoke('get_plugin_manifest', { pluginId: string }) -> any`**：
  精确读取指定插件的原始 `manifest.json` 清单对象。
* **`invoke('set_plugin_enabled_unified', { pluginId: string, enabled: boolean }) -> boolean`**：
  **【全类型插件统一开关总线】**：
  无需在市场中分别判断皮肤、壁纸和功能插件，传入 `pluginId` 与目标布尔值，宿主自动路由分派（`skin` 自动生效/取消激活皮肤，`background` 自动生效/取消激活壁纸，其它插件自动切换开关并持久化）。

### 12. 全局快捷键微内核总线 IPC 军火库 (Universal Shortcut Bus API)
OpenRevo 内核现已将 Windows 底层键盘钩子（`WH_KEYBOARD_LL`）抽象为原生的全局快捷键分发总线，插件开发者**完全不需要写 C/C++ 或拉起外部 exe 守护进程**：

* **`invoke('get_registered_shortcuts') -> ShortcutBinding[]`**：获取当前微内核已注册生效的所有快捷键清单。
* **`invoke('register_custom_shortcut', { binding: ShortcutBinding }) -> boolean`**：动态注册自定义按键组合（支持 `Ctrl+Alt+F`、`Win+Shift+R`、`F10` 等），支持自动联动 OSD 浮层。
* **`invoke('unregister_custom_shortcut', { id: string }) -> boolean`**：注销指定快捷键。
* **`invoke('trigger_shortcut_action', { action: ShortcutAction, osd?: boolean }) -> void`**：手动执行指定动作（可用于界面按钮快捷模拟）。

#### `ShortcutBinding` 契约定义：
```typescript
interface ShortcutBinding {
  id: string;              // 唯一标识 (如 "my_fan_boost")
  name: string;            // 人类可读名称
  combo: string;           // 组合键字符串 (如 "Ctrl+Shift+F", "Win+Alt+R")
  action: ShortcutAction;  // 绑定的动作类型与载荷
  osd?: boolean;           // 触发时是否自动唤起机甲 OSD 屏幕浮层 (默认 true)
  enabled?: boolean;       // 是否启用 (默认 true)
  swallow?: boolean;       // 是否拦截吃掉按键不传给前台程序 (默认 false)
  plugin_id?: string;      // 所属插件 ID
}

type ShortcutAction =
  | { type: "ToggleFanBoost" }
  | { type: "CyclePowerMode" }
  | { type: "SetPowerMode", payload: number }
  | { type: "ToggleRefreshRate" }
  | { type: "ToggleTouchpad" }
  | { type: "ToggleWinLock" }
  | { type: "ToggleFnLock" }
  | { type: "CycleKeyboardBrightness" }
  | { type: "TriggerOsdPreview" }
  | { type: "VolumeMute" }
  | { type: "VolumeUp" }
  | { type: "VolumeDown" }
  | { type: "MonitorOff" }
  | { type: "LockScreen" }
  | { type: "LaunchApp", payload: { path: string, args?: string[] } }
  | { type: "PluginEvent", payload: { event_name: string, payload: any } };
```

### 13. 多模具底层驱动网关与物理防爆盾 (Raw Hardware Driver Gateway)
为彻底赋能社区高阶极客，OpenRevo 现已将内核支持的 **4 种主板模具底层通信通道** 抽象为开放的底层驱动网关。插件开发者无需再被卡在 Ring0 驱动通信、管理员提权或 WMI 复杂封包上，可根据模具类型直接调用微内核执行底层 EC/主板交互：

#### 4 大支持的底层模具协议 (`RawEcProtocol`)：
1. **`TongfangDirectEc`**：同方模具原生 `ACPIDriver` IOCTL 通道，支持任意有效物理地址的单字节高速读写；
2. **`BldWmiMailbox`**：宝龙达/蛟龙模具 WMI 邮箱协议，支持向邮箱通道下发变长多字节报文并读取响应；
3. **`GenericAcpiWmi`**：通用标准 ACPI WMI MULONG 传输通道；
4. **`IteLightingBus`**：ITE 独立按键流光与背光芯片硬件通道。

#### 核心 IPC 军火库速查：
* **`invoke('get_raw_ec_capabilities') -> RawEcCapabilities`**：嗅探当前物理机器已就绪的模具通信协议列表（如判断本机是同方 IOCTL 还是宝龙达 Mailbox）。
* **`invoke('raw_ec_read', { request: { protocol: string, address: number } }) -> number`**：从指定模具协议的 EC RAM 物理地址读取 1 字节数值（返回 `0x00 ~ 0xFF`）。
* **`invoke('raw_ec_write', { request: { protocol: string, address: number, value: number } }) -> boolean`**：向指定模具协议的 EC RAM 物理地址写入 1 字节数值。受物理防爆盾严格保护。
* **`invoke('raw_mailbox_invoke', { request: { in_bytes: number[] } }) -> number[]`**：向宝龙达等邮箱协议模具直接下发原始字节流并返回响应报文。

#### 硬件物理防爆盾（Hardware Blast Shield）与安全规范：
1. **地址边界限制**：物理地址必须严格位于 `0x0000 ~ 0x0FFF` 之间，超出即拒绝执行；
2. **禁区写入熔断**：内核底层硬编码拦截致命保留引脚（如 `0x0000` 复位极值触发），杜绝主板永久变砖；
3. **权限清单隔离**：插件必须在 `manifest.json` 中显式声明 `"permissions": ["hardware:raw_ec_read", "hardware:raw_ec_write"]`；
4. **市场审核认证机制**：
   - **官方认证徽章（Verified by OpenRevo）**：代码经过审计，小白用户可放心启用；
   - **社区未认证高危插件（Community Experimental）**：带有高危警告角标，首次启用时强制弹出红色确认对话框，用户自担硬件风险。
### 14. 公共监听与自动化规则总线 (Universal Watcher & Trigger Hub)
为彻底解决社区开发者**“为了跑几行后台监控逻辑，被迫开一个隐藏透明挂件偷跑轮询”**的痛点与能耗弊端，OpenRevo 现已内置原生的**微内核公共监听与反应中心（Watcher & Trigger Hub）**。

插件开发者无需自建任何 `setInterval`、无需创建任何隐藏 Webview，只需向宿主声明或注册监听条件，宿主便会在已有的极低开销硬件遥测与系统事件循环中统一评估并在命中时毫秒级自动触发动作或推流！

#### 1. 痛点根除与运行机制：
* **零额外内存与视窗（Zero GUI / 0 MB）**：彻底告别“隐形 Webview2 挂件”，纯规则插件无需编写任何 HTML/JS 界面；
* **防后台休眠与精准触发**：由 Rust 内核的原生无锁心跳统一调度，彻底免疫 Windows/Chromium 对后台隐藏窗口的计时器降频节流；
* **防独显唤醒守则延续**：若注册的规则仅关注 CPU 或电源，NVIDIA 独立显卡严格保持 D3Cold 零功耗深睡；
* **物理防爆盾时序保护**：规则中涉及的所有 EC 寄存器写入，严格受到 Hardware Blast Shield 的地址边界校验与互斥锁保护。

#### 2. 清单声明式规则示例 (`manifest.json`)
开发者可在插件的 `manifest.json` 中直接通过 `watchers`（或 `rules`）声明自动化规则，宿主加载插件时自动生效，禁用插件时自动析构：

```json
{
  "id": "mode4-fan-unblocker",
  "name": "自定义模式55度解锁补丁",
  "version": "1.0.0",
  "plugin_type": "service",
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
    }
  ]
}
```

#### 3. 支持的 4 大监听条件类型 (`WatcherCondition`)：
1. **`telemetry` (硬件遥测阈值与状态)**：
   * `power_mode`: 指定特定性能模式（如 `4`）；
   * `power_mode_in`: 指定模式数组（如 `[2, 4]`）；
   * `cpu_temp_gt` / `cpu_temp_gte` / `cpu_temp_lt`: CPU 结温比较阈值；
   * `gpu_temp_gt` / `gpu_temp_gte`: GPU 温度比较阈值；
   * `battery_level_lt`: 电池电量低于指定百分比；
   * `is_charging`: 是否处于充电态；
   * `fan_boost`: 强冷是否开启。
2. **`foreground_process` (前台进程与游戏追踪)**：
   * `targets`: 关注的目标进程名列表（如 `["b1.exe", "cyberpunk2077.exe"]`）；
   * `trigger_on`: 触发时机：`"enter"`（切换到前台时）、`"exit"`（切出前台时）或 `"both"`。
3. **`power_event` (电源与系统边沿事件)**：
   * `event`: `"ac_connected"`（插入充电器）、`"ac_disconnected"`（拔出充电器/走电池）、`"screen_off"`、`"screen_on"`。
4. **`raw_ec_diff` (受控 EC 寄存器差量监控)**：
   * `address`: 目标物理地址（支持十进制整数或十六进制字符串如 `"0x0740"`）；
   * `mask`: 可选位掩码，仅关注特定位段变化；
   * 仅在数值发生实际跳变时触发。

#### 4. 支持的执行动作类型 (`WatcherAction`)：
* **`set_power_mode`**：`{ "action": "set_power_mode", "mode": 1 | 2 | 3 | 4 }`
* **`set_fan_boost`**：`{ "action": "set_fan_boost", "enable": boolean }`
* **`delay_ms`**：`{ "action": "delay_ms", "duration": number }`（毫秒延时）
* **`raw_ec_write`**：`{ "action": "raw_ec_write", "address": "0x...", "value": number }`
* **`raw_ec_modify_mask`**：`{ "action": "raw_ec_modify_mask", "address": "0x...", "mask": "0x...", "value": boolean }`
* **`notify_osd`**：`{ "action": "notify_osd", "title": string, "message": string }`
* **`plugin_event`**：`{ "action": "plugin_event", "event_name": string, "payload": any }`

#### 5. 动态 IPC 军火库速查与沙盒事件桥：
* **`invoke('get_registered_watchers') -> WatcherInfo[]`**：查询当前所有活跃监听器的状态、所属插件、冷却倒计时与触发历史；
* **`invoke('register_custom_watcher', { watcher: WatcherDefinition }) -> boolean`**：动态注册监听器；
* **`invoke('unregister_custom_watcher', { id: string }) -> boolean`**：动态注销指定监听器；
* **`invoke('trigger_watcher_manual', { id: string }) -> boolean`**：手动模拟触发；
* **沙盒桥调用**：在微插槽或 Tab 中直接通过 `window.__OPENREVO_BRIDGE__.registerWatcher(...)` 注册，视窗卸载时自动级联清理。

---


## 6. 自适应遥测与动态心跳总线 (Telemetry Bus)

### 1. 核心铁律：防独显唤醒 (Zero dGPU Wake-lock)
双显卡笔记本在轻度办公时，NVIDIA 独立显卡处于 **D3Cold 极深零功耗休眠 (0W)**。
* **严禁无脑索取 GPU 字段**：若挂件仅需要 CPU 结温和风扇转速，`manifest.json` 中**坚决不要声明 `gpu_*` 字段**！
* **内核保障**：若所有运行中的插件均未订阅 GPU，微内核**绝不调用 NVAPI**，让独显继续保持零功耗停机。

### 2. 纯陪伴小人 / 语音助手的“零功耗深睡模式”
纯陪伴小人或 AI 语音助手不需要任何传感器，只需在 `manifest.json` 中声明：
```json
"telemetry": {
  "enabled": false
}
```
内核看到此标记，即使挂着 5 个小人，**硬件遥测线程依然安详深睡，硬件零负担，CPU 维持极深 C-State**。

### 3. 前端消费数据总线流
在任何 Webview 挂件中，监听全局公频广播：
```javascript
import { listen } from '@tauri-apps/api/event';

// 监听每 1.5s 广播的硬件状态流
const unlisten = await listen('hardware-state-updated', (event) => {
  const data = event.payload; // HardwareState 强类型对象
  console.log(`CPU温度: ${data.cpu_temp}℃, 风扇转速: ${data.cpu_fan_rpm}RPM`);
});
```

---

## 7. 经典实战用例集 (从桌面桌宠到无头脚本)

### 用例 1: 50 行代码开发一个桌面异形机甲桌宠 (Widget)

新建文件夹 `%APPDATA%\OpenRevo\plugins\eva-pet\`，放置 `manifest.json` 与 `index.html`：

**`manifest.json`**：
```json
{
  "id": "eva-pet",
  "name": "EVA 机甲桌宠",
  "version": "1.0.0",
  "type": "widget",
  "window": { "width": 200, "height": 200, "transparent": true, "alwaysOnTop": true }
}
```

**`index.html`**：
```html
<!DOCTYPE html>
<html>
<head>
  <style>
    body { margin: 0; background: transparent; overflow: hidden; user-select: none; }
    #core {
      width: 180px; height: 180px; border-radius: 50%;
      background: rgba(15, 23, 42, 0.85); backdrop-filter: blur(16px);
      border: 2px solid var(--glow, #38bdf8);
      box-shadow: 0 0 24px var(--glow, #38bdf8);
      display: flex; flex-direction: column; align-items: center; justify-content: center;
      color: #fff; font-family: monospace; cursor: grab;
      transition: all 0.3s ease;
    }
  </style>
</head>
<body>
  <div id="core">
    <span style="font-size: 11px; opacity: 0.6;">S² 能量核心</span>
    <span id="temp" style="font-size: 38px; font-weight: 900;">--°</span>
    <span id="fan" style="font-size: 10px; opacity: 0.7;">-- RPM</span>
  </div>

  <script type="module">
    import { listen } from '@tauri-apps/api/event';
    import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

    // 拖拽窗口支持（推荐使用 pointerdown 单一入口，严禁与 data-tauri-drag-region 混用导致底层冲突）
    document.getElementById('core').addEventListener('pointerdown', (e) => {
      if (e.button === 0) getCurrentWebviewWindow().startDragging();
    });

    // 消费硬件遥测流
    listen('hardware-state-updated', (e) => {
      const data = e.payload;
      const temp = data.cpu_temp || 45;
      document.getElementById('temp').innerText = `${temp}°`;
      document.getElementById('fan').innerText = `${data.cpu_fan_rpm || 0} RPM`;

      // 温度驱动核心变色：>80℃ 猩红, >68℃ 琥珀橙, 平常天蓝
      const glow = temp > 80 ? '#ef4444' : (temp > 68 ? '#f59e0b' : '#38bdf8');
      document.getElementById('core').style.setProperty('--glow', glow);
    });
  </script>
</body>
</html>
```

---

### 用例 2: 桌面“一键强冷”大开关挂件 (Widget)

用户嫌每次打开控制中心麻烦，只想在桌面上摆个硕大的实体质感按钮，按一下强冷，再按一下恢复：

```html
<button id="turbo-btn" style="width: 140px; height: 50px; background: #ef4444; color: #fff; border-radius: 12px; font-weight: bold; cursor: pointer;">
  🔥 切换全速强冷
</button>

<script type="module">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let isBoosting = false;
  const btn = document.getElementById('turbo-btn');

  // 点击立刻下发硬件强冷
  btn.addEventListener('click', async () => {
    isBoosting = !isBoosting;
    await invoke('set_fan_boost', { enable: isBoosting });
  });

  // 同步物理按键或托盘触发的状态改变
  listen('fan-boost-changed', (e) => {
    isBoosting = e.payload;
    btn.innerText = isBoosting ? '❄️ 强冷运行中' : '🔥 开启全速强冷';
    btn.style.background = isBoosting ? '#06b6d4' : '#ef4444';
  });
</script>
```

---

### 用例 3: 纯文本脚本自定义开机仪式灯效 (Headless Service - 零 Webview)

用户想自定义电脑开机时键盘的流光欢迎仪式，**完全不需要任何 Webview 窗口，仅占用 <100KB 内存**：

**`manifest.json`**：
```json
{
  "id": "cyber-welcome",
  "name": "赛博开机灯效仪式",
  "version": "1.0.0",
  "type": "service",
  "target": "lighting.welcome",
  "entry": "welcome.rhai"
}
```

**`welcome.rhai`**（嵌入式脚本，在 Rust 内部极速执行）：
```rust
// 当系统检测到用户开机登录时自动触发
fn on_startup_ceremony() {
    // 阶段 1: 冰蓝呼吸唤醒
    set_keyboard_rgb(0, 255, 255);
    sleep(400);
    // 阶段 2: 霓虹粉紫能量注入
    set_keyboard_rgb(255, 0, 128);
    sleep(400);
    // 阶段 3: 狂暴红脉冲
    set_keyboard_rgb(255, 30, 30);
    sleep(300);
}
```

---

### 用例 4: 桌面 AI 助手 Function Calling 联动控制

用户吼一声：“**帮我开一键强冷**”或“**有点热，吹大点风**”。  
桌面 AI 助手（Brain）解析意图后，通过 Function Calling 直接下发硬件指令：

```javascript
// AI Agent 内部注册的 Tools Schema
const tools = [
  {
    name: 'set_fan_boost',
    description: '开启或关闭机械革命笔记本全速强冷模式 (Fan Boost)',
    parameters: {
      type: 'object',
      properties: { enable: { type: 'boolean' } },
      required: ['enable']
    }
  }
];

// 当大模型返回 Tool Call 时执行
async function handleToolCall(toolCall) {
  if (toolCall.name === 'set_fan_boost') {
    const { enable } = JSON.parse(toolCall.arguments);
    // 直接下发微内核 Ring0 指令！
    await invoke('set_fan_boost', { enable });
    return `已成功${enable ? '开启' : '关闭'}强冷模式！`;
  }
}
```

---

### 用例 5: 完全重写前端！自建独立替代控制台 (Alternative Shell)

如果你觉得官方的 React 亚克力主界面不符合你的极客审美，**你可以完全重写一个前端**：

1. **设置开机不弹官方主窗口**：
   在 `%APPDATA%\OpenRevo\config.json` 中配置：
   ```json
   {
     "wake_window_mode": "mini",
     "autostart": true
   }
   ```
2. **注册自己的 `shell` 插件**：
   在插件目录中声明 `"type": "shell"`，OpenRevo 会拉起你的主窗口。你的窗口可以使用 Vue3、Svelte、SolidJS 或者 WebGL 3D 渲染整台笔记本的剖面结构，通过本手册第 5 节的 8 大硬件指令全面接管机器！

---

### 用例 6: 进阶机甲桌宠工程规范 —— 边缘吸附灵动小球、动态换肤与跨插件事件总线 (Advanced Widget)

如果你不仅想实现一个简单的状态展示板，而是希望开发一个**具备多套动态皮肤、动画序列、拖拽至屏幕边缘自动折叠为半露微光小球、并且能与其他插件联动呼应**的完整级桌面桌宠挂件，推荐采用以下标准工程解构：

#### 1. 进阶工业级工程目录树 (Standard Layout)
位于 `%APPDATA%\OpenRevo\plugins\eva01-companion/`：
```text
eva01-companion/
│
├── manifest.json              # [核心清单] 插件元数据、窗口物理尺寸、权限声明与遥测传感器列表
├── index.html                 # [视窗容器] Webview 视窗入口（纯 HTML5，透明无边框）
│
├── css/
│   ├── main.css               # 主布局、毛玻璃亚克力与交互微动效
│   ├── dock.css               # 边缘停靠、折叠小球形态与展开动画关键帧 (@keyframes)
│   └── themes/                # 动态换肤样式包 (CSS 变量设计令牌)
│       ├── eva01-purple.css   # 初号机经典紫绿配色
│       ├── eva02-asuka.css    # 二号机暴走红橙配色
│       └── cyber-dark.css     # 极客透明单色微光配色
│
├── js/
│   ├── app.js                 # 主逻辑调度入口、生命周期与 DOM 初始化
│   ├── dock.js                # 屏幕边缘碰撞检测、吸附停靠与小球折叠状态机
│   ├── bus.js                 # 跨插件事件总线广播与监听器 (Cross-Plugin Bus)
│   └── skin.js                # 动态无感热换肤引擎与本地偏好持久化
│
└── assets/                    # 静态资产仓库
    ├── animations/            # 动态动画序列（Lottie JSON / APNG / WebM 透明动图 / SVG 矢量）
    │   ├── idle.svg           # 待机平稳呼吸形态
    │   ├── boost.svg          # 强冷暴走喷射形态
    │   └── ball-orbit.svg     # 边缘贴边小球微动特效
    ├── sounds/                # 拟物音效（可选，如点击、暴走提示音）
    │   └── switch.mp3
    └── preview.png            # 插件在 OpenRevo 插件管理面板中展示的缩略封面
```

#### 2. 清单权威声明 (`manifest.json`)
```json
{
  "id": "eva01-companion",
  "name": "EVA-01 初号机灵动桌宠",
  "version": "1.2.0",
  "author": "OpenRevo Community",
  "description": "异形拟态机甲桌宠，支持边缘吸附小球、初号机/二号机动态换肤与跨插件事件联动。",
  "type": "widget",
  "entry": "index.html",
  "window": {
    "width": 240,
    "height": 180,
    "transparent": true,
    "alwaysOnTop": true,
    "resizable": false
  },
  "telemetry": {
    "enabled": true,
    "desiredIntervalMs": 1500,
    "sensors": [
      "cpu_temp",
      "cpu_fan_rpm",
      "fan_boost"
    ]
  },
  "permissions": [
    "telemetry:read",
    "fan:control",
    "power:control"
  ]
}
```

#### 3. 边缘吸附与折叠小球机制 (`dock.js` + `dock.css`)
利用 Tauri 的 `getCurrentWebviewWindow()` 动态嗅探窗口物理外框与当前屏幕边界。当挂件被拖拽至屏幕右侧边界（阈值 30px）时，窗口物理尺寸无缝收缩为 `48×48`，DOM 自动切换为吸附小球样式；当鼠标移入小球判定区时，自动弹开恢复完整面板：

```javascript
// js/dock.js
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { currentMonitor } from '@tauri-apps/api/window';
import { LogicalSize, LogicalPosition } from '@tauri-apps/api/dpi';

const appWindow = getCurrentWebviewWindow();
let isDocked = false;

// 拖拽释放后触发贴边校验
export async function checkEdgeDock() {
  const monitor = await currentMonitor();
  if (!monitor) return;

  const scale = monitor.scaleFactor;
  const winPos = await appWindow.outerPosition();
  const screenW = monitor.size.width / scale;
  const curX = winPos.x / scale;

  // 靠近右侧边缘（小于 30px）触发吸附折叠
  if (screenW - (curX + 240) < 30) {
    isDocked = true;
    document.body.classList.add('docked-bubble');
    // 窗口平滑收缩为 48x48 悬浮微球，贴紧屏幕最右侧
    await appWindow.setSize(new LogicalSize(48, 48));
    await appWindow.setPosition(new LogicalPosition(screenW - 48, winPos.y / scale));
  }
}

// 悬停展开完整看板
export function initDockEvents() {
  const bubble = document.getElementById('bubble-hitbox');
  bubble.addEventListener('pointerenter', async () => {
    if (isDocked) {
      document.body.classList.remove('docked-bubble');
      await appWindow.setSize(new LogicalSize(240, 180));
    }
  });

  document.getElementById('core-panel').addEventListener('pointerleave', async () => {
    if (isDocked) {
      document.body.classList.add('docked-bubble');
      await appWindow.setSize(new LogicalSize(48, 48));
    }
  });
}
```

```css
/* css/dock.css */
:root {
  --core-accent: #6366f1;
  --core-glow: rgba(99, 102, 241, 0.4);
}

/* 正常展开看板 */
#core-panel {
  width: 240px;
  height: 180px;
  border-radius: 16px;
  background: rgba(15, 23, 42, 0.85);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

/* 边缘折叠小球形态 */
body.docked-bubble #core-panel {
  display: none;
}
body.docked-bubble #bubble-hitbox {
  display: flex;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(15, 23, 42, 0.9);
  border: 2px solid var(--core-accent);
  box-shadow: 0 0 16px var(--core-glow);
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transform: translateX(10px); /* 贴边半露小球胶囊 */
}
```

#### 4. 多套皮肤无感热切换与资产组织 (`skin.js`)
皮肤切换采用 CSS Design Tokens 隔离，绝不在 JS 中写死任何样式数值：
```javascript
// js/skin.js
const THEMES = ['eva01-purple', 'eva02-asuka', 'cyber-dark'];

export function applyTheme(themeName) {
  // 1. 动态挂载主题样式表
  const link = document.getElementById('theme-stylesheet');
  link.href = `css/themes/${themeName}.css`;
  
  // 2. 动态切换动画形象
  const animImg = document.getElementById('companion-avatar');
  animImg.src = `assets/animations/${themeName}-idle.svg`;

  // 3. 持久化到插件本地独立存储
  localStorage.setItem('companion_theme', themeName);
}
```

#### 5. 跨插件与主视窗公频事件联动 (`bus.js`)
利用 OpenRevo 底层统一的 IPC 事件总线，插件之间可以跨视窗自由互通：

- **插件 A（桌宠挂件）在硬件剧烈过热时向公频广播自定义事件**：
  ```javascript
  import { emit } from '@tauri-apps/api/event';

  // 结温突破 85℃ 时广播“初号机暴走”联动信号
  export async function broadcastBerserk(temp) {
    await emit('plugin:eva01:berserk', {
      source: 'eva01-companion',
      temp,
      action: 'fan_boost_suggested'
    });
  }
  ```

- **插件 B（键盘流光插件 / AI 语音助手插件）监听并同步响应**：
  ```javascript
  import { listen } from '@tauri-apps/api/event';

  // 监听插件 A 的广播
  await listen('plugin:eva01:berserk', (event) => {
    const { temp } = event.payload;
    console.log(`收到初号机暴走联动: 当前结温 ${temp}℃`);
    
    // 联动 1: 键盘立即闪烁猩红警报流光
    // 联动 2: AI 看板娘切换为惊恐动画帧
  });
  ```

### 用例 7: 主窗口主题皮肤与插槽化布局重排 (Skin Plugin —— 一切皆插件)

> **目标**：不修改 OpenRevo 任何一行业务逻辑代码，仅通过纯 CSS 与声明式配置，彻底改变 OpenRevo 主窗口的视觉风格、色系基调、磨砂亚克力深度，并利用 CSS Grid 插槽重排仪表盘模块，甚至重新定义主窗口的初始物理尺寸与缩放能力。

#### 1. 核心设计原则：为什么“皮肤即插件”？
* **统一目录规范**：严禁创建孤立的 `skins/` 文件夹。所有皮肤均存放在 `%APPDATA%\OpenRevo\plugins\<skin_id>\`，在主控制台“插件管理”页享有统一的启闭开关、版本展示与元数据治理。
* **物理级 0 内存开销**：皮肤不创建独立 Webview 进程，纯粹将 `theme.css` 注入宿主主窗口的 `<style id="openrevo-theme-skin">`，实现毫秒级热装卸与零内存损耗。
* **独立单选激活**：皮肤管理属于主题通道，同一时间只激活一个主窗口皮肤，停用后无缝回退至默认主题与 960×740 视窗基准。
* **资产自动内联（零手动 Base64）**：在 CSS 中直接写标准相对路径（如 `url('assets/bg.webp')`），Rust 后端在读取时**自动在内存中无损解析并内联为 Base64 Data URI**，开发者绝对无须使用外部 Python 脚本提前转码！
* **全局穿透（Global Root Penetration）**：激活皮肤时，宿主会在 `document.documentElement`（`<html>` 根节点）上全量同步 `data-skin="<skin_id>"`。样式选择器可以直通所有浮动弹窗（`.sys-modal-card`）、菜单与 Tooltip，绝无样式脱节逃逸。

#### 2. 皮肤工程目录规范
```text
%APPDATA%\OpenRevo\plugins\mecha-eva01-skin\
├── manifest.json              # 核心元数据与主窗口视窗几何声明
├── theme.css                  # 皮肤全量样式表 (覆写 Tokens 与 Grid 插槽)
└── assets\                    # 本地背景贴图与字体资产 (直接在 CSS 中通过相对路径引用)
    ├── eva01_bg.webp
    └── neon_core.svg
```

#### 3. 插件清单声明 (`manifest.json`)
```json
{
  "id": "mecha-eva01-skin",
  "name": "EVA 初号机觉醒皮肤",
  "version": "1.0.0",
  "author": "NERV Engineering",
  "description": "初号机标志性紫绿觉醒电竞配色，自适应宽屏排版，支持传感器插槽重排与全息背景贴图。",
  "plugin_type": "skin",
  "theme_css": "theme.css",
  "window": {
    "width": 1080,
    "height": 780,
    "resizable": true
  }
}
```
* **`plugin_type: "skin"`**（或 `"type": "skin"`）：声明为主控制台皮肤，在插件面板中渲染为【主窗皮肤】专属角标。
* **`theme_css`**：指定相对于插件目录的 CSS 入口（默认 `"theme.css"`）。
* **`window.width / height`**：声明该皮肤的最佳版面尺寸。当用户切换到此皮肤时，微内核自动触发 `resize_window` 平滑调整主窗口几何尺寸并自动屏幕居中。
* **`window.resizable: true`**：若该皮肤支持大屏响应式布局，可解锁主窗口的自由拖拽缩放能力。

#### 4. 全界面标准插槽矩阵速查 (Slot Matrix Contract)
OpenRevo 主控制台各个界面骨架与内容区块均具有**绝对稳定的语义类名与 `data-slot` 属性契约**，开发者可放心针对以下标准插槽进行样式重塑或网格重排：

| 语义插槽类名 | `data-slot` 标记 | 默认定位与布局特性 | 说明 |
| :--- | :--- | :--- | :--- |
| **`.openrevo-shell`** | `shell` | Flex Column (支持切换为 Grid) | 主窗体总外壳容器 (100vw × 100vh 满高) |
| **`.shell-titlebar`** | `titlebar` | Flex (高度固定 ~44px, `flex-shrink: 0`) | 顶栏标题栏、Logo 与窗口操作控件区 |
| **`.shell-navbar`** | `navbar` | Flex (横向滚动胶囊栏, `flex-shrink: 0`) | 顶栏 Sub-Nav 插件与核心模块页签切换栏 |
| **`.shell-viewport`** | `viewport` | Flex 1 (`flex: 1 1 0%`, 纵向独立滚动) | 核心视口容器，吞满剩余垂直高度 |
| **`.overview-slot-grid`** | `overview-grid`| Flex Column (`gap: 16px`) | 状态概览页的主网格容器 |
| **`.overview-slot-telemetry`**| `telemetry` | Block | 顶部遥测监控区插槽 |
| **`.telemetry-cards-grid`** | `telemetry-grid`| Grid (`grid-template-areas: "cpu gpu fan vram"`) | 4 联硬件监控卡片对称网格 |
| **`.telemetry-card-cpu`** | `cpu` | `grid-area: cpu` | CPU 温度/功耗/频率/负载卡片 |
| **`.telemetry-card-gpu`** | `gpu` | `grid-area: gpu` | 独立 GPU 温度/功耗/频率/负载卡片 |
| **`.telemetry-card-fan`** | `fan` | `grid-area: fan` | 双通道散热风扇 RPM 与强冷状态卡片 |
| **`.telemetry-card-vram`**| `vram` | `grid-area: vram` | 显存占用量与 GDDR 负载卡片 |
| **`.overview-slot-power`** | `power` | Block | 性能释放模式 (办公/均衡/狂暴/自定义) 插槽 |
| **`.overview-slot-quickactions`** | `quickactions` | Block | 中部高频微调 (强冷/离电低刷/刷新率) 插槽 |
| **`.overview-slot-switches`** | `switches` | Block | 底部 9 联原厂电竞级外设开关网格插槽 |
| **`.mini-drawer-root`** | `mini-root` | Flex Column / 迷你抽屉 | 迷你悬浮抽屉主容器 |
| **`.mini-slot-*`** | `header`...`footer` | 8 联插槽区域 | 包含 `.mini-slot-header`, `.mini-slot-power`, `.mini-slot-gpu`, `.mini-slot-display`, `.mini-slot-lighting`, `.mini-slot-battery`, `.mini-slot-switches`, `.mini-slot-footer` |
| **`.osd-root`** | `osd-root` | Flex (100vw × 100vh) | OSD 屏幕弹窗最外层容器（含 `data-osd-pos`） |
| **`.osd-container`** | `osd-container` | Inline-Flex | OSD 弹窗卡片本体（含 `data-osd-theme`） |
| **`.osd-slot-icon`** | `icon` | Flex 居中 | OSD 左侧图标与背底底盘插槽 |
| **`.osd-slot-content`**| `content` | Flex 水平居中 | OSD 右侧文本内容与进度槽插槽 |
| **`.osd-slot-title`** | `title` | Flex | OSD 核心标题大字插槽 |
| **`.osd-slot-dot`** | `status-dot` | Block (圆形点) | OSD 开启/关闭指示状态呼吸点插槽 |
| **`.osd-slot-meter`** | `meter` | Flex (间距 4px) | OSD 键盘背光/阶梯档位进度条插槽 |
| **`.osd-slot-meter-bar`**| `meter-bar`| Block (含 `data-level` 与 `data-active`) | OSD 档位独立小格插槽 |

#### 5. 样式编写实战与插槽重排 (`theme.css`)
所有选择器以 `[data-skin="<skin_id>"]` 或 `html[data-skin="<skin_id>"]` 作为根命名空间：

```css
/* ============================================================
   1. 覆写 Design System Tokens (色彩/圆角/光效基调)
   ============================================================ */
html[data-skin="mecha-eva01-skin"] {
  /* 品牌强调色：初号机装甲紫与荧光绿 */
  --accent-pwr: #a855f7;
  --accent-pwr-dim: rgba(168, 85, 247, 0.2);
  --accent-pwr-border: rgba(168, 85, 247, 0.45);
  --accent-beast: #22c55e;
  
  /* 表面层级：深化暗夜紫亚克力 */
  --surface-base: rgba(10, 6, 18, 0.94);
  --surface-overlay: rgba(16, 10, 26, 0.85);
  --surface-card: rgba(26, 16, 42, 0.75);
  --border-subtle: rgba(168, 85, 247, 0.2);
  --border-medium: rgba(168, 85, 247, 0.4);
  --border-highlight: #22c55e;
  
  /* 圆角与字体 */
  --radius-md: 4px;
  --radius-lg: 6px;
  --font-mech: 'Orbitron', 'Chakra Petch', sans-serif;
}

/* ============================================================
   2. 主容器全息贴图 (直接使用相对路径，Rust 运行时自动内存内联)
   ============================================================ */
[data-skin="mecha-eva01-skin"] .openrevo-shell {
  background-image: 
    radial-gradient(circle at 90% 10%, rgba(34, 197, 94, 0.15), transparent 40%),
    radial-gradient(circle at 10% 90%, rgba(168, 85, 247, 0.18), transparent 50%),
    url("assets/eva01_bg.webp") !important;
  background-size: cover !important;
  background-position: center !important;
}

/* ============================================================
   3. 状态概览页：颠倒 4 联硬件监控卡片顺序 (Grid Areas 实战)
   ============================================================ */
/* 仅用一行 CSS 将显卡与显存置于最左侧，实现极客战术 HUD 布局 */
[data-skin="mecha-eva01-skin"] .telemetry-cards-grid {
  grid-template-areas: "gpu vram cpu fan" !important;
}

/* ============================================================
   4. 模块垂直次序重排 (Flex Order 实战)
   ============================================================ */
/* 让底部 9 联快捷开关插槽上浮到性能模式之前 */
[data-skin="mecha-eva01-skin"] .overview-slot-switches {
  order: 1;
}
[data-skin="mecha-eva01-skin"] .overview-slot-power {
  order: 2;
}

/* ============================================================
   5. 全局弹窗穿透修饰 (Portal Penetration 实战)
   ============================================================ */
html[data-skin="mecha-eva01-skin"] .sys-modal-card {
  border-color: #a855f7 !important;
  box-shadow: 0 0 35px rgba(168, 85, 247, 0.35) !important;
}

/* ============================================================
   6. OSD 屏幕快捷弹窗联动修饰 (OSD Slot Customization 实战)
   ============================================================ */
html[data-skin="mecha-eva01-skin"] .osd-container {
  background: rgba(10, 6, 18, 0.92) !important;
  border: 1.5px solid #22c55e !important;
  border-radius: 4px !important; /* 战术机甲切角 */
  box-shadow: 0 0 25px rgba(34, 197, 94, 0.35) !important;
}
html[data-skin="mecha-eva01-skin"] .osd-slot-title {
  color: #22c55e !important;
  font-family: 'Orbitron', monospace !important;
  letter-spacing: 1.2px !important;
}
html[data-skin="mecha-eva01-skin"] .osd-slot-meter-bar.active {
  background: #a855f7 !important; /* 装甲紫指示条 */
  box-shadow: 0 0 8px #a855f7 !important;
}
```

---

### 用例 8: 独立背景壁纸插件与零代码壁纸包 (Background Plugin)

> **目标**：不与任何 Fluent 皮肤互斥！用户可以单独启用一套背景壁纸插件，或者与任何主题皮肤自由叠用。

#### 1. 核心架构：双通道独立注入机制
* **独立样式通道**：背景样式注入到 `<style id="openrevo-custom-background">`，状态持久化在 `config.json` 的 `active_background`。
* **独立开关控制**：在“插件”页中渲染为【背景壁纸】专属青色徽标，可独立开启/关闭，完全不影响当前激活的 `active_skin`。
* **自动全窗与抽屉双覆盖**：背景同时覆盖大面板（`.openrevo-shell`）和迷你桌面抽屉（`.mini-drawer-root`）。

#### 2. 方式 A：编写自由 CSS 模式（可控透明度与遮罩）
```text
%APPDATA%\OpenRevo\plugins\bg-cyberpunk\
├── manifest.json
├── background.css
└── assets\
    └── wallpaper.jpg
```

**`manifest.json`**：
```json
{
  "id": "bg-cyberpunk",
  "name": "夜之城赛博朋克全息壁纸",
  "version": "1.0.0",
  "author": "NightCity Geek",
  "description": "高精度赛博朋克夜景全息壁纸，带 40% 磨砂遮罩保护文字可读性。",
  "plugin_type": "background",
  "background_css": "background.css"
}
```

**`background.css`**（直接写相对路径即可）：
```css
.openrevo-shell,
.mini-drawer-root {
  background-image: 
    linear-gradient(rgba(11, 14, 20, 0.45), rgba(11, 14, 20, 0.65)),
    url("assets/wallpaper.jpg") !important;
  background-size: cover !important;
  background-position: center !important;
  background-repeat: no-repeat !important;
}
```

#### 3. 方式 B：极简零代码模式（连一行 CSS 都不需要写！）
如果你只想让一张好看的二次元壁纸生效，**完全不需要写 `background.css`**！
只需在插件根目录下放入图片，并在 `manifest.json` 里声明：

```json
{
  "id": "bg-minimal",
  "name": "极简极客壁纸",
  "version": "1.0.0",
  "author": "OpenRevo Fan",
  "description": "零代码快速壁纸包",
  "plugin_type": "background",
  "image": "wallpaper.png"
}
```
* **全自动合成**：OpenRevo 内核若未检测到 `background.css`，会自动寻找 `manifest.image` 或目录下的 `background.png/jpg/webp`、`wallpaper.jpg`、`bg.png`，在 Rust 内存中自动生成全套适配 CSS（满屏铺满、居中不重复）注入宿主！

---

### 用例 9: 硬件级伴随提权服务开发范式 (Sidecar Service Integration - 以 `ec-config` 为例)

> **痛点背景**：某些硬件调优插件（如 EC 物理寄存器读写、PBO 压频调压代理）需要依赖后台常驻的原生执行文件（如 Go / C++ / Python / Node.js 打包的 `.exe`），以前需要让用户手动双击 `启动服务.bat` 弹出一个丑陋且易误关闭的 CMD 黑框。
> **全新解法**：借助 OpenRevo 内置的伴随服务监督器（Companion Service Supervisor），声明即托管！

#### 1. 清单声明 (`manifest.json`)
```json
{
  "id": "ec-config",
  "name": "EC 物理配置工具",
  "version": "1.0.0",
  "plugin_type": "tab",
  "entry": "dist/index.html",
  "service": {
    "executable": "agent/ec_service.exe",
    "args": ["--port", "18945", "--silent"],
    "port": 18945,
    "elevated": true,
    "auto_start": false,
    "auto_stop": true,
    "description": "EC 物理寄存器桥接服务"
  }
}
```

#### 2. 前端界面一键启停与通信 (`app.js`)
在插件的沙盒 `iframe` 内，通过标准 `postMessage` 与宿主双向桥接，无需直接接触底层进程 API：

```javascript
// 1. 发起启动服务
document.getElementById('start-btn').addEventListener('click', () => {
  window.parent.postMessage({ action: 'START_PLUGIN_SERVICE' }, '*');
});

// 2. 发起停止服务
document.getElementById('stop-btn').addEventListener('click', () => {
  window.parent.postMessage({ action: 'STOP_PLUGIN_SERVICE' }, '*');
});

// 3. 监听宿主下发的状态同步事件
window.addEventListener('message', (event) => {
  if (event.data?.type === 'OPENREVO_PLUGIN_SERVICE_STATUS') {
    const { status } = event.data;
    const statusLabel = document.getElementById('srv-status');
    
    if (status.running) {
      if (status.healthy) {
        statusLabel.textContent = `🟢 服务已就绪 (端口 ${status.port}, PID ${status.pid})`;
        statusLabel.className = 'status-ok';
      } else {
        statusLabel.textContent = `🟡 正在探活回环端口...`;
        statusLabel.className = 'status-warn';
      }
    } else {
      statusLabel.textContent = status.last_error 
        ? `🔴 启动失败: ${status.last_error}` 
        : `⚪ 服务已停止`;
      statusLabel.className = status.last_error ? 'status-error' : 'status-dim';
    }
  }
});

// 4. 页面初始化时主动查询一次当前状态
window.parent.postMessage({ action: 'GET_PLUGIN_SERVICE_STATUS' }, '*');
```

#### 3. 宿主原生特性与安全保障
* **宿主顶栏原生联动**：无论前端是否写了控制按钮，宿主在选项卡右上方自动渲染伴随服务胶囊指示器，用户可随时点击一键启停。
* **无黑框提权与进程树防孤儿**：宿主使用 `CREATE_NO_WINDOW` 静默拉起，并在宿主退出时通过 `TerminateProcess` 彻底回收，防止用户任务管理器中残留游荡进程。

---

### 用例 10: 原生体验第三方插件市场开发对接范式 (Plugin Market Integration - 以 `plugin-market` 为例)

> **目标**：打造一个生态插件市场选项卡（`plugin_type: "tab"`），支持用户在线浏览、一键下载安装、一键卸载、免重启热生效、外链跳转与复制。

#### 1. 检查宿主能力契约 (`get_plugin_host_capabilities`)
在市场启动时，调用能力探测接口：
```javascript
import { invoke } from '@tauri-apps/api/core';

async function checkHost() {
  const caps = await invoke('get_plugin_host_capabilities');
  console.log(`宿主版本: ${caps.version}, 架构: ${caps.arch}`);
  
  // 检查是否支持伴随服务与市场安装器
  if (!caps.capabilities.includes('market_installer')) {
    alert('当前 OpenRevo 版本过低，请升级后使用在线安装功能！');
  }
}
```

#### 2. 在线插件一键安全静默安装 (`install_plugin_from_url`)
```javascript
async function installPlugin(pluginMeta) {
  try {
    toast(`正在拉取并安装 ${pluginMeta.name}...`);
    
    // 宿主底层校验 HTTPS、防路径穿越、自动解压至 plugins 目录并热加载
    const newPlugin = await invoke('install_plugin_from_url', {
      url: pluginMeta.download_url,
      expectedSha256: pluginMeta.sha256 // 可选防篡改哈希
    });
    
    toast(`安装成功！插件 ID: ${newPlugin.id}`);
    refreshInstalledList();
  } catch (err) {
    alert(`安装失败: ${err}`);
  }
}
```

#### 3. 安全卸载送入回收站 (`uninstall_plugin`)
```javascript
async function uninstallPlugin(pluginId) {
  if (!confirm(`确定要卸载插件 ${pluginId} 吗？`)) return;
  
  try {
    // 自动停止伴随服务、清理配置开关、并送入系统回收站
    await invoke('uninstall_plugin', { pluginId });
    toast('插件已移入回收站并卸载');
    refreshInstalledList();
  } catch (err) {
    alert(`卸载失败: ${err}`);
  }
}
```

#### 4. 统一开关控制 (`set_plugin_enabled_unified`)
无需针对主题皮肤、背景壁纸与扩展组件分别写三套 IPC 分支，统一调用：
```javascript
async function togglePlugin(pluginId, targetEnabled) {
  // 宿主自动区分 skin/background/功能插件 并正确更新配置
  await invoke('set_plugin_enabled_unified', {
    pluginId: pluginId,
    enabled: targetEnabled
  });
}
```

#### 5. 安全外链与剪贴板突破
```javascript
// 在默认浏览器中打开作者 GitHub 主页（绕过沙盒 iframe 无法 window.open 限制）
async function openAuthorLink(url) {
  await invoke('open_external_url', { url });
}

// 复制安装命令或链接到系统剪贴板（绕过沙盒限制）
async function copyLink(text) {
  await invoke('write_clipboard', { text });
  toast('已复制到剪贴板');
}
```

---

### 用例 11: 声明式零代码全局快捷键与按键宏映射插件 (Universal Shortcut Bus)

> **目标**：为机械革命笔记本定制一套全局快捷键工坊，支持在游戏独占全屏中随时按键瞬切强冷、性能模式、屏幕刷新率，甚至一键息屏或调起宏。**无需写任何 C/C++ 代码，无需拉起外部 exe 守护进程，享受宿主 0 内存开销的底座红利！**

#### 方式 A：纯声明式零代码配置 (`manifest.json`)
只需在插件 `manifest.json` 中配置 `shortcuts` 数组，宿主在启用该插件时自动将快捷键注入 Ring3 键盘总线；禁用或卸载插件时自动全量注销：

```json
{
  "id": "pro-gamer-keys",
  "name": "极客电竞快捷键工坊",
  "version": "1.0.0",
  "plugin_type": "tab",
  "entry": "index.html",
  "shortcuts": [
    {
      "id": "gamer_turbo_boost",
      "name": "一键全速暴走强冷",
      "combo": "Ctrl+Shift+F",
      "action": { "type": "ToggleFanBoost" },
      "osd": true
    },
    {
      "id": "gamer_power_cycle",
      "name": "循环切换性能释放模式",
      "combo": "Ctrl+Shift+M",
      "action": { "type": "CyclePowerMode" },
      "osd": true
    },
    {
      "id": "gamer_hz_switch",
      "name": "刷新率瞬切 (高刷/省电)",
      "combo": "Ctrl+Alt+R",
      "action": { "type": "ToggleRefreshRate" },
      "osd": true
    },
    {
      "id": "gamer_monitor_sleep",
      "name": "挂机一键息屏",
      "combo": "Ctrl+Shift+L",
      "action": { "type": "MonitorOff" },
      "osd": false
    }
  ]
}
```

#### 方式 B：前端可视化录制与动态注册 API
在插件界面中，让用户像玩游戏改键位一样按下组合键并动态注册：

```javascript
import { invoke } from '@tauri-apps/api/core';

// 1. 查询当前已生效的所有快捷键
const list = await invoke('get_registered_shortcuts');
console.log('当前快捷键列表:', list);

// 2. 动态注册用户自定义按键
await invoke('register_custom_shortcut', {
  binding: {
    id: 'user_boss_key',
    name: '老板键：一键静音 + 锁屏',
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

### 用例 12: 声明式自动化监听与规则看门狗 (Universal Watcher & Trigger Hub —— 彻底根除隐藏桌宠偷跑轮询)

> **痛点背景**：在早期，部分极客开发者为了实现“监控 CPU 结温达到 55℃ 时自动刷新 EC 状态字”或“启动游戏时自动切狂暴模式”，在插件里声明一个 `widget`（桌面悬浮挂件），将其设为全透明隐藏并使用 `setInterval` 进行 100ms 死循环轮询。  
> **这种做法存在致命缺陷**：
> 1. **白白浪费内存**：一个 WebView2 实例增量占用约 15MB~25MB 内存；
> 2. **后台节流失效**：Windows 与 Chromium 会自动将不可见窗口的定时器强行节流冻结，导致切模逻辑经常失效或严重滞后；
> 3. **唤醒独显功耗爆炸**：挂件前端轮询不当会强制唤醒本处于 D3Cold 深睡的 NVIDIA 独立显卡。
>
> **官方终极解法**：借助 OpenRevo 微内核内建的 **Universal Watcher & Trigger Hub**，实现纯声明式规则监控。**0 MB 内存、0 额外视窗、0 代码死循环，由宿主内核在无锁遥测心跳中统一调度！**

#### 1. 极简项目结构
一个纯自动化规则插件，**根目录下只需一个 `manifest.json` 文件**，连 HTML、CSS、JS 都完全不需要写：
```text
%APPDATA%\OpenRevo\plugins\mode4-fan-unblocker\
└── manifest.json
```

#### 2. 清单配置 (`manifest.json`)
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

#### 3. 核心机制优势解析
* **`cooldown_ms` 防抖冷却**：规则命中后进入冷却锁，防止温度在临界线抖动时引发高频重复触发与硬件冲突；
* **地址与掩码语法宽容**：`address` 与 `mask` 既支持字符串十六进制（`"0x0751"`、`"0x10"`），也支持十进制数字（`1873`、`16`）；
* **宿主生命周期全托管**：在宿主【插件】列表中启用该插件即刻注入微内核总线开始看门狗监控；关闭插件时自动全量卸载，不留任何残留副作用；
* **版本兼容守护 (`min_host_version`)**：声明 `"min_host_version": "0.9.3"`，若旧版宿主尝试加载，界面将明确亮起警告徽章并安全拦截，引导用户升级。

---

## 8. UI 设计语言与 Design Tokens 复用指南

为了保证在所有原生主题（默认赛博、极简黑白工业风 `mono`）以及第三方插件中质感完全统一，**严禁在样式中硬编码十六进制颜色数值**，必须复用 `src/index.css` 官方 Design System Tokens：

| Token 变量名 | 默认值 (暗色基准) | 作用语义 |
| :--- | :--- | :--- |
| **`--surface-base`** | `rgba(11, 14, 20, 0.94)` | 窗口最底层背景表面 |
| **`--surface-card`** | `rgba(22, 27, 34, 0.75)` | 悬浮功能卡片、微面板表面 |
| **`--surface-card-hover`** | `rgba(33, 38, 45, 0.85)` | 卡片鼠标悬浮激活态 |
| **`--surface-overlay`** | `rgba(15, 23, 42, 0.85)` | 下拉菜单、二级子面板、折叠卡片背景 |
| **`--surface-sunken`** | `rgba(0, 0, 0, 0.35)` | 凹陷式数据底盘、仪表背景 |
| **`--surface-pill`** | `rgba(255, 255, 255, 0.05)` | 胶囊按钮、药丸标签默认表面 |
| **`--border-subtle`** | `rgba(255, 255, 255, 0.08)` | 微弱边框，用于卡片分割 |
| **`--border-medium`** | `rgba(255, 255, 255, 0.15)` | 中度边框，用于可交互控件与按钮外廓 |
| **`--border-highlight`** | `rgba(255, 255, 255, 0.30)` | 高亮焦点与选中态描边 |
| **`--text-primary`** | `#f8fafc` | 主标题、核心数值与高强调文字 |
| **`--text-secondary`** | `#cbd5e1` | 次级标题与重要说明 |
| **`--text-muted`** | `#94a3b8` | 功能描述、次要辅助文本 |
| **`--text-dim`** | `#64748b` | 计量单位、微小提示与未激活状态 |
| **`--status-ok`** | `#10b981` | 正常运行、节能/办公状态 (翡翠绿) |
| **`--status-warn`** | `#f59e0b` | 告警、原厂避让、降频提示 (琥珀黄) |
| **`--status-error`** | `#ef4444` | 危险操作、蓝屏熔断看门狗、超温 (绯红) |
| **`--radius-md`** | `8px` | 标准按钮、输入框、微调卡片圆角 |
| **`--radius-lg`** | `12px` | 大功能卡片、对话框外框圆角 |
| **`--transition-fast`** | `150ms cubic-bezier(0.4, 0, 0.2, 1)` | 按钮悬停、开关微动效过渡 |

---

## 9. 调试与安全模式排错 (Safe Mode 二分法)

开发插件时，难免遇到死循环或语法错误。OpenRevo 建立了**实机 5 秒二分法排错机制**：

### 异常自愈指南
当第三方插件异常导致崩溃闪退时，**绝对无需重装 OpenRevo**：
1. 打开记事本：`%APPDATA%\OpenRevo\config.json`；
2. 找到 `custom_plugin_toggles` 节点；
3. 将疑似崩溃的插件 ID 改为 `false`：
   ```json
   "custom_plugin_toggles": {
     "faulty-plugin": false
   }
   ```
4. 保存并重新打开 OpenRevo，主内核将跳过该插件，秒级恢复稳定运行！
