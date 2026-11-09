> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 7: 主窗口主题皮肤与插槽化布局重排 (Skin Plugin —— 一切皆插件)

> **目标**：不修改 OpenRevo 任何一行业务逻辑代码，仅通过纯 CSS 与声明式配置，彻底改变 OpenRevo 主窗口的视觉风格、色系基调、磨砂亚克力深度，并利用 CSS Grid 插槽重排仪表盘模块，甚至重新定义主窗口的初始物理尺寸与缩放能力。

## 1. 核心设计原则：为什么“皮肤即插件”？
* **统一目录规范**：严禁创建孤立的 `skins/` 文件夹。所有皮肤均存放在 `%APPDATA%\OpenRevo\plugins\<skin_id>\`，在主控制台“插件管理”页享有统一的启闭开关、版本展示与元数据治理。
* **物理级 0 内存开销**：皮肤不创建独立 Webview 进程，纯粹将 `theme.css` 注入宿主主窗口的 `<style id="openrevo-theme-skin">`，实现毫秒级热装卸与零内存损耗。
* **独立单选激活**：皮肤管理属于主题通道，同一时间只激活一个主窗口皮肤，停用后无缝回退至默认主题与 960×740 视窗基准。
* **资产自动内联（零手动 Base64）**：在 CSS 中直接写标准相对路径（如 `url('assets/bg.webp')`），Rust 后端在读取时**自动在内存中无损解析并内联为 Base64 Data URI**，开发者绝对无须使用外部 Python 脚本提前转码！
* **全局穿透（Global Root Penetration）**：激活皮肤时，宿主会在 `document.documentElement`（`<html>` 根节点）上全量同步 `data-skin="<skin_id>"`。样式选择器可以直通所有浮动弹窗（`.sys-modal-card`）、菜单与 Tooltip，绝无样式脱节逃逸。

## 2. 皮肤工程目录规范
```text
%APPDATA%\OpenRevo\plugins\mecha-eva01-skin\
├── manifest.json              # 核心元数据与主窗口视窗几何声明
├── theme.css                  # 皮肤全量样式表 (覆写 Tokens 与 Grid 插槽)
└── assets\                    # 本地背景贴图与字体资产 (直接在 CSS 中通过相对路径引用)
    ├── eva01_bg.webp
    └── neon_core.svg
