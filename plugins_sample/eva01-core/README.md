# EVA-01 初号机 S² 能量核心悬浮窗 (EVA-01 S² Core Tactical HUD)

> **类型**：桌面异形悬浮挂件 (`plugin_type: "widget"`)  
> **面向对象**：EVA 初号机粉丝、赛博机甲极客、笔记本硬件高频调优玩家与高负载游戏电竞玩家

---

## 插件概述

`eva01-core` 是 OpenRevo 官方的**标杆级异形机甲桌面悬浮窗插件**。它彻底摆脱了传统监控软件单调死板的数据表格，将新世纪福音战士（Evangelion）初号机与 NERV 战术仪表视觉语言，深度融入机械革命笔记本的底层硬件调控：

* **不是简单的静态皮肤**：真正直连 OpenRevo 微内核硬件网关与自适应遥测总线；
* **高频操作秒速直达**：点击核心一键触发“暴走”强冷排热，点击模式卡片一键瞬切性能模式；
* **双态自由折叠**：支持在 **260×190 完整 HUD 看板** 与 **130×42 极简贴边灵动胶囊** 之间丝滑变形；
* **纯代码 Web Audio 合成音效**：零外部音频资产依赖，利用原生 Web Audio API 毫秒级合成 NERV 终端按键音与暴走紧急警报。

---

## 核心特性与硬件映射矩阵

| EVA-01 战术设定 | 笔记本硬件映射 | 视觉动效与操作反馈 |
| :--- | :--- | :--- |
| **S² 能量核心 / 暴走 (Berserk)** | **全速强冷 (Fan Boost 5400+ RPM)** | 点击中央旋转核心，全界面瞬间由初号绿转为**充血猩红**，外环定子超频暴走旋转，伴随冲击波脉冲动效与短促的 EVA 紧急警报蜂鸣。 |
| **结温活动极限 (Thermal Alert)** | **CPU 物理结温三级阶梯** | • `< 68℃`：**NORMAL**（初号机经典荧光绿 `#00ff66`）；<br>• `68 ~ 81℃`：**CAUTION**（警戒琥珀橙 `#f59e0b`）；<br>• `≥ 82℃`：**EMERGENCY**（超温猩红 `#ef4444`）。 |
| **同步率 (Sync Rate)** | **CPU 功耗与综合系统负载** | 依据当前实时释放瓦数（W）与结温，动态换算为 `40% ~ 100%` 的同步率读数。 |
| **内部电源 (Internal Battery)** | **笔记本电池电量与续航预估** | 接通 AC 电源时显示绿光 `EXTERNAL AC`；一旦拔掉电源走电池，立即闪烁红字呈现经典 EVA 倒计时 `INT-BAT 03:45`。 |
| **性能模式矩阵 (Power Mode)** | **办公 / 均衡 / 狂暴 模式循环** | 点击左下角模式卡片，循环下发 `set_power_mode`，并播放经典确认双音阶。 |
| **极简灵动胶囊态 (Capsule)** | **动态视窗几何平滑重塑** | 点击最小化按钮，视窗平滑收缩为 `130×42` 微型胶囊条，仅保留核心结温、转速与极速强冷键；双击胶囊即可恢复完整看板。 |
| **防误触拖拽锁定** | **Win32 窗口位置保护** | 点击右上角 🔓/🔒 图标，可锁定当前桌面坐标，防止在激烈的电竞对战中误触将挂件拖走。 |
| **防独显唤醒 (Zero dGPU Wake)** | **绿色节能续航保护** | 清单内仅声明 CPU、风扇与电源遥测，轻度办公打字时 NVIDIA 独显严格保持在 **D3Cold 0W 深度休眠**。 |

---

## 目录结构

```text
eva01-core/
├── manifest.json              # 视窗几何、遥测传感器列表与能力权限声明
├── index.html                 # 纯纯一体化现代化 HTML5 视窗（内含矢量 SVG、Web Audio 与微内核适配桥）
└── README.md                  # 插件设计与使用说明文档
```

---

## 插件清单 (`manifest.json`)

```json
{
  "id": "eva01-core",
  "name": "EVA-01 初号机 S2 能量核心",
  "version": "2.0.0",
  "author": "OpenRevo Official & Community",
  "description": "官方标杆级异形拟态机甲桌宠挂件，支持结温脉冲呼吸、风扇转速联动、离电倒计时、双态极简折叠与一键强冷暴走切换。",
  "plugin_type": "widget",
  "entry": "index.html",
  "min_host_version": "0.9.3",
  "window": {
    "width": 260,
    "height": 190,
    "transparent": true,
    "always_on_top": true,
    "resizable": false,
    "skip_taskbar": true
  },
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
      "is_charging"
    ]
  },
  "permissions": [
    "telemetry:read",
    "fan:control",
    "power:control"
  ]
}
```

---

## 交互使用指南

1. **拖拽移动**：按住悬浮窗空白区域或顶部标题栏即可平滑拖拽至屏幕任意位置（OpenRevo 会自动持久化记忆最后位置）。
2. **位置锁定**：点击顶栏 🔓 图标变为 🔒，即可锁定拖拽。
3. **静音模式**：点击 🔊 图标切换为 🔇，随时关闭 UI 反馈音效。
4. **一键暴走强冷**：点击中央 S² 旋转核心或胶囊条上的 ⚡ 按钮，瞬间进入全速排热状态。
5. **切换性能模式**：点击左下角 `POWER MODE` 卡片，快速在办公、均衡与狂暴模式间切换。
6. **双态折叠**：点击顶栏 ➖ 按钮收缩为极简胶囊态；双击胶囊或点击 ⛶ 按钮即可恢复全屏 HUD 看板。

