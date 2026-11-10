> 文档首页：[docs/README.md](../README.md) · 插件与扩展：[plugins/README.md](README.md)

# 10. 调试与安全模式排错 (Safe Mode 二分法)

开发插件时，难免遇到死循环或语法错误。OpenRevo 建立了**实机 5 秒二分法排错机制**：

## 异常自愈指南
当第三方插件异常导致崩溃闪退时，**绝对无需重装 OpenRevo**：
1. 打开记事本：`%APPDATA%\OpenRevo\config.json`；
2. 找到 `custom_plugin_toggles` 节点；
3. 将疑似崩溃的插件 ID 改为 `false`：
   ```json
   "custom_plugin_toggles": {
     "faulty-plugin": false
   }
   ```
4. 保存并重新打开 OpenRevo，主内核将跳过该插件，秒级恢复稳定运行！

---

## 相关页面

- 上一页：[UI 设计语言与 Design Tokens](ui-design.md)
- 返回目录：[插件与扩展导览](README.md)
