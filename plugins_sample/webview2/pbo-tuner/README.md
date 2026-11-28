# PBO 精准调校 (SMU Debug) — OpenRevo 社区插件

> 仿 [SMUDebugTool](https://github.com/irusanov/SMUDebugTool) 的 AMD Ryzen PBO 精准调校中心：通过 PawnIO 内核驱动直连 SMU（RSMU mailbox），分核 Curve Optimizer 降压、全核 CO、PBO Scalar、FMax 睿频上限、Profile 保存读取，全部参数直接写进处理器固件，不进 BIOS、也不改动宿主源码。

---

## 一、 能做什么

| 模块 | 能力 | 数据通道 |
| :--- | :--- | :--- |
| 平台信息 | CPU 代号、SMU 版本、PM 表版本 | `POST /api/smu_init` |
| 分核 CO | 按 CCD 分组的每核 Curve Optimizer 滑杆（−50 ~ +50），单核读回、单核写入、All −/+ / All Zero | `POST /api/smu_get_co_single_core` · `/api/smu_set_co_single_core` · `/api/smu_set_co_all_cores` |
| PBO Scalar | 加速幅度 1x ~ 10x（线上单位为 `scalarX1000`） | `POST /api/smu_get_pbo_scalar` · `/api/smu_set_pbo_scalar` |
| FMax | 睿频上限 0 ~ 6000 MHz 读写（步进 25），用于压高频尖峰 | `POST /api/smu_get_fmax` · `/api/smu_set_fmax` |
| Profile | 当前 CO / Scalar / FMax 组合存档与一键回写 | `POST /api/smu_profile_save` · `/api/smu_profile_load` |
| 服务端口 | 自定义伴随服务监听端口（默认 18955），写入 manifest 后自动重启交接 | `POST /api/port_check` · `/api/config_port` |

分核 CO 不是「按逻辑处理器编号盲写」：插件先用 CPUID 解析出 CCD / 物理核拓扑（含 Zen5 芯片组扁平 NodeId、以及 7845HX 这类**物理核被熔丝屏蔽**的型号），再逐槽探测 SMU 实际存活的 CO 槽位，最后才按 `(ccd << 28) | (physical_core << 20)` 组掩码写入。因此在 8 核、12 核、16 核以及双 CCD 不对称的 HX 型号上都不会写错核或写飞。

## 二、 前置环境

1. **AMD Ryzen 处理器**：插件走 RSMU mailbox，仅 AMD 平台可用（已在 Fire Range / Dragon Range / Phoenix 等移动 HX 型号上实测）。
2. **安装 PawnIO**：<https://pawnio.eu/> —— 本插件不自带签名驱动，也不做第三方内核注入，全部寄存器访问借道 PawnIO 已签名的内核执行器。未安装时插件会明确报错而不是静默失败。
3. **管理员权限**：manifest 中 `service.elevated = true`，宿主会以管理员上下文拉起伴随服务（读取/写入 MSR 与 SMU mailbox 必需）。

## 三、 安装

把整个 `pbo-tuner/` 目录（**不要多层文件夹**）放进：

```text
%APPDATA%\OpenRevo\plugins\pbo-tuner\
```

重启宿主后在控制台左侧页签里即可看到「PBO 精准调校」。仓库内已附带与源码对应的 `sidecar/pbo-service.exe`，开箱即可运行；若想自行编译见第七节。

## 四、 目录结构

```text
pbo-tuner/
├── manifest.json          # tab 插件声明 + service 托管配置（默认端口 18955）
├── index.html             # 单文件前端（纯 HTML/CSS/JS，无构建步骤）
├── README.md
└── sidecar/               # 独立 Rust crate，不依赖宿主 src-tauri workspace
    ├── Cargo.toml         # 仅 serde / serde_json，无第三方驱动依赖
    ├── src/main.rs        # 手写 HTTP 服务 + 路由
    ├── src/smu.rs         # RSMU mailbox 命令封装、CPUID 拓扑、CO 槽位探测
    ├── src/pawnio.rs      # PawnIO 设备加载与 IOCTL 调用
    ├── RyzenSMU.bin       # PawnIO 模块（随插件分发，无需用户自备）
    └── pbo-service.exe    # 构建产物，随仓库分发
```

## 五、 安全边界与已知限制

- **只做「写入已有参数」这一件事**：请先在 BIOS / SMUDebugTool 里把负压调稳，再用本插件做开机自动写入与日常微调。**不建议拿它当第一次摸索负压的工具**——写错值不会自动回滚，需要手动 All Zero 或进 BIOS 复位。
- 分核 CO 采用「探测存活槽 → 按 rank 映射」的策略，极端型号（例如熔丝配置非常规的工程样品）可能探测失败并拒绝写入，此时会在页面顶部明确提示而不是照猜写值。
- SMU 版本 / PM 表版本只读；PM 表内的逐核 VID 等遥测不在本插件范围内（不同 SKU 的 PM 表布局差异过大，容易读出垃圾数据）。
- Profile 仅保存 CO / Scalar / FMax 三项，不备份 BIOS 里的其它设置。

## 六、 API 速览

除 `/api/health`（GET）外，全部接口都是 `POST` + 扁平 JSON body（前端 `rpc()` 统一 POST）：

```text
GET  /api/health                    {status, version, port, cpu}
POST /api/smu_init                  CPU 代号 / 核心拓扑 / SMU 版本 / PM 表版本
POST /api/smu_get_core_layout       CCD 分组与每核 CO 槽位
POST /api/smu_get_co_single_core    {core, ccd, ccx, physical_core?}
POST /api/smu_set_co_single_core    {core, ccd, ccx, physical_core?, margin}
POST /api/smu_set_co_all_cores      {margin}
POST /api/smu_get_pbo_scalar        当前 Scalar（scalarX1000）
POST /api/smu_set_pbo_scalar        {scalarX1000}
POST /api/smu_get_fmax              当前 FMax (MHz)
POST /api/smu_set_fmax              {frequencyMhz}
POST /api/smu_profile_save          {content: "<Profile 的 JSON 字符串>"}
POST /api/smu_profile_load          Profile JSON 字符串
POST /api/port_check                {port} 目标端口可绑定探测（自身监听视为可用）
POST /api/config_port               {port} 改写 manifest 端口并热重启服务
POST /api/log_config                {enabled} 开关服务端文件日志；不传 = 查询状态
POST /api/debug_log                 服务端日志尾部（页面「服务日志」面板）
```

## 七、 自行编译 sidecar

```bash
cd pbo-tuner/sidecar
cargo build --release          # 产物：target/release/pbo-service.exe
cp target/release/pbo-service.exe ./pbo-service.exe
```

crate 刻意声明了独立的 `[workspace]`，在本目录 `cargo build` 不会触碰宿主源码与宿主依赖。改完源码记得把新 exe 覆盖回 `sidecar/`，页面顶部的 `health.version` 与 `src/main.rs` 里的 `VERSION` 常量应一致。

## 八、 排错

| 现象 | 原因 / 处理 |
| :--- | :--- |
| 页面提示「服务未就绪」 | `pbo-service.exe` 没被拉起：确认插件目录结构没有多套一层、exe 未被杀软隔离、宿主以管理员运行 |
| 提示无法打开 PawnIO 设备 | 未安装 PawnIO，或安装后未重启；装好后重开插件页 |
| CO 读取全是 0 且 `coSourceTag` 显示探测失败 | 该型号的 CO 槽位探测未通过，插件拒绝盲写；请附 `smu_init` 与 `smu_get_core_layout` 的返回开 issue |
| 写入后跑分反而下降 / 蓝屏 | 负压过头：立刻 All Zero 回中性值，再回到 BIOS 里放宽 |
