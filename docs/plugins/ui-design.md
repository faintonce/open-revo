> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 9. UI 设计语言与双引擎 Design Tokens 复用指南

为了保证在所有原生主题（默认暗黑赛博 `cyber/dark`、纯白纸质 `light`、极简黑白工业风 `mono`）以及第三方插件中质感完全统一，**严禁在样式中硬编码十六进制颜色数值**。

OpenRevo 采用**双引擎统一 Token 系统**：同一套 Design Tokens 既能在传统 WebView2（HTML/CSS 变量注入）中生效，也会由微内核（`system::slint_theme`）自动提取解析并实时注入到 Slint 原生主窗口（`openrevo-main`）与 Slint 桌面 OSD 悬浮窗（`openrevo-osd`）的全局 `AppTheme` 状态机中。

---

## 1. 核心 Design Tokens 字典与映射契约

无论在 CSS 文件（如 `theme.css`）还是在插件清单的 JSON 配置（`tokens` / `tokens_light` / `tokens_dark` / `tokens_mono`）中，变量名与语义均严格对齐：

| CSS 变量名 | JSON 字段名 | 默认值 (暗色基准) | 作用语义 |
| :--- | :--- | :--- | :--- |
| **`--bg-root`** | `bg_root` | `#0b0e14` | 视窗全局根背景底色 |
| **`--surface-base`** | `surface_base` | `rgba(11, 14, 20, 0.94)` | 窗口工作区第一层表面底色 |
| **`--surface-card`** | `surface_card` | `rgba(22, 27, 34, 0.75)` | 悬浮功能卡片、设置行微面板表面 |
| **`--surface-card-hover`** | `surface_card_hover` | `rgba(33, 38, 45, 0.85)` | 卡片鼠标悬浮激活态 |
| **`--surface-overlay`** / `--surface-elevated` | `surface_elevated` | `rgba(15, 23, 42, 0.85)` | 下拉菜单、二级子面板、折叠卡片背景 |
| **`--surface-sunken`** | `surface_sunken` | `rgba(0, 0, 0, 0.35)` | 凹陷式数据底盘、仪表背景 |
| **`--surface-pill`** | `surface_pill` | `rgba(255, 255, 255, 0.05)` | 胶囊按钮、药丸标签默认表面 |
| **`--border-subtle`** | `border_subtle` | `rgba(255, 255, 255, 0.08)` | 微弱边框，用于卡片分割 |
| **`--border-medium`** | `border_medium` | `rgba(255, 255, 255, 0.15)` | 中度边框，用于可交互控件与按钮外廓 |
| **`--border-highlight`** | `border_highlight` | `rgba(255, 255, 255, 0.30)` | 高亮焦点与选中态描边 |
| **`--text-primary`** | `text_primary` | `#f8fafc` | 主标题、核心数值与高强调文字 |
| **`--text-secondary`** | `text_secondary` | `#cbd5e1` | 次级标题与重要说明 |
| **`--text-muted`** | `text_muted` | `#94a3b8` | 功能描述、次要辅助文本 |
| **`--text-dim`** | `text_dim` | `#64748b` | 计量单位、微小提示与未激活状态 |
| **`--color-brand`** / `--accent-brand` | `accent_brand` | `#ff4438` | 主题品牌强调色（主Logo、版本徽章） |
| **`--color-office`** / `--mode-office` | `mode_office` | `#10b981` | 办公/节能模式强调色 (翡翠绿) |
| **`--color-balance`** / `--mode-balance` | `mode_balance` | `#06b6d4` | 均衡模式强调色 (冰青蓝) |
| **`--color-beast`** / `--mode-beast` | `mode_beast` | `#f97316` | 狂暴性能模式强调色 (炽焰橙) |
| **`--color-fan`** / `--accent-fan` | `accent_fan` | `#38bdf8` | 强冷散热/风扇状态色 (天空蓝) |
| **`--color-gold`** / `--accent-gold` | `accent_gold` | `#f59e0b` | 赞助者/特权/金牌状态色 (琥珀金) |
| **`--accent-active`** | `accent_active` | `#ff4438` | 交互控件当前激活态文本与强调色 |
| **`--accent-active-bg`** | `accent_active_bg` | `rgba(255, 68, 56, 0.12)` | 激活控件微光半透明背景底色 |
| **`--accent-active-border`**| `accent_active_border` | `rgba(255, 68, 56, 0.40)` | 激活控件微光外边框色 |
| **`--accent-heart`** | `accent_heart` | `#ef4444` | 危险操作/爱心赞助强调色 (绯红) |
| **`--tag-brand-text`** | `tag_brand_text` | `#ffffff` | 品牌高亮胶囊内文字前景色 |
| **`--radius-xs`** | `radius_xs` | `4px` | 超小徽章、状态点圆角 |
| **`--radius-sm`** | `radius_sm` | `6px` | 胶囊按钮、输入框小圆角 |
| **`--radius-md`** | `radius_md` | `8px` | 标准按钮、微调卡片圆角 |
| **`--radius-lg`** | `radius_lg` | `12px` | 大功能卡片、对话框外框圆角 |
| **`--font-family`** | `font_family` | `system-ui` | 全局首选文字字体族（支持中文无缝适配） |

