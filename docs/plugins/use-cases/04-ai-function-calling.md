> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 4: 桌面 AI 助手 Function Calling 联动控制

用户吼一声：“**帮我开一键强冷**”或“**有点热，吹大点风**”。\
桌面 AI 助手（Brain）解析意图后，通过 Function Calling 直接下发硬件指令：

```javascript
// AI Agent 内部注册的 Tools Schema
const tools = [
  {
    name: 'set_fan_boost',
    description: '开启或关闭机械革命笔记本全速强冷模式 (Fan Boost)',
    parameters: {
      type: 'object',
      properties: { enable: { type: 'boolean' } },
      required: ['enable']
    }
  }
];

// 当大模型返回 Tool Call 时执行
async function handleToolCall(toolCall) {
  if (toolCall.name === 'set_fan_boost') {
    const { enable } = JSON.parse(toolCall.arguments);
    // 直接下发微内核 Ring0 指令！
    await invoke('set_fan_boost', { enable });
    return `已成功${enable ? '开启' : '关闭'}强冷模式！`;
  }
}
```

---

## 相关页面

- 上一页：[用例 3：纯文本脚本自定义开机仪式灯效 (Headless Service - 零 Webview)](03-headless-boot-lighting.md)
- 下一页：[用例 5：完全重写前端！自建独立替代控制台 (Alternative Shell)](05-alternative-shell.md)
- 返回用例索引：[经典实战用例集](README.md)
