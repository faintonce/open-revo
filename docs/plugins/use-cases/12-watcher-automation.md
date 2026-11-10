> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 12: 声明式自动化监听与规则看门狗 (Universal Watcher & Trigger Hub —— 彻底根除隐藏桌宠偷跑轮询)

> **痛点背景**：在早期，部分极客开发者为了实现“监控 CPU 结温达到 55℃ 时自动刷新 EC 状态字”或“启动游戏时自动切狂暴模式”，在插件里声明一个 `widget`（桌面悬浮挂件），将其设为全透明隐藏并使用 `setInterval` 进行 100ms 死循环轮询。\
> **这种做法存在致命缺陷**：
> 1. **白白浪费内存**：一个 WebView2 实例增量占用约 15MB~25MB 内存；
> 2. **后台节流失效**：Windows 与 Chromium 会自动将不可见窗口的定时器强行节流冻结，导致切模逻辑经常失效或严重滞后；
> 3. **唤醒独显功耗爆炸**：挂件前端轮询不当会强制唤醒本处于 D3Cold 深睡的 NVIDIA 独立显卡。
>
> **官方终极解法**：借助 OpenRevo 微内核内建的 **Universal Watcher & Trigger Hub**，实现纯声明式规则监控。**0 MB 内存、0 额外视窗、0 代码死循环，由宿主内核在无锁遥测心跳中统一调度！**

## 1. 极简项目结构
一个纯自动化规则插件，**根目录下只需一个 `manifest.json` 文件**，连 HTML、CSS、JS 都完全不需要写：
```text
%APPDATA%\OpenRevo\plugins\mode4-fan-unblocker\
└── manifest.json
```

## 2. 清单配置 (`manifest.json`)
```json
{
  "id": "mode4-fan-unblocker",
  "name": "自定义模式55度解锁补丁",
  "version": "1.0.0",
  "author": "OpenRevo Community",
  "description": "监控结温并在进入自定义模式且>=55℃时自动执行安全切模与EC状态字刷新。",
  "plugin_type": "service",
  "min_host_version": "0.9.3",
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
    },
    {
      "id": "game_auto_beast",
      "name": "黑神话启动自动切狂暴",
      "enabled": true,
      "cooldown_ms": 5000,
      "when": {
        "type": "foreground_process",
        "targets": ["b1.exe", "b1-win64-shipping.exe"],
        "trigger_on": "enter"
      },
      "actions": [
        { "action": "set_power_mode", "mode": 3 },
        { "action": "notify_osd", "title": "游戏感知联动", "message": "已自动切换至狂暴模式" }
      ]
    }
  ]
}
```

## 3. 核心机制优势解析
* **`cooldown_ms` 防抖冷却**：规则命中后进入冷却锁，防止温度在临界线抖动时引发高频重复触发与硬件冲突；
* **地址与掩码语法宽容**：`address` 与 `mask` 既支持字符串十六进制（`"0x0751"`、`"0x10"`），也支持十进制数字（`1873`、`16`）；
* **宿主生命周期全托管**：在宿主【插件】列表中启用该插件即刻注入微内核总线开始看门狗监控；关闭插件时自动全量卸载，不留任何残留副作用；
* **版本兼容守护 (`min_host_version`)**：声明 `"min_host_version": "0.9.3"`，若旧版宿主尝试加载，界面将明确亮起警告徽章并安全拦截，引导用户升级。

---

## 相关页面

- 上一页：[用例 11：声明式零代码全局快捷键与按键宏映射插件 (Universal Shortcut Bus)](11-universal-shortcuts.md)
- 下一页：[用例 13：正交混搭架构实战 —— WebView2 富交互大控制台与 Slint 原生极速常驻桌面挂件 (Hybrid Architecture)](13-hybrid-architecture.md)
- 返回用例索引：[经典实战用例集](README.md)
