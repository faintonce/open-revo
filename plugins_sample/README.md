# OpenRevo 插件示例源码

这里索引随仓库分发的**可运行示例代码**，用于对照 [插件与扩展开发手册](../docs/plugins/README.md) 与实际清单。
每行进一个示例目录；「清单入口」列为 `manifest.json` 中声明的入口文件，「文档」列标出该示例是否自带本地 README。

| 示例 | 引擎 | 清单入口 | 文档 |
| :--- | :--- | :--- | :--- |
| [cpu-guardian](webview2/cpu-guardian/) | WebView2 | 根 `entry`: `index.html`，另含 `service` 伴随提权服务 | [本地 README](webview2/cpu-guardian/README.md) |
| [eva01-core](webview2/eva01-core/) | WebView2 | 根 `entry`: `index.html`（`plugin_type: widget`） | [本地 README](webview2/eva01-core/README.md) |
| [shortcut-fan-boost](webview2/shortcut-fan-boost/) | 无 GUI（`service`） | 无入口文件，纯声明式 `shortcuts` 无头服务 | [本地 README](webview2/shortcut-fan-boost/README.md) |
| [shortcut-workshop](webview2/shortcut-workshop/) | WebView2 | 根 `entry`: `index.html` | [本地 README](webview2/shortcut-workshop/README.md) |
| [theme-cyber-mecha](webview2/theme-cyber-mecha/) | 宿主 WebView2（`skin`） | `theme_css`: `theme.css`，无页面入口 | [本地 README](webview2/theme-cyber-mecha/README.md) |
| [battery-hardware-limit](webview2/battery-hardware-limit/) | WebView2 | 未写 `entry`，依赖宿主隐式 `index.html` | 缺失，暂无本地 README |
| [gpu-curve-tuner](webview2/gpu-curve-tuner/) | WebView2 | 根 `entry`: `index.html` | 缺失，暂无本地 README |
| [tongfang-cloud-icc](webview2/tongfang-cloud-icc/) | WebView2 | 根 `entry`: `index.html` | 缺失，暂无本地 README |
| [slint-demo](slint/slint-demo/) | Slint | 根 `entry` 与 `tab.entry`: `main.slint` | 缺失，暂无本地 README |
| [hybrid-battery](slint/hybrid-battery/) | WebView2 + Slint 混搭 | `tab.entry`: `dist/index.html`（当前缺失），`widget.entry`: `widget.slint` | 缺失，暂无本地 README |
| [revo-llm-hub](webview2/revo-llm-hub/) | WebView2 | 根 `entry`: `index.html`（AI LLM 工具中枢与硬件直调） | [本地 SKILL.md](webview2/revo-llm-hub/SKILL.md) |
| [revo-pet-companion](webview2/revo-pet-companion/) | WebView2 | 根 `entry`: `index.html`（透明桌面桌宠与状态互动） | [本地 SKILL.md](webview2/revo-pet-companion/SKILL.md) |

## 说明

- 上表 12 个示例中包含丰富的设计形态，各示例附带的 README/SKILL 或清单均可作为二次开发模板。
- `hybrid-battery` 的 `tab` 声明了 `dist/index.html`，但该构建产物当前不在仓库中；Slint 挂件部分可直接运行。
- `battery-hardware-limit` 的清单未写 `entry`，由宿主回退到隐式 `index.html`。
- `shortcut-fan-boost` 是无头服务示例，没有页面入口，只声明全局快捷键。

## 接下来

- 按任务挑选示例与阅读顺序：[插件与扩展导览](../docs/plugins/README.md)
- 清单字段与能力声明：[插件清单权威规范 · 第 4 章](../docs/plugins/manifest-reference.md#4-插件清单权威规范-plugin-manifest-reference)
- 回到全部文档入口：[文档首页](../docs/README.md)
