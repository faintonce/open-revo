> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 1: 50 行代码开发一个桌面异形机甲桌宠 (Widget)

新建文件夹 `%APPDATA%\OpenRevo\plugins\eva-pet\`，放置 `manifest.json` 与 `index.html`：

**`manifest.json`**：
```json
{
  "id": "eva-pet",
  "name": "EVA 机甲桌宠",
  "version": "1.0.0",
  "type": "widget",
  "window": { "width": 200, "height": 200, "transparent": true, "alwaysOnTop": true }
}
```

**`index.html`**：
```html
<!DOCTYPE html>
<html>
<head>
  <style>
    body { margin: 0; background: transparent; overflow: hidden; user-select: none; }
    #core {
      width: 180px; height: 180px; border-radius: 50%;
      background: rgba(15, 23, 42, 0.85); backdrop-filter: blur(16px);
      border: 2px solid var(--glow, #38bdf8);
      box-shadow: 0 0 24px var(--glow, #38bdf8);
      display: flex; flex-direction: column; align-items: center; justify-content: center;
      color: #fff; font-family: monospace; cursor: grab;
      transition: all 0.3s ease;
    }
  </style>
</head>
<body>
  <div id="core">
    <span style="font-size: 11px; opacity: 0.6;">S² 能量核心</span>
    <span id="temp" style="font-size: 38px; font-weight: 900;">--°</span>
    <span id="fan" style="font-size: 10px; opacity: 0.7;">-- RPM</span>
  </div>

  <script type="module">
    import { listen } from '@tauri-apps/api/event';
    import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

    // 拖拽窗口支持（推荐使用 pointerdown 单一入口，严禁与 data-tauri-drag-region 混用导致底层冲突）
    document.getElementById('core').addEventListener('pointerdown', (e) => {
      if (e.button === 0) getCurrentWebviewWindow().startDragging();
    });

    // 消费硬件遥测流
    listen('hardware-state-updated', (e) => {
      const data = e.payload;
      const temp = data.cpu_temp || 45;
      document.getElementById('temp').innerText = `${temp}°`;
      document.getElementById('fan').innerText = `${data.cpu_fan_rpm || 0} RPM`;

      // 温度驱动核心变色：>80℃ 猩红, >68℃ 琥珀橙, 平常天蓝
      const glow = temp > 80 ? '#ef4444' : (temp > 68 ? '#f59e0b' : '#38bdf8');
      document.getElementById('core').style.setProperty('--glow', glow);
    });
  </script>
</body>
</html>
```

---

## 相关页面

- 上一页：[用例索引](README.md)
- 下一页：[用例 2：桌面“一键强冷”大开关挂件 (Widget)](02-cooling-toggle-widget.md)
- 返回用例索引：[经典实战用例集](README.md)