---

## 2. 单文件三态切换规范 (Light / Dark / Mono Multi-Mode)

OpenRevo 主控制台顶栏提供了**主题色调三态切换按钮**（`Dark 深色 -> Light 浅色 -> Mono 版画黑白` 循环）。

皮肤插件无需针对浅色和暗色分别编写两个独立的插件文件夹，**完全可以在单份 CSS 或单份 `manifest.json` 中统一支持三套色调**：

### 方式 A：CSS 选择器作用域驱动（推荐）
在 `theme.css` 中，微内核解析引擎会智能根据当前模式提取对应的选择器规则块：

```css
/* 1. 浅色态 (Light) —— 顶栏切换为浅色时生效 */
:root,
html[data-skin],
[data-theme=light] {
  --bg-root: #F5EEDA;
  --surface-base: #FCF8EC;
  --surface-card: #FFFCF3;
  --text-primary: #35291C;
  --color-brand: #D9772F;
}

/* 2. 深色态 (Dark/Cyber) —— 顶栏切换为深色时生效 */
html[data-theme=cyber],
[data-theme=cyber],
[data-theme=dark] {
  --bg-root: #332B24;
  --surface-base: #3E342B;
  --surface-card: #42382E;
  --text-primary: #F7F0E5;
  --color-brand: #E79A50;
}

/* 3. 版画黑白 (Mono) —— 顶栏切换为 Mono 时生效 */
html[data-theme=mono],
[data-theme=mono] {
  --bg-root: #1F1D1A;
  --surface-base: #282522;
  --surface-card: #2E2A27;
  --text-primary: #FAF6F0;
  --color-brand: #FAF6F0;
}
```

### 方式 B：清单 JSON 声明式驱动
在 `manifest.json` 中直接按模式定义三个专属 Token 对象：

```json
{
  "id": "theme-paper-cartoon",
  "name": "纸质油画卡通皮肤",
  "plugin_type": "skin",
  "tokens_light": {
    "bg_root": "#F5EEDA",
    "surface_card": "#FFFCF3",
    "text_primary": "#35291C"
  },
  "tokens_dark": {
    "bg_root": "#332B24",
    "surface_card": "#42382E",
    "text_primary": "#F7F0E5"
  },
  "tokens_mono": {
    "bg_root": "#1F1D1A",
    "surface_card": "#2E2A27",
    "text_primary": "#FAF6F0"
  }
}
```
*当宿主检测到用户点击顶栏切换主题时，微内核将以毫秒级响应并向所有 Slint 窗口广播重新渲染，完全零黑屏、零闪烁。*

---

## 3. Slint 原生页面排版与滚动准则 (Slint Flickable Best Practices)

不同于浏览器的 HTML 文档流默认溢出滚动，**Slint 原生窗口不会自动为超出视口的元素添加滚动条**。

在设计或编写 Slint 原生组件/Tab 页（如 GPU 模式调控页、传感器长列表）时，必须遵守以下排版原则：

1. **纵向超长内容必须包裹在 `Flickable` 中**：
   ```slint
   import { ScrollBar } from "std-widgets.slint";

   export component MyLongPanel inherits Rectangle {
       flick := Flickable {
           x: 0;
           y: 0;
           width: parent.width;
           height: parent.height;
           viewport-y: -scrollbar.value;

           VerticalLayout {
               padding: 16px;
               spacing: 12px;
               // ... 业务卡片组
           }
       }

       scrollbar := ScrollBar {
           x: parent.width - 8px;
           y: 4px;
           width: 6px;
           height: parent.height - 8px;
           orientation: vertical;
           visible: flick.viewport-height > flick.height;
           maximum: flick.viewport-height - flick.height;
           page-size: flick.height;
       }
   }
   ```
2. **滚动条自适应隐现**：通过 `visible: flick.viewport-height > flick.height;` 控制滚动条仅在内容真实溢出时显示，保持界面干净灵动。

---

## 4. 原生表单与暗黑视窗避坑守则 (WebView2 Dark Mode Gotchas)

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
