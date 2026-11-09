> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 4. 插件清单权威规范 (Plugin Manifest Reference)

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

## 清单字段详细说明

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
| `entry` | `string` | 可选 | `widget`/`tab`/`service` | 默认入口相对路径，如 `index.html` 或 `welcome.rhai` |
| `tab` | `object` | 可选 | 全类型 | **【控制台专属页签出口】**声明插件在控制台工作区的 Tab 入口（含 `entry`、`type`、`title`、`icon`，支持 WebView2 与 Slint 独立指定） |
| `widget` | `object` | 可选 | 全类型 | **【桌面独立悬浮视窗出口】**声明插件在桌面的独立挂件（含 `entry`、`type`、`window`，支持 WebView2 与 Slint 独立指定） |
| `window` | `object` | 可选 | `widget`/`tab`/`skin`/`shell` | 视窗高级几何与外廓属性（详见下方 `window` 子对象详解） |
| `telemetry` | `object` | 可选 | 全类型 | **【声明式遥测需求】**声明硬件遥测传感器需求（详见下方 `telemetry` 子对象与第 6 节按需订阅机制） |
| `permissions` | `string[]`| 可选 | 全类型 | 声明所需权限（未声明将被安全网关拦截） |
| `watchers` / `rules` | `object[]` | 可选 | 全类型 | **【声明式自动化规则总线】**无需 Webview 视窗，宿主内核统一调度温度联动、游戏切模与硬件保护规则 |
| `shortcuts` | `object[]` | 可选 | 全类型 | **【声明式全局快捷键总线】**Ring3 穿透级热键绑定，自动联动机甲 OSD 弹窗与动作流 |
| `service` | `object` | 可选 | `tab`/`widget`/`slot` | **【声明式伴随提权服务 / Sidecar】**声明插件依赖的后端本地原生进程服务（见下方 `service` 子对象详解，Slint/Tauri 全运行时统一支持） |

### `tab` 控制台工作区视窗子对象详细配置表
| 子属性 | 类型 | 默认值 | 适用场景与说明 |
| :--- | :--- | :--- | :--- |
| `entry` | `string` | `"dist/index.html"` 或 `"main.slint"` | 面板入口相对路径（如 `"dist/index.html"` 或 `"tab.slint"`） |
| `type` / `engine` | `string` | 自动识别 | **【渲染引擎显式指定】**：可选 `"webview2"`（富 Web 生态，默认支持 HTML/React/Vue/JS）或 `"slint"`（声明式原生嵌入）。若未显式指定，根据入口后缀（`.slint` vs `.html`）自动探测 |
| `title` | `string` | 插件 `name` | 在控制台左侧/顶栏呈现的自定义页签标题 |
| `icon` | `string` | 可选 | 页签专属图标相对路径或图标名 |

### `widget` 桌面悬浮视窗子对象详细配置表
| 子属性 | 类型 | 默认值 | 适用场景与说明 |
| :--- | :--- | :--- | :--- |
| `entry` | `string` | `"index.html"` 或 `"widget.slint"` | 挂件入口相对路径（如 `"widget.slint"` 或 `"index.html"`） |
| `type` / `engine` | `string` | 自动识别 | **【挂件渲染引擎显式指定】**：可选 `"slint"`（推荐：0MB Chromium 开销原生极速桌面条）或 `"webview2"`（传统 HTML 悬浮窗）。若未显式指定，根据入口后缀自动探测 |
| `window` | `object` | 默认几何属性 | 桌面视窗几何、透明度与置顶层级（详见下方 `window` 子对象） |

### `window` 视窗子对象详细配置表 (全场景悬浮与专业大窗口)
| 子属性 | 类型 | 默认值 | 适用场景与说明 |
| :--- | :--- | :--- | :--- |
| `width` | `number` | `240` | 初始逻辑宽度（如 XTU/SMU 建议设为 `920`） |
| `height` | `number` | `320` | 初始逻辑高度（如 XTU/SMU 建议设为 `650`） |
| `resizable` | `boolean` | `false` | 是否允许鼠标拖拽窗口边缘缩放 |
| `decorations` | `boolean` | `false` | **【原生标题栏】**：设为 `true` 即可显示标准 Windows 标题栏（包含最小化、最大化、关闭按钮）。大型复杂工具页（XTU/SMU/风扇曲线）强烈建议开启！ |
| `transparent` | `boolean` | `true` | 是否启用全透明 Alpha 通道（异形桌宠设为 `true`，复杂实色工具面板设为 `false`） |
| `always_on_top` | `boolean` | `true` | 是否保持在其他软件窗口上方置顶（大型复杂工具通常设为 `false`） |
| `skip_taskbar` | `boolean` | `true` | 是否在 Windows 系统任务栏跳过显示（设为 `false` 可在任务栏显示独立图标，支持 `Alt+Tab` 秒切） |

