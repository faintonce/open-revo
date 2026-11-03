# OpenRevo (开源机械革命 / 同方笔记本控制中心与微内核硬件基座)

<div align="center">

简体中文 | [English](README_en.md)

</div>

<div align="center">

![OpenRevo Banner](./docs/logo_s.jpg)

[![Version](https://img.shields.io/badge/Release-v0.9.3-6366f1.svg?style=flat-square)](https://github.com/faintonce/open-revo/releases)
[![License](https://img.shields.io/badge/License-GPL--3.0-10b981.svg?style=flat-square)](LICENSE)
[![Framework](https://img.shields.io/badge/Framework-Tauri%20v2%20+%20Rust-f59e0b.svg?style=flat-square)](https://v2.tauri.app/)
[![Frontend](https://img.shields.io/badge/Frontend-React%2018%20+%20TypeScript-38bdf8.svg?style=flat-square)](https://react.dev/)
[![Free & Open Source](https://img.shields.io/badge/Open%20Source-100%25%20Free-rose.svg?style=flat-square)](#-严正声明开源免费--严禁倒卖)

**专为机械革命 (MECHREVO / TongFang) 游戏本打造的现代化、轻量级、微内核化全栈开源控制中枢**  
从替代臃肿卡顿、内存泄漏且保守受限的官方后台服务，全面演进为面向同方全模具生态的**高可扩展硬件微内核底座**。  
单文件绿色运行（~2MB / ~15MB RAM），官方控件零依赖，开放底层硬件网关与自动化事件中枢，带来纯净极速的硬件掌控体验。

</div>

---

### 🛡️ 严正声明：开源免费 · 严禁倒卖

> **OpenRevo 为 100% 永久免费且开源的公益社区项目，任何人在任何时间均可免费下载、使用与编译！**  
> **严禁任何个人、二道贩子在闲鱼、淘宝、拼多多、抖音等任何平台或私域群聊中对本软件进行换皮、打包倒卖、收费分发或捆绑销售！**  
> **若您是通过付费购买获得了本软件，说明您遭遇了不良商家的恶意欺诈，请立即向交易平台申请退款并向平台举报侵权商家！**

---

## 🖥️ 界面预览

以下截图展示了 OpenRevo 的核心交互界面与高频功能入口：

### 主界面总览 (Dashboard)
<img src="docs/screenshots/dashboard.jpg" width="600" alt="Dashboard overview" />

### 极速托盘抽屉 (MiniDrawer)
<img src="docs/screenshots/miniDraw.jpg" width="200" alt="MiniDrawer" />

### BetterRGB 键盘背光调校室 (Lighting Studio)
<img src="docs/screenshots/better%20RGB.jpg" width="600" alt="Lighting Studio" />

### 散热 / 功耗精细调控 (Cooling & Power)
<img src="docs/screenshots/coolingSys.jpg" width="600" alt="Cooling System" />

---

## 💡 为什么选择 OpenRevo？（从控制中心到微内核硬件基座的质变）

* ⚡ **极速轻量，告别臃肿**：采用 **Rust 原生核心 + Tauri v2 + Webview2** 现代化架构，单文件免安装绿色版体积仅约 **2MB**，常驻内存低至约 **15MB**，彻底根除官方控制中心动辄数百兆后台服务常驻卡顿的顽疾。
* 🧠 **微内核事件中枢 (Universal Watcher & Trigger Hub)**：内置原生公共监听与反应中心，支持遥测阈值、前台应用感知、电源插拔边沿、受控 EC 差量 4 大监听通道，支持**清单声明式规则（Manifest Rules）**，零内存零 GUI 优雅托管后台自动化联动。
* 🛡️ **底层模具驱动安全穿透网关 & 硬件防爆盾**：面向极客与 AI 工具开放 `TongfangDirectEc`、`BldWmiMailbox`（宝龙达蛟龙 16 Pro 专用）、`GenericAcpiWmi` 与 `IteLightingBus` 4 大底层协议通道，内置**硬件防爆盾（Hardware Blast Shield）**拦截越界与危险寄存器，兼顾极致扩展与物理绝对安全。
* ⌨️ **低级全局键盘钩子总线 (`WH_KEYBOARD_LL`)**：Win32 高性能非阻塞物理按键感知，复合修饰键防抖，全屏独占游戏事件无感穿透，实时联动轻量级原生 OSD 状态指示。
* 🌈 **独创键盘背光双引擎**：内置 **BetterRGB 空间高刷流光 (15~60 FPS)** 与 **原厂硬件固件 (0% CPU)** 双引擎，支持平滑自适应流光、拔电降级与退出无缝托管，彻底解决 2024/2025/2026 款机革背光卡顿。
* 🧊 **智能外置水冷机中控 (BLE)**：原生低功耗蓝牙拓扑通信，免配对即开即连，支持水泵转速/风扇曲线精细调校与全方位安全遥测。
* 🎛️ **极速托盘抽屉 (MiniDrawer)**：点击系统托盘图标毫秒级呼出微型控制台，模式切换、风扇强冷、屏幕刷新率调节触手可及。
* 🎨 **电竞暗黑美学 & 双工业主题**：全面对标专业调校工具，支持 **极致单色工业风 (`mono`)** 与 **赛博多彩分区风 (`cyber`)** 自由切换。
* 🤝 **开源机型硬件配置文件 (`config/ec_registry.json`)**：硬件地址与底层逻辑全面解耦，欢迎社区机友提交 PR 适配全系同方/机革模具！

---

## ✨ 核心功能全景

### 1. 🧠 微内核自动化中枢与反应总线 (Universal Watcher Hub)
- **告别后台偷跑与挂件轮询**：彻底颠覆传统插件被迫启动透明隐藏 Webview 挂件导致内存暴增与计时器休眠降频的缺陷；
- **声明式自动化规则 (Declarative Manifest Rules)**：支持在插件 manifest 中直接声明 `watchers` 规则数组，无需编写一行前端代码即可实现“进入指定游戏自动切狂暴模式”、“温度降至安全阈值自动回退”等高级时序控制；
- **4 大底层监听评估通道**：
  - `telemetry`：CPU/GPU 结温、功耗模式、风扇实时转速阈值；
  - `foreground_process`：极轻量 Win32 前台游戏与专业软件感知；
  - `power_event`：AC/DC 充电器拔插、开合盖、屏幕睡眠/唤醒状态流；
  - `raw_ec_diff`：受防爆盾保护的物理 EC 状态差量追踪；
- **NVAPI 防独显误唤醒守则**：全链路遵循低功耗休眠原则，无监控需求时独显保持休眠，插件零增量内存占用。

### 2. 🛡️ 底层硬件网关与防爆盾 (Raw Driver Gateway & Blast Shield)
- 面向社区插件开发者与 AI Agent 暴露安全的硬件通信原语：
  - **同方原生 EC (`TongfangDirectEc`)**：同方模具原生寄存器直接安全读写；
  - **宝龙达 WMI 邮箱 (`BldWmiMailbox`)**：蛟龙 16 Pro 等宝龙达特定机型邮箱指令交互通道；
  - **通用 OEM ACPI-WMI (`GenericAcpiWmi`)**：通用 WMI 硬件方法调用；
  - **ITE 单片机灯控总线 (`IteLightingBus`)**：8291/8297 等单片机原生数据包直通；
- **硬件防爆盾 (Hardware Blast Shield)**：在内核交互层布设安全断言矩阵，强制拦截 `0x0000..0x0FFF` 以外的非法越界访问，并内置关键危险寄存器黑名单，物理阻断任意越界写入或恶意破坏。

### 3. 🌈 键盘背光双引擎架构与高级拓扑覆盖
- **极客高刷推流引擎 (BetterRGB 空间投影算法)**：
  - 空间多维几何插值，15 FPS ~ 60 FPS 逐帧平滑动态渲染，提供波浪、正弦呼吸、同心涟漪、丝滑流沙、烈焰翻滚等 14 种绚丽动效；
  - 感谢 [Better RGB](https://github.com/ZavierChen/BetterRGB-ITE8291-Adaptive) 的先驱探索；
- **原厂硬件固件引擎 (0% CPU 模式)**：
  - 由键盘主控芯片硬件定时器独立驱动，零 CPU 占用，极致省电；
- **高级硬件拓扑覆盖 (Lighting Topology Override)**：
  - 支持图形化覆盖各机型灯控区域映射，解决错位或偏色问题；
- **智能交接守护**：拔掉电源平滑降级硬件流光，插电瞬间恢复高刷渲染，软件退出背光绝不灭灯。

### 4. 🎚️ 极客自定义调校与性能模式 (Custom Tuning)
- **四大运行模式**：办公模式 (Office)、均衡模式 (Balance)、狂暴模式 (Turbo/Beast)、极客自定义调优模式 (Custom)；
- **CPU 功耗释放**：精细管控长时间限制 (PL1)、突增功耗 (PL2) 与瞬态电流 (PL4)；
- **GPU 动态增益调控**：深度管控 Dynamic Boost 调度与显卡 TGP Offset 功耗释阶；
- **多阶梯阻尼风扇曲线**：CPU / GPU 独立多点温控曲线，内置阻尼滤波杜绝风扇转速剧烈忽高忽低（风扇震荡）；
- **实体 Q-Key 状态机闭环**：原生状态机闭环修复机身物理按键指示灯偏色卡死。

### 5. 🧊 智能外置水冷机管理 (Water Cooler Management)
- 原生低功耗蓝牙 (BLE) 拓扑通信，开机自愈免配对秒连；
- 实时遥测水泵扬程、水箱水位、风扇转速与水流速率；
- 独创水冷机自定义风扇曲线调校与离线安全报警守护。

### 6. 🔋 供电、电池健康与显卡拓扑
- **三档电池养护阈值**：长效 100%、日常均衡 80%、工作站长寿 60% 闭环养护；
- **显卡拓扑多模控制**：独显直连 (dGPU)、混合输出 (Hybrid) 与纯核显安全管线；
- **MiniLED 控光**：MiniLED 局域控光与系统自愈恢复支持；
- **外设战术开关**：触控板锁定、摄像头硬件防窥、Win 键防误触、通电开机 (AC Recovery)。

---

## 🏗️ 架构设计全景图 (Microkernel Architecture)

```
┌────────────────────────────────────────────────────────────────────────┐
│                   插件生态 & AI Agent & 用户交互层                      │
│   - React 18 现代化暗黑控制台 (Lucide Icons, 双主题系统)                │
│   - 极速托盘抽屉 (MiniDrawer) / 实时 OSD 状态指示                      │
│   - 第三方沙盒插件 / 伴生后台服务 / 声明式规则 (Manifest Rules)        │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ 标准动态异步 IPC / 安全沙箱通信桥
┌───────────────────────────────────▼────────────────────────────────────┐
│                    OpenRevo 微内核中枢 (Microkernel Hub)                │
│  ┌───────────────────────────────┐   ┌───────────────────────────────┐ │
│  │  Universal Watcher Hub (监听) │   │  Global Shortcut Bus (钩子)   │ │
│  │  遥测阈值 / 进程感知 / 电源边沿│   │  WH_KEYBOARD_LL 全局非阻塞调度│ │
│  └───────────────────────────────┘   └───────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │         底层模具驱动网关 (Raw Hardware Driver Gateway)            │ │
│  │   TongfangDirectEc │ BldWmiMailbox │ GenericAcpiWmi │ IteBus      │ │
│  └───────────────────────────────┬───────────────────────────────────┘ │
│                                  │ 经过
│  ┌───────────────────────────────▼───────────────────────────────────┐ │
│  │             硬件防爆盾 (Hardware Blast Shield)                    │ │
│  │   物理寄存器范围严格断言 (0x000..0x0FFF) + 敏感危险控制寄存器黑名单 │ │
│  └───────────────────────────────┬───────────────────────────────────┘ │
└───────────────────────────────────┼────────────────────────────────────┘
                                    │ 调用
┌───────────────────────────────────▼────────────────────────────────────┐
│            硬件抽象层与开源机型配置 (HAL & Hardware Profiles)           │
│  - 编译期异或混淆钥匙库 (Vault Core)                                    │
│  - 开源机型配置文件：src-tauri/config/ec_registry.json                 │
│  - 真实物理驱动 (DriverHAL) / 零设备依赖仿真驱动 (MockHAL)              │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 🤝 社区多机型适配与 PR 共创指南

OpenRevo 致力于打造属于全同方/机革玩家的通用硬件基础设施。  
主板硬件物理地址已完全解耦至 [`src-tauri/config/ec_registry.json`](src-tauri/config/ec_registry.json)。

如果您拥有尚未完全适配的机型（例如蛟龙系列、旷世系列、耀世系列、无界系列、极光系列等各年份模具）：
1. 参考 `src-tauri/config/ec_registry.json` 中标准化的寄存器与掩码结构；
2. 抓取或测试您机型的 EC 偏移（功耗墙、电池阈值、风扇 RPM、RGB 寄存器）；
3. **欢迎向本项目提交 Pull Request！** 让我们一起把所有机革模具点亮！

---

## 🛠️ 技术栈选型

| 层次 | 技术选型 | 优势与设计考量 |
| :--- | :--- | :--- |
| **应用框架** | [Tauri v2](https://v2.tauri.app/) | 超轻量级、原生 Rust 驱动、内存安全、无 Chromium 冗余开销 |
| **后端语言** | [Rust 2021](https://www.rust-lang.org/) | 极致性能、零成本抽象、原生 Windows Shell / Win32 API 直连 |
| **前端技术** | React 18 + TypeScript + Vite | 毫秒级响应速度、严谨组件类型、高自由度现代化交互 |
| **硬件协议** | Direct EC / WMI ACPI / HID / BLE | 深度硬件通信无死角覆盖，全链路防爆盾安全加固 |
| **网络更新** | ureq + Schannel (Windows 原生) | 告别 OpenSSL 臃肿依赖，极致二进制体积瘦身至 2MB，双源智能竞速 |

---

## 🚀 快速上手

### 📥 方式一：直接下载使用（推荐）
前往项目 [Releases 页面](https://github.com/faintonce/open-revo/releases) 下载最新版本单文件免安装绿色版：
1. 下载 `OpenRevo.exe`（仅约 2MB）；
2. 右键以管理员身份运行；
3. 为避免冲突，**接管官方控制中心**是推荐操作；
4. 软件自动最小化至任务栏托盘，随时左键或右键呼出控制。

---

### 💻 方式二：从源码编译（开发者）

#### 环境准备
- **Node.js** >= 18
- **pnpm** >= 9
- **Rust 工具链** (stable-x86_64-pc-windows-msvc)
- **Visual Studio C++ Build Tools** (含 Windows SDK)

#### 1. 克隆代码仓库
```bash
git clone https://github.com/faintonce/open-revo.git
cd open-revo
```

#### 2. 安装前端依赖
```powershell
pnpm install
```

#### 3. 运行开发环境
```powershell
pnpm tauri dev
```

#### 4. 打包构建独立发布版本
```powershell
pnpm tauri build
```

---

## 📜 开源协议

本项目采用 **[GNU General Public License v3.0 (GPLv3)](LICENSE)** 强互惠协议开源。

- **自由使用与修改**：您可以自由运行、研究、修改与分享本项目的源代码；
- **强制保持开源 (Copyleft)**：任何基于 OpenRevo 修改、衍生或集成的分发版本，必须同样以 GPLv3 协议完全免费开源其全部源代码，**严禁闭源换皮、商业倒卖或据为私有**；
- **著作权声明**：分发时必须保留原作者的版权声明、反倒卖声明与 GPLv3 许可文件全文。

---

## ⚠️ 免责声明

1. 本软件为开源社区独立项目，**非机械革命 (MECHREVO) 或同方官方出品**，与上述品牌无任何商业从属关系；
2. 软件由热心开发者在业余时间维护开发，作者不对使用过程中因用户极限超频、非法篡改硬件参数导致的硬件损伤或数据丢失承担连带责任；
3. 调整功耗墙与风扇曲线时，请保持在安全合理的硬件阈值内，爱护爱机。

<div align="center">

**OpenRevo 致力于让每一个机械革命用户享受纯净、流畅、高效的极速掌控体验！**  
如果这个项目帮到了你，欢迎给仓库点个 ⭐ **Star** 支持持续开发！

</div>
