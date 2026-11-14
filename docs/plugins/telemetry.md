> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 6. 自适应遥测与动态心跳总线 (Telemetry Bus & Consumer Lifecycle)

## 1. 核心铁律：防独显唤醒 (Zero dGPU Wake-lock)
双显卡笔记本在轻度办公时，NVIDIA 独立显卡处于 **D3Cold 极深零功耗休眠 (0W)**。
* **严禁无脑索取 GPU 字段**：若挂件仅需要 CPU 结温和风扇转速，`manifest.json` 中**坚决不要声明 `gpu_*` 字段**！
* **内核保障**：若所有运行中的插件均未订阅 GPU，微内核**绝不调用 NVAPI**，让独显继续保持零功耗停机。

## 2. 按需消费生命周期驱动 (Consumer-Driven Lifecycle)
OpenRevo 坚决摒弃“宿主无论何时都以最高频疯狂轮询”的设计。微内核内建了**动态消费者记账系统**：
* **宿主全自动托管**：
  * 当用户点击进入插件 Tab、打开桌面悬浮挂件 (Widget) 或弹出独立视窗时，宿主自动读取 `manifest.telemetry` 并向内核注册消费者 (`register_telemetry_consumer`)；
  * 当用户切走该 Tab、最小化到托盘或关闭挂件窗口时，宿主自动注销消费者 (`unregister_telemetry_consumer`)；
* **零消费自动停跳休眠**：
  * 当全系统活跃的遥测消费者数量归零时（例如主界面最小化至托盘且无任何桌面挂件），遥测工作线程自动暂停休眠，CPU 维持极深 C-State，彻底避免电池暗耗！

## 3. 硬件遥测强类型实体字典 (`HardwareState`)
当消费者注册就绪后，宿主以自适应周期向公频广播 `hardware-state-updated` 事件，载荷对象包含以下完整硬件字段：

```typescript
interface HardwareState {
  // --- CPU 核心参数 ---
  cpu_temp: number | null;         // CPU 结温 (℃)
  cpu_fan_rpm: number | null;      // CPU 风扇当前转速 (RPM)
  cpu_power_w: number | null;      // CPU 封装实时功耗 (W)
  cpu_freq_mhz: number | null;     // CPU 主频 (MHz)
  cpu_usage_pct: number | null;    // CPU 综合使用率 (0 ~ 100%)

  // --- 独立显卡参数 (仅当有插件声明 gpu_* 时采样) ---
  gpu_temp: number | null;         // GPU 核心温度 (℃)
  gpu_fan_rpm: number | null;      // GPU 风扇当前转速 (RPM)
  gpu_power_w: number | null;      // GPU 实时功耗 (W)
  gpu_freq_mhz: number | null;     // GPU 核心频率 (MHz)
  gpu_usage_pct: number | null;    // GPU 综合利用率 (0 ~ 100%)
  gpu_vram_used_mb: number | null; // 显存已分配容量 (MB)
  gpu_vram_total_mb: number | null;// 显存总物理容量 (MB)

  // --- 电池与供电状态 (全新加入) ---
  battery_level: number | null;    // 当前电池电量百分比 (0 ~ 100)
  is_charging: boolean | null;     // 是否正在充电 (true: 充入电量中; false: 未充电或已断充)
  is_ac_online: boolean | null;    // 外部电源适配器是否物理连接在线

  // --- 系统当前状态 ---
  power_mode: number;              // 1-办公, 2-均衡, 3-狂暴, 4-自定义
  fan_boost: boolean;              // 一键强冷是否处于暴走激活态
}
```

## 4. 前端监听与消费范式

### 4.1 WebView2 / 网页挂件监听范式 (JavaScript)

在 Webview、沙盒 iframe 或独立挂件中，推荐使用事件总线监听：
```javascript
import { listen } from '@tauri-apps/api/event';

// 方式 A：标准事件总线监听
const unlisten = await listen('hardware-state-updated', (event) => {
  const data = event.payload; // HardwareState 强类型对象
  console.log(`电池电量: ${data.battery_level}%, 外部供电: ${data.is_ac_online}, CPU: ${data.cpu_temp}℃`);
});

// 方式 B：原生 Wry / 跨环境通用事件监听
window.addEventListener('openrevo-telemetry', (e) => {
  const data = e.detail;
  console.log(`实时 GPU 温度: ${data.gpu_temp}℃, 实时功耗: ${data.gpu_power_w}W`);
});
```

### 4.2 Slint 原生挂件数据注入范式 (Declarative Push & Pull)

