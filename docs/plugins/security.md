> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 3. 核心安全军规与沙盒边界 (Rule 4 硬件防火墙)

笔记本硬件涉及高压供电、功耗墙释放与风扇转速。任何代码错误或越权写入都可能导致主板断电甚至硬件损坏。所有插件开发必须遵守以下铁律：

1. **绝对禁止裸物理寄存器交互（Rule 4 硬件资产钥匙库守则）**：
   * 严禁在插件中请求类似 `ec_write(0x1844, 0x01)` 或 dump 整块 EC RAM。
   * OpenRevo 仅对外暴露**“语义化硬件控制指令”**（如 `set_fan_boost(true)`）。
2. **硬件物理边界防呆（Guardrails）**：
   * 功耗墙调节范围由机型固件严格钳位（例如 PL4 必须 `<= 255W`，防 0xFF 翻转单字节溢出为 45W 慢充）。
   * 电池养护限额严格限制在 `60% ~ 100%` 之间。
3. **非侵入式执行（Non-blocking）**：
   * 严禁在主线程执行耗时同步阻塞；网络请求与大模型调用必须采用异步流式处理。
4. **插件删除必须走系统回收站（Recycle Bin）**：
   * 卸载或删除插件目录时，必须调用 Windows Shell 回收站 API（如 `uninstall_plugin` 的既有行为），保证用户可恢复。
   * 严禁永久删除，严禁使用 `cmd` 通配符批量删除（如 `rd /s /q`、`del /f /s /q`）。能力细节见 [capabilities.md](capabilities.md)。

---

## 相关页面

- 上一页：[架构与插件形态](architecture.md)
- 下一页：[插件清单权威规范](manifest-reference.md)
- 返回目录：[插件与扩展导览](README.md)
