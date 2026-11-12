# OpenRevo 开发者圣经：全场景插件与扩展开发手册 (Plugin Developer Bible)

> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

> **版本**：`v1.1 Official Specification`\
> **面向对象**：OpenRevo 社区极客、二次元/机甲皮肤设计师、独立客户端重构者、AI Agent 开发者、硬件自动化极客\
> **设计哲学**：基于微内核与能力网关，实现“按需分配运行时（Runtime on Demand）、物理沙盒隔离、可逆副作用、零污染与即插即用”。

本页是该手册的唯一索引。原整册手册已按章节与用例拆分到本目录，正文、代码块与表格完整保留；本页只负责导航与阅读顺序。

## 章节地图

| 章节 | 内容 | 阅读 |
| :--- | :--- | :--- |
| 1 | 微内核架构宣言：OpenRevo 的本质是什么？ | [架构与插件形态](architecture.md#1-微内核架构宣言openrevo-的本质是什么) |
| 2 | 插件形态与能力声明大一统矩阵 | [架构与插件形态](architecture.md#2-插件形态与能力声明大一统矩阵-capability-driven-architecture) |
| 2.1 | 正交混搭渲染架构（WebView2 + Slint） | [架构与插件形态](architecture.md#21-正交混搭渲染架构-orthogonal-hybrid-architecture-webview2--slint) |
| 3 | 核心安全军规与沙盒边界（Rule 4） | [核心安全军规](security.md#3-核心安全军规与沙盒边界-rule-4-硬件防火墙) |
| 4 | 插件清单权威规范（manifest） | [插件清单权威规范](manifest-reference.md#4-插件清单权威规范-plugin-manifest-reference) |
| 5 | 硬件 IPC 控制能力军火库速查 | [能力速查](capabilities.md#5-硬件-ipc-控制能力军火库速查-capabilities-cheat-sheet) |
| 6 | 自适应遥测与动态心跳总线 | [遥测总线](telemetry.md#6-自适应遥测与动态心跳总线-telemetry-bus--consumer-lifecycle) |
| 7 | 全域双向广播与跨插件事件总线 | [事件总线](events.md#7-全域双向广播通信与跨插件事件总线-universal-broadcast-bus) |
| 8 | 经典实战用例集（13 个用例） | [用例索引](use-cases/README.md#8-经典实战用例集-从桌面桌宠到无头脚本) |
| 9 | UI 设计语言与 Design Tokens | [UI 设计语言](ui-design.md#9-ui-设计语言与-design-tokens-复用指南) |
| 10 | 调试与安全模式排错 | [调试与排错](troubleshooting.md#10-调试与安全模式排错-safe-mode-二分法) |

## 实战用例（13）

| # | 用例 | 阅读 |
| :--- | :--- | :--- |
| 1 | 50 行代码开发一个桌面异形机甲桌宠 (Widget) | [阅读](use-cases/01-desktop-widget.md#用例-1-50-行代码开发一个桌面异形机甲桌宠-widget) |
| 2 | 桌面“一键强冷”大开关挂件 (Widget) | [阅读](use-cases/02-cooling-toggle-widget.md#用例-2-桌面一键强冷大开关挂件-widget) |
| 3 | 纯文本脚本自定义开机仪式灯效 (Headless Service - 零 Webview) | [阅读](use-cases/03-headless-boot-lighting.md#用例-3-纯文本脚本自定义开机仪式灯效-headless-service---零-webview) |
| 4 | 桌面 AI 助手 Function Calling 联动控制 | [阅读](use-cases/04-ai-function-calling.md#用例-4-桌面-ai-助手-function-calling-联动控制) |
| 5 | 完全重写前端！自建独立替代控制台 (Alternative Shell) | [阅读](use-cases/05-alternative-shell.md#用例-5-完全重写前端自建独立替代控制台-alternative-shell) |
| 6 | 进阶机甲桌宠工程规范 —— 边缘吸附灵动小球、动态换肤与跨插件事件总线 (Advanced Widget) | [阅读](use-cases/06-advanced-widget.md#用例-6-进阶机甲桌宠工程规范--边缘吸附灵动小球动态换肤与跨插件事件总线-advanced-widget) |
| 7 | 主窗口主题皮肤与插槽化布局重排 (Skin Plugin —— 一切皆插件) | [阅读](use-cases/07-skin-plugin.md#用例-7-主窗口主题皮肤与插槽化布局重排-skin-plugin--一切皆插件) |
| 8 | 独立背景壁纸插件与零代码壁纸包 (Background Plugin) | [阅读](use-cases/08-background-wallpaper.md#用例-8-独立背景壁纸插件与零代码壁纸包-background-plugin) |
| 9 | 硬件级伴随提权服务开发范式 (Sidecar Service Integration - 以 `ec-config` 为例) | [阅读](use-cases/09-sidecar-service.md#用例-9-硬件级伴随提权服务开发范式-sidecar-service-integration---以-ec-config-为例) |
| 10 | 原生体验第三方插件市场开发对接范式 (Plugin Market Integration - 以 `plugin-market` 为例) | [阅读](use-cases/10-plugin-market.md#用例-10-原生体验第三方插件市场开发对接范式-plugin-market-integration---以-plugin-market-为例) |
| 11 | 声明式零代码全局快捷键与按键宏映射插件 (Universal Shortcut Bus) | [阅读](use-cases/11-universal-shortcuts.md#用例-11-声明式零代码全局快捷键与按键宏映射插件-universal-shortcut-bus) |
| 12 | 声明式自动化监听与规则看门狗 (Universal Watcher & Trigger Hub —— 彻底根除隐藏桌宠偷跑轮询) | [阅读](use-cases/12-watcher-automation.md#用例-12-声明式自动化监听与规则看门狗-universal-watcher--trigger-hub--彻底根除隐藏桌宠偷跑轮询) |
| 13 | 正交混搭架构实战 —— WebView2 富交互大控制台与 Slint 原生极速常驻桌面挂件 (Hybrid Architecture) | [阅读](use-cases/13-hybrid-architecture.md#用例-13-正交混搭架构实战--webview2-富交互大控制台与-slint-原生极速常驻桌面挂件-hybrid-architecture) |

## 按任务选择

- **写第一个桌面挂件**：[用例 1](use-cases/01-desktop-widget.md) 与 [用例 2](use-cases/02-cooling-toggle-widget.md)；对照示例 [eva01-core](../../plugins_sample/webview2/eva01-core/README.md)
- **写原生 Slint 挂件**：示例 [slint-demo](../../plugins_sample/slint/slint-demo/) 与 [hybrid-battery](../../plugins_sample/slint/hybrid-battery/)，混搭规则见 [2.1 正交混搭渲染架构](architecture.md#21-正交混搭渲染架构-orthogonal-hybrid-architecture-webview2--slint)
- **做皮肤或背景壁纸**：[用例 7](use-cases/07-skin-plugin.md) 与 [用例 8](use-cases/08-background-wallpaper.md)；示例 [theme-cyber-mecha](../../plugins_sample/webview2/theme-cyber-mecha/README.md)
- **写无头自动化**：[用例 3](use-cases/03-headless-boot-lighting.md)、[用例 11](use-cases/11-universal-shortcuts.md) 与 [用例 12](use-cases/12-watcher-automation.md)；示例 [shortcut-fan-boost](../../plugins_sample/webview2/shortcut-fan-boost/README.md)
- **对接插件市场**：[用例 10](use-cases/10-plugin-market.md)
- **查清单字段**：[插件清单权威规范](manifest-reference.md)

## 示例与相关文档

- [插件示例源码索引](../../plugins_sample/README.md)：12 个随仓库分发的示例与清单入口
- 已有本地说明的示例：[cpu-guardian](../../plugins_sample/webview2/cpu-guardian/README.md)、[eva01-core](../../plugins_sample/webview2/eva01-core/README.md)、[shortcut-fan-boost](../../plugins_sample/webview2/shortcut-fan-boost/README.md)、[shortcut-workshop](../../plugins_sample/webview2/shortcut-workshop/README.md)、[theme-cyber-mecha](../../plugins_sample/webview2/theme-cyber-mecha/README.md)
- 插件开发 Skill（AI 工具入口）：[skill.md](../skill.md)
- 机型功耗、风扇曲线与 `models.json`：[机型配置与功耗预设定制指南](../core/models-customization.md)

## 接下来

- 第一次写插件：从 [架构与插件形态](architecture.md) 开始，再读 [插件清单权威规范](manifest-reference.md)
- 想直接跑起来：看 [用例索引](use-cases/README.md) 或 [插件示例源码索引](../../plugins_sample/README.md)
- 回到全部文档入口：[文档首页](../README.md)
