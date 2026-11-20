import React, { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Keyboard,
  ChevronDown,
  Check,
  Palette,
} from 'lucide-react';

interface LightingPresetState {
  kb_enabled: boolean;
  kb_effect: number;
  kb_brightness: number;
  kb_speed: number;
  kb_direction: number;
  kb_color: string;
  kb_engine?: 'hardware' | 'streamer';
  kb_fps?: number;
  streamer_effect?: number;
}

interface KeyboardHardwareInfo {
  hardware_type: string; // 'PerKeyRgb' | 'SingleZoneRgb' | 'SingleColor' | 'None'
  has_ite8291: boolean;
  has_ite7001: boolean;
  has_logo?: boolean;
  description: string;
  brightness_level: number;
  is_on: boolean;
}

const KBD_PRESET_COLORS = [
  { name: '冰晶蓝', hex: '#38bdf8' },
  { name: '机械青', hex: '#00ffff' },
  { name: '电竞紫', hex: '#a855f7' },
  { name: '极光绿', hex: '#10b981' },
  { name: '烈焰红', hex: '#f43f5e' },
  { name: '暖阳金', hex: '#fbbf24' },
  { name: '幻影粉', hex: '#ec4899' },
  { name: '纯净白', hex: '#ffffff' },
];

const PERKEY_MODES = [
  { id: 3, name: '炫彩波浪' },
  { id: 1, name: '静态恒亮' },
  { id: 2, name: '单色呼吸' },
  { id: 21, name: '游戏高亮' },
  { id: 5, name: '炫彩彩虹' },
  { id: 0, name: '关闭背光' },
];

const STREAMER_DRAWER_MODES = [
  { id: 101, name: '极客波浪' },
  { id: 100, name: '正弦呼吸' },
  { id: 102, name: '同心涟漪' },
  { id: 103, name: '丝滑流沙' },
  { id: 106, name: '全彩彩虹' },
  { id: 113, name: '静态恒亮' },
  { id: 0, name: '关闭背光' },
];

const SINGLE_ZONE_MODES = [
  { id: 1, name: '单色常亮' },
  { id: 2, name: '彩虹循环' },
  { id: 0, name: '关闭背光' },
];

const SINGLE_COLOR_MODES = [
  { id: 1, name: '开启照明' },
  { id: 0, name: '关闭背光' },
];




import { HardwareState } from '../../types/hardware';
import { ModelCapabilities } from '../../plugins/plugin_manifest';

export interface DrawerLightingCardProps {
  status?: HardwareState;
  capabilities?: ModelCapabilities;
  onExpandDashboard: (tab?: 'overview' | 'gpu' | 'tuning' | 'lighting' | 'water' | 'system' | 'sponsor') => void;
}

