> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 2: 桌面“一键强冷”大开关挂件 (Widget)

用户嫌每次打开控制中心麻烦，只想在桌面上摆个硕大的实体质感按钮，按一下强冷，再按一下恢复：

```html
<button id="turbo-btn" style="width: 140px; height: 50px; background: #ef4444; color: #fff; border-radius: 12px; font-weight: bold; cursor: pointer;">
  🔥 切换全速强冷
</button>

<script type="module">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let isBoosting = false;
  const btn = document.getElementById('turbo-btn');

  // 点击立刻下发硬件强冷
  btn.addEventListener('click', async () => {
    isBoosting = !isBoosting;
    await invoke('set_fan_boost', { enable: isBoosting });
  });

  // 同步物理按键或托盘触发的状态改变
  listen('fan-boost-changed', (e) => {
    isBoosting = e.payload;
    btn.innerText = isBoosting ? '❄️ 强冷运行中' : '🔥 开启全速强冷';
    btn.style.background = isBoosting ? '#06b6d4' : '#ef4444';
  });
</script>
```

---

## 相关页面

- 上一页：[用例 1：50 行代码开发一个桌面异形机甲桌宠 (Widget)](01-desktop-widget.md)
- 下一页：[用例 3：纯文本脚本自定义开机仪式灯效 (Headless Service - 零 Webview)](03-headless-boot-lighting.md)
- 返回用例索引：[经典实战用例集](README.md)
