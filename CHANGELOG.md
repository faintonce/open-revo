# 更新日志 (Changelog)

本文档记录 OpenRevo 项目的所有重大更新、功能特性、问题修复与硬件兼容性演进。
版本格式严格遵循 [语义化版本 2.0.0 (Semantic Versioning)](https://semver.org/lang/zh-CN/)。

---

## [0.9.4] - 2026-10-29

### 🔋 电池限充架构跃迁与插件化 (Decoupled Charge Limit Architecture)
- **官方演示插件升级 (`battery-hardware-limit` v2.0.0)**：
  - 功能彻底由内核抽离为独立的 Tab 类型插件，最低宿主版本锁定要求 `0.9.4`。
  - **支持 1% ~ 100% 任意数值无级调节**：打破官方固化模式限制，支持 50% 极限防鼓包至 90% 准满电等多档位自由调节与数字微调。
  - **基于同方固件“脉冲触发存储法”**：通过 2.5 秒脉冲触发固件自动同步数值至 EC 电池供电存储区（`0x087F`），用完即还，关机、拔电源、重启均持久保持，无需任何常驻后台进程。
  - **彻底解决键盘映射错乱问题**：从内核移除每 2 秒死循环强写 `ADDR_PLATFORM_REAL_ID = 0x04` 的暴力 Watchdog，彻底消除游戏本机型（蛟龙、旷世等）因身份篡改导致的键盘错码与 Fn 键失常。
  - **集成多协议硬件实验室**：前台全开放总线协议切换（TongfangDirectEc / GenericAcpiWmi / BldWmiMailbox）与通道自检自查能力，支持一键导出机型参数清单。
- **主界面视觉净化**：
  - 剔除电源管理中长效模式、平衡模式、工作站模式卡片标题后打补丁拼接的 `(100%)`、`(80%)`、`(60%)` 绿字标签，还原纯净原厂 BIOS 模式语义。
  - 增强 `PluginTabHost` 全局通信桥接能力，全面贯通 `window.parent.__OPENREVO_BRIDGE__` 与 RPC 回调 `reqId` 派发。

---

## [Unreleased] - 待发布

### 🛡️ 13/14代 CPU 寿元卫士官方插件 (CPU Guardian Plugin)
- **专为 13900HX / 14900HX 打造的防缩缸调谐中心 (`cpu-guardian`)**：
  - **合规依附 Intel 官方 7.14 内核通道**：通过极简伴生桥接 `revo-xtu-bridge` 与官方 `XTU3SERVICE` 建立 RPC 通信，严禁任何可能引发 `0x50` 蓝屏的第三方脱机裸起，100% 保留微软 WHQL 认证设备树安全性。
  - **大核睿频削峰与稳态降压**：提供预先验证的“稳态长寿档（5.3G 削峰 + 稳降 60mV + 280A IccMax）”与“轻度防浪涌档（5.5G + 稳降 40mV）”，遍历所有 Active Performance Cores 实施单调性削峰，斩断 1.50V+ 死亡瞬变电涌。
  - **OpenRevo 沙箱持久化与防蓝屏看门狗**：接入 `plugin_storage` 沙箱存储引擎独立持久化配置，档位激活自动防呆置灰，内置 30 秒不稳定自动安全回滚机制。
  - **极简瘦身与纯净环境指引**：彻底清退旧版无用驱动散包（体积暴降至 ~1MB），前台智能检测 XTU 底座就绪状态，配备官方 7.x 避坑下载指引并支持无需常驻 XTU 客户端的纯净运行模式。

### 🎮 游戏 OSD 原生化 (Phase 2: Native Slint OSD Integration)
- **Multi-call 单二进制伴生架构 (Zero Secondary Exe Distribution)**：
  - 采用业界成熟的 Multi-call 架构（类似 busybox/git），主程序 `open-revo.exe` 启动时自调用 `--osd-companion` 拉起专属 Slint 独立进程，**对外分发严格保持单个 `open-revo.exe` 便携文件**。
  - **匿名管道 (Anonymous Stdin Pipe) 0ms 零权限损耗通信**：彻底规避 Windows 命名管道 UAC 权限隔离 (`ACCESS_DENIED`)，父进程直接管道句柄直写，子进程从标准输入监听 JSON，父进程退出即刻优雅自杀，零孤儿进程残留。
  - **精准原生 HWND 捕获与硬件热键/开关全链路连通**：
    - 精确过滤 winit 0x0 内部辅助消息窗口，原生 DWM 透明置顶穿透与无焦点展示。
    - 进程初始化即刻同步配置清单，无须进设置即可自动按保存的位置与主题呈现。
    - 全面接驳物理 Q 键模式切换、Fn+1 强冷开关、Win 键锁、Fn 键锁以及触摸板开关，所有硬件动作 0ms 实时唤起 Slint 原生浮窗。
  - **全量键盘锁定键与外设开关 OSD 深度对齐**：
    - 彻底根除底层低级键盘钩子（`WH_KEYBOARD_LL`）中因异步 35ms 线程调用 `GetKeyState` 导致的线程局部性死锁与状态覆盖故障。
    - 改用硬件前态瞬时取反结合 180ms 时间戳防抖，消除 `WM_KEYUP` 丢失死锁风险，确保 Caps Lock、Num Lock、Scroll Lock 响应 100% 敏捷稳定。
    - 完整支持 Caps Lock、Num Lock、Scroll Lock、摄像头硬件遮蔽、麦克风静音、Wi-Fi、蓝牙、始终唤醒、键盘背光亮度及键盘自动休眠超时的全量原生 Slint 动画通知。


### 🏗️ 架构重构 (Phase 1: Architecture & Core Decoupling)
- **独立硬件调度微内核 (`openrevo-core`)**：
  - 彻底剥离 Tauri 及任何 GUI / Web 运行时依赖，定位为纯粹、高内聚、无头化的底层硬件控制中枢。
  - 支持编译为无依赖纯净 Rust `lib` 及标准 C-ABI `cdylib` 动态链接库，为第三方生态、CLI 命令行以及原生 Shell 提供标准化硬件底座。
- **跨平台通用硬件事件总线 (`CoreEventSink`)**：
  - 抽象平台无关的非阻塞异步事件派发机制，集中纳管物理模式切换按键（Q 键）、风扇强冷开关（Fn+1）、OSD 动态悬浮唤醒、按键锁状态变更及功耗计划同步。
  - 宿主端无缝实现 `TauriEventSink` 事件桥接，实现毫秒级事件透传与 100% 向后兼容。

### ⚡ 优化与下沉 (Changed & Refactored)
- **核心硬件资产与调度系统全量下沉至 `openrevo-core`**：
  - **HAL 抽象层**：多品牌机型自适应探测器（`platform_probe`）、统一驱动接口（`driver_impl` / `traits`）。
  - **设备驱动栈**：EC 读写交互桥、GPU NVAPI 桥接、ITE 键盘/灯带/神光同步驱动、BLE 蓝牙水冷机调控、Mulong/IP3/BLD 专有驱动管理器。
  - **系统级服务**：PPM 电源计划调度器、原厂服务互斥接管引擎（`service_mgr`）、Windows 自启动与后台任务管理、屏幕分辨率与高刷色彩配置。
  - **硬件状态开关矩阵**：独显直连/混合输出切换、触控板锁定、Fn 键逻辑反转、Logo 氛围灯、Win 键锁等硬件状态机全量解耦移入核心。
- **二进制体积极致精简与性能飞跃**：
  - 斩断 Tauri 过程宏在硬件数据结构上的单态化膨胀与冗余序列化蹦床，使 LLVM 跨 Crate 死代码消除（DCE）充分生效。
  - 宿主层 15 个 Tauri IPC 命令层保持完全兼容。

### 🛡️ 硬件与安全 (Security & Safety)
- **钥匙库安全隔离规范升级**：
  - 微内核启用独立的编译期代码生成器 `build.rs`，动态自愈加载 `config/ec_registry.json` 混淆常量字典，彻底封死裸 EC 物理地址硬编码风险。
  - 静态安全门禁持续在线，全量防御寄存器泄露与越权读写。

---

<!-- 历史版本发布记录将归档于下方 -->
