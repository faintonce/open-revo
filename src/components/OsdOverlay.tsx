import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface OsdConfig {
  enabled: boolean;
  theme: 'cinema' | 'fade' | 'pill';
  scale: string; // "80%" | "100%" | "120%" | "140%"
  position: 'top-left' | 'bottom-center' | 'center';
}

interface OsdDisplayState {
  title: string;
  icon: React.ReactNode;
  accent: string;
  glow?: string;
  iconBg?: string;
  iconBorder?: string;
  steps?: number; // 0 to 4 for brightness
  dot?: 'on' | 'off' | 'neutral';
}

const OSD_PALETTES = {
  office: {
    accent: 'var(--color-office)',
    glow: 'var(--status-ok-dim)',
    iconBg: 'rgba(16, 185, 129, 0.15)',
    iconBorder: 'rgba(16, 185, 129, 0.35)',
  },
  balance: {
    accent: 'var(--color-balance)',
    glow: 'var(--status-info-dim)',
    iconBg: 'rgba(59, 130, 246, 0.15)',
    iconBorder: 'rgba(59, 130, 246, 0.35)',
  },
  beast: {
    accent: 'var(--color-beast)',
    glow: 'rgba(244, 63, 94, 0.4)',
    iconBg: 'rgba(244, 63, 94, 0.15)',
    iconBorder: 'rgba(244, 63, 94, 0.35)',
  },
  geek: {
    accent: 'var(--color-brand)',
    glow: 'var(--border-glow)',
    iconBg: 'rgba(99, 102, 241, 0.15)',
    iconBorder: 'rgba(99, 102, 241, 0.35)',
  },
  fan: {
    accent: 'var(--color-fan)',
    glow: 'rgba(6, 182, 212, 0.4)',
    iconBg: 'rgba(6, 182, 212, 0.15)',
    iconBorder: 'rgba(6, 182, 212, 0.35)',
  },
  warn: {
    accent: 'var(--color-gold)',
    glow: 'var(--status-warn-dim)',
    iconBg: 'rgba(245, 158, 11, 0.15)',
    iconBorder: 'rgba(245, 158, 11, 0.35)',
  },
  enabled: {
    accent: 'var(--status-info)',
    glow: 'var(--status-info-dim)',
    iconBg: 'rgba(56, 189, 248, 0.15)',
    iconBorder: 'rgba(56, 189, 248, 0.35)',
  },
  disabled: {
    accent: 'var(--status-error)',
    glow: 'var(--status-error-dim)',
    iconBg: 'rgba(239, 68, 68, 0.15)',
    iconBorder: 'rgba(239, 68, 68, 0.35)',
  },
  neutral: {
    accent: 'var(--text-secondary)',
    glow: 'rgba(203, 213, 225, 0.2)',
    iconBg: 'rgba(255, 255, 255, 0.08)',
    iconBorder: 'var(--border-subtle)',
  },
};

