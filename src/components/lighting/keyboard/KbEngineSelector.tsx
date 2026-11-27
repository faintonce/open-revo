import React from 'react';
import { ShieldCheck, Zap, Sparkles, Star } from 'lucide-react';

export interface KbEngineSelectorProps {
  engine: 'hardware' | 'streamer' | 'custom';
  fps?: number;
  onSelectHardware: () => void;
  onSelectStreamer: () => void;
  onSelectCustom: () => void;
  onChangeFps: (fps: number) => void;
}

export const KbEngineSelector: React.FC<KbEngineSelectorProps> = ({
  engine,
  fps = 30,
  onSelectHardware,
  onSelectStreamer,
  onSelectCustom,
  onChangeFps,
}) => {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        background: 'var(--surface-sunken)',
        border: '1px solid var(--border-subtle)',
        padding: '7px 12px',
        borderRadius: 'var(--radius-md)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <span style={{ fontSize: '11px', fontWeight: 700, color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>
          Движок драйвера:
        </span>
        <div style={{ display: 'flex', gap: '4px' }}>
          <button
            type="button"
            onClick={onSelectHardware}
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              borderRadius: 'var(--radius-full)',
              fontWeight: 700,
              cursor: 'pointer',
              border: engine === 'hardware' ? '1px solid var(--status-ok)' : '1px solid var(--border-subtle)',
              background: engine === 'hardware' ? 'var(--status-ok-dim)' : 'var(--surface-badge)',
              color: engine === 'hardware' ? 'var(--status-ok)' : 'var(--text-dim)',
              transition: 'all var(--transition-fast)',
            }}
            title="Рендерится аппаратным таймером внутри прошивки микроконтроллера ITE 8291; нагрузка на CPU — 0%"
          >
            <ShieldCheck size={12} className="inline mr-1" /> Заводская прошивка (0% CPU)
          </button>
          <button
            type="button"
            onClick={onSelectStreamer}
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              borderRadius: 'var(--radius-full)',
              fontWeight: 700,
              cursor: 'pointer',
              border: engine === 'streamer' ? '1px solid var(--color-brand)' : '1px solid var(--border-subtle)',
              background: engine === 'streamer' ? 'var(--border-glow)' : 'var(--surface-badge)',
              color: engine === 'streamer' ? 'var(--color-brand)' : 'var(--text-dim)',
              transition: 'all var(--transition-fast)',
            }}
            title="Рассчитывается пространственно-геометрическим алгоритмом BetterRGB и потоком передаётся драйвером на клавиатуру покадрово при 60 FPS"
          >
            <Zap size={12} className="inline mr-1" /> Экспертный высокий FPS (алгоритм BetterRGB)
          </button>
          <button
            type="button"
            onClick={onSelectCustom}
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              borderRadius: 'var(--radius-full)',
              fontWeight: 700,
              cursor: 'pointer',
              border: engine === 'custom' ? '1px solid var(--status-error)' : '1px solid var(--border-subtle)',
              background: engine === 'custom' ? 'var(--status-error-dim)' : 'var(--surface-badge)',
              color: engine === 'custom' ? 'var(--status-error)' : 'var(--text-dim)',
              transition: 'all var(--transition-fast)',
            }}
            title="Студия пользовательских световых скриптов: импорт внешних .brfx-скриптов и тонкая настройка параметров в реальном времени"
          >
            <Sparkles size={12} className="inline mr-1" /> Пользовательские скрипты (BetterRGB)
          </button>
        </div>
      </div>

      {engine !== 'hardware' ? (
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-dim)', whiteSpace: 'nowrap' }}>Частота кадров потока подсветки:</span>
          <div style={{ display: 'flex', gap: '3px' }}>
            {[15, 30, 45, 60].map((fpsVal) => {
              const active = fps === fpsVal;
              return (
                <button
                  type="button"
                  key={fpsVal}
                  onClick={() => onChangeFps(fpsVal)}
                  style={{
                    padding: '3px 8px',
                    fontSize: '11px',
                    fontWeight: 700,
                    borderRadius: 'var(--radius-xs)',
                    cursor: 'pointer',
                    border: active ? '1px solid var(--color-brand)' : '1px solid var(--border-subtle)',
                    background: active ? 'var(--border-glow)' : 'transparent',
                    color: active ? 'var(--text-primary)' : 'var(--text-dim)',
                    transition: 'all var(--transition-fast)',
                  }}
                >
                  <span style={{ display: 'inline-flex', alignItems: 'center', gap: '2px' }}>
                    {fpsVal}
                    {fpsVal === 30 ? <Star size={10} className="fill-current" /> : fpsVal === 60 ? <Zap size={10} className="fill-current" /> : null}
                  </span>
                </button>
              );
            })}
          </div>
        </div>
      ) : (
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '11px', color: 'var(--status-ok)', opacity: 0.9 }}>
          <span style={{ display: 'inline-block', width: '6px', height: '6px', borderRadius: 'var(--radius-full)', backgroundColor: 'var(--status-ok)' }} />
          Вычисления выполняет само оборудование — поток с хоста не нужен
        </span>
      )}
    </div>
  );
};
