> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 5. 硬件 IPC 控制能力军火库速查 (Capabilities Cheat Sheet)

在独立 Webview 或插件前端中，只要引入 `@tauri-apps/api/core` 的 `invoke`，即可直接调用微内核暴露的 8 大硬件控制指令：

```javascript
import { invoke } from '@tauri-apps/api/core';
```

## 1. 一键全速强冷 (Fan Boost)
* **指令**：`invoke('set_fan_boost', { enable: boolean })`
* **硬件动作**：向 EC 写入强冷标志位，风扇瞬间以 5400+ RPM 暴走排气；托盘图标加风扇角标，广播 `fan-boost-changed`。

## 2. 性能模式切换 (Power Mode)
* **指令**：`invoke('set_power_mode', { mode: 1 | 2 | 3 | 4 })`
* **模式映射**：`1-办公(Office)`, `2-均衡(Balance)`, `3-狂暴(Turbo/Beast)`, `4-自定义(Custom)`。
* **硬件动作**：刷新 CPU PL1/PL2/PL4 功耗偏置，调整风扇曲线与动态背光，广播 `physical-mode-switched`。

## 3. 电池健康养护限额 (Battery Protect)
* **指令**：`invoke('set_battery_limit', { limit: number })`
* **参数范围**：`60 ~ 100`（通常设定为 60%、80% 或 100%）。
* **硬件动作**：写入 EC 电池控制寄存器。达到指定电量后硬件直接断充，避免电芯持续高压鼓包。

## 4. 屏幕刷新率瞬切 (Display Refresh Rate)
* **指令**：`invoke('switch_refresh_rate', { hz: number })`
* **常用参数**：`60`（离电省电）或 `165` / `240`（游戏电竞）。
* **硬件动作**：调用 Win32 底层显示配置无黑屏快速切换。

## 5. 显卡模式直连切换 (GPU MUX)
* **指令**：`invoke('set_gpu_mode', { mode: string })`
* **模式选项**：`"discrete"` (独显直连性能最大化), `"hybrid"` (混合输出兼顾续航)。

## 6. 四分区全彩键盘背光 (RGB Lighting)
* **指令**：`invoke('apply_four_zone_colors', { zone1: [r,g,b], zone2: [r,g,b], zone3: [r,g,b], zone4: [r,g,b] })`
* **硬件动作**：调用 ITE 键盘灯控接口下发色彩。

## 7. 第二代外置水冷魔盒调控 (BLE Cooler)
* **指令**：`invoke('set_water_cooler_speed', { level: number })`
* **硬件动作**：WinRT 蓝牙低功耗向水冷盒下发泵速与转速挡位。

## 8. 硬件外设软开关 (Device Switches)
* **指令**：`invoke('set_device_switch', { device: string, enabled: boolean })`
* **设备标识**：`"camera"` (摄像头), `"mic"` (麦克风), `"touchpad"` (触控板), `"winlock"` (Win键锁定)。

## 9. 皮肤与背景样式管理 IPC 军火库 (Plugin Styles & Assets)
OpenRevo 现已全面提供原生双通道样式加载与持久化命令，插件或外部脚本可直接调用：
* **`invoke('load_plugin_theme_css', { pluginId: string })`**：读取指定皮肤插件的 CSS，自动将 CSS 中的相对路径图片转为内存 Data URI 并返回，供注入 `<style id="openrevo-theme-skin">`。
* **`invoke('load_plugin_background_css', { pluginId: string })`**：读取指定背景插件的 CSS（或直接读取图片资产自动合成全套背景规则），供注入 `<style id="openrevo-custom-background">`。
* **`invoke('get_active_skin')` / `invoke('set_active_skin', { skinId: string | null })`**：获取或设置当前全局激活的主题皮肤 ID。
* **`invoke('get_active_background')` / `invoke('set_active_background', { backgroundId: string | null })`**：获取或设置当前全局激活的背景壁纸 ID。
* **`invoke('get_custom_plugins')`**：重新枚举 `%APPDATA%\OpenRevo\plugins` 目录，即时返回所有插件元数据（支持前端热刷新，无需重启宿主）。
* **`invoke('open_plugins_dir')`**：唤起系统文件管理器直接定位并打开插件根目录。

