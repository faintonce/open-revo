import React from 'react';
import { Sun } from 'lucide-react';
import {
  LightingState,
  KeyboardHardwareInfo,
  LOGO_MODES,
  EC_LOGO_MODES,
  PRESET_COLORS,
} from './lightingConstants';
import { LightHeaderBanner } from './LightHeaderBanner';

interface LogoLightingViewProps {
  lighting: LightingState;
  kbHwInfo: KeyboardHardwareInfo | null;
  sendLogo: (patch: Partial<LightingState>) => void;
}

export const LogoLightingView: React.FC<LogoLightingViewProps> = ({
  lighting,
  kbHwInfo,
  sendLogo,
}) => {
  const isEcDirectLogo = Boolean(kbHwInfo?.is_ec_logo);
  const activeLogoModes = isEcDirectLogo ? EC_LOGO_MODES : LOGO_MODES;
  const isLogoSupported = Boolean(kbHwInfo && kbHwInfo.has_logo);
  const currentLogoEffect =
    isEcDirectLogo && lighting.logo_effect !== 1 && lighting.logo_effect !== 2 && lighting.logo_effect !== 20
      ? 1
      : lighting.logo_effect;

  React.useEffect(() => {
    if (isEcDirectLogo && lighting.logo_effect !== 1 && lighting.logo_effect !== 2 && lighting.logo_effect !== 20) {
      sendLogo({ logo_effect: 1 });
    }
  }, [isEcDirectLogo, lighting.logo_effect, sendLogo]);

  const badgeText = !isLogoSupported
    ? 'Оборудование не обнаружено'
    : isEcDirectLogo
    ? 'Щит-логотип с прямым управлением от EC платы (PWM)'
    : kbHwInfo?.has_ite7000
    ? 'Отдельный контроллер логотипа ITE (048D:7000)'
    : kbHwInfo?.has_ite7001
    ? 'Отдельный многоцветный контроллер логотипа ITE (048D:7001, текстовый логотип)'
    : 'Контроллер полноцветного светящегося логотипа';

  const badgeVariant = !isLogoSupported
    ? 'default'
    : isEcDirectLogo
    ? 'info'
    : 'accent';

  const description = !isLogoSupported
    ? 'Платформа текущей модели не имеет отдельного светящегося логотипа на крышке A — функция умно отключена'
    : isEcDirectLogo
    ? 'Прямое аппаратное управление через EC платы (щит-логотип: одноцветное свечение, дыхание и радужное дыхание)'
    : 'Отдельный многоцветный светящийся логотип на крышке A (бегущий огонь, волны, встречные потоки, метеоры и другие полноцветные эффекты)';

  return (
    <div className="glass-card" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
      <LightHeaderBanner
        title="Многоцветный логотип на крышке A (A-Cover LOGO)"
        badgeText={badgeText}
        badgeVariant={badgeVariant}
        description={description}
        enabled={lighting.logo_enabled}
        onToggle={() => sendLogo({ logo_enabled: !lighting.logo_enabled })}
        disabled={!isLogoSupported}
        activeColorClass="active-emerald"
      />

      <div>
        <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginBottom: '8px', fontWeight: 600 }}>
          Режимы подсветки логотипа (Logo Modes)
        </div>
        <div
          style={{
            display: 'grid',
            gridTemplateColumns:
              activeLogoModes.length <= 3
                ? `repeat(${activeLogoModes.length}, 1fr)`
                : 'repeat(auto-fill, minmax(130px, 1fr))',
            gap: '10px',
          }}
        >
          {activeLogoModes.map((m) => {
            const isActive = currentLogoEffect === m.id;
            return (
              <button
                type="button"
                key={m.id}
                onClick={() => sendLogo({ logo_effect: m.id })}
                className={`mode-btn ${isActive ? 'active-beast' : ''}`}
                style={{
                  padding: '12px 8px',
                  textAlign: 'center',
                  background: isActive ? 'var(--status-error-dim)' : 'var(--surface-badge)',
                  border: isActive ? '1px solid var(--color-beast)' : '1px solid var(--border-subtle)',
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

      {/* Logo Brightness & HSV Precision Color System */}
      <div
        className="lighting-sub-panel"
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1.2fr',
          gap: '16px',
        }}
      >
        {/* Left: Brightness */}
        <div style={{ display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: '12px' }}>
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-secondary)' }}>
                <Sun className="w-3.5 h-3.5" style={{ color: 'var(--status-warn)' }} />
                Яркость подсветки логотипа
              </span>
              <span style={{ color: 'var(--color-beast)', fontWeight: 700 }}>
                {lighting.logo_brightness === 0 ? 'Выключено (0)' : `${lighting.logo_brightness} ступ.`}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="4"
              step="1"
              value={lighting.logo_brightness}
              onChange={(e) => sendLogo({ logo_brightness: Number(e.target.value) })}
              style={{ width: '100%', accentColor: 'var(--color-beast)', cursor: 'pointer' }}
            />
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginTop: '8px' }}>
            <div
              style={{
                width: '36px',
                height: '36px',
                borderRadius: 'var(--radius-sm)',
                backgroundColor: lighting.logo_color,
                boxShadow: `0 0 16px ${lighting.logo_color}`,
                border: '2px solid rgba(255, 255, 255, 0.4)',
              }}
            />
            <div>
              <div style={{ fontSize: '12px', fontWeight: 700, color: 'var(--text-primary)' }}>Мгновенный цвет логотипа</div>
              <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)' }}>
                RGB: {lighting.logo_color.toUpperCase()}
              </div>
            </div>
          </div>
        </div>

        {/* Right: Color Presets & Direct Picker */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
          <div style={{ fontSize: '11px', color: 'var(--text-muted)', fontWeight: 600 }}>Референсы палитр и быстрое применение:</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
            {PRESET_COLORS.map((c) => {
              const isSelected = lighting.logo_color.toLowerCase() === c.hex.toLowerCase();
              return (
                <button
                  type="button"
                  key={c.hex}
                  onClick={() => sendLogo({ logo_color: c.hex })}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    background: isSelected ? 'var(--surface-card-active)' : 'var(--surface-badge)',
                    border: isSelected ? `1px solid ${c.hex}` : '1px solid var(--border-subtle)',
                    padding: '4px 8px',
                    borderRadius: 'var(--radius-xs)',
                    cursor: 'pointer',
                    transition: 'all var(--transition-fast)',
                  }}
                >
                  <div style={{ width: '12px', height: '12px', borderRadius: 'var(--radius-full)', backgroundColor: c.hex }} />
                  <span style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>{c.name}</span>
                </button>
              );
            })}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '6px' }}>
            <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Пользовательская палитра:</span>
            <input
              type="color"
              value={lighting.logo_color}
              onChange={(e) => sendLogo({ logo_color: e.target.value })}
              style={{ width: '32px', height: '28px', border: 'none', background: 'transparent', cursor: 'pointer' }}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
