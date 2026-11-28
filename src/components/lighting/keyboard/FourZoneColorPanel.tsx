import React from 'react';
import { Palette } from 'lucide-react';
import { PRESET_COLORS } from '../lightingConstants';
import { trText, useI18n } from '../../../i18n/I18nContext';

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
  useI18n();
  const zoneLabels = [
    { name: trText('分区 1 (左侧区)'), key: 0 },
    { name: trText('分区 2 (中左区)'), key: 1 },
    { name: trText('分区 3 (中右区)'), key: 2 },
    { name: trText('分区 4 (右侧区)'), key: 3 },
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
            4 分区独立调色面板 (048D:CE00 专属):
          </span>
        </div>
        <div style={{ display: 'flex', gap: '6px' }}>
          <button
            type="button"
            onClick={() => onApplyZonePreset([mainColor, mainColor, mainColor, mainColor])}
            title="将当前主色调应用到全部 4 个分区"
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
            一键同步主色
          </button>
          <button
            type="button"
            onClick={() => onApplyZonePreset(['#00FFFF', '#0000FF', '#FF00FF', '#FF0000'])}
            title="预设：霓虹幻彩四区"
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
            霓虹幻彩
          </button>
          <button
            type="button"
            onClick={() => onApplyZonePreset(['#FF0055', '#FF0055', '#00D4FF', '#00D4FF'])}
            title="预设：红蓝对冲"
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
            红蓝对冲
          </button>
          <button
            type="button"
            onClick={() => onApplyZonePreset(['#10B981', '#059669', '#34D399', '#6EE7B7'])}
            title="预设：极客矩阵"
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
            极客矩阵
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
          <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>{trText('快速调色板:')}</span>
          {PRESET_COLORS.map((c) => (
            <div
              key={c.hex}
              onClick={() => {
                onChangeMainColor(c.hex);
                onApplyZonePreset([c.hex, c.hex, c.hex, c.hex]);
              }}
              title={`应用 ${c.name} 到所有分区`}
              className="color-dot"
              style={{
                backgroundColor: c.hex,
                border: '1px solid var(--border-medium)',
              }}
            />
          ))}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>{trText('自定义单色:')}</span>
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
