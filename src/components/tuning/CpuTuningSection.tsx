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
          <span>{isAmdPlatform ? 'Потребление и температура AMD CPU' : 'Потребление и температура CPU'}</span>
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
          Текущая модель — ультрабук: пользовательские лимиты мощности CPU и температурные пороги не поддерживаются; мощность распределяется автоматически preset-режимами.
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
                  {isAmdPlatform ? 'Верхний температурный лимит (TjMax):' : 'Верхний температурный лимит:'}
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
                        ? '⚠️ Очень высокий риск: значение выше 100°C легко вызывает аппаратное отключение платы или аварийную блокировку частоты EC на 55 Вт; при блокировке нажмите кнопку сброса в правом верхнем углу для мгновенного восстановления'
                        : undefined
                    }
                  >
                    {tempTargetEnabled ? `${effectiveTempTarget}°C` : `${tempTargetMax}°C (без лимита)`}
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
                    {tempTargetEnabled ? 'Включено' : 'Выключить'}
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
                        ? (effectiveTempTarget < 95 ? `Лимит частоты -${95 - effectiveTempTarget}°C` : 'На полную')
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
                      <span>Лимит: {tempTargetMax}°C</span>
                      <button
                        type="button"
                        onClick={() => onAdjustTempTargetMax?.(-1)}
                        disabled={tempTargetMax <= tempTargetMin}
                        title="Уменьшить верхний температурный лимит с шагом 1°C"
                        className="tuning-step-btn"
                      >
                        -
                      </button>
                      <button
                        type="button"
                        onClick={() => onAdjustTempTargetMax?.(1)}
                        disabled={tempTargetMax >= maxTempWall}
                        title="Увеличить верхний температурный лимит с шагом 1°C"
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
        label={isAmdPlatform ? 'Длительное устойчивое потребление (SPL):' : 'Длительное потребление PL1:'}
        hint={isAmdPlatform ? 'Эквивалент PL1' : undefined}
        tooltip={isAmdPlatform ? 'AMD Sustained Power Limit (устойчивое длительное потребление, аналог Intel PL1)' : 'Intel Long Duration Power Limit (PL1)'}
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
        label={isAmdPlatform ? 'Кратковременное пиковое потребление (sPPT):' : 'Кратковременное потребление PL2:'}
        hint={isAmdPlatform ? 'Эквивалент PL2' : undefined}
        tooltip={isAmdPlatform ? 'AMD Slow Package Power Tracking (пиковое потребление под нагрузкой обычно до 28 секунд, аналог Intel PL2)' : 'Intel Short Duration Power Limit (PL2)'}
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
        label={isAmdPlatform ? 'Мгновенный пик (fPPT):' : 'Мгновенный пик PL4:'}
        hint={isAmdPlatform ? (!cpuSupportsFppt ? 'Не открыто (мощностью управляет SPL)' : 'Эквивалент PL4') : undefined}
        tooltip={
          isAmdPlatform
            ? (!cpuSupportsFppt
                ? 'Прошивка платы этой платформы не отдаёт отдельный регистр fPPT: оборудованием управляет единый контур SPL/sPPT'
                : 'AMD Fast Package Power Tracking (мгновенное ограничение тока дросселя на уровне микросекунд, аналог Intel PL4)')
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
