# OpenRevo 文档

本目录是 OpenRevo 的公开文档入口：**机主**在这里找安装、调优与排错说明，**开发者**在这里进入核心引擎 SDK 与插件开发体系。
文档按主题分目录组织，正文保留在各自页面中，这里只提供导航与阅读顺序。

## 快速开始

- [项目总览、下载与源码构建](../README.md)
- [机型配置与功耗预设定制指南](core/models-customization.md)
- [插件示例源码索引](../plugins_sample/README.md)

## 核心引擎与 SDK

- [核心引擎与 SDK 导览](core/README.md)：这一节的入口与阅读顺序
- [Core SDK 接入与生态开发指南](core/sdk-integration.md)：C# / WinUI 3、C / C++、Linux 与 C-ABI
- [openrevo-ffi crate 级实现细节](../src-tauri/crates/openrevo-ffi/README.md)

## 插件与扩展

- [插件与扩展导览](plugins/README.md)：手册章节地图、13 个实战用例与任务分流
- [AI Agent 文档入口](skill.md)：AI Agent 从这里开始，再按任务分流到各章节
- [插件示例源码索引](../plugins_sample/README.md)

## 参考资料

- [变更日志](../CHANGELOG.md)
- [开源协议](../LICENSE)

## 接下来

- 只想先把软件跑起来：[项目总览与安装](../README.md)
- 要写插件：[插件与扩展导览](plugins/README.md)
- 要接入引擎：[核心引擎与 SDK 导览](core/README.md)
- 想先看能跑的东西：[插件示例源码索引](../plugins_sample/README.md)
