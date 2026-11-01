import React from 'react';
import { Palette } from 'lucide-react';
import { PRESET_COLORS } from './lightingConstants';

export interface LightColorRowProps {
  label?: string;
  color: string;
  onChangeColor: (hex: string) => void;
  iconColor?: string;
}

export const LightColorRow: React.FC<LightColorRowProps> = ({
  label = '主色调：',
  color,
  onChangeColor,
  iconColor = 'var(--color-brand)',
}) => {
  return (
    <div
      className="lighting-sub-panel"
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <Palette className="w-4 h-4" style={{ color: iconColor }} />
        <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)' }}>{label}</span>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          {PRESET_COLORS.map((c) => {
            const isSelected = color.toLowerCase() === c.hex.toLowerCase();
            return (
              <div
                key={c.hex}
                onClick={() => onChangeColor(c.hex)}
                title={c.name}
                className={`color-dot ${isSelected ? 'active' : ''}`}
                style={{
                  backgroundColor: c.hex,
                  boxShadow: isSelected ? `0 0 8px ${c.hex}` : 'none',
                }}
              />
            );
          })}
        </div>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <input
          type="color"
          value={color}
          onChange={(e) => onChangeColor(e.target.value)}
          style={{ width: '28px', height: '28px', border: 'none', background: 'transparent', cursor: 'pointer' }}
        />
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', color: 'var(--text-muted)' }}>
          {color.toUpperCase()}
        </span>
      </div>
    </div>
  );
};
