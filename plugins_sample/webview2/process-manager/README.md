# 进程管理 (Process Manager)

> **类型**：tab 插件（WebView2 内嵌面板）+ 本地伴随服务 `process-service.exe`
> **模块**：进程 · 连接与端口 · 文件占用 · 监听端口自定义

---

## 概述

提供系统级检视能力：进程管理、网络连接表、文件占用排查。
前端只做渲染，所有数据来自本地伴随服务 `process-service.exe`（默认 127.0.0.1:18956，可在「端口」页签自定义），
由宿主按 manifest `service` 配置托管启停（`elevated: true`，随守护进程以管理员运行）。

## 模块

| 页签 | 能力 | 数据通道 |
|---|---|---|
| 进程 | 全进程表（CPU%/内存/路径/父 PID）、搜索、点列排序、结束进程 / 结束进程树 | `GET /api/processes`、`POST /api/process/kill` |
| 连接 | TCP/UDP 全量端点表（含 PID 与进程名）、状态着色、按端口/进程搜索、TCP 强制断连（DELETE_TCB） | `GET /api/connections`、`POST /api/connections/close` |
| 文件占用 | 粘贴文件或文件夹路径，Restart Manager 列出所有占用进程，可直接结束 | `GET /api/file/occupiers` |
| 端口 | 自定义 process-service 监听端口：实时检测占用（含占用者进程名），Apply 后服务以新端口自动重启 | `GET /api/port/check`、`POST /api/config/port` |

## 目录结构

```text
process-manager/
├── manifest.json            # tab 插件声明 + service 托管配置（默认端口 18956，可被端口页改写）
├── index.html               # 四页签前端（纯 HTML/CSS/JS，无构建步骤）
├── README.md
└── sidecar/                 # 独立 Rust crate，不依赖 src-tauri
    ├── Cargo.toml           # sysinfo + windows-sys
    ├── src/main.rs          # 手写 HTTP 服务 + Windows API 封装
    └── process-service.exe      # 构建产物，随仓库分发（开箱即用）
```

## API 速览

```
GET  /api/health                  {status, version, port, elevated}
GET  /api/processes               进程快照（CPU% 为任务管理器口径：100% = 全部核心）
POST /api/process/kill            {pid, tree}
GET  /api/connections             {tcp[], udp[]}
POST /api/connections/close       {local_addr, local_port, remote_addr, remote_port}
GET  /api/file/occupiers?path=    Restart Manager 占用列表
GET  /api/port/check?port=N       {free, self?, owner?, pid?} 127.0.0.1 绑定探测
POST /api/config/port             {port} 重写 manifest service 端口并重启到新端口
```

## 端口自定义

- Apply 前置条件：目标端口在 127.0.0.1 上可绑定（`/api/port/check` 用真实 bind 探测，被占用时返回占用进程名与 PID）
- Apply 流程：服务改写所在插件目录的 `manifest.json`（`service.port` / `health_url` / `args` 同步更新）→ 拉起新端口实例 → 旧实例退出；新实例启动时对 bind 做最长 ~5s 重试以吸收交接间隙
- 端口持久化在 manifest 中（宿主每次启动服务都会重读 manifest），宿主重启后依然生效
- 前端缓存端口于 localStorage，并以 `/api/health` 返回的真实端口自愈

## 构建与部署

仓库内已附带 `sidecar/process-service.exe`，把整个 `process-manager/` 复制到 `%APPDATA%\OpenRevo\plugins\` 后重启宿主面板即可使用（插件列表出现「进程管理」）。

需要改服务源码时：

```bash
cd process-manager/sidecar
cargo build --release          # 产物：target/release/process-service.exe
cp target/release/process-service.exe ./process-service.exe
```

crate 自带独立 `[workspace]`，在本目录 `cargo build` 不会触碰宿主 `src-tauri` 依赖。仓库 CI 也提供 `process-service.yml`（改动 sidecar 源码自动触发 / 手动 `workflow_dispatch`），可下载 artifact `process-service` 直接替换。

> 清单里的 `version`（插件版本）与服务 crate 的 `version`（`/api/health` 返回的 `version`）独立演进：服务只在二进制接口发生变化时才升版本。

## 权限说明

- 服务以管理员运行（继承宿主计划任务权限）：结束任意进程树、断开任意 TCP 连接、查系统文件占用均不受 UIPI 限制
- 服务仅在插件面板打开时按需拉起，宿主退出时自动结束（`auto_stop: true`），无常驻后台
- 文件占用走 Restart Manager（资源管理器「文件正在使用」同源通道），不注入、不驱动

## 许可

随 open-revo 仓库分发。核心依赖：sysinfo (MIT/Apache-2.0)、windows-sys (MIT/Apache-2.0)、serde (MIT/Apache-2.0)。
