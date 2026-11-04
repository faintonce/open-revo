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

    return () => {
      mounted = false;
      unlistenPromise.then((unsub) => unsub());
      unlistenBrightnessPromise.then((unsub) => unsub());
      unlistenHwPromise.then((unsub) => unsub());
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
  // 六色轮换默认颜色（恢复默认时用）
  const [cycleResetToken, setCycleResetToken] = useState(0);

  const handleResetDefaults = () => {
    if (subTab === 'keyboard') {
      // 保持当前效果不变，只恢复亮度/速度/方向/主色与六色轮换默认色
      sendKeyboard({
        kb_enabled: true,
        kb_brightness: 3,
        kb_speed: 2,
        kb_direction: 0,
        kb_color: '#6366F1',
      });
      if (kbHwInfo?.is_four_zone) {
        sendFourZoneColors(['#00FFFF', '#0000FF', '#FF00FF', '#FF0000']);
      }
      // 六色轮换：恢复默认 6 色 + 清掉本地自定义
      try {
        localStorage.removeItem('openrevo_kb_cycle_colors');
        invoke('set_keyboard_cycle_colors', {
          colors: ['#D30607', '#D55B0A', '#D4A906', '#05D308', '#0709D6', '#D308D6'],
        }).catch(() => { /* ignore */ });
      } catch { /* ignore */ }
      setCycleResetToken((t) => t + 1);
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
            <span>键盘背光</span>
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
              <span>海岸灯条</span>
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
              <span>氛围灯条</span>
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
              <span>A面 LOGO</span>
            </button>
          )}
        </div>

        {/* Global Sleep Timer */}
        <div
          title="系统闲置无操作时自动关闭键盘、灯带与Logo灯效（敲击任意按键或移动鼠标瞬间唤醒）"
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
          <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>闲置熄灯:</span>
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
            <span>迎宾仪式</span>
          </div>

          <button
            type="button"
            onClick={handleToggleCeremony}
            title="开机启动或休眠唤醒时，自动上演全域硬件协同迎宾灯效秀（全机型自适应）"
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
            <span>{(lighting.ceremony_enabled ?? true) ? '已启用' : '已停用'}</span>
          </button>

          <button
            type="button"
            onClick={handleTriggerWelcome}
            disabled={isApplying}
            title="立即触发一次全矩阵迎宾仪式灯效演示"
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
            <span>{isApplying ? '演示中...' : '预览全域仪式'}</span>
          </button>
        </div>

        {/* Right: Actions */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
          <button
            type="button"
            onClick={handleSaveAsStartup}
            title="将当前调好的各项灯效设定为系统开机及唤醒恢复的默认底色"
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
            <span>{saveSuccess ? '已设为开机默认' : '设为默认'}</span>
          </button>

          <button
            type="button"
            onClick={handleResetDefaults}
            title="恢复当前设备出厂默认灯效设置"
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
            <span>恢复默认</span>
          </button>

          <button
            type="button"
            onClick={() => setIsTopologyModalOpen(true)}
            title="手动纠正模具与物理灯效硬件设备拓扑映射（如单区误判为4区、或开启漏检外设）"
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
            <span>硬件纠偏</span>
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
          cycleResetToken={cycleResetToken}
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
