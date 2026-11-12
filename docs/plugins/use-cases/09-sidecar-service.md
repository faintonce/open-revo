> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 9: 硬件级伴随提权服务开发范式 (Sidecar Service Integration - 以 `ec-config` 为例)

> **痛点背景**：某些硬件调优插件（如 EC 物理寄存器读写、PBO 压频调压代理）需要依赖后台常驻的原生执行文件（如 Go / C++ / Python / Node.js 打包的 `.exe`），以前需要让用户手动双击 `启动服务.bat` 弹出一个丑陋且易误关闭的 CMD 黑框。
> **全新解法**：借助 OpenRevo 内置的伴随服务监督器（Companion Service Supervisor），声明即托管！

## 1. 清单声明 (`manifest.json`)
```json
{
  "id": "ec-config",
  "name": "EC 物理配置工具",
  "version": "1.0.0",
  "plugin_type": "tab",
  "entry": "dist/index.html",
  "service": {
    "executable": "agent/ec_service.exe",
    "args": ["--port", "18945", "--silent"],
    "port": 18945,
    "elevated": true,
    "auto_start": false,
    "auto_stop": true,
    "description": "EC 物理寄存器桥接服务"
  }
}
```

## 2. 前端界面一键启停与通信 (`app.js`)
在插件的沙盒 `iframe` 内，通过标准 `postMessage` 与宿主双向桥接，无需直接接触底层进程 API：

```javascript
// 1. 发起启动服务
document.getElementById('start-btn').addEventListener('click', () => {
  window.parent.postMessage({ action: 'START_PLUGIN_SERVICE' }, '*');
});

// 2. 发起停止服务
document.getElementById('stop-btn').addEventListener('click', () => {
  window.parent.postMessage({ action: 'STOP_PLUGIN_SERVICE' }, '*');
});

// 3. 监听宿主下发的状态同步事件
window.addEventListener('message', (event) => {
  if (event.data?.type === 'OPENREVO_PLUGIN_SERVICE_STATUS') {
    const { status } = event.data;
    const statusLabel = document.getElementById('srv-status');

    if (status.running) {
      if (status.healthy) {
        statusLabel.textContent = `🟢 服务已就绪 (端口 ${status.port}, PID ${status.pid})`;
        statusLabel.className = 'status-ok';
      } else {
        statusLabel.textContent = `🟡 正在探活回环端口...`;
        statusLabel.className = 'status-warn';
      }
    } else {
      statusLabel.textContent = status.last_error
        ? `🔴 启动失败: ${status.last_error}`
        : `⚪ 服务已停止`;
      statusLabel.className = status.last_error ? 'status-error' : 'status-dim';
    }
  }
});

// 4. 页面初始化时主动查询一次当前状态
window.parent.postMessage({ action: 'GET_PLUGIN_SERVICE_STATUS' }, '*');
```

## 3. 宿主原生特性与安全保障
* **宿主顶栏原生联动**：无论前端是否写了控制按钮，宿主在选项卡右上方自动渲染伴随服务胶囊指示器，用户可随时点击一键启停。
* **无黑框提权与进程树防孤儿**：宿主使用 `CREATE_NO_WINDOW` 静默拉起，并在宿主退出时通过 `TerminateProcess` 彻底回收，防止用户任务管理器中残留游荡进程。

---

## 相关页面

- 上一页：[用例 8：独立背景壁纸插件与零代码壁纸包 (Background Plugin)](08-background-wallpaper.md)
- 下一页：[用例 10：原生体验第三方插件市场开发对接范式 (Plugin Market Integration - 以 `plugin-market` 为例)](10-plugin-market.md)
- 返回用例索引：[经典实战用例集](README.md)
