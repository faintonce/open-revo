> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 12: 声明式自动化监听与规则看门狗 (Universal Watcher & Trigger Hub —— 彻底根除隐藏桌宠偷跑轮询)

> **痛点背景**：在早期，部分极客开发者为了实现“监控 CPU 结温达到 55℃ 时自动刷新 EC 状态字”或“启动游戏时自动切狂暴模式”，在插件里声明一个 `widget`（桌面悬浮挂件），将其设为全透明隐藏并使用 `setInterval` 进行 100ms 死循环轮询。\
> **这种做法存在致命缺陷**：
> 1. **白白浪费内存**：一个 WebView2 实例增量占用约 15MB~25MB 内存；
> 2. **后台节流失效**：Windows 与 Chromium 会自动将不可见窗口的定时器强行节流冻结，导致切模逻辑经常失效或严重滞后；
> 3. **唤醒独显功耗爆炸**：挂件前端轮询不当会强制唤醒本处于 D3Cold 深睡的 NVIDIA 独立显卡。
>
> **官方终极解法**：借助 OpenRevo 微内核内建的 **Universal Watcher & Trigger Hub**，实现纯声明式规则监控。**0 MB 内存、0 额外视窗、0 代码死循环，由宿主微内核在无锁遥测心跳与系统事件中统一调度！**

---

## 1. 架构总览：两级配置加载与沙盒优先级体系

在 OpenRevo 0.9.8+ 体系中，微内核彻底解耦了对任何特定业务插件的硬编码水合（Hydration），升级为**完全通用、沙盒隔离的声明式原子工作流**。

看门狗加载严格遵循 **两级沙盒优先级（Sandbox Precedence）** 机制：

```text
               宿主冷启动 / 插件热启用
                          │
                          ▼
    是否存在沙盒配置文件 plugin_storage/<id>/watchers.json？
                 ├── 是 ──► [用户优先] 加载沙盒自定义规则（若内容为 [] 则表示用户主动关闭守护）
                 └── 否 ──► [出厂后备] 读取插件根目录 manifest.json 中声明的 watchers
                          │
                          ▼
             注入微内核 WatcherHub 调度引擎
        (开机引导、电源事件、遥测心跳、周期性保持)
```

这意味着：
- **无头插件（Headless Service）**：只需写好 `manifest.json`，宿主开机直接读取生效，永不启动任何 Web 视窗；
- **富前端插件（Interactive UI）**：用户在界面调整参数后，通过 SDK 动态提交看门狗，宿主自动持久化到该插件的 `plugin_storage` 沙盒并即刻热重载，开机自启动时优先恢复用户的最新配置。

---

## 2. 模式 A：纯无头插件 (Headless Watcher Service)

如果开发者对自己的机型底层寄存器或触发逻辑非常熟悉，根本不需要任何前端 UI 调节界面，**根目录下只需一个 `manifest.json` 文件**，连 HTML、CSS、JS 都完全不需要写：

```text
%APPDATA%\OpenRevo\plugins\mode4-fan-unblocker\
└── manifest.json
```

### 清单配置范例 (`manifest.json`)
```json
{
  "id": "mode4-fan-unblocker",
  "name": "自定义模式55度解锁补丁",
  "version": "1.0.0",
  "author": "OpenRevo Community",
  "description": "监控结温并在进入自定义模式且>=55℃时自动执行安全切模与EC状态字刷新。",
  "plugin_type": "service",
  "min_host_version": "0.9.8",
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
    },
    {
      "id": "battery_limit_pulse_guard",
      "name": "开机与插电硬件脉冲触发电池限充",
      "enabled": true,
      "cooldown_ms": 1000,
      "when": {
        "type": "any",
        "conditions": [
          { "type": "startup" },
          { "type": "power_event", "event": "ac_connected" }
        ]
      },
      "actions": [
        {
          "action": "pulse_write",
          "address": "0x07B4",
          "active": "0x02",
          "restore": "0x00",
          "delay_ms": 150
        },
        { "action": "notify_osd", "title": "电池健康守护", "message": "开机/电源接入：硬件限充脉冲触发成功" }
      ]
    },
    {
      "id": "ec_gate_interval_keepalive",
      "name": "底层硬件门控周期性保持心跳",
      "enabled": true,
      "cooldown_ms": 0,
      "when": {
        "type": "interval",
        "every_ms": 4000
      },
      "actions": [
        { "action": "raw_ec_write", "address": "0x07C3", "value": 4 }
      ]
    }
  ]
}
```

