import React from 'react';
import { Cpu } from 'lucide-react';
import { BiosMenuStatus } from '../../types/hardware';
import { CpuPlSliderRow } from './CpuPlSliderRow';
import { CpuBiosOcRow } from './CpuBiosOcRow';
import { HardwareTelemetryBar } from './HardwareTelemetryBar';

export interface CpuTuningSectionProps {
  cpuTemp: number;
  cpuFanRpm?: number;
  cpuPowerW?: number;
  gpuTemp?: number;
  gpuFanRpm?: number;
  gpuPowerW?: number;
  gpuName?: string;
  discreteGpu?: boolean;
  minPl: number;
  maxPl1: number;
  maxPl2: number;
  maxPl4: number;
  minMaxPl1?: number;
  maxMaxPl1?: number;
  minMaxPl2?: number;
  maxMaxPl2?: number;
  minMaxPl4?: number;
  maxMaxPl4?: number;
  onAdjustMaxPl1?: (delta: number) => void;
  onAdjustMaxPl2?: (delta: number) => void;
  onAdjustMaxPl4?: (delta: number) => void;
  pl4Double?: boolean;
  pl4DoubleBusy?: boolean;
  onTogglePl4Double?: (enabled: boolean) => void;
  tempTargetEnabled: boolean;
  tempTarget: number;
  tempTargetMin?: number;
  tempTargetMax?: number;
  maxTempWall?: number;
  onAdjustTempTargetMax?: (delta: number) => void;
  pl4Margin?: number;
  pl1: number;
  pl2: number;
  pl4: number;
  cpuVendor?: 'amd' | 'intel' | string;
  cpuTuningType?: 'intel_pl' | 'amd_smu' | 'none' | string;
  cpuSupportsFppt?: boolean;
  cpuName?: string;
  biosOcEnabled: boolean;
  biosStatus: BiosMenuStatus | null;
  biosToggling: boolean;
  onToggleTempTarget: (enabled: boolean) => void;
  onChangeTempTarget: (temp: number) => void;
  onChangePl1: (pl1: number) => void;
  onChangePl2: (pl2: number) => void;
  onChangePl4: (pl4: number) => void;
  onToggleBiosOc: () => void;
}

