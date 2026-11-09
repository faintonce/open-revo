# 一键强冷极速热键 (Quick Fan Boost Shortcut)

> **类型**：纯声明式无头快捷键插件 (`plugin_type: "service"`)  
> **特性**：零 Webview 开销、0 MB 额外内存损耗、毫秒级硬件直达、原生机甲 OSD 联动

---

## 插件特性

1. **极致轻量 (Zero-GUI / 0 MB)**：
   本插件不需要开启任何网页窗口或后台常驻进程，仅通过在 `manifest.json` 中声明微内核热键规则，由 OpenRevo 统一注入 Ring3 底层全局键盘钩子（`WH_KEYBOARD_LL`）。
2. **全速强冷极速开闭**：
   在任意游戏全屏或前台桌面下，按下 **`Ctrl+Alt+F`** 即可瞬间切换全速强冷（5400+ RPM 暴走排气），再次按下恢复原曲线。
3. **原生战术机甲 OSD 弹窗**：
   按键触发时，屏幕中央会自动弹出 OpenRevo 战术 OSD 屏幕通知，实时反馈强冷开关状态，不遮挡游戏视线。
4. **即插即用，零污染**：
   在 OpenRevo【插件】管理列表中开启即刻生效，关闭或卸载时微内核自动秒级全量注销按键绑定，不留任何系统残留。

---

## 清单配置 (`manifest.json`)

```json
{
  "id": "shortcut-fan-boost",
  "name": "一键强冷极速热键",
  "version": "1.0.0",
  "author": "OpenRevo Official & Community",
  "description": "官方演示插件：纯声明式无头全局热键（零 Webview、零额外内存占用），使用快捷键（默认 Ctrl+Alt+F）随时极速开关全速强冷模式，联动原生机甲 OSD 弹窗。",
  "plugin_type": "service",
  "min_host_version": "0.9.3",
  "shortcuts": [
    {
      "id": "quick_fan_boost_toggle",
      "name": "极速开关全速强冷",
      "combo": "Ctrl+Alt+F",
      "action": {
        "type": "ToggleFanBoost"
      },
      "osd": true,
      "enabled": true,
      "swallow": false
    }
  ]
}
```

---

## 如何自定义热键

若想修改为其他组合键（例如 `Ctrl+Shift+F`、`Ctrl+F1` 或 `Win+F10`）：
1. 打开本插件目录下的 `manifest.json`；
2. 将 `"combo": "Ctrl+Alt+F"` 改为你习惯的组合键；
3. 在 OpenRevo 控制台插件管理中点击“刷新”或重新开关此插件即可即刻生效！

