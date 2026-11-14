> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 8: 独立背景壁纸插件与零代码壁纸包 (Background Plugin)

> **目标**：不与任何 Fluent 皮肤互斥！用户可以单独启用一套背景壁纸插件，或者与任何主题皮肤自由叠用。

## 1. 核心架构：双通道独立注入机制
* **独立样式通道**：背景样式注入到 `<style id="openrevo-custom-background">`，状态持久化在 `config.json` 的 `active_background`。
* **独立开关控制**：在“插件”页中渲染为【背景壁纸】专属青色徽标，可独立开启/关闭，完全不影响当前激活的 `active_skin`。
* **自动全窗与抽屉双覆盖**：背景同时覆盖大面板（`.openrevo-shell`）和迷你桌面抽屉（`.mini-drawer-root`）。

## 2. 方式 A：编写自由 CSS 模式（可控透明度与遮罩）
```text
%APPDATA%\OpenRevo\plugins\bg-cyberpunk\
├── manifest.json
├── background.css
└── assets\
    └── wallpaper.jpg
```

**`manifest.json`**：
```json
{
  "id": "bg-cyberpunk",
  "name": "夜之城赛博朋克全息壁纸",
  "version": "1.0.0",
  "author": "NightCity Geek",
  "description": "高精度赛博朋克夜景全息壁纸，带 40% 磨砂遮罩保护文字可读性。",
  "plugin_type": "background",
  "background_css": "background.css"
}
```

**`background.css`**（直接写相对路径即可）：
```css
.openrevo-shell,
.mini-drawer-root {
  background-image:
    linear-gradient(rgba(11, 14, 20, 0.45), rgba(11, 14, 20, 0.65)),
    url("assets/wallpaper.jpg") !important;
  background-size: cover !important;
  background-position: center !important;
  background-repeat: no-repeat !important;
}
```

## 3. 方式 B：极简零代码模式（连一行 CSS 都不需要写！）
如果你只想让一张好看的二次元壁纸生效，**完全不需要写 `background.css`**！
只需在插件根目录下放入图片，并在 `manifest.json` 里声明：

```json
{
  "id": "bg-minimal",
  "name": "极简极客壁纸",
  "version": "1.0.0",
  "author": "OpenRevo Fan",
  "description": "零代码快速壁纸包",
  "plugin_type": "background",
  "image": "wallpaper.png"
}
```
* **全自动合成**：OpenRevo 内核若未检测到 `background.css`，会自动寻找 `manifest.image` 或目录下的 `background.png/jpg/webp`、`wallpaper.jpg`、`bg.png`，在 Rust 内存中自动生成全套适配 CSS（满屏铺满、居中不重复）注入宿主！

## 4. 迷你抽屉专属壁纸与双图独立适配（Main & Drawer 双图机制）

OpenRevo 拥有两种形态的界面：**920px 宽屏主控制台** 与 **竖向便携迷你抽屉 (Drawer)**。
由于抽屉是竖向布局，很多创作者在做图时会为了更贴合专门给抽屉单做一张竖版插画。宿主对此提供了完全解耦的双图机制：

### 规则 A：单图共用（默认零配置）
如果插件仅提供了一张图（如 `image: "wallpaper.png"` 或根目录下仅有 `wallpaper.png`），主窗口与迷你抽屉将**无缝共享同一张壁纸**，内存中自动复用单次解码对象，极速省流。

### 规则 B：双图独立呈现
如果创作者为抽屉单独设计了壁纸，只需声明 `drawer_image`（或在目录放置 `drawer.png`），两张壁纸即可分开生效：

```json
{
  "id": "bg-nacho-neko",
  "name": "Nacho Neko 双形态壁纸",
  "version": "1.1.0",
  "plugin_type": "background",
  "image": "assets/main_horizontal.png",
  "bg_image_opacity": 0.25,
  "drawer_image": "assets/drawer_vertical.png",
  "drawer_opacity": 0.35
}
```

* **字段别名支持**：
  - 主图支持：`image` / `wallpaper` / `main_image`；
  - 抽屉图支持：`drawer_image` / `drawer_wallpaper` / `drawer_bg`；
  - 抽屉透明度支持：`drawer_opacity` / `drawer_bg_image_opacity`（若未指定，自动 fallback 继承主图透明度）。
* **自动隐式文件嗅探**：
  若未在 manifest 中显式声明，宿主会自动在插件目录下探测 `drawer.png`、`drawer.jpg`、`drawer_bg.png`、`drawer-bg.png` 作为抽屉专属图。

---

## 相关页面

- 上一页：[用例 7：主窗口主题皮肤与插槽化布局重排 (Skin Plugin —— 一切皆插件)](07-skin-plugin.md)
- 下一页：[用例 9：硬件级伴随提权服务开发范式 (Sidecar Service Integration - 以 `ec-config` 为例)](09-sidecar-service.md)
- 返回用例索引：[经典实战用例集](README.md)
