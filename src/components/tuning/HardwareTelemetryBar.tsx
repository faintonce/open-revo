import React from 'react';
import { Activity } from 'lucide-react';

export interface PowerWattBadgeProps {
  watts?: number;
  type: 'cpu' | 'gpu';
  isApuOrIgpu?: boolean;
}

export const PowerWattBadge: React.FC<PowerWattBadgeProps> = ({
  watts,
  type,
  isApuOrIgpu = false,
}) => {
  const color = type === 'cpu' ? 'var(--color-balance)' : 'var(--status-ok)';
  const formatted =
    watts !== undefined && watts > 0
      ? `${watts >= 100 ? watts.toFixed(0) : watts.toFixed(1)} W`
      : isApuOrIgpu && type === 'gpu'
        ? 'Общая (APU)'
        : '-- W';

  return (
    <div
      style={{
        fontSize: '12px',
        fontWeight: 700,
        color,
        letterSpacing: '0.3px',
        fontFamily: 'var(--font-mono)',
      }}
    >
      {formatted}
    </div>
  );
};

export interface HardwareTelemetryBarProps {
  cpuTemp: number;
  cpuFanRpm?: number;
  cpuPowerW?: number;
  gpuTemp?: number;
  gpuFanRpm?: number;
  gpuPowerW?: number;
  isApuOrIgpu?: boolean;
}

export const HardwareTelemetryBar: React.FC<HardwareTelemetryBarProps> = ({
  cpuTemp,
  cpuFanRpm,
  cpuPowerW,
  gpuTemp,
  gpuFanRpm,
  gpuPowerW,
  isApuOrIgpu = false,
}) => {
  const totalPowerW = (cpuPowerW || 0) + (gpuPowerW || 0);

  return (
    <div
      className="tuning-sub-card"
      style={{
        marginTop: 'auto',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '5px', color: 'var(--status-warn)', fontSize: '11px', fontWeight: 600 }}>
          <Activity className="w-3.5 h-3.5" />
          <span>Мониторинг оборудования в реальном времени</span>
        </div>
        <span style={{ fontSize: '10px', color: 'var(--text-dim)' }}>
          {isApuOrIgpu ? 'Потребление пакета APU:' : 'Суммарно при двойной нагрузке:'}{' '}
          <strong style={{ color: 'var(--text-primary)', fontWeight: 700 }}>
            {isApuOrIgpu
              ? cpuPowerW !== undefined && cpuPowerW > 0
                ? `${cpuPowerW.toFixed(1)} W`
                : '--'
              : totalPowerW > 0
                ? `${totalPowerW.toFixed(0)} W`
                : '--'}
          </strong>
        </span>
      </div>

      {/* Row 1: CPU Telemetry */}
      <div
        style={{
          background: 'rgba(0, 0, 0, 0.28)',
          borderRadius: 'var(--radius-sm)',
          padding: '5px 8px',
          border: '1px solid rgba(59, 130, 246, 0.2)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span
            style={{
              fontSize: '9px',
              fontWeight: 700,
              color: 'var(--color-balance)',
              background: 'rgba(59, 130, 246, 0.15)',
              padding: '1px 5px',
              borderRadius: 'var(--radius-xs)',
            }}
          >
            CPU
          </span>
          <span style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
            {cpuTemp > 0 ? `${cpuTemp}°C` : '--'}
          </span>
          {cpuFanRpm !== undefined && cpuFanRpm > 0 && (
            <span style={{ fontSize: '10px', color: 'var(--text-dim)', marginLeft: '4px' }}>
              Fan {cpuFanRpm} RPM
            </span>
          )}
        </div>
        <PowerWattBadge watts={cpuPowerW} type="cpu" />
      </div>

      {/* Row 2: GPU Telemetry */}
      <div
        style={{
          background: 'rgba(0, 0, 0, 0.28)',
          borderRadius: 'var(--radius-sm)',
          padding: '5px 8px',
          border: '1px solid rgba(16, 185, 129, 0.2)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span
            style={{
              fontSize: '9px',
              fontWeight: 700,
              color: 'var(--status-ok)',
              background: 'var(--status-ok-dim)',
              padding: '1px 5px',
              borderRadius: 'var(--radius-xs)',
            }}
          >
            GPU
          </span>
          <span style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
            {gpuTemp !== undefined && gpuTemp > 0 ? `${gpuTemp}°C` : '--'}
          </span>
          {gpuFanRpm !== undefined && gpuFanRpm > 0 && (
            <span style={{ fontSize: '10px', color: 'var(--text-dim)', marginLeft: '4px' }}>
              Fan {gpuFanRpm} RPM
            </span>
          )}
        </div>
        <PowerWattBadge watts={gpuPowerW} type="gpu" isApuOrIgpu={isApuOrIgpu} />
      </div>
    </div>
  );
};