Slint 挂件（`widget.type = "slint"`）作为超轻量独立原生视窗运行时，宿主已内建高性能双向 IPC 注入管道，提供两种最符合 Slint 习惯的数据接入模式：

#### 范式 A：自动属性注入 (Declarative Property Binding，最推荐)
只要在 `widget.slint` 根组件中声明对应字段，宿主遥测心跳采样到新数据时会**自动无感推送到组件属性**中（中划线与下划线命名均支持）：

```slint
export component MyMonitorWidget inherits Window {
    // --- 核心遥测属性（由宿主微内核全自动推送赋值）---
    in property <int> cpu-temp;           // CPU 核心温度 (℃)
    in property <int> gpu-temp;           // GPU 核心温度 (℃)
    in property <float> cpu-power;        // CPU 实时功耗 (W)
    in property <float> gpu-power;        // GPU 实时功耗 (W)
    in property <int> cpu-fan-rpm;        // CPU 风扇转速 (RPM)
    in property <int> gpu-fan-rpm;        // GPU 风扇转速 (RPM)
    in property <float> cpu-usage;        // CPU 使用率 (%)
    in property <float> gpu-usage;        // GPU 使用率 (%)
    in property <int> fps;                // 游戏实时 FPS
    in property <float> frametime-ms;     // 实时帧时间 (ms)
    in property <int> battery-level;      // 电池剩余电量 (0 ~ 100)
    in property <bool> is-charging;       // 是否正在充电
    in property <int> power-mode;         // 电源档位 (1:办公, 2:均衡, 3:狂暴, 4:极客)
    in property <bool> fan-boost;         // 一键强冷状态

    // --- 悬浮条穿透与拖拽控制状态 ---
    in property <bool> is-locked;             // 当前是否处于锁定穿透状态
    in property <bool> ctrl-preview-unlocked; // 用户是否正在按住 Ctrl 临时唤醒拖拽把手

    // --- 全量原始 JSON（可选）---
    in property <string> telemetry-json;

    // UI 声明即响应，零多余代码：
    Text {
        text: "CPU: " + root.cpu-temp + "℃ | GPU: " + root.gpu-temp + "℃";
        color: root.ctrl-preview-unlocked ? #38bdf8 : #f8fafc;
    }
}
```

#### 范式 B：按需回调查询 (Microkernel Callback)
若需要在 Slint 定时器或事件中主动拉取最新硬件快照，可直接调用宿主注入的标准 Callback：
```slint
// 1. 获取最新全量遥测 JSON 字符串快照
let json_str = root.get_hardware_status();

// 2. 单项传感器精准数值查询（查询失败或未就绪返回 -1.0）
let temp = root.get_sensor_value("cpu_temp");
let gpu_w = root.get_sensor_value("gpu_power");
```

---

## 5. 悬浮挂件的“穿透点击 (Click-Through)”与“可拖拽 (Draggable)”兼容机制

悬浮监控条（如屏幕顶部/角落的透明 FPS/温度条）经常面临一个物理矛盾：**打游戏时不希望鼠标误触悬浮条（全穿透），但在需要挪动位置时又必须能点中并拖动它**。

OpenRevo 创新性地提供了**双轨兼容机制**：
1. **键盘修饰键临时唤醒（零门槛拖动）**：
   * 挂件平时处于全穿透状态（`is-locked = true`），鼠标点击直接穿透到下层游戏或桌面；
   * 当用户在键盘上**按住 `Ctrl` 键**时，宿主底层键盘钩子会即刻解除穿透，并在 Slint/WebView2 中将 `ctrl-preview-unlocked` 设为 `true`。此时用户可以鼠标按住悬浮条随意拖拽摆放；松开 `Ctrl` 键后立即自动恢复全穿透。
2. **状态开关切换（小锁头 / 设置页 / 托盘）**：
   * **manifest 声明**：可在 `widget.window` 中配置 `"click_through": true`，使挂件启动即默认穿透；
   * **Slint 代码调用**：调用内置回调 `root.toggle_click_through()` 或 `root.set_click_through(false)` 切换锁定状态；
   * **WebView2 代码调用**：调用 `window.toggleClickThrough()` 或 `window.setClickThrough(false)`；
   * **系统托盘一键总控**：用户随时可以在 Windows 托盘右键菜单中点击 `🧲 挂件穿透锁定 / 解锁拖动` 切换所有挂件状态。

---

## 相关页面

- 上一页：[硬件 IPC 控制能力速查](capabilities.md)
- 下一页：[全域双向广播与跨插件事件总线](events.md)
- 返回目录：[插件与扩展导览](README.md)
