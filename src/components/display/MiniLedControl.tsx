import React from 'react';
import { Layers } from 'lucide-react';
import { trText, useI18n } from '../../i18n/I18nContext';

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
  useI18n();
  return (
    <div className="glass-card miniled-control-card">
      {/* Left: icon + label */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexShrink: 0 }}>
        <div className={`miniled-icon-badge ${enabled ? 'active' : ''}`}>
          <Layers className="w-4 h-4" />
        </div>
        <div>
          <div style={{ fontSize: '12.5px', fontWeight: 700, color: 'var(--text-primary)' }}>
            内置屏幕分区背光 (MiniLED Local Dimming)
          </div>
          <div style={{ fontSize: '10.5px', color: 'var(--text-dim)' }}>
            {enabled
              ? trText('当前状态：已开启 (深黑高对比模式，仅对内置屏幕生效)')
              : trText('当前状态：已关闭 (全屏均匀背光模式)')}
          </div>
        </div>
      </div>

      {/* Right: toggle switch */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexShrink: 0 }}>
        <button
          onClick={() => onToggle(!enabled)}
          disabled={isToggling}
          className={`miniled-toggle-switch ${enabled ? 'active' : ''}`}
          title={enabled ? '点击关闭 MiniLED 分区背光' : trText('点击开启 MiniLED 分区背光')}
          aria-label="切换MiniLED分区背光"
        >
          <div className="miniled-toggle-knob" />
        </button>
      </div>
    </div>
  );
};
