import React from 'react';
import { Layers } from 'lucide-react';

export interface MiniLedControlProps {
  enabled: boolean;
  isToggling: boolean;
  onToggle: (nextVal: boolean) => void;
}

/**
 * MiniLedControl — 内置屏幕分区背光 (MiniLED Local Dimming) 控制行
 * 遵循 OpenRevo Design System 规范与 Mono 黑白主题全量适配。
 */
export const MiniLedControl: React.FC<MiniLedControlProps> = ({
  enabled,
  isToggling,
  onToggle,
}) => {
  return (
    <div className="glass-card miniled-control-card">
      {/* Left: icon + label */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexShrink: 0 }}>
        <div className={`miniled-icon-badge ${enabled ? 'active' : ''}`}>
          <Layers className="w-4 h-4" />
        </div>
        <div>
          <div style={{ fontSize: '12.5px', fontWeight: 700, color: 'var(--text-primary)' }}>
            Позонная подсветка встроенного экрана (MiniLED Local Dimming)
          </div>
          <div style={{ fontSize: '10.5px', color: 'var(--text-dim)' }}>
            {enabled
              ? 'Текущее состояние: включено (глубоко-чёрный высококонтрастный режим, действует только на встроенный экран)'
              : 'Текущее состояние: выключено (равномерная полноэкранная подсветка)'}
          </div>
        </div>
      </div>

      {/* Right: toggle switch */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexShrink: 0 }}>
        <button
          onClick={() => onToggle(!enabled)}
          disabled={isToggling}
          className={`miniled-toggle-switch ${enabled ? 'active' : ''}`}
          title={enabled ? 'Нажмите, чтобы выключить позонную подсветку MiniLED' : 'Нажмите, чтобы включить позонную подсветку MiniLED'}
          aria-label="Переключить позонную подсветку MiniLED"
        >
          <div className="miniled-toggle-knob" />
        </button>
      </div>
    </div>
  );
};
