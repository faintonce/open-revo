# OpenRevo Core Native SDK (openrevo-ffi)

> 相关文档：[Core SDK 接入指南](../../../docs/core/sdk-integration.md) · [核心引擎与 SDK 导览](../../../docs/core/README.md)
>
> 本 README 保持 crate 级快速上手为唯一权威，指南侧重生态接入与跨语言约定。

`openrevo-ffi` 是 OpenRevo 官方提供的跨语言原生 C-ABI 动态链接库导出层。它将底层的硬件控制算法、机型钥匙库适配、性能模式调度以及状态遥测抽象为极其稳定、防崩（Panic-Proof）的 C 语言函数接口。

无论你是开发 **WinUI 3 (C#)**、**WPF**、**Qt / C++**、**Python**、**Electron / Node.js**，还是基于 Linux 打造控制中心，只需加载该动态库即可获得与官方相同的全套底层硬件调控能力。

---

## 🚀 快速上手 (Quickstart)

### 1. 编译动态库产物

在项目根目录下执行：
```bash
cargo build --release -p openrevo-ffi
```
编译产物位于 `target/release/`：
- **Windows**: `openrevo_core.dll`
- **Linux**: `libopenrevo_core.so`

### 2. 第三方项目集成文件

在 `crates/openrevo-ffi/` 目录中已为你预制了以下接入文件：
- 📄 **C 语言标准头文件**: `include/openrevo.h`
- 💻 **C# / WinUI 3 开箱即用类**: `bindings/csharp/OpenRevoSdk.cs`

---

## 💻 C# / WinUI 3 极速接入 (5 分钟上手)

### 步骤 1：引入 SDK 封装类
将 [`OpenRevoSdk.cs`](bindings/csharp/OpenRevoSdk.cs) 直接复制到你的 WinUI 3 或 WPF 项目中。

### 步骤 2：配置动态库自动拷贝
在你的 `.csproj` 中加入以下配置，确保编译时将 `openrevo_core.dll` 自动复制到输出目录：
```xml
<ItemGroup>
  <None Include="openrevo_core.dll">
    <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
  </None>
</ItemGroup>
```

### 步骤 3：编写业务逻辑

```csharp
using OpenRevo.Sdk;

// 1. 初始化引擎并监听硬件事件（如机身物理 Q-Key 模式切换、风扇强冷按键）
if (OpenRevoSdk.Initialize())
{
    Console.WriteLine($"[OpenRevo] 核心引擎已加载，版本: {OpenRevoSdk.Version}");

    // 订阅异步事件
    OpenRevoSdk.OnEvent += (eventType, jsonPayload) =>
    {
        switch (eventType)
        {
            case "physical-mode-switched":
                Console.WriteLine($"物理模式切换: {jsonPayload}"); // "0", "1", "2"
                break;
            case "fan-boost-changed":
                Console.WriteLine($"风扇强冷开关: {jsonPayload}"); // "true", "false"
                break;
            case "telemetry-tick":
                // 实时硬件传感器推流
                break;
        }
    };
}

// 2. 切换性能模式 (0 = 办公/省电, 1 = 均衡/游戏, 2 = 狂暴/野兽)
bool success = OpenRevoSdk.SetPowerMode(2);

// 3. 一键查询当前笔记本完整的硬件快照 (JSON 格式)
string hardwareJson = OpenRevoSdk.GetHardwareSnapshotJson();
Console.WriteLine(hardwareJson);

// 4. 程序退出时平滑释放
OpenRevoSdk.Shutdown();
```

---

## 🛠 C / C++ 原生接入

包含头文件并直接调用：

```c
#include "openrevo.h"
#include <stdio.h>

void on_hardware_event(const char* event_type, const char* json_payload, void* user_data) {
    printf("[Event] %s: %s\n", event_type, json_payload);
}

int main() {
    // 1. 初始化
    if (openrevo_init() != 0) {
        printf("Failed to init OpenRevo core!\n");
        return -1;
    }

    // 2. 注册异步事件回调
    openrevo_register_event_callback(on_hardware_event, NULL);

    // 3. 设置狂暴模式
    openrevo_set_power_mode(2);

    // 4. 获取硬件快照 JSON 并打印
    char* json = openrevo_get_hardware_snapshot_json();
    if (json) {
        printf("Hardware: %s\n", json);
        // ⚠️ 黄金守则：使用完毕后必须由 openrevo_free_string 释放！
        openrevo_free_string(json);
    }

    // 5. 退出
    openrevo_shutdown();
    return 0;
}
```

---

## 📖 C-ABI API 函数手册

所有导出函数均内置 `catch_unwind` 异常防护，绝不会因为底层异常击穿 FFI 边界。

| 函数名 | 返回类型 | 说明 | 内存释放要求 |
| :--- | :--- | :--- | :--- |
| `openrevo_init()` | `int32_t` | 初始化全局核心引擎。`0` 成功，`-1` 失败。 | 无 |
| `openrevo_version()` | `const char*` | 获取引擎语义化版本号字符串（如 `"0.9.6"`）。 | **禁止释放**（静态内存） |
| `openrevo_is_active()` | `int32_t` | 检查引擎是否处于运行状态。`1` 激活，`0` 停止。 | 无 |
| `openrevo_shutdown()` | `void` | 优雅停止引擎，向后台线程发送中断信号并清理硬件句柄。 | 无 |
| `openrevo_register_event_callback(cb, data)` | `int32_t` | 注册硬件异步事件与遥测推流回调函数。 | 无 |
| `openrevo_get_hardware_snapshot_json()` | `char*` | 查询包含 CPU、GPU、屏幕、EC 状态的完整快照 JSON。 | **必须使用 `openrevo_free_string` 释放** |
| `openrevo_get_power_presets_json()` | `char*` | 查询当前机型出厂固化的各模式功耗预设 JSON。 | **必须使用 `openrevo_free_string` 释放** |
| `openrevo_set_power_mode(mode)` | `int32_t` | 切换 EC 性能模式 (`0`: 办公, `1`: 均衡, `2`: 狂暴)。 | 无 |
| `openrevo_set_telemetry_active(active)` | `void` | 设置遥测心跳状态 (`1`: 激活轮询, `0`: 暂停轮询)。 | 无 |
| `openrevo_is_telemetry_active()` | `int32_t` | 检查遥测心跳是否处于激活状态。 | 无 |
| `openrevo_free_string(ptr)` | `void` | **核心内存守则**：释放由 Core 动态分配的 JSON 字符串指针。 | - |

---

## 🔄 平滑热替换升级规范 (Drop-in Replacement)

对于第三方前端开发者，OpenRevo Core 承诺以下向后兼容准则：

1. **ABI 零破坏**：在 `0.x` 小版本及后续相同主版本中，已导出的 C 函数签名保持不变。
2. **机型字典透明更新**：后续版本新增笔记本机型适配或更新功耗曲线时，第三方前端开发者**不需要重新编译代码**，只需将最新的 `openrevo_core.dll` 直接覆盖到软件运行目录，即可无缝支持新笔记本与新硬件！