---

## 3. 模式 B：富前端交互 + 动态守护同步 (Interactive UI + Dynamic Watcher)

对于需要可视化调整曲线、设置目标数值（如电池限充百分比、显卡降压 V/F 目标频率）的插件，**前端不仅负责渲染交互，还充当看门狗规则的“编译器”**。

### 工作流流程
1. **用户交互**：用户在 Webview 控制面板中调整滑块或输入框（例如电池限充 80%，或显卡 850mV@2250MHz）；
2. **动态编译并同步**：用户点击“保存/应用”时，前端组装对应的 `watchers` 规则列表，调用宿主 API 下发；
3. **微内核持久化与热生效**：宿主直接写入 `plugin_storage/<plugin_id>/watchers.json` 并立即在当前运行的 `WatcherHub` 中生效；
4. **关闭守护**：用户若在界面选择“关闭守护”，前端下发空数组 `[]`。宿主在沙盒落盘空规则，开机时精准识别为“用户已主动关闭”，绝不会误恢复 `manifest.json` 的出厂预设。

### 前端调用代码范例 (`index.html`)

```javascript
// 组装业务规则
const watchers = [
  {
    id: 'gpu-undervolt-guard',
    name: '微内核显卡降压自愈守护 (开机与电源接入)',
    plugin_id: 'gpu-curve-tuner',
    enabled: true,
    cooldown_ms: 2000,
    when: {
      type: 'any',
      conditions: [
        { type: 'startup' },
        { type: 'power_event', event: 'ac_connected' }
      ]
    },
    actions: [
      {
        action: 'call_command',
        command: 'set_gpu_undervolt_curve',
        args: {
          targetVoltMv: 850,
          maxBoostMhz: 2250
        }
      }
    ]
  }
];

// 统一提交看门狗 (优先使用 window.OpenRevo SDK，兼容微前端与原生 Tauri)
if (window.OpenRevo && typeof window.OpenRevo.setPluginWatchers === 'function') {
  await window.OpenRevo.setPluginWatchers('gpu-curve-tuner', watchers);
} else if (window.parent && window.parent.__OPENREVO_BRIDGE__ && typeof window.parent.__OPENREVO_BRIDGE__.setPluginWatchers === 'function') {
  await window.parent.__OPENREVO_BRIDGE__.setPluginWatchers('gpu-curve-tuner', watchers);
} else {
  // 原生 Tauri IPC 调用
  await window.__TAURI__.core.invoke('set_plugin_watchers', {
    pluginId: 'gpu-curve-tuner',
    watchers: watchers
  });
}
```

---

## 4. 完整的原子动作集 (Watcher Actions Reference)

微内核原生内置了丰富的通用原子动作，自由组合即可满足各类复杂控制需求：