## 10. 伴随提权服务生命周期控制 (Companion Service Supervisor)
OpenRevo 为伴随原生后台微服务提供了完整的主机控制指令（同时原生支持在选项卡顶栏通过胶囊按钮直接操作）：
* **`invoke('start_plugin_service', { pluginId: string }) -> PluginServiceStatus`**：拉起指定插件声明的原生可执行文件（继承管理员提权无黑框运行，自动启动 TCP 探活轮询）。
* **`invoke('stop_plugin_service', { pluginId: string }) -> PluginServiceStatus`**：优雅终止指定插件的伴随服务进程。
* **`invoke('get_plugin_service_status', { pluginId: string }) -> PluginServiceStatus`**：实时查询指定插件的服务运行状态、PID、回环端口与端口连通性。
* **`invoke('restart_plugin_service', { pluginId: string }) -> PluginServiceStatus`**：一键重启服务。

`PluginServiceStatus` 强类型状态对象定义：
```typescript
interface PluginServiceStatus {
  running: boolean;        // 进程是否处于活动状态
  pid: number | null;      // Windows 系统进程 ID
  port: number | null;     // 声明的本地回环 TCP 端口
  healthy: boolean;        // TCP 端口是否已成功连通探活
  last_error: string | null; // 最近一次启动或运行异常报错
  uptime_secs: number;     // 持续运行秒数
}
```

### 沙盒 iframe 双向事件桥 (PostMessage Bridge)
对于运行在沙盒 `iframe` 中的插件前端（如 `ec-config`），无需直接调用 Tauri 原生模块，直接通过跨文档通信与宿主对话：
```javascript
// 1. 向宿主发起控制指令
window.parent.postMessage({ action: 'START_PLUGIN_SERVICE' }, '*');
window.parent.postMessage({ action: 'STOP_PLUGIN_SERVICE' }, '*');
window.parent.postMessage({ action: 'RESTART_PLUGIN_SERVICE' }, '*');
window.parent.postMessage({ action: 'GET_PLUGIN_SERVICE_STATUS' }, '*');

// 2. 监听宿主下发的状态变更广播
window.addEventListener('message', (event) => {
  if (event.data?.type === 'OPENREVO_PLUGIN_SERVICE_STATUS') {
    const { status, pluginId } = event.data;
    console.log(`[服务状态] 进程运行: ${status.running}, 端口健康: ${status.healthy}`);
  }
});
```

## 11. 插件市场与生命周期管理 IPC 军火库 (Plugin Market & Lifecycle API)
为支持官方及第三方社区开发者构建流畅的原生“插件市场”（如 `plugin-market` Tab 插件），OpenRevo 宿主提供了完整的生命周期管理 IPC，告别让用户手动下载解压或手动改配置的原始体验：

* **`invoke('get_plugin_host_capabilities') -> HostCapabilities`**：
  获取宿主版本号、操作系统、芯片架构与支持的能力特性标签列表，供市场做运行时兼容性检查。
  ```typescript
  interface HostCapabilities {
    version: string;
    tauri_version: string;
    os: string;
    arch: string;
    capabilities: string[]; // ["service_supervisor", "market_installer", "dual_channel_themes", ...]
  }
  ```
* **`invoke('install_plugin_from_url', { url: string, expectedSha256?: string }) -> InstallResult`**：
  **【一键安全拉取并静默安装】**：
  * 强制限制 `https://` 协议，杜绝中间人劫持；
  * 可选校验 `expectedSha256` 完整性指纹；
  * 自动解压至临时目录并严格断言路径边界（防 Zip Slip 穿越）；
  * 解析验证 `manifest.json` 合法性（限制 ID 为纯字母数字下划线中划线，长度 `<= 64`）；
  * 安全原子化移动至 `%APPDATA%\OpenRevo\plugins\<plugin_id>\`；
  * 自动触发内部重载并广播 `plugins-changed`。
* **`invoke('uninstall_plugin', { pluginId: string }) -> boolean`**：
  **【安全无痕卸载】**：
  * 若该插件正运行伴随服务，先自动停机；
  * 自动在 `config.json` 中将其从 `custom_plugin_toggles`、`active_skin`、`active_background` 中清理剔除；
  * **严格遵守安全守则，调用 Windows Shell API 将插件目录移入系统回收站**，物理保留可恢复性，绝不盲目物理抹除；
  * 触发重载并广播 `plugins-changed`。
* **`invoke('reload_plugins') -> CustomPlugin[]`**：
  免重启强制刷新并重新扫描插件目录，返回最新插件列表。
* **`invoke('open_external_url', { url: string }) -> boolean`**：
  在系统默认浏览器中打开白名单 URL（限制 `https://` / `http://`），突破沙盒 iframe 无法 `window.open` 的限制，杜绝高危协议提权。
