> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 5: 完全重写前端！自建独立替代控制台 (Alternative Shell)

如果你觉得官方的 React 亚克力主界面不符合你的极客审美，**你可以完全重写一个前端**：

1. **设置开机不弹官方主窗口**：
   在 `%APPDATA%\OpenRevo\config.json` 中配置：
   ```json
   {
     "wake_window_mode": "mini",
     "autostart": true
   }
   ```
2. **注册自己的 `shell` 插件**：
   在插件目录中声明 `"type": "shell"`，OpenRevo 会拉起你的主窗口。你的窗口可以使用 Vue3、Svelte、SolidJS 或者 WebGL 3D 渲染整台笔记本的剖面结构，通过本手册第 5 节的 8 大硬件指令全面接管机器！

---

## 相关页面

- 上一页：[用例 4：桌面 AI 助手 Function Calling 联动控制](04-ai-function-calling.md)
- 下一页：[用例 6：进阶机甲桌宠工程规范 —— 边缘吸附灵动小球、动态换肤与跨插件事件总线 (Advanced Widget)](06-advanced-widget.md)
- 返回用例索引：[经典实战用例集](README.md)
