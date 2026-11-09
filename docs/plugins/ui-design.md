> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 9. UI 设计语言与 Design Tokens 复用指南

为了保证在所有原生主题（默认赛博、极简黑白工业风 `mono`）以及第三方插件中质感完全统一，**严禁在样式中硬编码十六进制颜色数值**，必须复用 `src/index.css` 官方 Design System Tokens：

| Token 变量名 | 默认值 (暗色基准) | 作用语义 |
| :--- | :--- | :--- |
| **`--surface-base`** | `rgba(11, 14, 20, 0.94)` | 窗口最底层背景表面 |
| **`--surface-card`** | `rgba(22, 27, 34, 0.75)` | 悬浮功能卡片、微面板表面 |
| **`--surface-card-hover`** | `rgba(33, 38, 45, 0.85)` | 卡片鼠标悬浮激活态 |
| **`--surface-overlay`** | `rgba(15, 23, 42, 0.85)` | 下拉菜单、二级子面板、折叠卡片背景 |
| **`--surface-sunken`** | `rgba(0, 0, 0, 0.35)` | 凹陷式数据底盘、仪表背景 |
| **`--surface-pill`** | `rgba(255, 255, 255, 0.05)` | 胶囊按钮、药丸标签默认表面 |
| **`--border-subtle`** | `rgba(255, 255, 255, 0.08)` | 微弱边框，用于卡片分割 |
| **`--border-medium`** | `rgba(255, 255, 255, 0.15)` | 中度边框，用于可交互控件与按钮外廓 |
| **`--border-highlight`** | `rgba(255, 255, 255, 0.30)` | 高亮焦点与选中态描边 |
| **`--text-primary`** | `#f8fafc` | 主标题、核心数值与高强调文字 |
| **`--text-secondary`** | `#cbd5e1` | 次级标题与重要说明 |
| **`--text-muted`** | `#94a3b8` | 功能描述、次要辅助文本 |
| **`--text-dim`** | `#64748b` | 计量单位、微小提示与未激活状态 |
| **`--status-ok`** | `#10b981` | 正常运行、节能/办公状态 (翡翠绿) |
| **`--status-warn`** | `#f59e0b` | 告警、原厂避让、降频提示 (琥珀黄) |
| **`--status-error`** | `#ef4444` | 危险操作、蓝屏熔断看门狗、超温 (绯红) |
| **`--radius-md`** | `8px` | 标准按钮、输入框、微调卡片圆角 |
| **`--radius-lg`** | `12px` | 大功能卡片、对话框外框圆角 |
| **`--transition-fast`** | `150ms cubic-bezier(0.4, 0, 0.2, 1)` | 按钮悬停、开关微动效过渡 |

## 原生表单与暗黑视窗避坑守则 (WebView2 Dark Mode Gotchas)

1. **原生 `<select>` 下拉菜单白底白字灾难防范**：
   在 Windows Edge WebView2 下，若页面为暗黑深色，原生 `<select>` 点击展开的弹出式下拉项默认会继承系统的浅色控件样式，导致在白底背景上渲染白色文字彻底不可见！
   **核心规范**：所有插件的前端样式中，必须显式声明 `color-scheme: dark;` 并为 `option` 指定深色背景：
   ```css
   select {
     color-scheme: dark;
     background-color: var(--surface-overlay, #0f172a);
     color: var(--text-primary, #f8fafc);
   }
   select option {
     background-color: #0b0f17;
     color: #f8fafc;
   }
   ```

2. **折叠高级选项与响应式视窗排版**：
   - 避免在容器上设置死高度（如 `height: 500px`）导致展开折叠面板或日志控制台时内容突出窗口边界截断；
   - 推荐使用 `display: flex; flex-direction: column; min-height: 0;` 配合 `overflow-y: auto;` 自适应视口尺寸；
   - 提示组件（如 `.toast`）默认应当设置隐藏态（`opacity: 0; visibility: hidden; pointer-events: none;`），严禁未触发时突兀裸露在窗口底部。

---

## 相关页面

- 上一页：[用例 13：正交混搭架构实战](use-cases/13-hybrid-architecture.md)
- 下一页：[调试与安全模式排错](troubleshooting.md)
- 返回目录：[插件与扩展导览](README.md)