```

## 3. 插件清单声明 (`manifest.json`)
```json
{
  "id": "mecha-eva01-skin",
  "name": "EVA 初号机觉醒皮肤",
  "version": "1.0.0",
  "author": "NERV Engineering",
  "description": "初号机标志性紫绿觉醒电竞配色，自适应宽屏排版，支持传感器插槽重排与全息背景贴图。",
  "plugin_type": "skin",
  "theme_css": "theme.css",
  "window": {
    "width": 1080,
    "height": 780,
    "resizable": true
  }
}
```
* **`plugin_type: "skin"`**（或 `"type": "skin"`）：声明为主控制台皮肤，在插件面板中渲染为【主窗皮肤】专属角标。
* **`theme_css`**：指定相对于插件目录的 CSS 入口（默认 `"theme.css"`）。
* **`window.width / height`**：声明该皮肤的最佳版面尺寸。当用户切换到此皮肤时，微内核自动触发 `resize_window` 平滑调整主窗口几何尺寸并自动屏幕居中。
* **`window.resizable: true`**：若该皮肤支持大屏响应式布局，可解锁主窗口的自由拖拽缩放能力。

## 4. 全界面标准插槽矩阵速查 (Slot Matrix Contract)
OpenRevo 主控制台各个界面骨架与内容区块均具有**绝对稳定的语义类名与 `data-slot` 属性契约**，开发者可放心针对以下标准插槽进行样式重塑或网格重排：

| 语义插槽类名 | `data-slot` 标记 | 默认定位与布局特性 | 说明 |
| :--- | :--- | :--- | :--- |
| **`.openrevo-shell`** | `shell` | Flex Column (支持切换为 Grid) | 主窗体总外壳容器 (100vw × 100vh 满高) |
| **`.shell-titlebar`** | `titlebar` | Flex (高度固定 ~44px, `flex-shrink: 0`) | 顶栏标题栏、Logo 与窗口操作控件区 |
| **`.shell-navbar`** | `navbar` | Flex (横向滚动胶囊栏, `flex-shrink: 0`) | 顶栏 Sub-Nav 插件与核心模块页签切换栏 |
| **`.shell-viewport`** | `viewport` | Flex 1 (`flex: 1 1 0%`, 纵向独立滚动) | 核心视口容器，吞满剩余垂直高度 |
| **`.overview-slot-grid`** | `overview-grid`| Flex Column (`gap: 16px`) | 状态概览页的主网格容器 |
| **`.overview-slot-telemetry`**| `telemetry` | Block | 顶部遥测监控区插槽 |
| **`.telemetry-cards-grid`** | `telemetry-grid`| Grid (`grid-template-areas: "cpu gpu fan vram"`) | 4 联硬件监控卡片对称网格 |
| **`.telemetry-card-cpu`** | `cpu` | `grid-area: cpu` | CPU 温度/功耗/频率/负载卡片 |
| **`.telemetry-card-gpu`** | `gpu` | `grid-area: gpu` | 独立 GPU 温度/功耗/频率/负载卡片 |
| **`.telemetry-card-fan`** | `fan` | `grid-area: fan` | 双通道散热风扇 RPM 与强冷状态卡片 |
| **`.telemetry-card-vram`**| `vram` | `grid-area: vram` | 显存占用量与 GDDR 负载卡片 |
| **`.overview-slot-power`** | `power` | Block | 性能释放模式 (办公/均衡/狂暴/自定义) 插槽 |
| **`.overview-slot-quickactions`** | `quickactions` | Block | 中部高频微调 (强冷/离电低刷/刷新率) 插槽 |
| **`.overview-slot-switches`** | `switches` | Block | 底部 9 联原厂电竞级外设开关网格插槽 |
| **`.mini-drawer-root`** | `mini-root` | Flex Column / 迷你抽屉 | 迷你悬浮抽屉主容器 |
| **`.mini-slot-*`** | `header`...`footer` | 8 联插槽区域 | 包含 `.mini-slot-header`, `.mini-slot-power`, `.mini-slot-gpu`, `.mini-slot-display`, `.mini-slot-lighting`, `.mini-slot-battery`, `.mini-slot-switches`, `.mini-slot-footer` |
| **`.osd-root`** | `osd-root` | Flex (100vw × 100vh) | OSD 屏幕弹窗最外层容器（含 `data-osd-pos`） |
| **`.osd-container`** | `osd-container` | Inline-Flex | OSD 弹窗卡片本体（含 `data-osd-theme`） |
| **`.osd-slot-icon`** | `icon` | Flex 居中 | OSD 左侧图标与背底底盘插槽 |
| **`.osd-slot-content`**| `content` | Flex 水平居中 | OSD 右侧文本内容与进度槽插槽 |
| **`.osd-slot-title`** | `title` | Flex | OSD 核心标题大字插槽 |
| **`.osd-slot-dot`** | `status-dot` | Block (圆形点) | OSD 开启/关闭指示状态呼吸点插槽 |
| **`.osd-slot-meter`** | `meter` | Flex (间距 4px) | OSD 键盘背光/阶梯档位进度条插槽 |
| **`.osd-slot-meter-bar`**| `meter-bar`| Block (含 `data-level` 与 `data-active`) | OSD 档位独立小格插槽 |

## 5. 样式编写实战与插槽重排 (`theme.css`)
所有选择器以 `[data-skin="<skin_id>"]` 或 `html[data-skin="<skin_id>"]` 作为根命名空间：

```css
/* ============================================================
   1. 覆写 Design System Tokens (色彩/圆角/光效基调)
   ============================================================ */
