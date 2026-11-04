# Windows CPU / GPU 只读采样实测记录

本文整理 Open-Revo Native 中已经取得实机读数或用户复测结果的采样方法，供 Core 维护者对照。不同构建的实测记录分别注明覆盖范围。采样沿用已有刷新节奏，缺失字段保持未知。

## CPU 频率、占用率与功耗估算

| 字段 | 读取方法 | 实测范围 |
| --- | --- | --- |
| CPU 占用率 | 对两次 `GetSystemTimes` 的 idle、kernel、user 累计时间做差。kernel 已含 idle，因此用 `(Δkernel + Δuser − Δidle) / (Δkernel + Δuser)`。第一帧没有基线时留空。 | 本机连续样本首帧为空，后续为 6% 和 9%。 |
| CPU 频率 | 用 `CallNtPowerInformation(ProcessorInformation)` 读取有效的 `CurrentMhz` 并求均值。采样器也可使用 PDH 的 `% Processor Performance` 与基准频率换算。 | 同批样本为 1997、1936、2083 MHz。记录未区分单帧走了哪个频率来源，验收范围为频率读数链路。 |
| CPU 功耗估算 | 将有效占用率与频率交给 Core 既有的估算模型，随结果传递估算标记。 | 约 10.6 W、12.5 W，均带估算标记。此批样本只覆盖估算分支。 |

参考 [GetSystemTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes) 与 [CallNtPowerInformation](https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-callntpowerinformation)。

## AMD Radeon 780M

无界 14 Pro Ryzen 7 7840HS 上，两个远程虚拟显示适配器与实体 780M 同时存在。名称重复时用 Windows 适配器身份区分，再把计数器和驱动读数归到已选中的实体 GPU。

| 字段 | 读取方法 | 实测范围 |
| --- | --- | --- |
| 适配器身份与名称 | 保留 DXGI 枚举的 LUID，以同一 LUID 关联 DXCore、D3DKMT 和设备接口，依据硬件、集显和虚拟类型过滤。纯集显场景仍有多个候选时使用活动内置屏关联的 LUID 消歧，剩余歧义时留空。 | 样本选中实体 Radeon 780M，并排除了两个同名虚拟候选。 |
| GPU 占用率 | PDH `GPU Engine(*)\Utilization Percentage` 按完整 LUID 筛选。将同一物理实例、同一引擎的进程样本相加，再取最忙引擎。第一轮采样建立基线，之后显示有效值。 | 同一实体适配器取得 8% 样本，用户也确认占用在采样开始后显示。 |
| 共享 GPU 内存 | PDH `GPU Adapter Memory(*)\Shared Usage` 取得适配器级字节数，DXGI `SharedSystemMemory` 提供共享上限。按同一 LUID 关联，使用量和上限各自有效时分别显示。展示单位采用十进制 GB。 | 1,405,251,584 B / 8,156,276,736 B，约 1.41 / 8.16 GB。 |
| 核心及内存频率 | 根据 PCI 设备身份匹配驱动中的同一 GPU。Windows 频率缺失时，读取 ADLX 当前指标中的 `GPUClockSpeed` 和 `GPUVRAMClockSpeed`。 | ADLX 返回 800 / 1000 MHz。 |
| GPU 温度、功耗 | 同一 ADLX GPU 查询 `IsSupportedGPUTemperature`、`IsSupportedGPUPower`，分别读取 `GPUTemperature`（°C）和 `GPUPower`（W）。只使用驱动确认支持且调用成功的有效值。 | 0.9.842 用户整体复测确认本轮问题已修复。该版本的温度、功耗缺少单独的数值导出。 |

参考 [Microsoft 的 WDDM GPU 统计口径](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/)、[DXCore 适配器属性](https://learn.microsoft.com/windows/win32/api/dxcore_interface/ne-dxcore_interface-dxcoreadapterproperty) 与 [AMD ADLX 当前指标示例](https://gpuopen.com/manuals/adlx/programming-with-adlx/adlx-samples/c-samples/performance-monitoring/perfgpumetrics/)。

## NVIDIA 独显

通过驱动随附的 NVML 获取同一设备的温度、核心/显存时钟、占用、专用显存用量和功耗。相关查询为 `nvmlDeviceGetTemperature`、`nvmlDeviceGetClockInfo`、`nvmlDeviceGetUtilizationRates`、`nvmlDeviceGetMemoryInfo` 和 `nvmlDeviceGetPowerUsage`。功耗从 mW 转为 W，显存保持字节数。采样按需调用并释放 NVML 生命周期，实测覆盖单块 NVIDIA 独显的场景。

一帧实机样本返回 43°C、核心 435 MHz、显存 6001 MHz、23% 占用、专用显存已用 3,033,814,144 B / 总计 8,585,740,288 B，以及约 12.86 W。多 GPU 归属需要独立的身份匹配证据。

参考 [NVIDIA NVML 设备查询](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html)。

## 回归时核对的行为

- 同名物理和虚拟 GPU 同时出现时，名称、占用和共享用量仍归到同一实体 GPU。计数器首帧和歧义场景保留未知状态。
- 共享 GPU 内存、独显专用显存、系统 RAM 分别表达。设备身份无法关联时，驱动传感器和 Windows 计数器各自保留来源。
- CPU 估算功耗保留标记。GPU 温度和功耗逐字段判定可用性，某一项缺失时保留其他有效读数。

这里的验证范围限于上述设备、样本和用户复测。移植到上游实现后，需在该实现上重新核对相同场景。
