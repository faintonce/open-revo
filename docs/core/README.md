# 核心引擎与 SDK

这一节面向**要把 OpenRevo 硬件能力接进自己程序**的开发者：C# / WinUI 3、WPF、C / C++、Python、Qt 或 Linux 控制台。
它回答三件事：引擎导出什么、怎么链接、升级会不会破坏既有代码。机型参数与功耗预设的调整不属于 SDK 接入，见下方机型指南。

## 先读

- [OpenRevo 核心引擎 SDK 接入与生态开发指南](sdk-integration.md)
- [机型配置与功耗预设定制指南](models-customization.md)

## SDK 指南章节地图

| 章节 | 你会得到什么 | 直达 |
| :--- | :--- | :--- |
| 1. 生态架构与设计理念 | 微内核分层、HAL 与第三方前端的关系 | [跳转](sdk-integration.md#1-生态架构与设计理念) |
| 2. 动态库构建与文件说明 | `cargo build -p openrevo-ffi` 产物与预置接入文件 | [跳转](sdk-integration.md#2-动态库构建与文件说明) |
| 3. C# / WinUI 3 极速接入指南 | P/Invoke 封装类的引入、配置与调用示例 | [跳转](sdk-integration.md#3-c--winui-3-极速接入指南) |
| 4. C / C++ 原生开发接入指南 | 头文件、回调注册与字符串释放约定 | [跳转](sdk-integration.md#4-c--c-原生开发接入指南) |
| 5. Linux 移植与跨平台开发说明 | `ec_sys` 读写节点与 NVML 显卡控制 | [跳转](sdk-integration.md#5-linux-移植与跨平台开发说明) |
| 6. C-ABI 接口参考手册 | 全部导出函数、参数与内存生命周期 | [跳转](sdk-integration.md#6-c-abi-接口参考手册) |
| 7. 平滑热替换升级准则 | 大版本一致的 drop-in 升级边界 | [跳转](sdk-integration.md#7-平滑热替换升级准则-drop-in-replacement) |

机型功耗、风扇曲线与 `models.json` 字段，见 [机型配置与功耗预设定制指南](models-customization.md)。

## 相关代码

- [openrevo-ffi crate README](../../src-tauri/crates/openrevo-ffi/README.md)：crate 级快速上手与实现细节
- [C 头文件 `openrevo.h`](../../src-tauri/crates/openrevo-ffi/include/openrevo.h)
- [C# 封装类 `OpenRevoSdk.cs`](../../src-tauri/crates/openrevo-ffi/bindings/csharp/OpenRevoSdk.cs)

## 接下来

- 想让自己的界面变成插件而不是独立程序：[插件与扩展导览](../plugins/README.md)
- 回到全部文档入口：[文档首页](../README.md)