* **`invoke('write_clipboard', { text: string }) -> boolean`**：
  突破沙盒 iframe 无法直接调用 `navigator.clipboard` 的限制，由宿主代为安全写入系统剪贴板。
* **`invoke('get_plugin_manifest', { pluginId: string }) -> any`**：
  精确读取指定插件的原始 `manifest.json` 清单对象。
* **`invoke('set_plugin_enabled_unified', { pluginId: string, enabled: boolean }) -> boolean`**：
  **【全类型插件统一开关总线】**：
  无需在市场中分别判断皮肤、壁纸和功能插件，传入 `pluginId` 与目标布尔值，宿主自动路由分派（`skin` 自动生效/取消激活皮肤，`background` 自动生效/取消激活壁纸，其它插件自动切换开关并持久化）。

## 12. 全局快捷键微内核总线 IPC 军火库 (Universal Shortcut Bus API)
OpenRevo 内核现已将 Windows 底层键盘钩子（`WH_KEYBOARD_LL`）抽象为原生的全局快捷键分发总线，插件开发者**完全不需要写 C/C++ 或拉起外部 exe 守护进程**：

* **`invoke('get_registered_shortcuts') -> ShortcutBinding[]`**：获取当前微内核已注册生效的所有快捷键清单。
* **`invoke('register_custom_shortcut', { binding: ShortcutBinding }) -> boolean`**：动态注册自定义按键组合（支持 `Ctrl+Alt+F`、`Win+Shift+R`、`F10` 等），支持自动联动 OSD 浮层。
* **`invoke('unregister_custom_shortcut', { id: string }) -> boolean`**：注销指定快捷键。
* **`invoke('trigger_shortcut_action', { action: ShortcutAction, osd?: boolean }) -> void`**：手动执行指定动作（可用于界面按钮快捷模拟）。

### `ShortcutBinding` 契约定义：
```typescript
interface ShortcutBinding {
  id: string;              // 唯一标识 (如 "my_fan_boost")
  name: string;            // 人类可读名称
  combo: string;           // 组合键字符串 (如 "Ctrl+Shift+F", "Win+Alt+R")
  action: ShortcutAction;  // 绑定的动作类型与载荷
  osd?: boolean;           // 触发时是否自动唤起机甲 OSD 屏幕浮层 (默认 true)
  enabled?: boolean;       // 是否启用 (默认 true)
  swallow?: boolean;       // 是否拦截吃掉按键不传给前台程序 (默认 false)
  plugin_id?: string;      // 所属插件 ID
}

type ShortcutAction =
  | { type: "ToggleFanBoost" }
  | { type: "CyclePowerMode" }
  | { type: "SetPowerMode", payload: number }
  | { type: "ToggleRefreshRate" }
  | { type: "ToggleTouchpad" }
  | { type: "ToggleWinLock" }
  | { type: "ToggleFnLock" }
  | { type: "CycleKeyboardBrightness" }
  | { type: "TriggerOsdPreview" }
  | { type: "VolumeMute" }
  | { type: "VolumeUp" }
  | { type: "VolumeDown" }
  | { type: "MonitorOff" }
  | { type: "LockScreen" }
  | { type: "LaunchApp", payload: { path: string, args?: string[] } }
  | { type: "PluginEvent", payload: { event_name: string, payload: any } };
```

## 13. 多模具底层驱动网关与物理防爆盾 (Raw Hardware Driver Gateway)
为彻底赋能社区高阶极客，OpenRevo 现已将内核支持的 **4 种主板模具底层通信通道** 抽象为开放的底层驱动网关。插件开发者无需再被卡在 Ring0 驱动通信、管理员提权或 WMI 复杂封包上，可根据模具类型直接调用微内核执行底层 EC/主板交互：

### 4 大支持的底层模具协议 (`RawEcProtocol`)：
1. **`TongfangDirectEc`**：同方模具原生 `ACPIDriver` IOCTL 通道，支持任意有效物理地址的单字节高速读写；
2. **`BldWmiMailbox`**：宝龙达/蛟龙模具 WMI 邮箱协议，支持向邮箱通道下发变长多字节报文并读取响应；
3. **`GenericAcpiWmi`**：通用标准 ACPI WMI MULONG 传输通道；
4. **`IteLightingBus`**：ITE 独立按键流光与背光芯片硬件通道。

