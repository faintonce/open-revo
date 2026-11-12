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

---

## 相关页面

- 上一页：[用例 7：主窗口主题皮肤与插槽化布局重排 (Skin Plugin —— 一切皆插件)](07-skin-plugin.md)
- 下一页：[用例 9：硬件级伴随提权服务开发范式 (Sidecar Service Integration - 以 `ec-config` 为例)](09-sidecar-service.md)
- 返回用例索引：[经典实战用例集](README.md)
