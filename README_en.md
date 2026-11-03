# OpenRevo (Open Control Center & Microkernel Hardware Hub for Mechrevo / TongFang Laptops)

<div align="center">

[简体中文](README.md) | English

</div>

<div align="center">

![OpenRevo Banner](./docs/logo_s.jpg)

[![Version](https://img.shields.io/badge/Release-v0.9.3-6366f1.svg?style=flat-square)](https://github.com/faintonce/open-revo/releases)
[![License](https://img.shields.io/badge/License-GPL--3.0-10b981.svg?style=flat-square)](LICENSE)
[![Framework](https://img.shields.io/badge/Framework-Tauri%20v2%20+%20Rust-f59e0b.svg?style=flat-square)](https://v2.tauri.app/)
[![Frontend](https://img.shields.io/badge/Frontend-React%2018%20+%20TypeScript-38bdf8.svg?style=flat-square)](https://react.dev/)
[![Free & Open Source](https://img.shields.io/badge/Open%20Source-100%25%20Free-rose.svg?style=flat-square)](#-strict-notice-free-and-open-source--resale-prohibited)

**A modern, lightweight, microkernelized full-stack open-source control center for Mechrevo (MECHREVO / TongFang) gaming laptops**  
Evolved from a lean replacement for bloated, laggy, memory-leaking official background services into an **extensible hardware microkernel platform** for the entire TongFang chassis ecosystem.  
Portable single-binary (~2MB / ~15MB RAM), zero official dependencies, open hardware driver gateways, and native automation event hubs.

</div>

---

### 🛡️ Strict Notice: Free and Open Source · Resale Prohibited

> **OpenRevo is a 100% free and permanently open-source community project. Anyone may download, use, and compile it free of charge at any time.**  
> **Individuals and resellers are strictly prohibited from repackaging, rebranding, reselling, charging for, redistributing, or bundling this software on Xianyu, Taobao, Pinduoduo, Douyin, private groups, or any other platform.**  
> **If you paid to obtain this software, you may have been defrauded by a malicious seller. Please request a refund through the transaction platform and report the infringing seller.**

---

## 🖥️ Interface Preview

### Dashboard Overview
<img src="docs/screenshots/dashboard.jpg" width="600" alt="Dashboard overview" />

### MiniDrawer (Quick Tray Drawer)
<img src="docs/screenshots/miniDraw.jpg" width="200" alt="MiniDrawer" />

### Lighting Studio (BetterRGB High-FPS Engine)
<img src="docs/screenshots/better%20RGB.jpg" width="600" alt="Lighting Studio" />

### Cooling & Power Tuning
<img src="docs/screenshots/coolingSys.jpg" width="600" alt="Cooling System" />

---

## 💡 Why OpenRevo? (From Control Center to Microkernel Platform)

* ⚡ **Ultra-Lightweight & Agile**: Powered by **Native Rust + Tauri v2 + Webview2**, single portable executable is only **~2MB**, idle memory footprint down to **~15MB**, completely eliminating bloated background services.
* 🧠 **Universal Watcher & Trigger Hub (Microkernel Automation)**: Built-in native monitoring channels: hardware telemetry thresholds, foreground app/game detection, AC/DC & lid power edge streams, and controlled EC delta tracking. Supports **Declarative Manifest Rules** without writing frontend JavaScript.
* 🛡️ **Raw Hardware Driver Gateway & Blast Shield**: Opens 4 low-level chassis driver protocols: `TongfangDirectEc`, `BldWmiMailbox`, `GenericAcpiWmi`, and `IteLightingBus`. Built-in **Hardware Blast Shield** asserts memory boundaries and blacklists critical registers for physical safety.
* ⌨️ **Low-Level Global Keyboard Shortcut Bus (`WH_KEYBOARD_LL`)**: High-performance non-blocking key state tracking, modifier key debouncing, full-screen gaming passthrough, and native lightweight OSD overlay.
* 🌈 **Dual Lighting Engines**: High-framerate **BetterRGB spatial streaming (15~60 FPS)** + **Factory hardware firmware (0% CPU)** with seamless handover upon power change or app exit.
* 🧊 **Smart BLE Liquid Cooler Hub**: Native Bluetooth Low Energy communication with zero pairing hassle; custom fan/pump curves and safety alarms.
* 🎛️ **Instant MiniDrawer**: Click tray icon to access performance profiles, fan boost, and display refresh rate within milliseconds.
* 🤝 **Open Hardware Profiles (`config/ec_registry.json`)**: Fully decoupled hardware addresses. Community PRs for new motherboard models are warmly welcomed!

---

## 🏗️ Architecture (Microkernel Architecture)

```
┌────────────────────────────────────────────────────────────────────────┐
│                   Plugin Ecosystem & AI Agents & UI Layer              │
│   - React 18 Modern Dark Console (Lucide Icons, Dual Themes)           │
│   - MiniDrawer / Real-time OSD Overlay                                 │
│   - Third-Party Sandboxed Plugins / Companion Services / Manifest Rules│
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Standard Async IPC / Sandbox Bridge
┌───────────────────────────────────▼────────────────────────────────────┐
│                    OpenRevo Microkernel Hub                            │
│  ┌───────────────────────────────┐   ┌───────────────────────────────┐ │
│  │  Universal Watcher Hub        │   │  Global Shortcut Bus          │ │
│  │  Telemetry / Apps / Power     │   │  WH_KEYBOARD_LL Passthrough   │ │
│  └───────────────────────────────┘   └───────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │         Raw Hardware Driver Gateway                               │ │
│  │   TongfangDirectEc │ BldWmiMailbox │ GenericAcpiWmi │ IteBus      │ │
│  └───────────────────────────────┬───────────────────────────────────┘ │
│                                  │ Filtered by
│  ┌───────────────────────────────▼───────────────────────────────────┐ │
│  │             Hardware Blast Shield                                 │ │
│  │   Boundary assertion (0x000..0x0FFF) + Critical register blacklist│ │
│  └───────────────────────────────┬───────────────────────────────────┘ │
└───────────────────────────────────┼────────────────────────────────────┘
                                    │ Dispatched to
┌───────────────────────────────────▼────────────────────────────────────┐
│            Hardware Abstraction Layer (HAL & Profiles)                 │
│  - Compile-time XOR Obfuscated Vault Core                              │
│  - Open Hardware Registry: src-tauri/config/ec_registry.json           │
│  - DriverHAL (Real Hardware) / MockHAL (Hardware-free Emulation)       │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 🤝 Model Adaptation & Community PR Guide

OpenRevo aims to be the universal hardware foundation for all Mechrevo & TongFang laptops.  
All EC register offsets are defined in [`src-tauri/config/ec_registry.json`](src-tauri/config/ec_registry.json).

If you have an unadapted model (Jiaolong, Kuangshi, Yaoshi, Jiguang, Wujie, Yilong, etc.):
1. Refer to the standardized schema in `src-tauri/config/ec_registry.json`;
2. Dump/test your model's EC offsets (PL limits, fan RPM, battery thresholds, RGB);
3. **Submit a Pull Request!** Let's power up all Mechrevo models together!

---

## 🚀 Quick Start

### 📥 Option 1: Direct Download (Recommended)
Download the standalone executable from the [Releases Page](https://github.com/faintonce/open-revo/releases):
1. Download `OpenRevo.exe` (~2MB);
2. Run as Administrator;
3. Taking over the official control center is recommended to prevent conflicts.

---

### 💻 Option 2: Build from Source

#### Prerequisites
- **Node.js** >= 18
- **pnpm** >= 9
- **Rust Toolchain** (stable-x86_64-pc-windows-msvc)
- **Visual Studio C++ Build Tools** (with Windows SDK)

```bash
# 1. Clone repository
git clone https://github.com/faintonce/open-revo.git
cd open-revo

# 2. Install dependencies
pnpm install

# 3. Start development
pnpm tauri dev

# 4. Build release bundle
pnpm tauri build
```

---

## 📜 License

This project is licensed under the **[GNU General Public License v3.0 (GPLv3)](LICENSE)**.
You are free to run, inspect, and modify the code. Any modified versions or derivative works distributed must remain 100% free and open-source under GPLv3. Commercial repackaging, closed-source reselling, and proprietary relicensing are strictly prohibited.

<div align="center">

**OpenRevo is committed to delivering pure, responsive, and powerful hardware control!**  
If this project helped you, please give it a ⭐ **Star** on GitHub!

</div>
