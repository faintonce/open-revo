import React from 'react';
import { ShieldCheck, Zap, Sparkles, Star } from 'lucide-react';
import { trText, useI18n } from '../../../i18n/I18nContext';

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
  useI18n();
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
        <span style={{ fontSize: '11px', fontWeight: 700, color: 'var(--text-muted)', whiteSpace: 'nowrap' }}> {trText('驱动引擎:')} </span>
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
            title="由 ITE 8291 单片机固件内部硬件定时器渲染，系统 0% CPU 占用"
          >
            <ShieldCheck size={12} className="inline mr-1" /> {trText('原厂固件 (0% CPU)')} </button>
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
            title="由 BetterRGB 空间几何算法计算并由驱动以 60FPS 逐帧推流到键盘"
          >
            <Zap size={12} className="inline mr-1" /> {trText('极客高刷 (BetterRGB 算法)')} </button>
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
            title="自定义动效脚本工作室：支持导入外部 .brfx 脚本与实时参数微调"
          >
            <Sparkles size={12} className="inline mr-1" /> {trText('定制脚本 (BetterRGB)')} </button>
        </div>
      </div>

      {engine !== 'hardware' ? (
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-dim)', whiteSpace: 'nowrap' }}>{trText('推流高刷帧率:')}</span>
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
          <span style={{ display: 'inline-block', width: '6px', height: '6px', borderRadius: 'var(--radius-full)', backgroundColor: 'var(--status-ok)' }} /> {trText('硬件自主运算，无需主机推流')} </span>
      )}
    </div>
  );
};
