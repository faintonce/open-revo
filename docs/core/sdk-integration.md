# OpenRevo 核心引擎 SDK 接入与生态开发指南

> 文档首页：[docs/README.md](../README.md) · 核心引擎与 SDK：[core/README.md](README.md)

**适用版本**：`>= v0.9.6`  
**核心定位**：硬件调度与跨语言中台引擎（Core Orchestration Engine）  
**产物形态**：`openrevo_core.dll` (Windows) / `libopenrevo_core.so` (Linux)

---

## 目录
1. [生态架构与设计理念](#1-生态架构与设计理念)
2. [动态库构建与文件说明](#2-动态库构建与文件说明)
3. [C# / WinUI 3 极速接入指南](#3-c--winui-3-极速接入指南)
4. [C / C++ 原生开发接入指南](#4-c--c-原生开发接入指南)
5. [Linux 移植与跨平台开发说明](#5-linux-移植与跨平台开发说明)
6. [C-ABI 接口参考手册](#6-c-abi-接口参考手册)
7. [平滑热替换升级准则 (Drop-in Replacement)](#7-平滑热替换升级准则-drop-in-replacement)

---

## 1. 生态架构与设计理念

OpenRevo 致力于让机械革命及广发同方/蓝天模具笔记本的硬件控制实现开源与社区化共建。

```
                  ┌───────────────────────────────────────────────┐
                  │                 第三方前端生态                │
                  │   WinUI 3  │   Qt / C++   │  Python  │ Linux  │
                  └───────────────────────┬───────────────────────┘
                                          │ (调用 C-ABI / PInvoke)
                                          ▼
┌─────────────────────────────────────────────────────────────────┐
│              openrevo_core.dll / libopenrevo_core.so             │
│        (导出统一的 C 语言函数, 内置 Panic 屏蔽与内存安全隔离)     │
└─────────────────────────────────┬───────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                        OpenRevoEngine (门面)                    │
│   ├── 性能模式切换 (Office / Balanced / Beast)                   │
│   ├── 机型硬件探测 (CPU / GPU / Displays / Board)                │
│   ├── 出厂功耗预设与风扇曲线                                    │
│   ├── 动态遥测心跳 (CPU/GPU 温度、功耗、风扇转速推流)            │
│   └── 硬件事件总线 (Q-Key 切换通知、风扇强冷广播)                │
└─────────────────────────────────┬───────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                     HAL 底层驱动适配层                           │
│   ├── Windows: WMI ACPI 接口 / Sys 内核驱动 (UWACPI / bld)      │
│   └── Linux:   /sys/kernel/debug/ec/ec_sys (原生读写)            │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. 动态库构建与文件说明

### 编译产物
在 `open-revo` 仓库根目录执行 Cargo 构建：
```powershell
cargo build --release -p openrevo-ffi
```
- Windows 输出：`target/release/openrevo_core.dll`
- Linux 输出：`target/release/libopenrevo_core.so`

### 官方预置接入脚手架
位于 `src-tauri/crates/openrevo-ffi/`：
- `include/openrevo.h`：标准 ANSI C 头文件，适用于 C/C++、Go (cgo)、Python (ctypes)。
- `bindings/csharp/OpenRevoSdk.cs`：预制好的 C# P/Invoke 安全封装类，支持事件委托和自动内存回收。

---

## 3. C# / WinUI 3 极速接入指南

### 步骤 1：集成代码文件
1. 将 `OpenRevoSdk.cs` 复制到你的 WinUI 3 工程源码目录下；
2. 将编译好的 `openrevo_core.dll` 放置在项目根目录，并在 `.csproj` 中标记为输出到运行目录：
```xml
<ItemGroup>
  <None Include="openrevo_core.dll">
    <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
  </None>
</ItemGroup>
```

### 步骤 2：代码调用示例

```csharp
using OpenRevo.Sdk;

// 1. 初始化引擎并监听按键事件
if (OpenRevoSdk.Initialize())
{
    Console.WriteLine($"[OpenRevo] 核心引擎加载成功，版本: {OpenRevoSdk.Version}");

    // 订阅机身按键与遥测事件
    OpenRevoSdk.OnEvent += (eventType, json) =>
    {
        switch (eventType)
        {
            case "physical-mode-switched":
                // 物理按键（Q-Key）切换了性能模式
                int mode = int.Parse(json);
                UpdateModeUi(mode);
                break;

            case "fan-boost-changed":
                // 风扇强冷开关状态
                bool boost = bool.Parse(json);
                UpdateFanBoostUi(boost);
                break;
        }
    };
}

// 2. 切换性能模式 (0: 办公/省电, 1: 均衡/游戏, 2: 狂暴/野兽)
bool ok = OpenRevoSdk.SetPowerMode(2);

// 3. 一键获取完整硬件快照 (JSON 字符串)
string snapshotJson = OpenRevoSdk.GetHardwareSnapshotJson();

// 4. 退出应用时注销
OpenRevoSdk.Shutdown();
```

---

## 4. C / C++ 原生开发接入指南

```c
#include "openrevo.h"
#include <stdio.h>

// 事件回调
void on_event(const char* event_type, const char* json_payload, void* user_data) {
    printf("[CoreEvent] %s => %s\n", event_type, json_payload);
}

int main() {
    if (openrevo_init() != 0) {
        printf("Engine initialization failed!\n");
        return -1;
    }

    openrevo_register_event_callback(on_event, NULL);
    openrevo_set_power_mode(1); // 均衡模式

    // 查询硬件快照
    char* json_str = openrevo_get_hardware_snapshot_json();
    if (json_str) {
        printf("Hardware: %s\n", json_str);
        
        // ⚠️ 极其重要：动态分配的 JSON 字符串必须由 openrevo_free_string 释放
        openrevo_free_string(json_str);
    }

    openrevo_shutdown();
    return 0;
}
```

---

## 5. Linux 移植与跨平台开发说明

针对有意开发 Linux 版本的开发者，OpenRevo Core 已经做了完整的跨平台设计：

1. **不需要 Windows 驱动文件**：你打包的 `.sys` 驱动仅在 Windows 下使用；在 Linux 下，核心硬件交互通过内核自带的 `ec_sys` 模块即可原生完成：
   ```bash
   sudo modprobe ec_sys write_support=1
   ```
2. **读写节点**：直接读取 `/sys/kernel/debug/ec/ec_sys`，所有在 OpenRevo 钥匙库登记的物理地址与风扇协议在 Linux 下均直接有效。
3. **NVML 显卡控制**：Linux 下官方提供 `libnvidia-ml.so`，遥测与功耗控制通过标准 NVML 进行。

---

## 6. C-ABI 接口参考手册

所有 C 接口均内置 `std::panic::catch_unwind` 隔离，确保宿主程序健壮运行。

| 函数名 | 返回类型 | 参数说明 | 内存与生命周期要求 |
| :--- | :--- | :--- | :--- |
| `openrevo_init` | `int32_t` | 无 | 返回 `0` 成功，`-1` 失败 |
| `openrevo_version` | `const char*` | 无 | 返回静态版本串，**严禁释放** |
| `openrevo_is_active` | `int32_t` | 无 | 返回 `1` 正在运行，`0` 停止 |
| `openrevo_shutdown` | `void` | 无 | 优雅停止所有后台监控并释放句柄 |
| `openrevo_register_event_callback` | `int32_t` | `callback`, `user_data` | 注册硬件事件推流函数 |
| `openrevo_get_hardware_snapshot_json` | `char*` | 无 | **必须使用 `openrevo_free_string` 释放** |
| `openrevo_get_power_presets_json` | `char*` | 无 | **必须使用 `openrevo_free_string` 释放** |
| `openrevo_set_power_mode` | `int32_t` | `uint8_t mode` (0/1/2) | 设置模式，成功返回 `0` |
| `openrevo_set_telemetry_active` | `void` | `int32_t active` (0/1) | 控制遥测心跳开启/暂停 |
| `openrevo_is_telemetry_active` | `int32_t` | 无 | 检查遥测心跳状态 |
| `openrevo_free_string` | `void` | `char* ptr` | 释放 Core 内部返回的堆字符串 |

---

## 7. 平滑热替换升级准则 (Drop-in Replacement)

对于第三方开发者，升级 OpenRevo 引擎**极其简单**：
- **只要大版本号一致**，第三方项目**不需要重新编译代码**；
- 只需要在 Release 页面下载最新的 `openrevo_core.dll` 并覆盖旧文件，即可自动生效新笔记本适配、功耗时序修复和性能提升！