### 核心 IPC 军火库速查：
* **`invoke('get_raw_ec_capabilities') -> RawEcCapabilities`**：嗅探当前物理机器已就绪的模具通信协议列表（如判断本机是同方 IOCTL 还是宝龙达 Mailbox）。
* **`invoke('raw_ec_read', { request: { protocol: string, address: number } }) -> number`**：从指定模具协议的 EC RAM 物理地址读取 1 字节数值（返回 `0x00 ~ 0xFF`）。
* **`invoke('raw_ec_write', { request: { protocol: string, address: number, value: number } }) -> boolean`**：向指定模具协议的 EC RAM 物理地址写入 1 字节数值。受物理防爆盾严格保护。
* **`invoke('raw_mailbox_invoke', { request: { in_bytes: number[] } }) -> number[]`**：向宝龙达等邮箱协议模具直接下发原始字节流并返回响应报文。

#### Slint 原生挂件直调硬件 Callbacks (脱离 WebView2 / Tauri IPC)：
在 `.slint` 声明式组件中，微内核执行器（`openrevo-widget`）在动态编译实例化时会自动注入以下硬件微内核 Callbacks，插件在 Slint 文件中声明同名 callback 即可直调硬件：
* `callback raw_ec_read(int) -> int`：自动嗅探本机最佳总线协议并读取 EC 寄存器（返回 0~255，失败返回 -1）；
* `callback raw_ec_read(string, int) -> int`：指定协议（如 `"TongfangDirectEc"`、`"GenericAcpiWmi"`、`"BldWmiMailbox"`）读取；
* `callback raw_ec_write(int, int) -> bool`：自动嗅探协议写入 EC 单字节；
* `callback raw_ec_write(string, int, int) -> bool`：指定协议写入 EC 单字节；
* `callback set_fan_boost(bool) -> bool`：硬件级一键强冷；
* `callback set_power_mode(int) -> bool`：性能模式切换（1: 办公, 2: 平衡, 3: 狂暴）；
* `callback set_battery_health(int) -> bool`：电池养护阈值调节（1: 100%, 2: 80%, 3: 60%）；
* `callback get_raw_ec_capabilities() -> string`：返回可用协议 JSON 数组字符串。

### 硬件物理防爆盾（Hardware Blast Shield）与安全规范：
1. **地址边界限制**：物理地址必须严格位于 `0x0000 ~ 0x0FFF` 之间，超出即拒绝执行；
2. **禁区写入熔断**：内核底层硬编码拦截致命保留引脚（如 `0x0000` 复位极值触发），杜绝主板永久变砖；
3. **权限清单隔离**：插件必须在 `manifest.json` 中显式声明 `"permissions": ["hardware:raw_ec_read", "hardware:raw_ec_write"]`；
4. **市场审核认证机制**：
   - **官方认证徽章（Verified by OpenRevo）**：代码经过审计，小白用户可放心启用；
   - **社区未认证高危插件（Community Experimental）**：带有高危警告角标，首次启用时强制弹出红色确认对话框，用户自担硬件风险。
## 14. 公共监听与自动化规则总线 (Universal Watcher & Trigger Hub)
为彻底解决社区开发者**“为了跑几行后台监控逻辑，被迫开一个隐藏透明挂件偷跑轮询”**的痛点与能耗弊端，OpenRevo 现已内置原生的**微内核公共监听与反应中心（Watcher & Trigger Hub）**。

插件开发者无需自建任何 `setInterval`、无需创建任何隐藏 Webview，只需向宿主声明或注册监听条件，宿主便会在已有的极低开销硬件遥测与系统事件循环中统一评估并在命中时毫秒级自动触发动作或推流！

### 1. 痛点根除与运行机制：
* **零额外内存与视窗（Zero GUI / 0 MB）**：彻底告别“隐形 Webview2 挂件”，纯规则插件无需编写任何 HTML/JS 界面；
* **防后台休眠与精准触发**：由 Rust 内核的原生无锁心跳统一调度，彻底免疫 Windows/Chromium 对后台隐藏窗口的计时器降频节流；
* **防独显唤醒守则延续**：若注册的规则仅关注 CPU 或电源，NVIDIA 独立显卡严格保持 D3Cold 零功耗深睡；
* **物理防爆盾时序保护**：规则中涉及的所有 EC 寄存器写入，严格受到 Hardware Blast Shield 的地址边界校验与互斥锁保护。

### 2. 清单声明式规则示例 (`manifest.json`)
开发者可在插件的 `manifest.json` 中直接通过 `watchers`（或 `rules`）声明自动化规则，宿主加载插件时自动生效，禁用插件时自动析构：