html[data-skin="mecha-eva01-skin"] {
  /* 品牌强调色：初号机装甲紫与荧光绿 */
  --accent-pwr: #a855f7;
  --accent-pwr-dim: rgba(168, 85, 247, 0.2);
  --accent-pwr-border: rgba(168, 85, 247, 0.45);
  --accent-beast: #22c55e;

  /* 表面层级：深化暗夜紫亚克力 */
  --surface-base: rgba(10, 6, 18, 0.94);
  --surface-overlay: rgba(16, 10, 26, 0.85);
  --surface-card: rgba(26, 16, 42, 0.75);
  --border-subtle: rgba(168, 85, 247, 0.2);
  --border-medium: rgba(168, 85, 247, 0.4);
  --border-highlight: #22c55e;

  /* 圆角与字体 */
  --radius-md: 4px;
  --radius-lg: 6px;
  --font-mech: 'Orbitron', 'Chakra Petch', sans-serif;
}

/* ============================================================
   2. 主容器全息贴图 (直接使用相对路径，Rust 运行时自动内存内联)
   ============================================================ */
[data-skin="mecha-eva01-skin"] .openrevo-shell {
  background-image:
    radial-gradient(circle at 90% 10%, rgba(34, 197, 94, 0.15), transparent 40%),
    radial-gradient(circle at 10% 90%, rgba(168, 85, 247, 0.18), transparent 50%),
    url("assets/eva01_bg.webp") !important;
  background-size: cover !important;
  background-position: center !important;
}

/* ============================================================
   3. 状态概览页：颠倒 4 联硬件监控卡片顺序 (Grid Areas 实战)
   ============================================================ */
/* 仅用一行 CSS 将显卡与显存置于最左侧，实现极客战术 HUD 布局 */
[data-skin="mecha-eva01-skin"] .telemetry-cards-grid {
  grid-template-areas: "gpu vram cpu fan" !important;
}

/* ============================================================
   4. 模块垂直次序重排 (Flex Order 实战)
   ============================================================ */
/* 让底部 9 联快捷开关插槽上浮到性能模式之前 */
[data-skin="mecha-eva01-skin"] .overview-slot-switches {
  order: 1;
}
[data-skin="mecha-eva01-skin"] .overview-slot-power {
  order: 2;
}

/* ============================================================
   5. 全局弹窗穿透修饰 (Portal Penetration 实战)
   ============================================================ */
html[data-skin="mecha-eva01-skin"] .sys-modal-card {
  border-color: #a855f7 !important;
  box-shadow: 0 0 35px rgba(168, 85, 247, 0.35) !important;
}

/* ============================================================
   6. OSD 屏幕快捷弹窗联动修饰 (OSD Slot Customization 实战)
   ============================================================ */
html[data-skin="mecha-eva01-skin"] .osd-container {
  background: rgba(10, 6, 18, 0.92) !important;
  border: 1.5px solid #22c55e !important;
  border-radius: 4px !important; /* 战术机甲切角 */
  box-shadow: 0 0 25px rgba(34, 197, 94, 0.35) !important;
}
html[data-skin="mecha-eva01-skin"] .osd-slot-title {
  color: #22c55e !important;
  font-family: 'Orbitron', monospace !important;
  letter-spacing: 1.2px !important;
}
html[data-skin="mecha-eva01-skin"] .osd-slot-meter-bar.active {
  background: #a855f7 !important; /* 装甲紫指示条 */
  box-shadow: 0 0 8px #a855f7 !important;
}
```

---

## 相关页面

- 上一页：[用例 6：进阶机甲桌宠工程规范 —— 边缘吸附灵动小球、动态换肤与跨插件事件总线 (Advanced Widget)](06-advanced-widget.md)
- 下一页：[用例 8：独立背景壁纸插件与零代码壁纸包 (Background Plugin)](08-background-wallpaper.md)
- 返回用例索引：[经典实战用例集](README.md)
