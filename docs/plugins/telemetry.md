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

在 Webview、沙盒 iframe 或独立挂件中，推荐使用事件总线监听：
```javascript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen('hardware-state-updated', (event) => {
  const data = event.payload; // HardwareState 强类型对象
  console.log(`电池电量: ${data.battery_level}%, 外部供电: ${data.is_ac_online}, CPU: ${data.cpu_temp}℃`);
});
```

---

## 相关页面

- 上一页：[硬件 IPC 控制能力速查](capabilities.md)
- 下一页：[全域双向广播与跨插件事件总线](events.md)
- 返回目录：[插件与扩展导览](README.md)