```json
{
  "id": "mode4-fan-unblocker",
  "name": "自定义模式55度解锁补丁",
  "version": "1.0.0",
  "plugin_type": "service",
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
    }
  ]
}
```

### 3. 支持的 4 大监听条件类型 (`WatcherCondition`)：
1. **`telemetry` (硬件遥测阈值与状态)**：
   * `power_mode`: 指定特定性能模式（如 `4`）；
   * `power_mode_in`: 指定模式数组（如 `[2, 4]`）；
   * `cpu_temp_gt` / `cpu_temp_gte` / `cpu_temp_lt`: CPU 结温比较阈值；
   * `gpu_temp_gt` / `gpu_temp_gte`: GPU 温度比较阈值；
   * `battery_level_lt`: 电池电量低于指定百分比；
   * `is_charging`: 是否处于充电态；
   * `fan_boost`: 强冷是否开启。
2. **`foreground_process` (前台进程与游戏追踪)**：
   * `targets`: 关注的目标进程名列表（如 `["b1.exe", "cyberpunk2077.exe"]`）；
   * `trigger_on`: 触发时机：`"enter"`（切换到前台时）、`"exit"`（切出前台时）或 `"both"`。
3. **`power_event` (电源与系统边沿事件)**：
   * `event`: `"ac_connected"`（插入充电器）、`"ac_disconnected"`（拔出充电器/走电池）、`"screen_off"`、`"screen_on"`。
4. **`raw_ec_diff` (受控 EC 寄存器差量监控)**：
   * `address`: 目标物理地址（支持十进制整数或十六进制字符串如 `"0x0740"`）；
   * `mask`: 可选位掩码，仅关注特定位段变化；
   * 仅在数值发生实际跳变时触发。

### 4. 支持的执行动作类型 (`WatcherAction`)：
* **`set_power_mode`**：`{ "action": "set_power_mode", "mode": 1 | 2 | 3 | 4 }`
* **`set_fan_boost`**：`{ "action": "set_fan_boost", "enable": boolean }`
* **`delay_ms`**：`{ "action": "delay_ms", "duration": number }`（毫秒延时）
* **`raw_ec_write`**：`{ "action": "raw_ec_write", "address": "0x...", "value": number }`
* **`raw_ec_modify_mask`**：`{ "action": "raw_ec_modify_mask", "address": "0x...", "mask": "0x...", "value": boolean }`
* **`notify_osd`**：`{ "action": "notify_osd", "title": string, "message": string }`
* **`plugin_event`**：`{ "action": "plugin_event", "event_name": string, "payload": any }`

### 5. 动态 IPC 军火库速查与沙盒事件桥：
* **`invoke('get_registered_watchers') -> WatcherInfo[]`**：查询当前所有活跃监听器的状态、所属插件、冷却倒计时与触发历史；
* **`invoke('register_custom_watcher', { watcher: WatcherDefinition }) -> boolean`**：动态注册监听器；
* **`invoke('unregister_custom_watcher', { id: string }) -> boolean`**：动态注销指定监听器；
* **`invoke('trigger_watcher_manual', { id: string }) -> boolean`**：手动模拟触发；
* **沙盒桥调用**：在微插槽或 Tab 中直接通过 `window.__OPENREVO_BRIDGE__.registerWatcher(...)` 注册，视窗卸载时自动级联清理。

## 15. 遥测消费动态注册与全域广播 IPC (Telemetry Consumer & Global Broadcast)
为了支持前端动态根据视图展开收起、弹窗或挂件启停，精准按需调节遥测采样与全域广播：
* **`invoke('register_telemetry_consumer', { consumerId: string, sensors: string[], desiredIntervalMs?: number }) -> boolean`**：
  为当前插件、视窗或功能动态注册遥测消费者，声明所需传感器并指定期望的心跳频率。
* **`invoke('unregister_telemetry_consumer', { consumerId: string }) -> boolean`**：
  注销指定消费者。若全系统活跃消费者计数归零，遥测引擎自动休眠降频。
* **`invoke('emit_plugin_broadcast', { event: string, payload: any }) -> boolean`**：
  **【全域双向广播发射塔】**：向整个微内核、所有前端视窗（包括 Tauri Webview、Slint 原生界面与沙盒 Native Wry）广播跨插件或宿主事件。

---

## 相关页面

- 上一页：[插件清单权威规范](manifest-reference.md)
- 下一页：[自适应遥测与动态心跳总线](telemetry.md)
- 返回目录：[插件与扩展导览](README.md)
