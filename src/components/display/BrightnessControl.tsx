import React from 'react';
import { Sun } from 'lucide-react';

export interface BrightnessControlProps {
  brightness: number;
  onChange: (val: number) => void;
  onQuick: (val: number) => void;
}

/**
 * BrightnessControl — 屏幕亮度调节横条。
 * 纯 UI 组件：所有状态与 invoke 调用由父级 hook 管理并通过 props 注入。
 * 插件化路径：未来可作为独立桌面挂件插槽直接渲染。
 */
export const BrightnessControl: React.FC<BrightnessControlProps> = ({
  brightness,
  onChange,
  onQuick,
}) => {
  return (
    <div
      className="glass-card"
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '14px',
        padding: '10px 16px',
        marginTop: '4px',
      }}
    >
      {/* Label */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', minWidth: '160px', flexShrink: 0 }}>
        <div
          style={{
            width: '32px',
            height: '32px',
            borderRadius: '7px',
            background: 'rgba(245, 158, 11, 0.15)',
            border: '1px solid rgba(245, 158, 11, 0.3)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <Sun className="w-4 h-4 text-amber-400" />
        </div>
        <div>
          <div style={{ fontSize: '12.5px', fontWeight: 700, color: 'var(--text-primary)' }}>Регулировка яркости экрана</div>
          <div style={{ fontSize: '10.5px', color: 'var(--text-dim)' }}>Текущая яркость:{brightness}%</div>
        </div>
      </div>

      {/* Slider */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flex: 1, maxWidth: '420px' }}>
        <Sun size={13} style={{ color: 'var(--text-muted)', flexShrink: 0 }} />
        <input
          type="range"
          min={0}
          max={100}
          value={brightness}
          onChange={(e) => onChange(Number(e.target.value))}
          className="rev-range"
          style={{ accentColor: '#f59e0b', height: '6px' }}
        />
        <Sun size={16} style={{ color: '#f59e0b', flexShrink: 0 }} />
      </div>

      {/* Quick presets */}
      <div style={{ display: 'flex', gap: '6px', flexShrink: 0 }}>
        {([25, 60, 100] as const).map((v) => (
          <button
            key={v}
            onClick={() => onQuick(v)}
            className="toggle-btn"
            style={{ padding: '3px 8px', fontSize: '11px' }}
            title={v === 25 ? 'Быстрое переключение на 25% (при слабом освещении)' : v === 60 ? 'Быстрое переключение на 60% (обычно, в помещении)' : 'Быстрое переключение на 100% (ярко, на улице)'}
          >
            {v}%
          </button>
        ))}
      </div>
    </div>
  );
};