### `telemetry` 硬件遥测按需声明子对象 (On-Demand Telemetry Contract)

> ⚠️ **核心铁律**：OpenRevo 遥测引擎采用**按需消费者生命周期驱动机制（Consumer-Driven Lifecycle）**。宿主不会在无需求时盲目高频轮询底层硬件。**插件若需要接收硬件遥测数据流，必须在 `manifest.json` 中显式声明 `telemetry`**！宿主将在插件视窗激活时自动注册 Consumer，离开或关闭时自动退订，彻底防止独显异常唤醒与电池空耗。

```json
"telemetry": {
  "enabled": true,
  "desiredIntervalMs": 1500,
  "sensors": [
    "cpu_temp",
    "cpu_fan_rpm",
    "cpu_power_w",
    "power_mode",
    "fan_boost",
    "battery_level",
    "is_charging",
    "is_ac_online"
  ]
}
```

| 子属性 | 类型 | 默认值 | 说明与防呆规范 |
| :--- | :--- | :--- | :--- |
| `enabled` | `boolean` | `true` | 是否启用硬件遥测订阅。设为 `false` 时完全不产生硬件采样心跳 |
| `desiredIntervalMs` | `number` | `1500` | 期望的刷新采样间隔（毫秒）。宿主取所有活跃消费者的最小公约数进行动态调度 |
| `sensors` | `string[]` | 全部基础字段 | **【按需字段清单】**声明插件所需的数据字段。**若不需要 GPU 状态，严禁声明 `gpu_*` 字段**，确保 NVIDIA 独立显卡安详保持 D3Cold 零功耗深睡 |

### `service` 伴随提权服务子对象详细配置表 (Sidecar / Companion Service)

针对需要借助原生可执行程序（如 Go、Rust、Python、Node.js 编译的 `.exe` 或 EC/驱动桥接代理）的硬件级扩展插件（如 `ec-config`、`cpu-guardian`、`mro-hud`），OpenRevo 提供了微内核级内置的**伴随服务监督器（Companion Service Supervisor）**。

> 🌟 **双运行时全量打通**：无论是传统的 **Tauri 前端**，还是超轻量的 **Slint 原生端 (Native Wry)**，伴随服务均由 Rust 微内核底座统一切入管理。插件无论在主窗口内嵌运行，还是弹出为独立视窗，均享有完全一致的自动拉起、端口探活与生命周期回收保障！

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
| `auto_start` | `boolean` | `false` | 打开插件界面时是否自动静默拉起伴随服务（若设为 `true`，用户点击插件 Tab 或宿主启动恢复启用插件时会自动拉起） |
| `auto_stop` | `boolean` | `true` | 宿主退出或关闭插件选项卡时，是否自动级联终止该服务进程，彻底防止残留孤儿进程 |
| `description` | `string` | 可选 | 服务的简短中文/英文描述，在顶栏服务状态面板悬停时展示 |

> 💡 **宿主启动自愈保障 (`bootstrap_enabled_plugins`)**：
> 针对常驻型工具或后台代理（如 `auto_start: true`），OpenRevo 在微内核启动阶段会自动扫描已启用的插件，直接在后台静默就绪伴随服务、快捷键与看门狗，无需用户每次手动打开界面触发。

> 💡 **Tab 插件一键弹出独立视窗 (Detachable Pop-out Tab)**：
> 无论插件被定义为 `widget` 还是 `tab`，宿主均在主界面选项卡右上角原生提供**【独立视窗运行】**按钮。用户既能在控制中心中内嵌使用，也能随时一键弹出为独立多屏调优窗口，零代码改动即可双模共存！

---

## 相关页面

- 上一页：[核心安全军规与沙盒边界](security.md)
- 下一页：[硬件 IPC 控制能力速查](capabilities.md)
- 返回目录：[插件与扩展导览](README.md)
