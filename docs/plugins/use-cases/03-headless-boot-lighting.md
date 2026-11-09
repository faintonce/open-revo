> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 3: 纯文本脚本自定义开机仪式灯效 (Headless Service - 零 Webview)

用户想自定义电脑开机时键盘的流光欢迎仪式，**完全不需要任何 Webview 窗口，仅占用 <100KB 内存**：

**`manifest.json`**：
```json
{
  "id": "cyber-welcome",
  "name": "赛博开机灯效仪式",
  "version": "1.0.0",
  "type": "service",
  "target": "lighting.welcome",
  "entry": "welcome.rhai"
}
```

**`welcome.rhai`**（嵌入式脚本，在 Rust 内部极速执行）：
```rust
// 当系统检测到用户开机登录时自动触发
fn on_startup_ceremony() {
    // 阶段 1: 冰蓝呼吸唤醒
    set_keyboard_rgb(0, 255, 255);
    sleep(400);
    // 阶段 2: 霓虹粉紫能量注入
    set_keyboard_rgb(255, 0, 128);
    sleep(400);
    // 阶段 3: 狂暴红脉冲
    set_keyboard_rgb(255, 30, 30);
    sleep(300);
}
```

---

## 相关页面

- 上一页：[用例 2：桌面“一键强冷”大开关挂件 (Widget)](02-cooling-toggle-widget.md)
- 下一页：[用例 4：桌面 AI 助手 Function Calling 联动控制](04-ai-function-calling.md)
- 返回用例索引：[经典实战用例集](README.md)
