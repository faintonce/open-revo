> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 11: 声明式零代码全局快捷键与按键宏映射插件 (Universal Shortcut Bus)

> **目标**：为机械革命笔记本定制一套全局快捷键工坊，支持在游戏独占全屏中随时按键瞬切强冷、性能模式、屏幕刷新率，甚至一键息屏或调起宏。**无需写任何 C/C++ 代码，无需拉起外部 exe 守护进程，享受宿主 0 内存开销的底座红利！**

## 方式 A：纯声明式零代码配置 (`manifest.json`)
只需在插件 `manifest.json` 中配置 `shortcuts` 数组，宿主在启用该插件时自动将快捷键注入 Ring3 键盘总线；禁用或卸载插件时自动全量注销：

```json
{
  "id": "pro-gamer-keys",
  "name": "极客电竞快捷键工坊",
  "version": "1.0.0",
  "plugin_type": "tab",
  "entry": "index.html",
  "shortcuts": [
    {
      "id": "gamer_turbo_boost",
      "name": "一键全速暴走强冷",
      "combo": "Ctrl+Shift+F",
      "action": { "type": "ToggleFanBoost" },
      "osd": true
    },
    {
      "id": "gamer_power_cycle",
      "name": "循环切换性能释放模式",
      "combo": "Ctrl+Shift+M",
      "action": { "type": "CyclePowerMode" },
      "osd": true
    },
    {
      "id": "gamer_hz_switch",
      "name": "刷新率瞬切 (高刷/省电)",
      "combo": "Ctrl+Alt+R",
      "action": { "type": "ToggleRefreshRate" },
      "osd": true
    },
    {
      "id": "gamer_monitor_sleep",
      "name": "挂机一键息屏",
      "combo": "Ctrl+Shift+L",
      "action": { "type": "MonitorOff" },
      "osd": false
    }
  ]
}
```

## 方式 B：前端可视化录制与动态注册 API
在插件界面中，让用户像玩游戏改键位一样按下组合键并动态注册：

```javascript
import { invoke } from '@tauri-apps/api/core';

// 1. 查询当前已生效的所有快捷键
const list = await invoke('get_registered_shortcuts');
console.log('当前快捷键列表:', list);

// 2. 动态注册用户自定义按键
await invoke('register_custom_shortcut', {
  binding: {
    id: 'user_boss_key',
    name: '老板键：一键静音 + 锁屏',
    combo: 'Ctrl+Alt+B',
    action: { type: 'LockScreen' },
    osd: false,
    enabled: true,
    swallow: true
  }
});

// 3. 注销快捷键
await invoke('unregister_custom_shortcut', { id: 'user_boss_key' });
```

---

## 相关页面

- 上一页：[用例 10：原生体验第三方插件市场开发对接范式 (Plugin Market Integration - 以 `plugin-market` 为例)](10-plugin-market.md)
- 下一页：[用例 12：声明式自动化监听与规则看门狗 (Universal Watcher & Trigger Hub —— 彻底根除隐藏桌宠偷跑轮询)](12-watcher-automation.md)
- 返回用例索引：[经典实战用例集](README.md)
