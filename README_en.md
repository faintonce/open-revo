# OpenRevo (Open Control Center for Mechrevo / TongFang Laptops)

<div align="center">

[简体中文](README.md) | English

</div>



> OpenRevo is a community-driven project focused on the Mechrevo / TongFang laptop ecosystem. This repository currently contains the publicly available parts of the project, including an overview, architecture notes, public documentation, and community resources.

> We are actively organizing and preparing the open-source subset for a broader release. Reverse-engineering data for specific devices, vendor-specific mappings, hardware detection details, and other sensitive platform assets are not included in this repository.

> This repository is intended to provide a transparent foundation for the project. More public materials will be added gradually as the open-source boundaries become clearer.

<div align="center">

![OpenRevo Banner](./docs/logo_s.jpg)

[![Version](https://img.shields.io/badge/Release-v0.6.0-6366f1.svg?style=flat-square)](https://github.com/)
[![License](https://img.shields.io/badge/License-MIT-10b981.svg?style=flat-square)](LICENSE)
[![Framework](https://img.shields.io/badge/Framework-Tauri%20v2%20+%20Rust-f59e0b.svg?style=flat-square)](https://v2.tauri.app/)
[![Frontend](https://img.shields.io/badge/Frontend-React%2018%20+%20TypeScript-38bdf8.svg?style=flat-square)](https://react.dev/)
[![Free & Open Source](https://img.shields.io/badge/Open%20Source-100%25%20Free-rose.svg?style=flat-square)](#-open-source-policy-and-strict-no-resale-notice)

**A modern, lightweight, open-source control center built specifically for Mechrevo (MECHREVO / TongFang) gaming laptops, with zero dependency on official control-center components**  
A clean and responsive alternative to bloated, laggy, memory-leaking, and overly conservative official background services. No official control-center components are required, while the feature set keeps expanding for a fast, plug-and-play hardware-control experience.

</div>


---

### 🛡️ Strict Notice: Free and Open Source · Resale Prohibited

> **OpenRevo is a 100% free and permanently open-source community project. Anyone may download and use it free of charge at any time.**  
> **Individuals and resellers are strictly prohibited from repackaging, rebranding, reselling, charging for, redistributing, or bundling this software on Xianyu, Taobao, Pinduoduo, Douyin, private groups, or any other platform.**  
> **If you paid to obtain this software, you may have been defrauded by a malicious seller. Please request a refund through the transaction platform and report the infringing seller.**

---

## Open-Source Roadmap

The project is committed to becoming fully open source without holding anything back. We are currently moving quickly, and the author needs to catch up on sleep after late-night development. Documentation is still being organized, so please wait patiently while more parts are released in stages.

## 🖥️ Interface Preview

The screenshots below show OpenRevo's currently public core interface and feature entry points, providing a quick overview of the project's direction and interaction model.

### Main Dashboard

<img src="docs/screenshots/dashboard.jpg" width="600" alt="Dashboard overview" />

### Mini Drawer

<img src="docs/screenshots/miniDraw.jpg" width="200" alt="Mini Drawer" />

### RGB Lighting Effects

<img src="docs/screenshots/better%20RGB.jpg" width="600" alt="RGB lighting effects" />

### Cooling / Power Control

<img src="docs/screenshots/coolingSys.jpg" width="600" alt="Cooling and power control" />

---

## 💡 Why Choose OpenRevo?

* ⚡ **Fast and lightweight:** Built with a **native Rust core + WebView2** architecture and packaged as a portable single-file application. The installer is under 2 MB and typical resident memory usage is approximately 15 MB, avoiding the hundreds of megabytes often consumed by official control-center services.
* 🌈 **Unique dual-engine keyboard lighting:** Includes **BetterRGB geek high-refresh streaming** and the **factory hardware firmware engine (0% CPU)**, with smooth adaptive effects and seamless handoff on exit to address lighting stutter on 2024/2025/2026 models.
* 🎛️ **Fast tray drawer (MiniDrawer):** Click the taskbar tray icon to instantly open a highly responsive control drawer. Switch modes, enable maximum fan cooling, or adjust the display refresh rate without opening the main window.
* 🎨 **Gaming dark aesthetic and dual industrial themes:** Inspired by professional tuning tools, with one-click switching between the **minimal monochrome industrial style (`mono`)** and the **colorful cyberpunk zone style (`cyber`)**.
* 🧩 **Adaptive multi-chassis support:** Automatically detects the physical model configuration and supports 8 chassis and 24 models, with precise adaptation and graceful fallback for per-key RGB, single-zone RGB, and single-color white backlighting.

---

## ✨ Core Features

### 1. 🌈 Dual-Engine Keyboard Lighting
- **Geek high-refresh streaming engine (BetterRGB spatial projection algorithm):**
  - No need to choose between the two engines; OpenRevo uses both. Special thanks to [Better RGB](https://github.com/ZavierChen/BetterRGB-ITE8291-Adaptive);
  - Spatial multi-dimensional interpolation and geometric projection, supporting dynamic frame-by-frame rendering from 15 FPS to 60 FPS;
  - Provides 14 vivid effects, including **Wave, Sine Breathing, Concentric Ripples, Smooth Quicksand, Electric Current, Rainbow Rotation, Digital Matrix, Lightning, and Rolling Flames**;
- **Factory hardware firmware engine (0% CPU, zero overhead mode):**
  - Driven independently by the keyboard controller's hardware timer, with effectively zero system CPU usage for maximum power efficiency;
- **Smart handoff and black-screen protection:**
  - When the software exits, the current dynamic effect is automatically mapped to the controller's built-in effect and handed off seamlessly, preventing the backlight from suddenly turning off or dimming;
  - When AC power is disconnected, the lighting smoothly falls back to low-power hardware mode and intelligently resumes high-refresh effects when power is restored.
- **Highly customizable, shareable configuration files:**
  - Write custom animations for the keyboard and apply them immediately by importing a configuration file;
  - The configuration format and supporting skill documentation will be released later.

### 2. 🎛️ Fast Tray Drawer (MiniDrawer)
- Resides in the Windows taskbar system tray and opens a miniature control panel instantly when clicked;
- Quick actions include the four performance modes, maximum fan cooling, keyboard backlight mode/color/brightness, display refresh rate (Hz), display brightness, and common hardware toggles.

### 3. 🎚️ Geek Tuning and Performance Modes
- **Four instant operating modes:** Office, Balance, Turbo/Beast, and Geek Custom tuning;
- **Open specifications and three-mode customization (`models.json`):** Hardware facts discovered from the factory are separated from user intent. The power limits and thermal strategies of the three factory modes can be customized through plain-text JSON (see [Model and Three-Mode Power Customization Guide](docs/models_customization_guide.md));
- **Fine-grained CPU power tuning:** Adjust long-duration power limit (PL1), short-duration boost power limit (PL2), and transient peak current (PL4);
- **Dynamic GPU boost control:** Manage the graphics card's Dynamic Boost range in real time;
- **Multi-step fan curves:** Independent multi-point CPU / GPU temperature-response curves with configurable damping sensitivity and smoothing against sudden increases;
- **Physical Q-Key state-machine loop:** A native state machine solves stuck or incorrectly colored physical-key indicators while keeping software and hardware lighting states synchronized in real time.

### 4. 🧊 Smart External Water Cooler Management
- Native Bluetooth Low Energy (BLE) topology communication with connection available without manual pairing;
- Real-time telemetry for the external cooler's pump state and cooling-fan speed;
- Multiple cooler strategies with linked operation and over-temperature protection alerts;
- A cooler control center with a temperature-wall curve is included for good measure.

### 5. 🔋 Power and Battery Health
- **Three battery charge thresholds:** Long-life mode (100%), daily balance (80%), and workstation longevity mode (60%);
- **USB charging while powered off:** Freely enable or disable USB peripheral power while the laptop is shut down;
- **Automatic power-on with AC Recovery:** A hardware-level switch for docks and desktop workstation setups.

### 6. 💻 GPU Modes and Tactical Toggles
- **Multi-GPU topology control:** Safely detect and switch between discrete GPU direct output (dGPU), hybrid output, and integrated-graphics-only modes;
- **Hardware peripheral toggles:** Touchpad lock, hardware camera privacy, Win-key lock, Fn-key lock, and one-click Wi-Fi / Bluetooth control.

### 7. 🔓 Cross-Generation Adaptive UEFI Advanced Menu
- Enable or hide the motherboard's advanced hardware-tuning menu directly from the operating system without entering the BIOS, making advanced tuning more convenient.

---

## 🏗️ Architecture and Layered Protection (HAL Architecture)

OpenRevo follows a modern layered and decoupled architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                 OpenRevo UI and Application Layer           │
│  - React 18 modern interface (Tailwind CSS, Lucide Icons)  │
│  - Dual industrial theme engine (mono / cyber)              │
│  - Fast tray mini drawer (MiniDrawer)                       │
│  - Tuning strategies and smooth curve interpolation         │
│  - Tauri v2 IPC, system power monitoring, and singleton    │
│    process guard                                             │
└──────────────────────────┬──────────────────────────────────┘
                           │ Unified hardware abstraction contract
┌──────────────────────────▼──────────────────────────────────┐
│             Hardware Abstraction Layer (MechrevoHardwareHAL)│
│                                                             │
│   ┌─────────────────────────┐   ┌─────────────────────────┐ │
│   │   MockHAL (simulator)    │   │   DriverHAL (hardware)   │ │
│   │   Clone and run the code│   │   Hardware safety-bound  │ │
│   │   without a physical     │   │   checks and real-device │ │
│   │   Mechrevo laptop        │   │   communication path    │ │
│   └─────────────────────────┘   └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

* **Community friendly:** The repository includes a general-purpose simulated driver (`MockHAL`) by default, so developers can clone, compile, and explore the complete interface without a specific Mechrevo laptop;
* **Safety first:** The physical driver layer includes strict hardware safety-boundary protections to prevent operations beyond the motherboard's electrical limits.

---

## 🛠️ Technology Stack

| Area | Technology | Benefits |
| :--- | :--- | :--- |
| **Desktop framework** | [Tauri v2](https://v2.tauri.app/) | Lightweight, low memory usage, secure sandbox |
| **Backend language** | [Rust](https://www.rust-lang.org/) | Memory safety, zero-cost abstractions, direct native Windows API access |
| **Frontend** | React 18 + TypeScript + Vite | Modern component architecture, fast response, strong typing |
| **Bluetooth communication** | btleplug (Windows BLE API) | Native low-power Bluetooth support with immediate connection |
| **UI design** | Lucide React + Modern CSS | Focused dark gaming aesthetic with dual-theme support |

---

## 📖 Documentation and Guides

* 📄 [**Model and Three-Mode Power Customization Guide (`models.json`)](docs/models_customization_guide.md):** Explains the model configuration architecture, the two-layer specification system (`factory_specs` and runtime `presets`), power and thermal wall tuning for the three modes, practical tuning examples, and common pitfalls.

---

## 🚀 Getting Started

### 📥 Option 1: Download and Use Directly (Recommended for Most Users)

Go to the project's [Releases](https://github.com/faintonce/open-revo) page and download the latest portable single-file release:
1. Download `OpenRevo.exe`;
2. Double-click to run it;
3. To avoid conflicts, **taking over from the official control center** is required; otherwise, the application will only run in MOCK demonstration mode;
4. After startup, the application minimizes to the taskbar tray automatically. Right- or left-click the tray icon to use it immediately.

---

### 💻 Option 2: Build from Source (Developers)

#### Prerequisites
- **Node.js** >= 18
- **pnpm** >= 9
- **Rust toolchain** (`stable-x86_64-pc-windows-msvc`)
- **Visual Studio C++ Build Tools** (including the Windows SDK)

#### 1. Clone the Repository
```bash
git clone https://github.com/your-username/OpenRevo.git
cd OpenRevo
```

#### 2. Install Frontend Dependencies
```powershell
pnpm install
```

#### 3. Run in Development
```powershell
# Run safety checks and static syntax validation
.\safe_check.ps1

# Start development mode (automatically runs with the Mock simulator)
pnpm tauri dev
```

#### 4. Build a Release
```powershell
# Build an optimized standalone Release version with the bundled script
.\safe_build.ps1
```

---

## 📜 License

This project is released under the **[MIT License](LICENSE)**.

- You may freely run, study, modify, and share the source code;
- You may use, copy, and modify the project in any project;
- You only need to retain the copyright and license notices when distributing it;
- The project encourages open collaboration and shared learning for community maintenance and further development.

---

## ⚠️ Disclaimer

1. This is an independent open-source community project and is **not produced by or commercially affiliated with Mechrevo (MECHREVO) or TongFang**;
2. The software is maintained and developed by volunteers in their spare time. The authors are not liable for hardware damage or data loss caused by extreme overclocking or illegal hardware-parameter modifications;
3. When adjusting voltage, power limits, or fan speeds, stay within the manufacturer's recommended safety thresholds and use the hardware responsibly.

<div align="center">

**OpenRevo is dedicated to giving every Mechrevo user a clean, smooth, and efficient performance-control experience!**  
If this project helps you, please give the repository a ⭐ **Star** to support continued development!

</div>
