# 奶油纸杯 (Slint 版三态皮肤插件样例) · Paper & Oil Cartoon

> 基于 ZTN-Silent 原创浅色纸质油画卡通配色移植。
> **适用引擎**：Slint 原生主窗口 (`openrevo-main`)、Slint 原生桌面 OSD 悬浮窗 (`openrevo-osd`)、WebView2 混合容器。

---

## 插件特色

1. **单文件三态循环切换**：
   - 用户点击控制台顶栏的主题按钮时，微内核会自动在三种风格之间实时平滑切换：
     - **Light（浅色奶油米纸）**：米黄纸质底色 (`#F5EEDA`) + 暖棕墨色字体 (`#35291C`) + 琥珀橙强调 (`#D9772F`)。
     - **Dark（深焙奶茶可可）**：深焙咖啡底色 (`#332B24`) + 燕麦白字体 (`#F7F0E5`) + 暖金橙强调 (`#E79A50`)。
     - **Mono（复古报纸版画）**：版画黑底 (`#1F1D1A`) + 纯白反差文字 (`#FAF6F0`)，专注硬核性能。
2. **双引擎无损驱动**：
   - 支持通过 `manifest.json` 内声明的 `tokens_light` / `tokens_dark` / `tokens_mono` 驱动；
   - 同时也包含标准 `theme.css`，方便在传统 WebView2 模式下获得 100% 相同的高保真视觉体验。
3. **桌面 OSD 联动**：
   - 随主窗口皮肤激活，快捷键弹出的 OSD 悬浮窗也会自动继承奶油/奶茶色底盘与状态指示灯。

---

## 目录结构

```text
theme-cream-cup/
├── manifest.json      # 核心元数据与三态 Tokens 声明
├── theme.css          # CSS 选择器作用域样式表
└── README.md          # 插件说明文档
```

## 测试与体验

1. 将本目录复制到 `%APPDATA%\OpenRevo\plugins\theme-cream-cup`；
2. 启动 OpenRevo，在“插件管理”或系统设置中启用“奶油纸杯 (Slint版)”；
3. 点击主界面顶栏右上角的主题按钮，观察 Light / Dark / Mono 三态切换效果。