export const CpuTuningSection: React.FC<CpuTuningSectionProps> = ({
  cpuTemp,
  cpuFanRpm,
  cpuPowerW,
  gpuTemp,
  gpuFanRpm,
  gpuPowerW,
  discreteGpu,
  cpuVendor,
  cpuTuningType,
  cpuSupportsFppt = true,
  minPl,
  maxPl1,
  maxPl2,
  maxPl4,
  minMaxPl1,
  maxMaxPl1,
  minMaxPl2,
  maxMaxPl2,
  minMaxPl4,
  maxMaxPl4,
  onAdjustMaxPl1,
  onAdjustMaxPl2,
  onAdjustMaxPl4,
  pl4Double,
  pl4DoubleBusy,
  onTogglePl4Double,
  tempTargetEnabled,
  tempTarget,
  tempTargetMin = 85,
  tempTargetMax = 95,
  maxTempWall = 100,
  onAdjustTempTargetMax,
  pl4Margin = 20,
  pl1,
  pl2,
  pl4,
  biosOcEnabled,
  biosStatus,
  biosToggling,
  onToggleTempTarget,
  onChangeTempTarget,
  onChangePl1,
  onChangePl2,
  onChangePl4,
  onToggleBiosOc,
}) => {
  const isApuOrIgpu = discreteGpu === false;

  const isAmdPlatform = React.useMemo(() => {
    return cpuVendor === 'amd' || cpuTuningType === 'amd_smu';
  }, [cpuVendor, cpuTuningType]);

  const isCpuTuningDisabled = cpuTuningType === 'none';

  return (
    <div className="tuning-card">
      {/* Header */}
      <div className="tuning-header">
        <div className="tuning-header-title" style={{ color: 'var(--color-balance)' }}>
          <Cpu className="w-4 h-4" />
          <span>{isAmdPlatform ? 'AMD CPU 功耗与温控' : 'CPU 功耗与温控'}</span>
        </div>
      </div>

      {/* Disabled Banner on Thin & Light Laptops */}
      {isCpuTuningDisabled && (
        <div
          className="sys-sub-panel"
          style={{
            padding: '8px 12px',
            marginBottom: '8px',
            borderRadius: 'var(--radius-sm)',
            fontSize: '11px',
            color: 'var(--text-dim)',
            lineHeight: 1.4,
          }}
        >
          当前机型为轻薄本架构，不支持自定义 CPU 功耗限制与温度墙调节。功耗由预设模式自动调度。
        </div>
      )}

      {/* Controls Container (dimmed & pointer-events disabled when tuning is unsupported) */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '8px',
          ...(isCpuTuningDisabled ? { opacity: 0.42, pointerEvents: 'none', filter: 'grayscale(0.7)' } : {}),
        }}
      >

        {/* Temperature Wall */}
        {(() => {
          const effectiveTempTarget = Math.min(tempTarget, tempTargetMax);
          return (
            <div className="tuning-row">
              <div className="tuning-row-header">
                <span className="tuning-row-label">
                  {isAmdPlatform ? '温度墙上限 (TjMax):' : '温度墙上限:'}
                </span>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <span
                    style={{
                      fontWeight: 700,
                      color: tempTargetEnabled
                        ? effectiveTempTarget >= 100
                          ? 'var(--status-error)'
                          : effectiveTempTarget >= 96
                          ? 'var(--color-turbo)'
                          : 'var(--status-info)'
                        : 'var(--text-dim)',
                      fontSize: '11px',
                    }}
                    title={
                      effectiveTempTarget >= 100
                        ? '⚠️ 极高风险：设为 100°C 以上极易触发主板硬件断电保护或 EC 55W 应急锁频；若遇锁死可点击右上角重置按钮一键脱困恢复'
                        : undefined
                    }
                  >
                    {tempTargetEnabled ? `${effectiveTempTarget}°C` : `${tempTargetMax}°C (不限)`}
                    {tempTargetEnabled && effectiveTempTarget >= 100 && ' ⚠️'}
                  </span>
                  <button
                    type="button"
                    onClick={() => onToggleTempTarget(!tempTargetEnabled)}
                    style={{
                      padding: '2px 8px',
                      borderRadius: 'var(--radius-xs)',
                      fontSize: '10px',
                      fontWeight: 600,
                      border: 'none',
                      cursor: 'pointer',
                      background: tempTargetEnabled ? 'var(--status-info-dim)' : 'var(--surface-badge)',
                      color: tempTargetEnabled ? 'var(--status-info)' : 'var(--text-muted)',
                      transition: 'all var(--transition-fast)',
                    }}
                  >
                    {tempTargetEnabled ? '已开启' : '关闭'}
                  </button>
                </div>
              </div>

              {tempTargetEnabled && (
                <>
                  <div style={{ padding: '2px 0 4px 0', display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <input
                      type="range"
                      min={tempTargetMin}
                      max={tempTargetMax}
                      step="1"
                      value={effectiveTempTarget}
                      onChange={(e) => onChangeTempTarget(parseInt(e.target.value, 10))}
                      style={{
                        flex: 1,
                        accentColor:
                          effectiveTempTarget >= 100
                            ? 'var(--status-error)'
                            : effectiveTempTarget >= 96
                            ? 'var(--color-turbo)'
                            : 'var(--status-info)',
                        height: '7px',
                        cursor: 'pointer',
                      }}
                    />
                    <span style={{ fontSize: '10px', color: 'var(--text-dim)', minWidth: '40px', textAlign: 'right' }}>
                      {isAmdPlatform
                        ? (effectiveTempTarget < 95 ? `限频 -${95 - effectiveTempTarget}°C` : '满血')
                        : `Tcc-${Math.max(0, (tempTargetMax > maxTempWall ? tempTargetMax : maxTempWall) - effectiveTempTarget)}`}
                    </span>
                  </div>
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      marginTop: '1px',
                      fontSize: '10px',
                      color: 'var(--text-dim)',
                    }}
                  >
                    <span>{tempTargetMin}°C</span>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                      <span>上限: {tempTargetMax}°C</span>
                      <button
                        type="button"
                        onClick={() => onAdjustTempTargetMax?.(-1)}
                        disabled={tempTargetMax <= tempTargetMin}
                        title="以 1°C 步长调小温度墙上限"
                        className="tuning-step-btn"
                      >
                        -
                      </button>
                      <button
                        type="button"
                        onClick={() => onAdjustTempTargetMax?.(1)}
                        disabled={tempTargetMax >= maxTempWall}
                        title="以 1°C 步长调大温度墙上限"
                        className="tuning-step-btn"
                      >
                        +
                      </button>
                    </div>
                  </div>
                </>
              )}
            </div>
          );
        })()}

      {/* PL1 Long Duration / AMD SPL */}
      <CpuPlSliderRow
        label={isAmdPlatform ? '长时持续功耗 (SPL):' : '长时功耗 PL1:'}
        hint={isAmdPlatform ? '等效 PL1' : undefined}
        tooltip={isAmdPlatform ? 'AMD Sustained Power Limit (持续稳定功耗，等效 Intel PL1)' : 'Intel Long Duration Power Limit (PL1)'}
        value={pl1}
        onChange={onChangePl1}
        min={minPl}
        max={maxPl1}
        minMax={minMaxPl1}
        maxMax={maxMaxPl1 ?? 260}
        onAdjustMax={onAdjustMaxPl1}
        colorVar="var(--color-balance)"
      />

      {/* PL2 Short Duration / AMD sPPT */}
      <CpuPlSliderRow
        label={isAmdPlatform ? '短时爆发功耗 (sPPT):' : '短时功耗 PL2:'}
        hint={isAmdPlatform ? '等效 PL2' : undefined}
        tooltip={isAmdPlatform ? 'AMD Slow Package Power Tracking (通常持续 28 秒的高负荷爆发功耗，等效 Intel PL2)' : 'Intel Short Duration Power Limit (PL2)'}
        value={pl2}
        onChange={onChangePl2}
        min={minPl}
        max={maxPl2}
        minMax={minMaxPl2}
        maxMax={maxMaxPl2 ?? 260}
        onAdjustMax={onAdjustMaxPl2}
        colorVar="var(--color-brand)"
      />

      {/* PL4 Peak Power / AMD fPPT */}
      <CpuPlSliderRow
        label={isAmdPlatform ? '极速峰值 (fPPT):' : '瞬时峰值 PL4:'}
        hint={isAmdPlatform ? (!cpuSupportsFppt ? '未开放 (硬件由SPL管理)' : '等效 PL4') : undefined}
        tooltip={
          isAmdPlatform
            ? (!cpuSupportsFppt
                ? '此模具主板固件未暴露 fPPT 独立寄存器，硬件由 SPL/sPPT 统一闭环调度'
                : 'AMD Fast Package Power Tracking (微秒级瞬时电感电流限制，等效 Intel PL4)')
            : 'Intel Peak Power Limit (PL4)'
        }
        disabled={isAmdPlatform && !cpuSupportsFppt}
        value={pl4}
        onChange={onChangePl4}
        min={minPl + pl4Margin}
        max={maxPl4}
        step={5}
        adjustStep={5}
        minMax={minMaxPl4 ?? (minPl + pl4Margin)}
        maxMax={maxMaxPl4 ?? 300}
        onAdjustMax={onAdjustMaxPl4}
        colorVar="var(--color-beast)"
      />

      {onTogglePl4Double && (
        <div className="quick-row" style={{ marginTop: '6px' }}>
          <div className="sys-row-label">
            PL4 双倍口径（2W/LSB）{pl4Double ? '：已开启' : '：已关闭'}
          </div>
          <div className="sys-row-controls">
            <button
              type="button"
              className={`toggle-btn ${pl4Double ? 'active-indigo' : ''}`}
              disabled={pl4DoubleBusy}
              onClick={() => onTogglePl4Double(!pl4Double)}
            >
              {pl4Double ? '双倍（上限 510W）' : '单倍（上限 255W）'}
            </button>
          </div>
        </div>
      )}

      {/* BIOS Advanced Switch */}
      <CpuBiosOcRow
        biosStatus={biosStatus}
        biosOcEnabled={biosOcEnabled}
        biosToggling={biosToggling}
        onToggleBiosOc={onToggleBiosOc}
      />
      </div>

      {/* Unified Real-Time Hardware Telemetry Monitor */}
      <HardwareTelemetryBar
        cpuTemp={cpuTemp}
        cpuFanRpm={cpuFanRpm}
        cpuPowerW={cpuPowerW}
        gpuTemp={gpuTemp}
        gpuFanRpm={gpuFanRpm}
        gpuPowerW={gpuPowerW}
        isApuOrIgpu={isApuOrIgpu}
      />
    </div>
  );
};