export const DrawerLightingCard: React.FC<DrawerLightingCardProps> = ({
  status: _status,
  capabilities,
  onExpandDashboard,
}) => {
  const [kbHwInfo, setKbHwInfo] = useState<KeyboardHardwareInfo | null>(null);
  const [kbdEngine, setKbdEngine] = useState<'hardware' | 'streamer'>('hardware');
  const [streamerEffect, setStreamerEffect] = useState<number>(1);
  const [kbdEffect, setKbdEffect] = useState<number>(3); // Default wave (3)
  const [kbdColor, setKbdColor] = useState<string>('#38bdf8');
  const [kbdBrightnessLevel, setKbdBrightnessLevel] = useState<number>(4); // 1..4 (or 1..3 for SingleColor)

  const [showKbdModeMenu, setShowKbdModeMenu] = useState(false);
  const [showKbdColorMenu, setShowKbdColorMenu] = useState(false);
  const [showKbdBrightMenu, setShowKbdBrightMenu] = useState(false);
  const [sleepTimeout, setSleepTimeout] = useState<number>(0);

  useEffect(() => {
    invoke<number>('get_keyboard_sleep_timeout')
      .then((to) => {
        if (typeof to === 'number') {
          setSleepTimeout(to);
        }
      })
      .catch(() => {});

    let unlistenTimeout: (() => void) | undefined;
    listen<number>('keyboard-sleep-timeout-changed', (e) => {
      if (typeof e.payload === 'number') {
        setSleepTimeout(e.payload);
      }
    }).then((fn) => { unlistenTimeout = fn; });

    return () => {
      if (unlistenTimeout) unlistenTimeout();
    };
  }, []);

  const handleSetTimeout = (sec: number) => {
    setSleepTimeout(sec);
    invoke('set_keyboard_sleep_timeout', { seconds: sec }).catch((err) =>
      console.warn('Set keyboard sleep timeout error:', err)
    );
  };



  useEffect(() => {
    const fetchLighting = () => {
      invoke<KeyboardHardwareInfo>('get_keyboard_hardware_info')
        .then((info) => {
          if (info) {
            setKbHwInfo(info);
            if (info.brightness_level) {
              setKbdBrightnessLevel(info.brightness_level);
            }
          }
        })
        .catch((err) => console.warn('Failed to get keyboard hardware info:', err));

      invoke<LightingPresetState>('get_lighting_status')
        .then((st) => {
          if (st.kb_engine === 'hardware' || st.kb_engine === 'streamer') {
            setKbdEngine(st.kb_engine);
          }
          if (typeof st.streamer_effect === 'number') {
            setStreamerEffect(st.streamer_effect);
          }
          if (!st.kb_enabled || st.kb_brightness === 0 || st.kb_effect === 0) {
            setKbdEffect(0);
          } else {
            setKbdEffect(st.kb_effect);
          }
          if (st.kb_brightness > 0) {
            setKbdBrightnessLevel(st.kb_brightness);
          }
          if (st.kb_color) {
            setKbdColor(st.kb_color);
          }
        })
        .catch((err) => console.warn('Failed to get initial lighting status:', err));
    };

    fetchLighting();
    window.addEventListener('focus', fetchLighting);

    let unlistenEngine: (() => void) | undefined;
    listen<string>('kb-engine-changed', (e) => {
      if (e.payload === 'hardware' || e.payload === 'streamer') {
        setKbdEngine(e.payload);
      }
    }).then((fn) => { unlistenEngine = fn; });

    let unlistenKbBrightness: (() => void) | undefined;
    listen<number>('keyboard-brightness-changed', (e) => {
      if (typeof e.payload === 'number') {
        const lvl = e.payload;
        setKbdBrightnessLevel(lvl);
        if (lvl === 0) {
          setKbdEffect(0);
        } else {
          setKbdEffect(1);
        }
      }
    }).then((fn) => { unlistenKbBrightness = fn; });

    let unlistenHwInfo: (() => void) | undefined;
    listen('lighting://hardware_info_changed', () => {
      fetchLighting();
    }).then((fn) => { unlistenHwInfo = fn; });

    let unlistenTopology: (() => void) | undefined;
    listen('lighting://topology_rehashed', () => {
      fetchLighting();
    }).then((fn) => { unlistenTopology = fn; });

    return () => {
      window.removeEventListener('focus', fetchLighting);
      if (unlistenEngine) unlistenEngine();
      if (unlistenKbBrightness) unlistenKbBrightness();
      if (unlistenHwInfo) unlistenHwInfo();
      if (unlistenTopology) unlistenTopology();
    };
  }, []);

  // 硬件形态判断
  const isDeclaredStepped = capabilities?.rgb_keyboard?.type === 'monochrome_stepped';
  const isSingleZone = kbHwInfo?.hardware_type === 'SingleZoneRgb' || (!isDeclaredStepped && capabilities?.rgb_keyboard?.type === 'single_zone_rgb');
  const isSingleColor = !isSingleZone && (isDeclaredStepped || kbHwInfo?.hardware_type === 'SingleColor');

  // 动态根据 capabilities.rgb_keyboard.steps 计算单色键盘背光档位定义
  const singleColorLevels = useMemo(() => {
    const rawSteps = capabilities?.rgb_keyboard?.steps;
    const validSteps = Array.isArray(rawSteps) && rawSteps.length > 0
      ? [...rawSteps].filter((s) => typeof s === 'number' && s > 0).sort((a, b) => b - a)
      : isSingleColor ? [2, 1] : [3, 2, 1];

    return validSteps.map((lvl) => {
      const max = validSteps[0] || 1;
      let name = '';
      if (validSteps.length === 2) {
        name = lvl === 2 ? '100% 强光' : '50% 弱光';
      } else if (validSteps.length === 3) {
        name = lvl === 3 ? '100% 满亮' : lvl === 2 ? '60% 柔和' : '30% 微光';
      } else {
        const pct = Math.round((lvl / max) * 100);
        name = lvl === max ? `${pct}% 满亮` : `${pct}% 档位`;
      }
      return { level: lvl, name };
    });
  }, [capabilities?.rgb_keyboard?.steps, isSingleColor]);

  const maxSingleColorLevel = useMemo(() => {
    return singleColorLevels[0]?.level || 2;
  }, [singleColorLevels]);

  const lastSingleColorLevelRef = React.useRef<number>(2);

  useEffect(() => {
    if (kbdBrightnessLevel > 0) {
      lastSingleColorLevelRef.current = kbdBrightnessLevel;
    }
  }, [kbdBrightnessLevel]);

  const isKbdTurnedOn = kbdEffect !== 0 && kbdBrightnessLevel > 0;

  const currentSingleColorLevel = useMemo(() => {
    const raw = kbdBrightnessLevel > 0 ? kbdBrightnessLevel : lastSingleColorLevelRef.current;
    if (raw > maxSingleColorLevel) return maxSingleColorLevel;
    if (raw < 1) return 1;
    return raw;
  }, [kbdBrightnessLevel, maxSingleColorLevel]);

  useEffect(() => {
    if (isSingleColor && kbdBrightnessLevel > maxSingleColorLevel) {
      setKbdBrightnessLevel(maxSingleColorLevel);
    }
  }, [isSingleColor, maxSingleColorLevel, kbdBrightnessLevel]);

  // 根据当前机型硬件能力动态计算可选模式列表
  const currentModes = useMemo(() => {
    if (isSingleColor) return SINGLE_COLOR_MODES;
    if (isSingleZone) return SINGLE_ZONE_MODES;
    if (kbdEngine === 'streamer') return STREAMER_DRAWER_MODES;
    return PERKEY_MODES;
  }, [isSingleColor, isSingleZone, kbdEngine]);

  const activeModeName = useMemo(() => {
    if (kbdEffect === 0) return '关闭背光';
    if (isSingleColor) return '开启照明';
    if (!isSingleZone && kbdEngine === 'streamer') {
      const normalized = typeof streamerEffect === 'number'
        ? (streamerEffect >= 100 ? streamerEffect : streamerEffect + 100)
        : 101;
      const found = STREAMER_DRAWER_MODES.find((m) => m.id === normalized);
      return found ? found.name : '极客波浪';
    }
    const found = currentModes.find((m) => m.id === kbdEffect);
    return found ? found.name : '炫彩波浪';
  }, [currentModes, kbdEffect, isSingleColor, isSingleZone, kbdEngine, streamerEffect]);

  const activeColorObj = useMemo(() => {
    const found = KBD_PRESET_COLORS.find((c) => c.hex.toLowerCase() === kbdColor.toLowerCase());
    return found || { name: '自定义', hex: kbdColor };
  }, [kbdColor]);


  // 键盘模式选择处理
  const selectKbdMode = (effectId: number) => {
    setShowKbdModeMenu(false);

    const enabled = effectId !== 0;
    const maxB = isSingleColor ? maxSingleColorLevel : 4;
    const remembered = lastSingleColorLevelRef.current > 0 ? Math.min(lastSingleColorLevelRef.current, maxB) : maxB;
    const bLevel = enabled ? (kbdBrightnessLevel > 0 ? (kbdBrightnessLevel > maxB ? maxB : kbdBrightnessLevel) : remembered) : 0;

    if (isSingleColor) {
      setKbdEffect(effectId);
      setKbdBrightnessLevel(bLevel);
      invoke('apply_keyboard_lighting', {
        enabled,
        effect: effectId,
        brightness: bLevel,
        speed: 2,
        direction: 1,
        color: '#ffffff',
        engine: 'hardware',
      }).catch((err) => console.warn('Apply keyboard lighting mode error:', err));
      return;
    }

    if (!isSingleZone && kbdEngine === 'streamer') {
      const streamerEff = effectId >= 100 ? effectId : (effectId === 0 ? 0 : effectId + 100);
      if (!enabled) {
        setKbdEffect(0);
      } else {
        setStreamerEffect(streamerEff);
      }
      invoke('apply_keyboard_lighting', {
        enabled,
        effect: streamerEff,
        brightness: bLevel,
        speed: 2,
        direction: 1,
        color: kbdColor,
        engine: 'streamer',
        streamer_effect: streamerEff,
        streamerEffect: streamerEff,
      }).catch((err) => console.warn('Apply keyboard lighting mode error:', err));
    } else {
      setKbdEffect(effectId);
      invoke('apply_keyboard_lighting', {
        enabled,
        effect: effectId,
        brightness: bLevel,
        speed: 2,
        direction: 1,
        color: kbdColor,
        engine: 'hardware',
      }).catch((err) => console.warn('Apply keyboard lighting mode error:', err));
    }
  };

  // 键盘色彩选择处理 (RGB 机型)
  const selectKbdColor = (hex: string) => {
    setKbdColor(hex);
    setShowKbdColorMenu(false);

    const isStreamer = !isSingleZone && kbdEngine === 'streamer';
    const rawStreamerEff = typeof streamerEffect === 'number' ? streamerEffect : 101;
    const normStreamer = rawStreamerEff >= 100 ? rawStreamerEff : rawStreamerEff + 100;
    const effToPass = isStreamer ? normStreamer : (kbdEffect === 0 ? 1 : kbdEffect);
    if (!isStreamer && kbdEffect === 0) {
      setKbdEffect(1);
    }

    const maxB = isSingleColor ? maxSingleColorLevel : 4;
    invoke('apply_keyboard_lighting', {
      enabled: true,
      effect: effToPass,
      brightness: kbdBrightnessLevel > 0 ? (kbdBrightnessLevel > maxB ? maxB : kbdBrightnessLevel) : maxB,
      speed: 2,
      direction: 1,
      color: hex,
      engine: kbdEngine,
      streamer_effect: normStreamer,
      streamerEffect: normStreamer,
    }).catch((err) => console.warn('Apply keyboard lighting color error:', err));
  };

  // 键盘单色亮度档位切换 (单色机型)
  const selectKbdBrightness = (level: number) => {
    const validLevel = level > maxSingleColorLevel ? maxSingleColorLevel : (level < 1 ? 1 : level);
    setKbdBrightnessLevel(validLevel);
    setShowKbdBrightMenu(false);

    setKbdEffect(1);
    invoke('apply_keyboard_lighting', {
      enabled: true,
      effect: 1,
      brightness: validLevel,
      speed: 2,
      direction: 1,
      color: '#ffffff',
      engine: 'hardware',
    }).catch((err) => console.warn('Apply single color brightness error:', err));
  };

  const isSleepTimeoutMode = Boolean(
    capabilities?.rgb_keyboard?.sleep_timeout_supported ||
    (capabilities?.rgb_keyboard?.supported === false && capabilities?.form_factor === 'thin_light')
  );

  if (isSleepTimeoutMode) {
    return (
      <div className="m-card">
        <div className="m-card-header">
          <div className="m-header-left">
            <Keyboard size={13} style={{ color: 'var(--theme-kbd)' }} />
            <span>键盘背光休眠延时</span>
          </div>

          <div className="m-header-right" style={{ color: 'var(--theme-kbd)', fontSize: '10px' }}>
            Fn+F8 硬件调光
          </div>
        </div>

        <div className="m-grid-4">
          {[
            { label: '常亮', sec: 0 },
            { label: '10秒', sec: 10 },
            { label: '30秒', sec: 30 },
            { label: '60秒', sec: 60 },
          ].map((item) => (
            <button
              key={item.sec}
              className="m-btn-action"
              style={
                sleepTimeout === item.sec
                  ? {
                      background: 'var(--theme-kbd-dim)',
                      borderColor: 'var(--theme-kbd-border)',
                      color: 'var(--theme-kbd)',
                      boxShadow: '0 0 10px var(--theme-kbd-glow)',
                    }
                  : {}
              }
              onClick={() => handleSetTimeout(item.sec)}
              title={item.sec === 0 ? '背光常亮不灭' : `无人操作 ${item.label} 后自动熄灭节能`}
            >
              <span style={{ fontSize: '11px', fontWeight: 600 }}>{item.label}</span>
            </button>
          ))}
        </div>
      </div>
    );
  }

  return (
      <div className="m-card">
        <div className="m-card-header">
          <div className="m-header-left">
            <Keyboard size={13} style={{ color: 'var(--theme-kbd)' }} />
            <span>笔记本键盘背光</span>
            {kbHwInfo && (
              <span
                style={{
                  fontSize: '9px',
                  color: isSingleColor ? '#94a3b8' : isSingleZone ? '#10b981' : 'var(--theme-kbd)',
                  background: 'rgba(255, 255, 255, 0.05)',
                  padding: '1px 5px',
                  borderRadius: '3px',
                  fontWeight: 600,
                }}
              >
                {isSingleColor ? '单色白光' : isSingleZone ? '单区RGB' : '全彩RGB'}
              </span>
            )}
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: isSingleColor ? '1fr 1fr' : '1.25fr 1.05fr 0.85fr', gap: '6px' }}>
          {/* 1. 模式自定义高对比度暗黑下拉 */}
          <div className="m-select-wrap" onClick={(e) => e.stopPropagation()}>
            <button
              className="m-select-btn"
              onClick={() => {
                setShowKbdModeMenu(!showKbdModeMenu);
                setShowKbdColorMenu(false);
                setShowKbdBrightMenu(false);
              }}
              title="切换键盘背光效果模式"
            >
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {activeModeName}
              </span>
              <ChevronDown size={11} style={{ opacity: 0.7, flexShrink: 0, marginLeft: '3px' }} />
            </button>

            {showKbdModeMenu && (
              <div className="m-select-menu">
                {currentModes.map((m) => {
                  const isItemActive = (!isSingleColor && !isSingleZone && kbdEngine === 'streamer')
                    ? (m.id === 0 ? kbdEffect === 0 : (streamerEffect === m.id && kbdEffect !== 0))
                    : (kbdEffect === m.id);
                  return (
                    <div
                      key={m.id}
                      className={`m-select-item ${isItemActive ? 'active' : ''}`}
                      onClick={() => selectKbdMode(m.id)}
                    >
                      <span>{m.name}</span>
                      {isItemActive && <Check size={11} style={{ color: 'var(--theme-kbd)' }} />}
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* 2. 色彩下拉 (RGB 机型) 或 亮度档位下拉 (单色机型) */}
          {isSingleColor ? (
            <div className="m-select-wrap" onClick={(e) => e.stopPropagation()}>
              <button
                className="m-select-btn"
                style={!isKbdTurnedOn ? { opacity: 0.5, borderColor: 'transparent', color: 'var(--text-muted)' } : {}}
                onClick={() => {
                  setShowKbdBrightMenu(!showKbdBrightMenu);
                  setShowKbdModeMenu(false);
                  setShowKbdColorMenu(false);
                }}
                title={isKbdTurnedOn ? "选择单色白光背光亮度档位" : "背光已关闭，点击选择档位开启照明"}
              >
                <span>{isKbdTurnedOn ? `档位 ${currentSingleColorLevel}档` : '已灭灯'}</span>
                <ChevronDown size={11} style={{ opacity: 0.7, flexShrink: 0, marginLeft: '3px' }} />
              </button>

              {showKbdBrightMenu && (
                <div className="m-select-menu">
                  {singleColorLevels.map((lvl) => {
                    const isItemActive = isKbdTurnedOn && currentSingleColorLevel === lvl.level;
                    return (
                      <div
                        key={lvl.level}
                        className={`m-select-item ${isItemActive ? 'active' : ''}`}
                        onClick={() => selectKbdBrightness(lvl.level)}
                      >
                        <span>{lvl.name}</span>
                        {isItemActive && <Check size={11} style={{ color: 'var(--theme-kbd)' }} />}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          ) : (
            <div className="m-select-wrap" onClick={(e) => e.stopPropagation()}>
              <button
                className="m-select-btn"
                onClick={() => {
                  setShowKbdColorMenu(!showKbdColorMenu);
                  setShowKbdModeMenu(false);
                  setShowKbdBrightMenu(false);
                }}
                title="选择背光预设主色调"
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
                  <span
                    style={{
                      width: '9px',
                      height: '9px',
                      borderRadius: '2px',
                      background: activeColorObj.hex,
                      boxShadow: `0 0 5px ${activeColorObj.hex}`,
                      flexShrink: 0,
                    }}
                  />
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {activeColorObj.name}
                  </span>
                </div>
                <ChevronDown size={11} style={{ opacity: 0.7, flexShrink: 0, marginLeft: '3px' }} />
              </button>

              {showKbdColorMenu && (
                <div className="m-select-menu">
                  {KBD_PRESET_COLORS.map((c) => (
                    <div
                      key={c.hex}
                      className={`m-select-item ${kbdColor.toLowerCase() === c.hex.toLowerCase() ? 'active' : ''}`}
                      onClick={() => selectKbdColor(c.hex)}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: '7px' }}>
                        <span
                          style={{
                            width: '8px',
                            height: '8px',
                            borderRadius: '2px',
                            background: c.hex,
                            boxShadow: `0 0 4px ${c.hex}`,
                          }}
                        />
                        <span>{c.name}</span>
                      </div>
                      {kbdColor.toLowerCase() === c.hex.toLowerCase() && (
                        <Check size={11} style={{ color: 'var(--theme-kbd)' }} />
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* 3. 更多：直达 RGB 灯效工坊大面板 (单色/轻薄本无需展示) */}
          {!isSingleColor && (
            <button
              className="m-btn-bat"
              style={{
                height: '31px',
                padding: '0 6px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '4px',
              }}
              title="展开控制中心大面板并直达 RGB 灯效工坊"
              onClick={() => onExpandDashboard('lighting')}
            >
              <Palette size={12} />
              <span>更多</span>
            </button>
          )}
        </div>
      </div>

  );
};