| 动作类型 (`action`) | 必填参数 | 说明 |
| :--- | :--- | :--- |
| `call_command` (或 `invoke_command`) | `command: string`<br>`args: object` (可选) | **全能命令反射**。直接调用任意 OpenRevo 宿主命令（如 `set_power_mode`, `set_battery_limit`, `set_gpu_undervolt_curve`, `set_gpu_clock_offsets`, `set_fan_boost` 等）。 |
| `pulse_write` | `address: string \| number`<br>`active: string \| number`<br>`restore: string \| number`<br>`delay_ms: number` (可选，默认 150) | **原生硬件脉冲写入**。宿主原子化执行：写有效值 -> 毫秒延时 -> 写恢复值。专为脉冲型限电、EC 中断触发设计，完全免除前端 `setTimeout` 延迟风险。 |
| `raw_ec_write` | `address: string \| number`<br>`value: string \| number` | 写入单个 EC 字节。支持十进制与十六进制（`"0x07C3"`）。 |
| `raw_ec_modify_mask` | `address: string \| number`<br>`mask: string \| number`<br>`value: boolean` | 掩码位置位或清零。`value: true` 置位，`value: false` 清零。 |
| `set_power_mode` | `mode: number` (1~4) | 切换性能模式（1: 安静, 2: 均衡, 3: 狂暴, 4: 自定义）。 |
| `delay_ms` | `duration: number` | 微内核级原生挂起延时（毫秒）。用于多步时序操作之间的间隔。 |
| `notify_osd` | `title: string`<br>`message: string` | 弹出系统右上角轻量 OSD 提示。 |

---

## 5. 触发条件集 (Watcher Conditions Reference)

| 触发器类型 (`when.type`) | 参数配置 | 触发时机 |
| :--- | :--- | :--- |
| `startup` | *(无)* | 宿主冷启动引导初始化完成时触发一次。 |
| `power_event` | `event: "ac_connected" \| "battery"` | 电源状态改变（适配器插入接入 AC 或拔下切电池）时触发。 |
| `interval` | `every_ms: number` (>= 500) | 微内核无锁后台定时器周期性执行。适用于需要持续防篡改的心跳门控。 |
| `telemetry` | `power_mode`, `cpu_temp_gte`, `gpu_temp_gte`, `cpu_temp_lte`, `gpu_temp_lte` | 遥测心跳监控。当机身温度或模式满足复合逻辑条件时触发。 |
| `foreground_process` | `targets: string[]`<br>`trigger_on: "enter" \| "exit"` | 前台应用感知。当特定进程（如游戏、渲染器）获得焦点或退出时触发。 |
| `any` | `conditions: Condition[]` | 逻辑“或”。数组中任意一个子条件达成即触发。常用于 `[startup, ac_connected]` 开机与电源联动自愈。 |
| `all` | `conditions: Condition[]` | 逻辑“与”。所有子条件必须全部满足才触发。 |

---

## 6. 核心优势与最佳实践总结

1. **绝对 0 MB 资源占用**：
   无论无头还是有头插件，守护规则一旦注入宿主，**无需常驻任何 WebView2 视窗或隐藏进程**，后台由 Rust 原生极速轮询调度，不漏触发且功耗几乎为零；
2. **防独显唤醒 (D3Cold Safe)**：
   微内核原生感应 NVIDIA GPU 电源状态。对显卡相关的守护规则，微内核优先在显卡本身就绪或交流电源连通时调度，严禁无故盲目唤醒处于深度休眠的独显；
3. **两级配置与出厂兜底机制**：
   插件开发者在 `manifest.json` 中定义默认出厂规则，用户安装后即刻可用；当用户通过界面自定义后，配置落盘至隔离的 `plugin_storage` 沙盒，既不污染插件原始分发包，又保障用户配置永不丢失；
4. **生命周期全闭环**：
   在宿主插件管理中禁用插件时，微内核会同步自动卸载该插件名下的所有活跃 Watcher，保证系统纯净无残留。

---

## 相关页面

- 上一页：[用例 11：声明式零代码全局快捷键与按键宏映射插件 (Universal Shortcut Bus)](11-universal-shortcuts.md)
- 下一页：[用例 13：正交混搭架构实战 —— WebView2 富交互大控制台与 Slint 原生极速常驻桌面挂件 (Hybrid Architecture)](13-hybrid-architecture.md)
- 返回用例索引：[经典实战用例集](README.md)

