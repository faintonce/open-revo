import React from 'react';
import { HardwareState } from '../../types/hardware';
import { Palette, Heart, Minus, X } from 'lucide-react';
import logoImg from '../../assets/logo.png';

interface DrawerHeaderProps {
  status: HardwareState;
  theme: 'mono' | 'cyber';
  onToggleTheme: () => void;
  onOpenSponsor: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

export const DrawerHeader: React.FC<DrawerHeaderProps> = ({
  status,
  theme,
  onToggleTheme,
  onOpenSponsor,
  onMinimize,
  onClose,
}) => {
  return (
    <div
      data-tauri-drag-region
      style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '2px 2px 6px 2px',
        borderBottom: '1px solid rgba(255, 255, 255, 0.06)',
        cursor: 'default',
      }}
    >
      <div data-tauri-drag-region style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
        <div className="logo-badge" data-tauri-drag-region style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <img
            src={logoImg}
            alt="OpenRevo Logo"
            data-tauri-drag-region
            style={{
              width: '19px',
              height: '19px',
              borderRadius: '3px',
              objectFit: 'contain',
              filter: `drop-shadow(0 0 6px ${status.power_mode === 3 ? '#f43f5e' : status.power_mode === 1 ? '#10b981' : '#38bdf8'})`,
            }}
          />
          <span data-tauri-drag-region style={{ fontWeight: 800, fontSize: '13px', letterSpacing: '0.06em' }}>OPENREVO</span>
          <span className="badge-tag" data-tauri-drag-region style={{ fontSize: '9px', padding: '1px 5px', borderRadius: '4px' }}>MINI</span>
        </div>
      </div>

      {/* Central Draggable Area */}
      <div
        data-tauri-drag-region
        style={{ flex: 1, height: '100%', minHeight: '22px', cursor: 'default' }}
      />

      <div
        data-tauri-drag-region="false"
        onMouseDown={(e) => e.stopPropagation()}
        style={{ display: 'flex', alignItems: 'center', gap: '3px', zIndex: 10 }}
      >
        {/* 主题切换开关 (单色/赛博) */}
        <button
          onClick={onToggleTheme}
          title={theme === 'mono' ? 'Переключиться на: тематическую палитру Cyber' : 'Переключиться на: минималистичную чисто монохромную палитру'}
          style={{
            background: theme === 'cyber' ? 'rgba(56, 189, 248, 0.15)' : 'transparent',
            border: theme === 'cyber' ? '1px solid rgba(56, 189, 248, 0.4)' : 'none',
            color: theme === 'cyber' ? '#38bdf8' : 'var(--text-muted)',
            cursor: 'pointer',
            padding: '3px 5px',
            borderRadius: '4px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.15s ease',
          }}
        >
          <Palette size={12} />
        </button>

        {/* 赞赏与支持 */}
        <button
          onClick={onOpenSponsor}
          title="Поддержать автора (донат и поддержка открытого кода)"
          style={{
            background: 'transparent',
            border: 'none',
            color: 'var(--text-muted)',
            cursor: 'pointer',
            padding: '3px 5px',
            borderRadius: '4px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.15s ease',
          }}
        >
          <Heart size={12} style={{ color: '#f43f5e' }} />
        </button>

        {/* 最小化 */}
        <button
          onClick={onMinimize}
          title="Свернуть в трей"
          style={{
            background: 'transparent',
            border: 'none',
            color: 'var(--text-muted)',
            cursor: 'pointer',
            padding: '3px 5px',
            borderRadius: '4px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <Minus size={12} />
        </button>

        {/* 关闭 */}
        <button
          onClick={onClose}
          title="Закрыть окно (фоновая служба в трее продолжит работать)"
          style={{
            background: 'transparent',
            border: 'none',
            color: 'var(--text-muted)',
            cursor: 'pointer',
            padding: '3px 5px',
            borderRadius: '4px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <X size={12} />
        </button>
      </div>
    </div>
  );
};
