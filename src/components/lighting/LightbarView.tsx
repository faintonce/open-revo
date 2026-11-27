import React from 'react';
import { Sun, Zap } from 'lucide-react';
import {
  LightingState,
  KeyboardHardwareInfo,
  LIGHTBAR_MODES,
} from './lightingConstants';
import { LightHeaderBanner } from './LightHeaderBanner';
import { LightColorRow } from './LightColorRow';

interface LightbarViewProps {
  lighting: LightingState;
  kbHwInfo: KeyboardHardwareInfo | null;
  sendLightbar: (patch: Partial<LightingState>) => void;
}

export const LightbarView: React.FC<LightbarViewProps> = ({
  lighting,
  kbHwInfo,
  sendLightbar,
}) => {
  const isEcLightbar = Boolean(kbHwInfo?.is_ec_lightbar);
  const isBaseLightbarSupported = Boolean(kbHwInfo && (kbHwInfo.has_front_lightbar || kbHwInfo.has_ite7001));

  return (
    <div className="glass-card" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
      <LightHeaderBanner
        title="Световая полоса по передней кромке корпуса (Front Coastline Lightbar)"
        badgeText={
          !isBaseLightbarSupported
            ? 'Оборудование не обнаружено'
            : isEcLightbar
            ? 'Управление световой полосой через EC платы'
            : 'Контроллер ITE Lightbar4 (048D:7001)'
        }
        badgeVariant={!isBaseLightbarSupported ? 'default' : 'info'}
        description={
          !isBaseLightbarSupported
            ? 'На платформе текущей модели нет аппаратной передней световой полосы — функция умно отключена'
            : isEcLightbar
            ? 'Прямое аппаратное управление через чип EC платы: статический цвет, ритмичное дыхание и динамическая подсветка'
            : 'Световые полосы по передней кромке и бокам корпуса: встречные потоки, волны и метеоры'
        }
        enabled={lighting.lb_enabled}
        onToggle={() => sendLightbar({ lb_enabled: !lighting.lb_enabled })}
        disabled={!isBaseLightbarSupported}
        activeColorClass="active-cyan"
      />

      <div>
        <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginBottom: '8px', fontWeight: 600 }}>
          Режимы эффектов Coastline (Lightbar Modes)
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(130px, 1fr))', gap: '10px' }}>
          {LIGHTBAR_MODES.map((m) => {
            const isActive = lighting.lb_effect === m.id;
            return (
              <button
                type="button"
                key={m.id}
                onClick={() => sendLightbar({ lb_effect: m.id })}
                className={`mode-btn ${isActive ? 'active-cyan' : ''}`}
                style={{
                  padding: '12px 10px',
                  textAlign: 'center',
                  background: isActive ? 'var(--status-info-dim)' : 'var(--surface-badge)',
                  border: isActive ? '1px solid var(--status-info)' : '1px solid var(--border-subtle)',
                }}
              >
                <div style={{ fontSize: '13px', fontWeight: 700, color: isActive ? 'var(--text-primary)' : 'var(--text-muted)' }}>
                  {m.name}
                </div>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', marginTop: '2px' }}>{m.desc}</div>
              </button>
            );
          })}
        </div>
      </div>

      <div
        className="lighting-sub-panel"
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
          gap: '14px',
        }}
      >
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-secondary)' }}>
              <Sun className="w-3.5 h-3.5" style={{ color: 'var(--status-warn)' }} />
              Яркость полосы
            </span>
            <span style={{ color: 'var(--status-info)', fontWeight: 700 }}>
              {lighting.lb_brightness === 0 ? 'Выключено (0)' : `${lighting.lb_brightness} ступ.`}
            </span>
          </div>
          <input
            type="range"
            min="0"
            max="4"
            step="1"
            value={lighting.lb_brightness}
            onChange={(e) => sendLightbar({ lb_brightness: Number(e.target.value) })}
            style={{ width: '100%', accentColor: 'var(--status-info)', cursor: 'pointer' }}
          />
        </div>

        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-secondary)' }}>
              <Zap className="w-3.5 h-3.5" style={{ color: 'var(--status-info)' }} />
              Скорость движения
            </span>
            <span style={{ color: 'var(--status-info)', fontWeight: 700 }}>{lighting.lb_speed + 1} ступ.</span>
          </div>
          <input
            type="range"
            min="0"
            max="4"
            step="1"
            value={lighting.lb_speed}
            onChange={(e) => sendLightbar({ lb_speed: Number(e.target.value) })}
            style={{ width: '100%', accentColor: 'var(--status-info)', cursor: 'pointer' }}
          />
        </div>
      </div>

      <LightColorRow
        label="Основной цвет световой полосы:"
        color={lighting.lb_color}
        onChangeColor={(hex) => sendLightbar({ lb_color: hex })}
        iconColor="var(--status-info)"
      />
    </div>
  );
};