export const OsdOverlay: React.FC = () => {
  const [config, setConfig] = useState<OsdConfig>({
    enabled: true,
    theme: 'cinema',
    scale: '100%',
    position: 'top-left',
  });

  const [active, setActive] = useState<boolean>(false);
  const [display, setDisplay] = useState<OsdDisplayState>({
    title: 'Режим Турбо',
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
        <path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z" />
      </svg>
    ),
    ...OSD_PALETTES.beast,
  });

  const [seqKey, setSeqKey] = useState<number>(0);

  const hideTimerRef = useRef<NodeJS.Timeout | null>(null);
  const configRef = useRef<OsdConfig>(config);
  configRef.current = config;

  const showOsd = (newState: OsdDisplayState) => {
    if (!configRef.current.enabled) return;

    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }

    invoke('show_osd_window').catch(() => {});
    setDisplay(newState);
    setActive(true);
    setSeqKey((prev) => prev + 1);

    hideTimerRef.current = setTimeout(() => {
      setActive(false);
      setTimeout(() => {
        invoke('hide_osd_window').catch(() => {});
      }, 250);
    }, 1600);
  };

  // Dedicated preview trigger: always pops up OSD HUD regardless of enabled status,
  // without mutating or persisting enabled=true (preserves disabled state if turned off).
  const showPreviewOsd = (previewCfg?: Partial<OsdConfig>) => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }

    if (previewCfg) {
      setConfig((prev) => ({
        ...prev,
        theme: previewCfg.theme || prev.theme,
        scale: previewCfg.scale || prev.scale,
        position: previewCfg.position || prev.position,
      }));
    }

    invoke('show_osd_window').catch(() => {});
    setDisplay({
      title: 'Режим Турбо',
      icon: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
          <path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z" />
        </svg>
      ),
      ...OSD_PALETTES.beast,
    });
    setActive(true);
    setSeqKey((prev) => prev + 1);

    hideTimerRef.current = setTimeout(() => {
      setActive(false);
      setTimeout(() => {
        invoke('hide_osd_window').catch(() => {});
      }, 250);
    }, 1600);
  };

  // Dynamic theme skin loader for OSD window
  const syncOsdSkin = async (skinId: string | null) => {
    const existing = (document.getElementById('openrevo-theme-skin') || document.getElementById('openrevo-custom-skin')) as HTMLStyleElement | null;
    if (existing) {
      existing.remove();
    }
    if (skinId) {
      document.documentElement.setAttribute('data-skin', skinId);
      try {
        const css = await invoke<string>('load_plugin_theme_css', { pluginId: skinId });
        const newStyle = document.createElement('style');
        newStyle.id = 'openrevo-theme-skin';
        newStyle.textContent = css;
        document.head.appendChild(newStyle);
      } catch (e) {
        console.warn('[OSD] Failed to load theme CSS for skin:', skinId, e);
      }
    } else {
      document.documentElement.removeAttribute('data-skin');
    }
  };

  useEffect(() => {
    const unlistenFns: (() => void)[] = [];

    // 0. Synchronize active theme skin on boot & listen to cross-window changes
    invoke<string | null>('get_active_skin')
      .then(syncOsdSkin)
      .catch(() => {});

    listen<{ skinId: string | null }>('active-skin-changed', (event) => {
      syncOsdSkin(event.payload.skinId);
    }).then((un) => unlistenFns.push(un));

    // 1. Fetch initial OSD configuration from backend
    invoke<any>('get_osd_config')
      .then((res) => {
        if (res) {
          setConfig({
            enabled: res.enabled ?? true,
            theme: res.theme || 'cinema',
            scale: res.scale || '100%',
            position: res.position || 'top-left',
          });
        }
      })
      .catch(() => {});

    // Helper to register listeners
    const addListener = async <T,>(event: string, handler: (payload: T) => void) => {
      const unlisten = await listen<T>(event, (e) => handler(e.payload));
      unlistenFns.push(unlisten);
    };

    // 0. Dedicated Preview Listener (Always displays, never modifies enabled switch)
    addListener<any>('trigger-osd-preview', (payload) => {
      showPreviewOsd(payload);
    });

    // 1. Config changed listener
    addListener<any>('osd-config-changed', (payload) => {
      const newEnabled = payload.enabled ?? true;
      const wasEnabled = configRef.current.enabled;
      const wasTheme = configRef.current.theme;
      const wasScale = configRef.current.scale;
      const wasPos = configRef.current.position;

      const newCfg: OsdConfig = {
        enabled: newEnabled,
        theme: payload.theme || 'cinema',
        scale: payload.scale || '100%',
        position: payload.position || 'top-left',
      };
      setConfig(newCfg);
      configRef.current = newCfg;

      if (!wasEnabled && newEnabled) {
        // Toggle switched from Disabled -> Enabled: show confirmation HUD
        showOsd({
          title: 'OSD включено',
          icon: (
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          ),
          ...OSD_PALETTES.office,
        });
      } else if (newEnabled && (wasTheme !== newCfg.theme || wasScale !== newCfg.scale || wasPos !== newCfg.position)) {
        // In enabled state, user switched Theme, Scale, or Position menu items: auto-preview new appearance immediately!
        showPreviewOsd(newCfg);
      }
    });

    // 2. Q-Key Mode switched
    addListener<any>('physical-mode-switched', (payload) => {
      const mode = typeof payload === 'number' ? payload : parseInt(String(payload), 10);
      if (mode === 1) {
        showOsd({
          title: 'Офисный режим',
          icon: (
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
            </svg>
          ),
          ...OSD_PALETTES.office,
        });
      } else if (mode === 2) {
        showOsd({
          title: 'Режим Баланса',
          icon: (
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
              <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
            </svg>
          ),
          ...OSD_PALETTES.balance,
        });
      } else if (mode === 3) {
        showOsd({
          title: 'Режим Турбо',
          icon: (
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
              <path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z" />
            </svg>
          ),
          ...OSD_PALETTES.beast,
        });
      } else if (mode === 4) {
        showOsd({
          title: 'Экспертная настройка',
          icon: (
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
              <line x1="4" y1="21" x2="4" y2="14" /><line x1="4" y1="10" x2="4" y2="3" /><line x1="12" y1="21" x2="12" y2="12" /><line x1="12" y1="8" x2="12" y2="3" /><line x1="20" y1="21" x2="20" y2="16" /><line x1="20" y1="12" x2="20" y2="3" /><line x1="1" y1="14" x2="7" y2="14" /><line x1="9" y1="8" x2="15" y2="8" /><line x1="17" y1="16" x2="23" y2="16" />
            </svg>
          ),
          ...OSD_PALETTES.geek,
        });
      }
    });

    // 3. Keyboard Backlight Brightness (0~4)
    addListener<any>('keyboard-brightness-changed', (payload) => {
      const lvl = typeof payload === 'number' ? payload : parseInt(String(payload), 10);
      const pctMap = [0, 25, 50, 75, 100];
      const pct = pctMap[lvl] ?? 100;
      showOsd({
        title: lvl === 0 ? 'Подсветка клавиатуры выключена' : `Подсветка клавиатуры: ${pct}%`,
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <rect x="2" y="4" width="20" height="16" rx="2" />
            <line x1="6" y1="8" x2="6.01" y2="8" />
            <line x1="10" y1="8" x2="10.01" y2="8" />
            <line x1="14" y1="8" x2="14.01" y2="8" />
            <line x1="18" y1="8" x2="18.01" y2="8" />
            <line x1="6" y1="12" x2="6.01" y2="12" />
            <line x1="18" y1="12" x2="18.01" y2="12" />
            <line x1="9" y1="16" x2="15" y2="16" />
          </svg>
        ),
        steps: lvl,
        ...OSD_PALETTES.enabled,
      });
    });

    // 3.1 Keyboard Backlight Sleep Timeout
    addListener<any>('keyboard-sleep-timeout-changed', (payload) => {
      const sec = typeof payload === 'number' ? payload : parseInt(String(payload), 10);
      showOsd({
        title: sec === 0 ? 'Подсветка клавиатуры постоянно' : `Подсветка: ${sec} с без действий — гашение`,
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <rect x="2" y="4" width="20" height="16" rx="2" />
            <line x1="6" y1="8" x2="6.01" y2="8" />
            <line x1="10" y1="8" x2="10.01" y2="8" />
            <line x1="14" y1="8" x2="14.01" y2="8" />
            <line x1="18" y1="8" x2="18.01" y2="8" />
            <line x1="6" y1="12" x2="6.01" y2="12" />
            <line x1="18" y1="12" x2="18.01" y2="12" />
            <line x1="9" y1="16" x2="15" y2="16" />
          </svg>
        ),
        dot: sec === 0 ? 'on' : 'neutral',
        ...(sec === 0 ? OSD_PALETTES.enabled : OSD_PALETTES.neutral),
      });
    });

    // 4. Touchpad toggle
    addListener<boolean>('touchpad-state-changed', (enabled) => {
      showOsd({
        title: enabled ? 'Тачпад включён' : 'Тачпад отключён',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <rect x="2" y="4" width="20" height="16" rx="3" />
            <line x1="2" y1="15" x2="22" y2="15" />
            <line x1="12" y1="15" x2="12" y2="20" />
          </svg>
        ),
        dot: enabled ? 'on' : 'off',
        ...(enabled ? OSD_PALETTES.enabled : OSD_PALETTES.disabled),
      });
    });

    // 5. Fan Boost toggle
    const handleFanBoost = (enabled: boolean) => {
      showOsd({
        title: enabled ? 'Форс-охлаждение включено' : 'Форс-охлаждение выключено',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <circle cx="12" cy="12" r="3" />
            <path d="M12 9V3a3 3 0 0 1 3 3v3h-3zM15 12h6a3 3 0 0 1-3 3h-3v-3zM12 15v6a3 3 0 0 1-3-3v-3h3zM9 12H3a3 3 0 0 1 3-3h3v3z" />
          </svg>
        ),
        dot: enabled ? 'on' : 'neutral',
        ...(enabled ? OSD_PALETTES.fan : OSD_PALETTES.neutral),
      });
    };
    addListener<boolean>('fan-boost-changed', handleFanBoost);
    addListener<boolean>('physical-fan-boost-switched', handleFanBoost);

    // 6. Win Key Lock toggle
    addListener<boolean>('win-lock-state-changed', (locked) => {
      showOsd({
        title: locked ? 'Клавиша Win заблокирована' : 'Клавиша Win разблокирована',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
        ),
        dot: locked ? 'off' : 'on',
        ...(locked ? OSD_PALETTES.warn : OSD_PALETTES.office),
      });
    });

    // 7. Fn Key Lock toggle
    addListener<boolean>('fn-lock-state-changed', (locked) => {
      showOsd({
        title: locked ? 'Клавиша Fn заблокирована' : 'Клавиша Fn разблокирована',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <rect x="2" y="4" width="20" height="16" rx="2" />
            <path d="M7 15V9h4M7 12h3M14 15v-4a2 2 0 1 1 4 0v4" />
          </svg>
        ),
        dot: locked ? 'on' : 'neutral',
        ...(locked ? OSD_PALETTES.geek : OSD_PALETTES.neutral),
      });
    });

    // 8. Camera toggle
    addListener<boolean>('camera-state-changed', (enabled) => {
      showOsd({
        title: enabled ? 'Камера включена' : 'Камера выключена',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
            <circle cx="12" cy="13" r="4" />
          </svg>
        ),
        dot: enabled ? 'on' : 'off',
        ...(enabled ? OSD_PALETTES.office : OSD_PALETTES.disabled),
      });
    });

    // 9. Mic toggle
    addListener<boolean>('mic-state-changed', (enabled) => {
      showOsd({
        title: enabled ? 'Микрофон включён' : 'Микрофон заглушён',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <line x1="12" y1="19" x2="12" y2="23" />
            <line x1="8" y1="23" x2="16" y2="23" />
          </svg>
        ),
        dot: enabled ? 'on' : 'off',
        ...(enabled ? OSD_PALETTES.office : OSD_PALETTES.disabled),
      });
    });

    // 10. Caps Lock toggle
    addListener<boolean>('caps-lock-changed', (isCapsOn) => {
      showOsd({
        title: isCapsOn ? 'Caps Lock включён' : 'Caps Lock выключен',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <path d="M12 4L6 10h4v6h4v-6h4z" />
            <line x1="6" y1="20" x2="18" y2="20" />
          </svg>
        ),
        dot: isCapsOn ? 'on' : 'neutral',
        ...(isCapsOn ? OSD_PALETTES.enabled : OSD_PALETTES.neutral),
      });
    });

    // 11. Num Lock toggle
    addListener<boolean>('num-lock-changed', (isNumOn) => {
      showOsd({
        title: isNumOn ? 'Num Lock включён' : 'Num Lock выключен',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <line x1="3" y1="9" x2="21" y2="9" />
            <line x1="9" y1="9" x2="9" y2="21" />
            <line x1="15" y1="9" x2="15" y2="21" />
            <line x1="3" y1="15" x2="21" y2="15" />
          </svg>
        ),
        dot: isNumOn ? 'on' : 'neutral',
        ...(isNumOn ? OSD_PALETTES.enabled : OSD_PALETTES.neutral),
      });
    });

    // 12. Scroll Lock toggle
    addListener<boolean>('scroll-lock-changed', (isScrollOn) => {
      showOsd({
        title: isScrollOn ? 'Scroll Lock включён' : 'Scroll Lock выключен',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <rect x="4" y="2" width="16" height="20" rx="3" />
            <path d="M12 7v5m0 0l-2-2m2 2l2-2" />
            <path d="M12 17v-5m0 0l-2 2m2-2l2 2" />
          </svg>
        ),
        dot: isScrollOn ? 'on' : 'neutral',
        ...(isScrollOn ? OSD_PALETTES.enabled : OSD_PALETTES.neutral),
      });
    });

    // 13. Wi-Fi toggle
    addListener<boolean>('wifi-state-changed', (enabled) => {
      showOsd({
        title: enabled ? 'Wi-Fi включён' : 'Wi-Fi выключен',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <path d="M5 12.55a11 11 0 0 1 14.08 0" />
            <path d="M1.42 9a16 16 0 0 1 21.16 0" />
            <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
            <line x1="12" y1="20" x2="12.01" y2="20" strokeWidth="3" strokeLinecap="round" />
          </svg>
        ),
        dot: enabled ? 'on' : 'off',
        ...(enabled ? OSD_PALETTES.enabled : OSD_PALETTES.disabled),
      });
    });

    // 14. Bluetooth toggle
    addListener<boolean>('bluetooth-state-changed', (enabled) => {
      showOsd({
        title: enabled ? 'Bluetooth включён' : 'Bluetooth выключен',
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <path d="m7 7 10 10-5 5V2l5 5L7 17" />
          </svg>
        ),
        dot: enabled ? 'on' : 'off',
        ...(enabled ? OSD_PALETTES.enabled : OSD_PALETTES.disabled),
      });
    });

    // 15. Custom Shortcut / Plugin OSD Message
    addListener<any>('custom-osd-message', (payload) => {
      if (!payload || !payload.title) return;
      const palette = (OSD_PALETTES as any)[payload.theme || payload.accent] || OSD_PALETTES.geek;
      showOsd({
        title: payload.title,
        icon: (
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.3">
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
          </svg>
        ),
        dot: payload.dot || 'neutral',
        ...palette,
      });
    });

    return () => {
      unlistenFns.forEach((fn) => fn());
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
    };
  }, []);

  // Compute CSS scale factor
  const getScaleValue = () => {
    switch (config.scale) {
      case '80%': return 0.8;
      case '120%': return 1.2;
      case '140%': return 1.4;
      default: return 1.0;
    }
  };

  const scaleValue = getScaleValue();
  const isTopLeft = config.position === 'top-left';

  return (
    <div
      className="osd-root"
      data-slot="osd-root"
      data-osd-pos={config.position}
      style={{
        width: '100vw',
        height: '100vh',
        background: 'transparent',
        margin: 0,
        padding: 0,
        boxSizing: 'border-box',
        overflow: 'hidden',
        pointerEvents: 'none',
        userSelect: 'none',
        display: 'flex',
        alignItems: 'center',
        justifyContent: isTopLeft ? 'flex-start' : 'center',
        paddingLeft: isTopLeft ? '16px' : '0',
      }}
    >
      <div
        className={`osd-container osd-slot-container osd-theme-${config.theme} ${active ? 'osd-active' : ''}`}
        data-slot="osd-container"
        data-osd-theme={config.theme}
        style={{
          '--accent-color': display.accent,
          '--glow-color': display.glow || 'var(--status-info-dim)',
          '--icon-bg': display.iconBg || 'var(--surface-badge)',
          '--icon-border': display.iconBorder || 'var(--border-subtle)',
          transform: active
            ? `scale(${scaleValue}) translateY(0)`
            : `scale(${scaleValue}) translateY(-8px)`,
          transformOrigin: isTopLeft ? 'left center' : 'center center',
          opacity: active ? 1 : 0,
          transition: 'opacity 0.22s cubic-bezier(0.16, 1, 0.3, 1), transform 0.22s cubic-bezier(0.16, 1, 0.3, 1)',
        } as React.CSSProperties}
      >
        {/* Icon Slot */}
        <div key={`icon-${seqKey}`} className="osd-icon-box osd-slot-icon" data-slot="icon">
          {display.icon}
        </div>

        {/* Content Slot */}
        <div key={`content-${seqKey}`} className="osd-content osd-slot-content" data-slot="content">
          <div className="osd-value osd-slot-title" data-slot="title">
            <span>{display.title}</span>
            {display.dot && (
              <span
                className={`status-dot osd-slot-dot ${
                  display.dot === 'on'
                    ? 'dot-on'
                    : display.dot === 'off'
                    ? 'dot-off'
                    : 'dot-neutral'
                }`}
                data-slot="status-dot"
                data-state={display.dot}
              />
            )}
          </div>

          {/* Level steps / Meter Slot (Keyboard backlight, etc.) */}
          {display.steps !== undefined && (
            <div className="osd-level-steps osd-slot-meter" data-slot="meter">
              {[1, 2, 3, 4].map((step) => (
                <div
                  key={step}
                  className={`level-bar osd-slot-meter-bar ${step <= display.steps! ? 'active' : ''}`}
                  data-slot="meter-bar"
                  data-level={step}
                  data-active={step <= display.steps!}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default OsdOverlay;
