> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 10: 原生体验第三方插件市场开发对接范式 (Plugin Market Integration - 以 `plugin-market` 为例)

> **目标**：打造一个生态插件市场选项卡（`plugin_type: "tab"`），支持用户在线浏览、一键下载安装、一键卸载、免重启热生效、外链跳转与复制。

## 1. 检查宿主能力契约 (`get_plugin_host_capabilities`)
在市场启动时，调用能力探测接口：
```javascript
import { invoke } from '@tauri-apps/api/core';

async function checkHost() {
  const caps = await invoke('get_plugin_host_capabilities');
  console.log(`宿主版本: ${caps.version}, 架构: ${caps.arch}`);

  // 检查是否支持伴随服务与市场安装器
  if (!caps.capabilities.includes('market_installer')) {
    alert('当前 OpenRevo 版本过低，请升级后使用在线安装功能！');
  }
}
```

## 2. 在线插件一键安全静默安装 (`install_plugin_from_url`)
```javascript
async function installPlugin(pluginMeta) {
  try {
    toast(`正在拉取并安装 ${pluginMeta.name}...`);

    // 宿主底层校验 HTTPS、防路径穿越、自动解压至 plugins 目录并热加载
    const newPlugin = await invoke('install_plugin_from_url', {
      url: pluginMeta.download_url,
      expectedSha256: pluginMeta.sha256 // 可选防篡改哈希
    });

    toast(`安装成功！插件 ID: ${newPlugin.id}`);
    refreshInstalledList();
  } catch (err) {
    alert(`安装失败: ${err}`);
  }
}
```

## 3. 安全卸载送入回收站 (`uninstall_plugin`)
```javascript
async function uninstallPlugin(pluginId) {
  if (!confirm(`确定要卸载插件 ${pluginId} 吗？`)) return;

  try {
    // 自动停止伴随服务、清理配置开关、并送入系统回收站
    await invoke('uninstall_plugin', { pluginId });
    toast('插件已移入回收站并卸载');
    refreshInstalledList();
  } catch (err) {
    alert(`卸载失败: ${err}`);
  }
}
```

## 4. 统一开关控制 (`set_plugin_enabled_unified`)
无需针对主题皮肤、背景壁纸与扩展组件分别写三套 IPC 分支，统一调用：
```javascript
async function togglePlugin(pluginId, targetEnabled) {
  // 宿主自动区分 skin/background/功能插件 并正确更新配置
  await invoke('set_plugin_enabled_unified', {
    pluginId: pluginId,
    enabled: targetEnabled
  });
}
```

## 5. 安全外链与剪贴板突破
```javascript
// 在默认浏览器中打开作者 GitHub 主页（绕过沙盒 iframe 无法 window.open 限制）
async function openAuthorLink(url) {
  await invoke('open_external_url', { url });
}

// 复制安装命令或链接到系统剪贴板（绕过沙盒限制）
async function copyLink(text) {
  await invoke('write_clipboard', { text });
  toast('已复制到剪贴板');
}
```

---

## 相关页面

- 上一页：[用例 9：硬件级伴随提权服务开发范式 (Sidecar Service Integration - 以 `ec-config` 为例)](09-sidecar-service.md)
- 下一页：[用例 11：声明式零代码全局快捷键与按键宏映射插件 (Universal Shortcut Bus)](11-universal-shortcuts.md)
- 返回用例索引：[经典实战用例集](README.md)
