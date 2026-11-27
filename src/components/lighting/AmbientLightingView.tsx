import React from 'react';
import { Sun, Zap } from 'lucide-react';
import {
  LightingState,
  KeyboardHardwareInfo,
  LIGHTBAR_MODES,
} from './lightingConstants';
import { LightHeaderBanner } from './LightHeaderBanner';
import { LightColorRow } from './LightColorRow';

interface AmbientLightingViewProps {
  lighting: LightingState;
  kbHwInfo: KeyboardHardwareInfo | null;
  sendHinge: (patch: Partial<LightingState>) => void;
}

export const AmbientLightingView: React.FC<AmbientLightingViewProps> = ({
  lighting,
  kbHwInfo,
  sendHinge,
}) => {
  const isHingeSupported = Boolean(kbHwInfo && kbHwInfo.has_hinge);

  return (
    <div className="glass-card" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
      <LightHeaderBanner
        title="Фоновая подсветка задней части корпуса (Rear Vent & Hinge Ambient Light)"
        badgeText={kbHwInfo?.has_ite7000 ? 'Контроллер ITE Hinge (048D:7000)' : 'Аппаратура переливающейся подсветки петель'}
        badgeVariant="info"
        description="Фоновые световые полосы на двойных задних вентиляционных решётках и петлях; двухрежимные волны, ритмичное дыхание и быстрые метеоры"
        enabled={lighting.hinge_enabled}
        onToggle={() => sendHinge({ hinge_enabled: !lighting.hinge_enabled })}
        disabled={!isHingeSupported}
        activeColorClass="active-cyan"
      />

      <div>
        <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginBottom: '8px', fontWeight: 600 }}>
          Режимы фоновой подсветки (Ambient Modes)
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '10px' }}>
          {LIGHTBAR_MODES.map((m) => {
            const isActive = lighting.hinge_effect === m.id;
            return (
              <button
                type="button"
                key={m.id}
                onClick={() => sendHinge({ hinge_effect: m.id })}
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
              Яркость фоновой подсветки
            </span>
            <span style={{ color: 'var(--status-info)', fontWeight: 700 }}>
              {lighting.hinge_brightness === 0 ? 'Выключено (0)' : `${lighting.hinge_brightness} ступ.`}
            </span>
          </div>
          <input
            type="range"
            min="0"
            max="4"
            step="1"
            value={lighting.hinge_brightness}
            onChange={(e) => sendHinge({ hinge_brightness: Number(e.target.value) })}
            style={{ width: '100%', accentColor: 'var(--status-info)', cursor: 'pointer' }}
          />
        </div>

        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-secondary)' }}>
              <Zap className="w-3.5 h-3.5" style={{ color: 'var(--status-info)' }} />
              Скорость движения
            </span>
            <span style={{ color: 'var(--status-info)', fontWeight: 700 }}>{lighting.hinge_speed + 1} ступ.</span>
          </div>
          <input
            type="range"
            min="0"
            max="4"
            step="1"
            value={lighting.hinge_speed}
            onChange={(e) => sendHinge({ hinge_speed: Number(e.target.value) })}
            style={{ width: '100%', accentColor: 'var(--status-info)', cursor: 'pointer' }}
          />
        </div>
      </div>

      <LightColorRow
        label="Основной цвет фоновой подсветки:"
        color={lighting.hinge_color}
        onChangeColor={(hex) => sendHinge({ hinge_color: hex })}
        iconColor="var(--status-info)"
      />
    </div>
  );
};
