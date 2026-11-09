import React from 'react';
import {
  KEYBOARD_MODES,
  FOURZONE_KEYBOARD_MODES,
  STREAMER_MODES,
} from '../lightingConstants';

export interface KeyboardModeGridProps {
  isSingleZone: boolean;
  isFourZone: boolean;
  engine: 'hardware' | 'streamer' | 'custom';
  currentEffect?: number;
  currentStreamerEffect?: number;
  onSelectEffect: (id: number) => void;
  onSelectStreamerEffect: (id: number) => void;
}

export const KeyboardModeGrid: React.FC<KeyboardModeGridProps> = ({
  isSingleZone,
  isFourZone,
  engine,
  currentEffect = 3,
  currentStreamerEffect = 101,
  onSelectEffect,
  onSelectStreamerEffect,
}) => {
  const isHardware = isFourZone || engine === 'hardware';

  const modes = isSingleZone
    ? [
        { id: 1, name: '单色常亮', desc: '全键盘同色静态照明' },
        { id: 2, name: '彩虹循环', desc: '全键盘呼吸渐变循环' },
        { id: 3, name: '单色呼吸', desc: 'EC 硬件呼吸（4 个固定色：白/红/绿/蓝）' },
        { id: 4, name: '六色轮换', desc: '六色循环轮换（软件驱动）' },
      ]
    : isFourZone
    ? FOURZONE_KEYBOARD_MODES
    : engine === 'hardware'
    ? KEYBOARD_MODES
    : STREAMER_MODES;

  const headerTitle = isFourZone
    ? '4区原厂硬件灯效 (ITE 8291 r2 固件驱动):'
    : engine === 'hardware'
    ? '原厂硬件独立灯效 (内置 MCU 固件算法 - 稳定节能):'
    : '极客高刷流光动效 (BetterRGB 空间投影算法 - 华丽多变):';

  const normalizedStreamerEff =
    currentStreamerEffect >= 100 ? currentStreamerEffect : currentStreamerEffect + 100;

  return (
    <div>
      <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginBottom: '5px', fontWeight: 600 }}>
        {headerTitle}
      </div>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: isSingleZone
            ? 'repeat(2, 1fr)'
            : isFourZone
            ? 'repeat(6, 1fr)'
            : engine === 'hardware'
            ? 'repeat(6, 1fr)'
            : 'repeat(7, 1fr)',
          gap: '6px',
        }}
      >
        {modes.map((m) => {
          const isCurrentActive = isHardware
            ? currentEffect === m.id
            : normalizedStreamerEff === m.id;

          const activeBorder = isHardware ? 'var(--status-ok)' : 'var(--color-brand)';
          const activeBg = isHardware ? 'var(--status-ok-dim)' : 'var(--border-glow)';

          return (
            <button
              type="button"
              key={m.id}
              onClick={() => {
                if (isHardware) {
                  onSelectEffect(m.id);
                } else {
                  onSelectStreamerEffect(m.id);
                }
              }}
              className={`mode-btn ${isCurrentActive ? (isHardware ? 'active-emerald' : 'active-indigo') : ''}`}
              style={{
                padding: '5px 4px',
                minHeight: '38px',
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'center',
                alignItems: 'center',
                textAlign: 'center',
                border: isCurrentActive ? `1px solid ${activeBorder}` : '1px solid var(--border-subtle)',
                background: isCurrentActive ? activeBg : 'var(--surface-badge)',
                transition: 'all var(--transition-fast)',
              }}
            >
              <div
                style={{
                  fontSize: '11px',
                  fontWeight: 700,
                  whiteSpace: 'nowrap',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  maxWidth: '100%',
                  color: isCurrentActive ? 'var(--text-primary)' : 'var(--text-muted)',
                }}
              >
                {m.name}
              </div>
              <div
                style={{
                  fontSize: '9px',
                  color: 'var(--text-dim)',
                  marginTop: '1px',
                  whiteSpace: 'nowrap',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  maxWidth: '100%',
                }}
              >
                {m.desc}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
};
