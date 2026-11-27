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
        { id: 1, name: 'Постоянный монохром', desc: 'Вся клавиатура статичным одним цветом' },
        { id: 2, name: 'Цикл радуги', desc: 'Циклическое градиентное дыхание всей клавиатуры' },
      ]
    : isFourZone
    ? FOURZONE_KEYBOARD_MODES
    : engine === 'hardware'
    ? KEYBOARD_MODES
    : STREAMER_MODES;

  const headerTitle = isFourZone
    ? '4-зонные заводские аппаратные эффекты (драйвер прошивки ITE 8291 r2):'
    : engine === 'hardware'
    ? 'Отдельные заводские аппаратные эффекты (встроенные алгоритмы прошивки MCU — стабильно и экономично):'
    : 'Высокочастотные световые эффекты (алгоритм пространственной проекции BetterRGB — яркие и разнообразные):';

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
