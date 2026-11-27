import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Wind, Fan, RotateCcw, Gauge } from 'lucide-react';
import { RevSwitch } from '../ui/RevSwitch';

export interface BldFanStrategyPanelProps {
  cpuFanRpm?: number;
  gpuFanRpm?: number;
  cpuTemp?: number;
  gpuTemp?: number;
  fanBoost?: boolean;
  maxRpm?: number;
  fanBoostMode?: string;
  onToggleFanBoost?: () => void;
}

export const BldFanStrategyPanel: React.FC<BldFanStrategyPanelProps> = ({
  cpuFanRpm = 0,
  gpuFanRpm = 0,
  cpuTemp = 0,
  gpuTemp = 0,
  fanBoost = false,
  maxRpm = 5800,
  fanBoostMode = 'direct_rpm_clamp',
  onToggleFanBoost,
}) => {
  const [speedCap, setSpeedCap] = useState<number>(maxRpm);
  const [isCapActive, setIsCapActive] = useState<boolean>(false);
  const debounceRef = useRef<any>(null);

  const handleSpeedCapChange = (val: number) => {
    setSpeedCap(val);
    setIsCapActive(true);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      invoke('set_bld_fan_speed_cap', { rpm: val }).catch(console.warn);
    }, 150);
  };

  const handleRestoreAuto = () => {
    setIsCapActive(false);
    setSpeedCap(maxRpm);
    invoke('set_bld_fan_speed_cap', { rpm: 0 }).catch(console.warn);
    if (fanBoost && onToggleFanBoost) {
      onToggleFanBoost();
    }
  };

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  return (
    <div className="tuning-card" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
      {/* Header */}
      <div className="tuning-header">
        <div className="tuning-header-title" style={{ color: 'var(--status-info)' }}>
          <Wind className="w-4 h-4" />
          <span>Заводская стратегия охлаждения и контур управления температурой</span>
        </div>
        <button
          type="button"
          onClick={handleRestoreAuto}
          className="tuning-step-btn"
          style={{ width: 'auto', padding: '2px 8px', fontSize: '10px', gap: '4px' }}
          title="Восстановить заводское адаптивное управление температурой, снять лимит оборотов и блокировку форс-охлаждения"
        >
          <RotateCcw className="w-3 h-3" />
          <span>По умолчанию</span>
        </button>
      </div>

      {/* Dual Fan Live Telemetry */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
          gap: '8px',
          background: 'var(--surface-badge)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-md)',
          padding: '8px 10px',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', fontSize: '11px', color: 'var(--text-muted)' }}>
            <span>Вентилятор CPU</span>
            <span>{cpuTemp > 0 ? `${cpuTemp}°C` : '--'}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Fan className={`w-3.5 h-3.5 ${cpuFanRpm > 0 ? 'animate-spin' : ''}`} style={{ color: 'var(--status-info)' }} />
            <span style={{ fontSize: '13px', fontWeight: 700, color: 'var(--text-primary)' }}>
              {cpuFanRpm > 0 ? `${cpuFanRpm} RPM` : 'Остановка в простое'}
            </span>
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', fontSize: '11px', color: 'var(--text-muted)' }}>
            <span>Вентилятор GPU</span>
            <span>{gpuTemp > 0 ? `${gpuTemp}°C` : '--'}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Fan className={`w-3.5 h-3.5 ${gpuFanRpm > 0 ? 'animate-spin' : ''}`} style={{ color: 'var(--color-turbo)' }} />
            <span style={{ fontSize: '13px', fontWeight: 700, color: 'var(--text-primary)' }}>
              {gpuFanRpm > 0 ? `${gpuFanRpm} RPM` : 'Остановка в простое'}
            </span>
          </div>
        </div>
      </div>

      {/* Single-row Fan Boost Switch */}
      <div
        className="tuning-sub-card"
        style={{
          flexDirection: 'row',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '8px 10px',
          background: fanBoost ? 'var(--status-info-dim)' : undefined,
          border: fanBoost ? '1px solid var(--status-info)' : undefined,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Fan className={`w-4 h-4 ${fanBoost ? 'animate-spin' : ''}`} style={{ color: fanBoost ? 'var(--status-info)' : 'var(--text-muted)' }} />
          <span style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '12px', whiteSpace: 'nowrap' }}>
            {fanBoostMode === 'direct_rpm_clamp' ? `Аппаратная физическая блокировка (${maxRpm} RPM)` : 'Мгновенное форс-охлаждение (Fan Boost)'}
          </span>
        </div>
        <RevSwitch
          checked={fanBoost}
          onChange={() => onToggleFanBoost && onToggleFanBoost()}
          size="sm"
          activeColor="var(--status-info)"
        />
      </div>

      {/* Fan Speed Cap Slider */}
      <div className="tuning-row" style={{ marginTop: '2px' }}>
        <div className="tuning-row-header">
          <span className="tuning-row-label" style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <Gauge className="w-3.5 h-3.5 text-cyan-400" />
            <span>Лимит оборотов вентилятора:</span>
          </span>
          <span style={{ fontSize: '11px', fontWeight: 700, color: isCapActive ? 'var(--status-info)' : 'var(--text-muted)' }}>
            {isCapActive ? `${speedCap} RPM (${Math.round((speedCap / maxRpm) * 100)}%)` : 'Заводская адаптация (без верхнего лимита)'}
          </span>
        </div>
        <div style={{ padding: '4px 0', display: 'flex', alignItems: 'center', gap: '8px' }}>
          <input
            type="range"
            min={2800}
            max={maxRpm}
            step={100}
            value={speedCap}
            onChange={(e) => handleSpeedCapChange(parseInt(e.target.value, 10))}
            style={{ flex: 1, accentColor: 'var(--status-info)', height: '7px', cursor: 'pointer' }}
          />
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10px', color: 'var(--text-dim)' }}>
          <span>2800 RPM (тихий лимит)</span>
          <span>{maxRpm} RPM (предел полной скорости)</span>
        </div>
      </div>
    </div>
  );
};

export const DirectFanStrategyPanel = BldFanStrategyPanel;
