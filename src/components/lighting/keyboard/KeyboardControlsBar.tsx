import React from 'react';
import { Sun, Zap, ArrowRight, ArrowLeft, ArrowUp, ArrowDown } from 'lucide-react';

export interface KeyboardControlsBarProps {
  isSingleColor: boolean;
  isSingleZone: boolean;
  isFourZone: boolean;
  direction: number;
  onDirectionChange: (dir: number) => void;
  displayBrightness: number;
  onBrightnessChange: (val: number) => void;
  displaySpeed: number;
  onSpeedChange: (val: number) => void;
}

export const KeyboardControlsBar: React.FC<KeyboardControlsBarProps> = ({
  isSingleColor,
  isSingleZone,
  isFourZone,
  direction,
  onDirectionChange,
  displayBrightness,
  onBrightnessChange,
  displaySpeed,
  onSpeedChange,
}) => {
  const directions = isFourZone
    ? [
        { id: 0, icon: ArrowRight, label: 'Слева направо' },
        { id: 1, icon: ArrowLeft, label: 'Справа налево' },
      ]
    : [
        { id: 0, icon: ArrowRight, label: 'Слева направо' },
        { id: 1, icon: ArrowLeft, label: 'Справа налево' },
        { id: 2, icon: ArrowUp, label: 'Снизу вверх' },
        { id: 3, icon: ArrowDown, label: 'Сверху вниз' },
      ];

  return (
    <div
      className="lighting-sub-panel"
      style={{
        display: 'grid',
        gridTemplateColumns: isSingleColor ? '1fr' : '1fr 1fr',
        gap: '10px',
      }}
    >
      {/* Left: Direction (Only for Multi-direction RGB) */}
      {!isSingleColor && !isSingleZone && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)' }}>Направление движения:</span>
            <div style={{ display: 'flex', gap: '4px' }}>
              {directions.map((d) => {
                const Icon = d.icon;
                const isSelected = direction === d.id;
                return (
                  <button
                    type="button"
                    key={d.id}
                    onClick={() => onDirectionChange(d.id)}
                    title={d.label}
                    style={{
                      background: isSelected ? 'var(--color-brand)' : 'var(--surface-badge)',
                      border: '1px solid var(--border-medium)',
                      color: isSelected ? 'var(--text-primary)' : 'var(--text-muted)',
                      width: '32px',
                      height: '32px',
                      borderRadius: 'var(--radius-sm)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      cursor: 'pointer',
                      transition: 'all var(--transition-fast)',
                    }}
                  >
                    <Icon className="w-4 h-4" />
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      )}

      {/* Right: Brightness & Speed Sliders */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-secondary)' }}>
              <Sun className="w-3.5 h-3.5" style={{ color: 'var(--status-warn)' }} />
              Уровень яркости
            </span>
            <span style={{ color: 'var(--color-brand)', fontWeight: 700 }}>
              {displayBrightness === 0
                ? 'Выключено (0)'
                : isSingleColor
                ? displayBrightness === 1
                  ? 'Слабая (50%)'
                  : 'Яркая (100%)'
                : `${displayBrightness} ступ.`}
            </span>
          </div>
          <input
            type="range"
            min="0"
            max={isSingleColor ? 2 : 4}
            step="1"
            value={displayBrightness}
            onChange={(e) => onBrightnessChange(Number(e.target.value))}
            style={{ width: '100%', accentColor: 'var(--color-brand)', cursor: 'pointer' }}
          />
        </div>

        {!isSingleColor && !isSingleZone && (
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-secondary)' }}>
                <Zap className="w-3.5 h-3.5" style={{ color: 'var(--status-info)' }} />
                Скорость движения
              </span>
              <span style={{ color: 'var(--status-info)', fontWeight: 700 }}>
                {displaySpeed + 1} ступ. ({displaySpeed === 4 ? 'Мгновенно' : displaySpeed === 0 ? 'Спокойно' : 'Средне'})
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="4"
              step="1"
              value={displaySpeed}
              onChange={(e) => onSpeedChange(Number(e.target.value))}
              style={{ width: '100%', accentColor: 'var(--status-info)', cursor: 'pointer' }}
            />
          </div>
        )}
      </div>
    </div>
  );
};
