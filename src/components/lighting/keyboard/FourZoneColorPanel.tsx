import React from 'react';
import { Palette } from 'lucide-react';
import { PRESET_COLORS } from '../lightingConstants';

export interface FourZoneColorPanelProps {
  currentZones: [string, string, string, string];
  mainColor: string;
  onZoneColorChange: (index: number, newColor: string) => void;
  onApplyZonePreset: (zones: [string, string, string, string]) => void;
  onChangeMainColor: (hex: string) => void;
}

export const FourZoneColorPanel: React.FC<FourZoneColorPanelProps> = ({
  currentZones,
  mainColor,
  onZoneColorChange,
  onApplyZonePreset,
  onChangeMainColor,
}) => {
  const zoneLabels = [
    { name: 'Зона 1 (левая)', key: 0 },
    { name: 'Зона 2 (лево-центр)', key: 1 },
    { name: 'Зона 3 (право-центр)', key: 2 },
    { name: 'Зона 4 (правая)', key: 3 },
  ];

  return (
    <div
      className="lighting-sub-panel"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        border: '1px solid rgba(56, 189, 248, 0.25)',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Palette className="w-4 h-4" style={{ color: 'var(--status-info)' }} />
          <span style={{ fontSize: '12px', fontWeight: 700, color: 'var(--text-primary)' }}>
            Панель раздельной окраски 4 зон (специально для 048D:CE00):
          </span>
        </div>
        <div style={{ display: 'flex', gap: '6px' }}>
          <button
            type="button"
            onClick={() => onApplyZonePreset([mainColor, mainColor, mainColor, mainColor])}
            title="Применить текущий основной цвет ко всем 4 зонам"
            style={{
              padding: '3px 8px',
              fontSize: '11px',
              borderRadius: 'var(--radius-xs)',
              background: 'var(--status-info-dim)',
              border: '1px solid rgba(56, 189, 248, 0.4)',
              color: 'var(--status-info)',
              cursor: 'pointer',
              fontWeight: 600,
            }}
          >
            Синхронизировать основной цвет одним нажатием
          </button>
          <button
            type="button"
            onClick={() => onApplyZonePreset(['#00FFFF', '#0000FF', '#FF00FF', '#FF0000'])}
            title="Пресет: неоновое разноцветье четырёх зон"
            style={{
              padding: '3px 8px',
              fontSize: '11px',
              borderRadius: 'var(--radius-xs)',
              background: 'var(--surface-badge)',
              border: '1px solid var(--border-medium)',
              color: 'var(--text-muted)',
              cursor: 'pointer',
            }}
          >
            Неоновое разноцветье
          </button>
          <button
            type="button"
            onClick={() => onApplyZonePreset(['#FF0055', '#FF0055', '#00D4FF', '#00D4FF'])}
            title="Пресет: красно-синие встречные потоки"
            style={{
              padding: '3px 8px',
              fontSize: '11px',
              borderRadius: 'var(--radius-xs)',
              background: 'var(--surface-badge)',
              border: '1px solid var(--border-medium)',
              color: 'var(--text-muted)',
              cursor: 'pointer',
            }}
          >
            Красно-синие потоки
          </button>
          <button
            type="button"
            onClick={() => onApplyZonePreset(['#10B981', '#059669', '#34D399', '#6EE7B7'])}
            title="Пресет: экспертная матрица"
            style={{
              padding: '3px 8px',
              fontSize: '11px',
              borderRadius: 'var(--radius-xs)',
              background: 'var(--surface-badge)',
              border: '1px solid var(--border-medium)',
              color: 'var(--text-muted)',
              cursor: 'pointer',
            }}
          >
            Экспертная матрица
          </button>
        </div>
      </div>

      {/* 4 Zone Individual Pickers */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(4, 1fr)',
          gap: '8px',
        }}
      >
        {zoneLabels.map((z) => (
          <div
            key={z.key}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              background: 'var(--surface-badge)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-sm)',
              padding: '6px 10px',
            }}
          >
            <div>
              <div style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-primary)' }}>{z.name}</div>
              <div style={{ fontSize: '10px', fontFamily: 'var(--font-mono)', color: 'var(--text-dim)' }}>
                {currentZones[z.key].toUpperCase()}
              </div>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              <input
                type="color"
                value={currentZones[z.key]}
                onChange={(e) => onZoneColorChange(z.key, e.target.value)}
                style={{
                  width: '24px',
                  height: '24px',
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                }}
              />
            </div>
          </div>
        ))}
      </div>

      {/* Master / Preset Color Bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingTop: '6px',
          borderTop: '1px solid var(--border-subtle)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>Быстрая палитра:</span>
          {PRESET_COLORS.map((c) => (
            <div
              key={c.hex}
              onClick={() => {
                onChangeMainColor(c.hex);
                onApplyZonePreset([c.hex, c.hex, c.hex, c.hex]);
              }}
              title={`Применить ${c.name} ко всем зонам`}
              className="color-dot"
              style={{
                backgroundColor: c.hex,
                border: '1px solid var(--border-medium)',
              }}
            />
          ))}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>Пользовательский монохром:</span>
          <input
            type="color"
            value={mainColor}
            onChange={(e) => {
              onChangeMainColor(e.target.value);
              onApplyZonePreset([e.target.value, e.target.value, e.target.value, e.target.value]);
            }}
            style={{ width: '24px', height: '24px', border: 'none', background: 'transparent', cursor: 'pointer' }}
          />
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', color: 'var(--text-muted)' }}>
            {mainColor.toUpperCase()}
          </span>
        </div>
      </div>
    </div>
  );
};
