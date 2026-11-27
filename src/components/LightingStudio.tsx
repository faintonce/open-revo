import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HardwareSupportFlags } from '../types/hardware';
import {
  Keyboard,
  Sparkles,
  Zap,
  Wind,
  RotateCcw,
  Clock,
  Save,
  Check,
  Cpu,
} from 'lucide-react';
import {
  LightingState,
  KeyboardHardwareInfo,
  SLEEP_OPTIONS,
} from './lighting/lightingConstants';
import { KeyboardLightingView } from './lighting/KeyboardLightingView';
import { LightbarView } from './lighting/LightbarView';
import { AmbientLightingView } from './lighting/AmbientLightingView';
import { LogoLightingView } from './lighting/LogoLightingView';
import { LightingTopologyOverrideModal } from './lighting/LightingTopologyOverrideModal';

interface LightingStudioProps {
  supportFlags?: HardwareSupportFlags;
}

export const LightingStudio: React.FC<LightingStudioProps> = ({ supportFlags }) => {
  const [subTab, setSubTab] = useState<'keyboard' | 'lightbar' | 'ambient' | 'logo'>('keyboard');
  const [kbHwInfo, setKbHwInfo] = useState<KeyboardHardwareInfo | null>(null);

  // If machine hardware doesn't support current subTab, fallback to keyboard
  useEffect(() => {
    if (subTab === 'lightbar' && !kbHwInfo?.has_front_lightbar && !kbHwInfo?.has_ite7001) {
      setSubTab('keyboard');
    } else if (subTab === 'ambient' && !kbHwInfo?.has_hinge) {
      setSubTab('keyboard');
    } else if (subTab === 'logo' && !kbHwInfo?.has_logo) {
      setSubTab('keyboard');
    }
  }, [supportFlags, kbHwInfo, subTab]);

  const [lighting, setLighting] = useState<LightingState>({
    kb_enabled: true,
    kb_effect: 3,
    kb_brightness: 3,
    kb_speed: 2,
    kb_direction: 0,
    kb_color: '#6366F1',
    kb_engine: 'hardware',
    kb_fps: 30,
    streamer_effect: 101,

    lb_enabled: true,
    lb_effect: 3,
    lb_brightness: 3,
    lb_speed: 2,
    lb_color: '#00FFFF',

    hinge_enabled: true,
    hinge_effect: 1,
    hinge_brightness: 3,
    hinge_speed: 2,
    hinge_color: '#00FFFF',

    kb_zones: ['#00FFFF', '#0000FF', '#FF00FF', '#FF0000'],

    logo_enabled: true,
    logo_effect: 1,
    logo_brightness: 3,
    logo_color: '#00FFFF',

    sleep_minutes: 10,
    ceremony_enabled: true,
  });

  const [isApplying, setIsApplying] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [isTopologyModalOpen, setIsTopologyModalOpen] = useState(false);

  const refreshHardwareAndLighting = () => {
    invoke<LightingState>('get_lighting_status')
      .then((st) => {
        if (st) setLighting(st);
      })
      .catch((_) => {});

    invoke<KeyboardHardwareInfo>('get_keyboard_hardware_info')
      .then((info) => {
        if (info) {
          setKbHwInfo(info);
          if (info.hardware_type === 'SingleZoneRgb' || info.hardware_type === 'SingleColor') {
            setLighting((prev) => ({
              ...prev,
              kb_enabled: info.is_on,
              kb_brightness: info.brightness_level > 0 ? info.brightness_level : prev.kb_brightness,
            }));
          }
        }
      })
      .catch((_) => {});
  };

  // Sync state on component mount
  useEffect(() => {
    let mounted = true;
    refreshHardwareAndLighting();

    invoke<KeyboardHardwareInfo>('get_keyboard_hardware_info')
      .then((info) => {
        if (mounted && info) {
          setKbHwInfo(info);
          if (info.hardware_type === 'SingleZoneRgb' || info.hardware_type === 'SingleColor') {
            setLighting((prev) => ({
              ...prev,
              kb_enabled: info.is_on,
              kb_brightness: info.brightness_level > 0 ? info.brightness_level : prev.kb_brightness,
            }));
          }
        }
      })
      .catch((_) => {});

    const unlistenPromise = listen<string>('kb-engine-changed', (event) => {
      if (mounted && event.payload) {
        setLighting((prev) => ({
          ...prev,
          kb_engine: event.payload as 'hardware' | 'streamer' | 'custom',
        }));
      }
    });

    const unlistenBrightnessPromise = listen<number>('keyboard-brightness-changed', (event) => {
      if (mounted && typeof event.payload === 'number') {
        const lvl = event.payload;
        setLighting((prev) => ({
          ...prev,
          kb_brightness: lvl > 0 ? lvl : prev.kb_brightness,
          kb_enabled: lvl > 0,
        }));
      }
    });

    const unlistenHwPromise = listen('lighting://hardware_info_changed', () => {
      if (mounted) {
        refreshHardwareAndLighting();
      }
    });

    const unlistenTopologyPromise = listen('lighting://topology_rehashed', () => {
      if (mounted) {
        refreshHardwareAndLighting();
      }
    });

    return () => {
      mounted = false;
      unlistenPromise.then((unsub) => unsub());
      unlistenBrightnessPromise.then((unsub) => unsub());
      unlistenHwPromise.then((unsub) => unsub());
      unlistenTopologyPromise.then((unsub) => unsub());
    };
  }, []);

  // Dispatch Keyboard
  const sendKeyboard = async (patch: Partial<LightingState>) => {
    const next = { ...lighting, ...patch };
    setLighting(next);
    const isStreamer = (next.kb_engine || 'hardware') === 'streamer' || next.kb_engine === 'custom';
    let rawStreamerEff = next.streamer_effect ?? 101;
    if (next.kb_engine === 'streamer' && (rawStreamerEff < 100 || rawStreamerEff >= 200)) {
      rawStreamerEff = 101;
    }
    const targetStreamerEff = rawStreamerEff >= 100 ? rawStreamerEff : rawStreamerEff + 100;
    const targetHwEff = next.kb_effect && next.kb_effect < 100 ? next.kb_effect : 3;
    try {
      await invoke('apply_keyboard_lighting', {
        enabled: next.kb_enabled,
        effect: isStreamer ? targetStreamerEff : targetHwEff,
        brightness: next.kb_brightness,
        speed: next.kb_speed,
        direction: next.kb_direction,
        color: next.kb_color,
        engine: next.kb_engine || 'hardware',
        fps: next.kb_fps || 30,
        streamer_effect: targetStreamerEff,
        streamerEffect: targetStreamerEff,
        custom_script_id: next.custom_script_id,
        customScriptId: next.custom_script_id,
      });
    } catch (e) {
      console.error('apply_keyboard_lighting failed:', e);
    }
  };

  // Dispatch Lightbar
  const sendLightbar = async (patch: Partial<LightingState>) => {
    const next = { ...lighting, ...patch };
    setLighting(next);
    try {
      await invoke('apply_lightbar_lighting', {
        enabled: next.lb_enabled,
        effect: next.lb_effect,
        brightness: next.lb_brightness,
        speed: next.lb_speed,
        color: next.lb_color,
      });
    } catch (e) {
      console.error('apply_lightbar_lighting failed:', e);
    }
  };

  // Dispatch Hinge Lightbar (尾部出风口海岸灯)
  const sendHinge = async (patch: Partial<LightingState>) => {
    const next = { ...lighting, ...patch };
    setLighting(next);
    try {
      await invoke('apply_hinge_lighting', {
        enabled: next.hinge_enabled ?? true,
        effect: next.hinge_effect ?? 1,
        brightness: next.hinge_brightness ?? 3,
        speed: next.hinge_speed ?? 2,
        color: next.hinge_color ?? '#00FFFF',
      });
    } catch (e) {
      console.error('apply_hinge_lighting failed:', e);
    }
  };

  // Dispatch Four-Zone Colors
  const sendFourZoneColors = async (zones: [string, string, string, string]) => {
    setLighting((prev) => ({ ...prev, kb_zones: zones }));
    try {
      await invoke('apply_four_zone_colors', { zones });
    } catch (e) {
      console.error('apply_four_zone_colors failed:', e);
    }
  };

  // Dispatch Logo
  const sendLogo = async (patch: Partial<LightingState>) => {
    const next = { ...lighting, ...patch };
    setLighting(next);
    try {
      await invoke('apply_logo_lighting', {
        enabled: next.logo_enabled,
        effect: next.logo_effect,
        brightness: next.logo_brightness,
        color: next.logo_color,
      });
    } catch (e) {
      console.error('apply_logo_lighting failed:', e);
    }
  };

  // Dispatch Sleep Timer
  const sendTimeout = async (minutes: number) => {
    setLighting((prev) => ({ ...prev, sleep_minutes: minutes }));
    try {
      await invoke('set_lighting_sleep_timer', { minutes });
    } catch (e) {
      console.error('set_lighting_sleep_timer failed:', e);
    }
  };

  // Toggle Ceremony Enabled
  const handleToggleCeremony = async () => {
    const nextVal = !(lighting.ceremony_enabled ?? true);
    setLighting((prev) => ({ ...prev, ceremony_enabled: nextVal }));
    try {
      await invoke('set_ceremony_enabled', { enabled: nextVal });
    } catch (e) {
      console.error('set_ceremony_enabled failed:', e);
    }
  };

  // Save current preset as startup base lighting
  const handleSaveAsStartup = async () => {
    try {
      await invoke('save_current_lighting_as_startup');
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2000);
    } catch (e) {
      console.error('save_current_lighting_as_startup failed:', e);
    }
  };

  // Trigger Welcome Animation
  const handleTriggerWelcome = async () => {
    if (isApplying) return;
    setIsApplying(true);
    try {
      await invoke('trigger_lighting_welcome');
    } catch (e) {
      console.error('trigger_lighting_welcome failed:', e);
    } finally {
      setTimeout(() => setIsApplying(false), 3500);
    }
  };

  // Restore Factory Defaults
  const handleResetDefaults = () => {
    if (subTab === 'keyboard') {
      sendKeyboard({
        kb_enabled: true,
        kb_effect: 3,
        kb_brightness: 3,
        kb_speed: 2,
        kb_direction: 0,
        kb_color: '#6366F1',
      });
      if (kbHwInfo?.is_four_zone) {
        sendFourZoneColors(['#00FFFF', '#0000FF', '#FF00FF', '#FF0000']);
      }
    } else if (subTab === 'lightbar') {
      sendLightbar({
        lb_enabled: true,
        lb_effect: 3,
        lb_brightness: 3,
        lb_speed: 2,
        lb_color: '#00FFFF',
      });
    } else if (subTab === 'ambient') {
      sendHinge({
        hinge_enabled: true,
        hinge_effect: 1,
        hinge_brightness: 3,
        hinge_speed: 2,
        hinge_color: '#00FFFF',
      });
    } else {
      sendLogo({
        logo_enabled: true,
        logo_effect: 1,
        logo_brightness: 3,
        logo_color: '#00FFFF',
      });
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
      {/* Top Device Navigation Tabs */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          background: 'var(--surface-sunken)',
          padding: '6px 12px',
          borderRadius: 'var(--radius-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <div style={{ display: 'flex', gap: '8px', overflowX: 'auto', alignItems: 'center' }}>
          <button
            type="button"
            onClick={() => setSubTab('keyboard')}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '6px 16px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '12px',
              fontWeight: 600,
              whiteSpace: 'nowrap',
              flexShrink: 0,
              background: subTab === 'keyboard' ? 'var(--border-glow)' : 'transparent',
              border: subTab === 'keyboard' ? '1px solid var(--color-brand)' : '1px solid transparent',
              color: subTab === 'keyboard' ? 'var(--text-primary)' : 'var(--text-muted)',
              cursor: 'pointer',
              transition: 'all var(--transition-fast)',
            }}
          >
            <Keyboard className="w-4 h-4" style={{ color: 'var(--color-brand)' }} />
            <span>Подсветка клавиатуры</span>
          </button>

          {Boolean(kbHwInfo?.has_front_lightbar || kbHwInfo?.has_ite7001) && (
            <button
              type="button"
              onClick={() => setSubTab('lightbar')}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                padding: '6px 16px',
                borderRadius: 'var(--radius-sm)',
                fontSize: '12px',
                fontWeight: 600,
                whiteSpace: 'nowrap',
                flexShrink: 0,
                background: subTab === 'lightbar' ? 'var(--status-info-dim)' : 'transparent',
                border: subTab === 'lightbar' ? '1px solid var(--status-info)' : '1px solid transparent',
                color: subTab === 'lightbar' ? 'var(--text-primary)' : 'var(--text-muted)',
                cursor: 'pointer',
                transition: 'all var(--transition-fast)',
              }}
            >
              <Zap className="w-4 h-4" style={{ color: 'var(--status-info)' }} />
              <span>Полоса Coastline</span>
            </button>
          )}

          {Boolean(kbHwInfo?.has_hinge) && (
            <button
              type="button"
              onClick={() => setSubTab('ambient')}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                padding: '6px 16px',
                borderRadius: 'var(--radius-sm)',
                fontSize: '12px',
                fontWeight: 600,
                whiteSpace: 'nowrap',
                flexShrink: 0,
                background: subTab === 'ambient' ? 'var(--status-info-dim)' : 'transparent',
                border: subTab === 'ambient' ? '1px solid var(--status-info)' : '1px solid transparent',
                color: subTab === 'ambient' ? 'var(--text-primary)' : 'var(--text-muted)',
                cursor: 'pointer',
                transition: 'all var(--transition-fast)',
              }}
            >
              <Wind className="w-4 h-4" style={{ color: 'var(--status-info)' }} />
              <span>Фоновая световая полоса</span>
            </button>
          )}

          {Boolean(kbHwInfo?.has_logo) && (
            <button
              type="button"
              onClick={() => setSubTab('logo')}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                padding: '6px 16px',
                borderRadius: 'var(--radius-sm)',
                fontSize: '12px',
                fontWeight: 600,
                whiteSpace: 'nowrap',
                flexShrink: 0,
                background: subTab === 'logo' ? 'var(--status-error-dim)' : 'transparent',
                border: subTab === 'logo' ? '1px solid var(--color-beast)' : '1px solid transparent',
                color: subTab === 'logo' ? 'var(--text-primary)' : 'var(--text-muted)',
                cursor: 'pointer',
                transition: 'all var(--transition-fast)',
              }}
            >
              <Sparkles className="w-4 h-4" style={{ color: 'var(--color-beast)' }} />
              <span>Логотип крышки A</span>
            </button>
          )}
        </div>

        {/* Global Sleep Timer */}
        <div
          title="При бездействии автоматически гасит подсветку клавиатуры, световых полос и логотипа (мгновенно пробуждается от любой клавиши или движения мыши)"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '4px 10px',
            borderRadius: 'var(--radius-sm)',
            background: 'var(--surface-pill)',
            border: '1px solid var(--border-medium)',
            whiteSpace: 'nowrap',
            flexShrink: 0,
          }}
        >
          <Clock className="w-3.5 h-3.5" style={{ color: 'var(--color-brand)' }} />
          <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Гашение в простое:</span>
          <select
            value={lighting.sleep_minutes}
            onChange={(e) => sendTimeout(Number(e.target.value))}
            className="rev-select"
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-primary)',
              fontSize: '11px',
              fontWeight: 600,
              padding: 0,
            }}
          >
            {SLEEP_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Global Lighting Ceremony & Actions Control Bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '12px',
          background: 'var(--surface-sunken)',
          padding: '6px 12px',
          borderRadius: 'var(--radius-md)',
          border: '1px solid var(--border-subtle)',
          flexWrap: 'wrap',
        }}
      >
        {/* Left: Ceremony Hub */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              marginRight: '4px',
              color: 'var(--text-secondary)',
              fontSize: '12px',
              fontWeight: 600,
              whiteSpace: 'nowrap',
            }}
          >
            <Sparkles className="w-3.5 h-3.5" style={{ color: 'var(--color-brand)' }} />
            <span>Приветственный ритуал</span>
          </div>

          <button
            type="button"
            onClick={handleToggleCeremony}
            title="При включении или пробуждении из сна автоматически проигрывается скоординированное приветственное световое шоу всей аппаратуры (адаптивно для всех моделей)"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '4px 10px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              fontWeight: 600,
              whiteSpace: 'nowrap',
              flexShrink: 0,
              cursor: 'pointer',
              background: (lighting.ceremony_enabled ?? true) ? 'var(--border-glow)' : 'var(--surface-badge)',
              border: (lighting.ceremony_enabled ?? true) ? '1px solid var(--color-brand)' : '1px solid var(--border-subtle)',
              color: (lighting.ceremony_enabled ?? true) ? 'var(--text-primary)' : 'var(--text-muted)',
              transition: 'all var(--transition-fast)',
            }}
          >
            <div
              style={{
                width: '6px',
                height: '6px',
                borderRadius: 'var(--radius-full)',
                backgroundColor: (lighting.ceremony_enabled ?? true) ? 'var(--color-brand)' : 'var(--text-dim)',
                boxShadow: (lighting.ceremony_enabled ?? true) ? '0 0 8px var(--color-brand)' : 'none',
              }}
            />
            <span>{(lighting.ceremony_enabled ?? true) ? 'Включено' : 'Отключено'}</span>
          </button>

          <button
            type="button"
            onClick={handleTriggerWelcome}
            disabled={isApplying}
            title="Немедленно запустить демонстрацию полного матричного приветственного светового шоу"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              background: isApplying ? 'rgba(99, 102, 241, 0.4)' : 'var(--color-brand)',
              border: 'none',
              color: 'var(--text-primary)',
              padding: '4px 12px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              fontWeight: 700,
              whiteSpace: 'nowrap',
              flexShrink: 0,
              cursor: isApplying ? 'default' : 'pointer',
              boxShadow: isApplying ? 'none' : '0 0 12px var(--border-glow)',
              transition: 'all var(--transition-fast)',
            }}
          >
            <Sparkles className="w-3.5 h-3.5" />
            <span>{isApplying ? 'Демонстрация...' : 'Предпросмотр полного ритуала подсветки'}</span>
          </button>
        </div>

        {/* Right: Actions */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
          <button
            type="button"
            onClick={handleSaveAsStartup}
            title="Сделать текущую настройку всех эффектов подсветки базовой для включения системы и пробуждения"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              padding: '4px 10px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              fontWeight: 600,
              whiteSpace: 'nowrap',
              flexShrink: 0,
              color: saveSuccess ? 'var(--status-ok)' : 'var(--text-muted)',
              background: saveSuccess ? 'var(--status-ok-dim)' : 'var(--surface-pill)',
              border: saveSuccess ? '1px solid var(--status-ok)' : '1px solid var(--border-medium)',
              cursor: 'pointer',
              transition: 'all var(--transition-fast)',
            }}
          >
            {saveSuccess ? <Check className="w-3.5 h-3.5" style={{ color: 'var(--status-ok)' }} /> : <Save className="w-3.5 h-3.5" />}
            <span>{saveSuccess ? 'Установлено по умолчанию при включении' : 'Сделать по умолчанию'}</span>
          </button>

          <button
            type="button"
            onClick={handleResetDefaults}
            title="Восстановить заводские настройки подсветки текущего устройства"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              padding: '4px 10px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              whiteSpace: 'nowrap',
              flexShrink: 0,
              color: 'var(--text-muted)',
              background: 'var(--surface-pill)',
              border: '1px solid var(--border-medium)',
              cursor: 'pointer',
              transition: 'all var(--transition-fast)',
            }}
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>По умолчанию</span>
          </button>

          <button
            type="button"
            onClick={() => setIsTopologyModalOpen(true)}
            title="Вручную исправить сопоставление платформы с топологией физического оборудования подсветки (например одна зона ошибочно определена как 4 зоны, или включить пропущенную периферию)"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              padding: '4px 10px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              fontWeight: 600,
              whiteSpace: 'nowrap',
              flexShrink: 0,
              color: 'var(--status-info)',
              background: 'var(--status-info-dim)',
              border: '1px solid var(--status-info)',
              cursor: 'pointer',
              transition: 'all var(--transition-fast)',
            }}
          >
            <Cpu className="w-3.5 h-3.5" />
            <span>Исправление оборудования</span>
          </button>
        </div>
      </div>


      {/* Main Lighting View Body */}
      {subTab === 'keyboard' && (
        <KeyboardLightingView
          lighting={lighting}
          kbHwInfo={kbHwInfo}
          sendKeyboard={sendKeyboard}
          sendFourZoneColors={sendFourZoneColors}
        />
      )}

      {subTab === 'lightbar' && (
        <LightbarView
          lighting={lighting}
          kbHwInfo={kbHwInfo}
          sendLightbar={sendLightbar}
        />
      )}

      {subTab === 'ambient' && (
        <AmbientLightingView
          lighting={lighting}
          kbHwInfo={kbHwInfo}
          sendHinge={sendHinge}
        />
      )}

      {subTab === 'logo' && (
        <LogoLightingView
          lighting={lighting}
          kbHwInfo={kbHwInfo}
          sendLogo={sendLogo}
        />
      )}

      {/* Hardware Topology Override Modal */}
      <LightingTopologyOverrideModal
        isOpen={isTopologyModalOpen}
        onClose={() => setIsTopologyModalOpen(false)}
        onApplied={() => {
          refreshHardwareAndLighting();
        }}
      />
    </div>
  );
};
