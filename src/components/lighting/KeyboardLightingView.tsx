import React, { useState, useEffect, useRef } from 'react';
import { Sun } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import {
  LightingState,
  KeyboardHardwareInfo,
  BrfxScript,
} from './lightingConstants';
import {
  BRFX_PRESETS,
  parseBrfxMetadata,
  BrfxSelectorBar,
  BrfxTuningPanel,
  BrfxSourceModal,
  BrfxTuningValues,
} from './custom';
import { LightHeaderBanner } from './LightHeaderBanner';
import { LightColorRow } from './LightColorRow';
import { KbEngineSelector } from './keyboard/KbEngineSelector';
import { KeyboardModeGrid } from './keyboard/KeyboardModeGrid';
import { KeyboardControlsBar } from './keyboard/KeyboardControlsBar';
import { FourZoneColorPanel } from './keyboard/FourZoneColorPanel';

interface KeyboardLightingViewProps {
  lighting: LightingState;
  kbHwInfo: KeyboardHardwareInfo | null;
  sendKeyboard: (patch: Partial<LightingState>) => void;
  sendFourZoneColors?: (zones: [string, string, string, string]) => void;
}

export const KeyboardLightingView: React.FC<KeyboardLightingViewProps> = ({
  lighting,
  kbHwInfo,
  sendKeyboard,
  sendFourZoneColors,
}) => {
  const isSingleColor = kbHwInfo?.hardware_type === 'SingleColor';
  const isSingleZone = kbHwInfo?.hardware_type === 'SingleZoneRgb';
  const isFourZone = Boolean(kbHwInfo?.is_four_zone || kbHwInfo?.hardware_type === 'FourZoneRgb');

  const [localBrightness, setLocalBrightness] = useState<number | null>(null);
  const brightnessDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [localSpeed, setLocalSpeed] = useState<number | null>(null);
  const speedDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Custom BRFX State
  const [scripts, setScripts] = useState<BrfxScript[]>(BRFX_PRESETS);
  const [activeScriptId, setActiveScriptId] = useState<string>(() => {
    return lighting.custom_script_id || localStorage.getItem('openrevo_active_brfx_id') || 'kamehameha';
  });
  const [activeTuningValues, setActiveTuningValues] = useState<BrfxTuningValues>({
    speed: 1.2,
    scale: 1.0,
    direction: 0,
    reactive: false,
    color: '#00F0FF',
  });
  const baselineValuesRef = useRef<Record<string, BrfxTuningValues>>({});
  const [isSourceModalOpen, setIsSourceModalOpen] = useState(false);

  // Initialize baselines from initial presets
  useEffect(() => {
    BRFX_PRESETS.forEach((p: BrfxScript) => {
      if (!baselineValuesRef.current[p.id]) {
        baselineValuesRef.current[p.id] = {
          speed: p.speed,
          scale: p.scale,
          direction: p.direction,
          reactive: p.reactive,
          color: p.color,
        };
      }
    });
  }, []);

  // Fetch scripts from disk via Tauri
  const loadScriptsFromDisk = async () => {
    try {
      const files = await invoke<Array<{ file_name: string; content: string; full_path: string }>>(
        'get_custom_brfx_list'
      );
      if (files && files.length > 0) {
        const loaded: BrfxScript[] = files.map((f) => {
          const fallbackId = f.file_name.replace(/\.[^/.]+$/, '');
          const parsed = parseBrfxMetadata(f.content, fallbackId);
          parsed.filePath = f.full_path;
          if (fallbackId.toLowerCase().includes('kamehameha')) parsed.builtinEffectId = 200;
          else if (fallbackId.toLowerCase().includes('lotus')) parsed.builtinEffectId = 201;
          else if (fallbackId.toLowerCase().includes('singularity')) parsed.builtinEffectId = 202;
          else if (fallbackId.toLowerCase().includes('marquee')) parsed.builtinEffectId = 203;
          else parsed.builtinEffectId = 200;

          if (!parsed.customText && fallbackId.toLowerCase().includes('marquee')) {
            parsed.customText = localStorage.getItem(`openrevo_marquee_text_${parsed.id}`) || 'I LOVE OPEN-REVO';
          }

          if (!baselineValuesRef.current[parsed.id]) {
            baselineValuesRef.current[parsed.id] = {
              speed: parsed.speed,
              scale: parsed.scale,
              direction: parsed.direction,
              reactive: parsed.reactive,
              color: parsed.color,
              customText: parsed.customText,
            };
          }
          return parsed;
        });
        setScripts(loaded);

        const savedId = localStorage.getItem('openrevo_active_brfx_id');
        let targetId = lighting.custom_script_id;
        if (!targetId && lighting.streamer_effect && lighting.streamer_effect >= 200) {
          const byEff = loaded.find((s) => s.builtinEffectId === lighting.streamer_effect);
          if (byEff) targetId = byEff.id;
        }
        if (!targetId) targetId = savedId || 'kamehameha';

        const curr = loaded.find((s) => s.id === targetId)
          || loaded.find((s) => lighting.streamer_effect && s.builtinEffectId === lighting.streamer_effect)
          || loaded[0];
        if (curr) {
          setActiveScriptId(curr.id);
          localStorage.setItem('openrevo_active_brfx_id', curr.id);
          setActiveTuningValues({
            speed: curr.speed,
            scale: curr.scale,
            direction: curr.direction,
            reactive: curr.reactive,
            color: curr.color,
            customText: curr.customText,
          });

          if (curr.customText) {
            invoke('set_custom_fx_text', { text: curr.customText }).catch((e) => console.warn(e));
          }

          if (curr.code) {
            invoke('apply_custom_brfx_script', {
              name: curr.name || curr.id,
              content: curr.code,
            }).catch((err) => console.warn('Failed to apply script to Rhai:', err));
          }
        }
      }
    } catch (err) {
      console.warn('Failed to load BRFX scripts from disk:', err);
    }
  };

  useEffect(() => {
    loadScriptsFromDisk();
  }, []);

  // Sync state if lighting prop updates from backend
  useEffect(() => {
    if (lighting.custom_script_id && lighting.custom_script_id !== activeScriptId) {
      const s = scripts.find((item) => item.id === lighting.custom_script_id);
      if (s) {
        setActiveScriptId(lighting.custom_script_id);
        localStorage.setItem('openrevo_active_brfx_id', lighting.custom_script_id);
        const textVal = s.customText || (s.id.includes('marquee') ? (localStorage.getItem(`openrevo_marquee_text_${s.id}`) || 'I LOVE OPEN-REVO') : undefined);
        setActiveTuningValues({
          speed: s.speed,
          scale: s.scale,
          direction: s.direction,
          reactive: s.reactive,
          color: s.color,
          customText: textVal,
        });
        if (textVal) {
          invoke('set_custom_fx_text', { text: textVal }).catch((e) => console.warn(e));
        }
        if (s.code) {
          invoke('apply_custom_brfx_script', {
            name: s.name || s.id,
            content: s.code,
          }).catch((err) => console.warn('Failed to apply script to Rhai:', err));
        }
      }
    }
  }, [lighting.custom_script_id, scripts]);

  const handleSelectScript = async (id: string) => {
    const s = scripts.find((item) => item.id === id);
    if (!s) return;
    setActiveScriptId(id);
    localStorage.setItem('openrevo_active_brfx_id', id);
    const textVal = s.customText || (s.id.includes('marquee') ? (localStorage.getItem(`openrevo_marquee_text_${s.id}`) || 'I LOVE OPEN-REVO') : undefined);
    const tuned: BrfxTuningValues = {
      speed: s.speed,
      scale: s.scale,
      direction: s.direction,
      reactive: s.reactive,
      color: s.color,
      customText: textVal,
    };
    setActiveTuningValues(tuned);
    if (textVal) {
      await invoke('set_custom_fx_text', { text: textVal }).catch((e) => console.warn(e));
    }

    if (s.code) {
      await invoke('apply_custom_brfx_script', {
        name: s.name || s.id,
        content: s.code,
      }).catch((err) => console.warn('Failed to apply script to Rhai:', err));
    }
    sendKeyboard({
      kb_engine: 'custom',
      streamer_effect: s.builtinEffectId || 200,
      kb_speed: Math.round(s.speed * 2),
      kb_direction: s.direction,
      kb_color: s.color,
      custom_scale: s.scale,
      custom_reactive: s.reactive,
      custom_script_id: s.id,
    });
  };

  const handleTuningChange = (patch: Partial<BrfxTuningValues>) => {
    const next = { ...activeTuningValues, ...patch };
    setActiveTuningValues(next);
    if (patch.customText !== undefined) {
      localStorage.setItem(`openrevo_marquee_text_${activeScriptId}`, patch.customText);
      invoke('set_custom_fx_text', { text: patch.customText }).catch((e) => console.warn(e));
    }
    sendKeyboard({
      kb_speed: patch.speed !== undefined ? Math.round(patch.speed * 2) : undefined,
      kb_direction: patch.direction,
      kb_color: patch.color,
      custom_scale: patch.scale,
      custom_reactive: patch.reactive,
    });
  };

  const handleResetTuning = () => {
    const base = baselineValuesRef.current[activeScriptId];
    if (base) {
      setActiveTuningValues({ ...base });
      sendKeyboard({
        kb_speed: Math.round(base.speed * 2),
        kb_direction: base.direction,
        kb_color: base.color,
        custom_scale: base.scale,
        custom_reactive: base.reactive,
      });
    }
  };

  const handleImportScript = async (content: string, filename: string) => {
    const id = filename.replace(/\.[^/.]+$/, '');
    const parsed = parseBrfxMetadata(content, id);
    try {
      await invoke('save_custom_brfx', { filename, content });
    } catch (e) {
      console.warn('Failed to save imported BRFX to disk:', e);
    }
    baselineValuesRef.current[parsed.id] = {
      speed: parsed.speed,
      scale: parsed.scale,
      direction: parsed.direction,
      reactive: parsed.reactive,
      color: parsed.color,
    };
    setScripts((prev) => [parsed, ...prev.filter((item) => item.id !== parsed.id)]);
    setActiveScriptId(parsed.id);
    setActiveTuningValues({
      speed: parsed.speed,
      scale: parsed.scale,
      direction: parsed.direction,
      reactive: parsed.reactive,
      color: parsed.color,
    });
    sendKeyboard({
      kb_engine: 'custom',
      streamer_effect: 200,
      kb_speed: Math.round(parsed.speed * 2),
      kb_direction: parsed.direction,
      kb_color: parsed.color,
      custom_scale: parsed.scale,
      custom_reactive: parsed.reactive,
      custom_script_id: parsed.id,
    });
  };

  const handleExportScript = () => {
    const s = scripts.find((item) => item.id === activeScriptId);
    if (!s) return;
    const blob = new Blob([s.code], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${s.id}.brfx`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleOpenInNotepad = async () => {
    const s = scripts.find((item) => item.id === activeScriptId);
    if (!s) return;
    try {
      await invoke('open_brfx_in_notepad', { filename: `${s.id}.brfx` });
    } catch (err) {
      console.warn('Failed to open BRFX in Notepad:', err);
    }
  };

  React.useEffect(() => {
    return () => {
      if (brightnessDebounceRef.current) {
        clearTimeout(brightnessDebounceRef.current);
      }
      if (speedDebounceRef.current) {
        clearTimeout(speedDebounceRef.current);
      }
    };
  }, []);

  const handleBrightnessChange = (val: number) => {
    const bounded = isSingleColor && val > 2 ? 2 : val;
    setLocalBrightness(bounded);
    if (brightnessDebounceRef.current) {
      clearTimeout(brightnessDebounceRef.current);
    }
    brightnessDebounceRef.current = setTimeout(() => {
      sendKeyboard({
        kb_brightness: bounded,
        kb_enabled: bounded > 0,
      });
      setLocalBrightness(null);
    }, 50);
  };

  const handleSpeedChange = (val: number) => {
    setLocalSpeed(val);
    if (speedDebounceRef.current) {
      clearTimeout(speedDebounceRef.current);
    }
    speedDebounceRef.current = setTimeout(() => {
      sendKeyboard({ kb_speed: val });
      setLocalSpeed(null);
    }, 50);
  };

  const displayBrightness = localBrightness !== null
    ? localBrightness
    : (lighting.kb_enabled ? (isSingleColor && lighting.kb_brightness > 2 ? 2 : lighting.kb_brightness) : 0);

  const displaySpeed = localSpeed !== null ? localSpeed : lighting.kb_speed;

  const currentZones: [string, string, string, string] = [
    lighting.kb_zones?.[0] || '#00FFFF',
    lighting.kb_zones?.[1] || '#0000FF',
    lighting.kb_zones?.[2] || '#FF00FF',
    lighting.kb_zones?.[3] || '#FF0000',
  ];

  const handleZoneColorChange = (index: number, newColor: string) => {
    const updated: [string, string, string, string] = [
      currentZones[0],
      currentZones[1],
      currentZones[2],
      currentZones[3],
    ];
    updated[index] = newColor;
    if (sendFourZoneColors) {
      sendFourZoneColors(updated);
    } else {
      sendKeyboard({ kb_zones: updated });
    }
  };

  const applyZonePreset = (zones: [string, string, string, string]) => {
    if (sendFourZoneColors) {
      sendFourZoneColors(zones);
    } else {
      sendKeyboard({ kb_zones: zones });
    }
  };

  const pageTitle = isSingleColor
    ? '键盘单色背光工作台'
    : isSingleZone
    ? '单区 RGB 键盘工作台'
    : isFourZone
    ? '4区 RGB 键盘工作台'
    : '键盘背光工作台';

  const badgeText = isSingleColor
    ? '单色白光背光 [EC硬件直通]'
    : isSingleZone
    ? '单区同色 RGB [EC硬件调光]'
    : isFourZone
    ? 'ITE 8291 r2 4区 RGB 控制器 (048D:CE00)'
    : 'ITE 8291 单键 RGB 控制器 (048D 逐键 HID)';

  const pageDesc = isSingleColor
    ? '检测到当前机型为单色 LED 背光，已自动精简无效控制项，支持开关、3档亮度与闲置熄灯'
    : isSingleZone
    ? '全键盘同色调光，支持纯色常亮、彩虹呼吸与4档亮度微调'
    : isFourZone
    ? '4 分区独立调色与 6 种原厂硬件动效，支持方向、亮度与色彩微调'
    : '12 种硬件内建与动态特效，支持方向、亮度与色彩毫秒级直接生效';

  const isKbOn = lighting.kb_enabled && lighting.kb_brightness > 0;

  return (
    <div className="glass-card" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
      {/* Header Banner & Switch */}
      <LightHeaderBanner
        title={pageTitle}
        badgeText={badgeText}
        badgeVariant={isSingleColor ? 'default' : isSingleZone ? 'success' : isFourZone ? 'info' : 'accent'}
        description={pageDesc}
        enabled={isKbOn}
        onToggle={() => {
          if (brightnessDebounceRef.current) {
            clearTimeout(brightnessDebounceRef.current);
            brightnessDebounceRef.current = null;
          }
          setLocalBrightness(null);
          if (isKbOn) {
            sendKeyboard({ kb_enabled: false });
          } else {
            const targetBrightness = lighting.kb_brightness === 0 ? (isSingleColor ? 2 : 4) : (isSingleColor && lighting.kb_brightness > 2 ? 2 : lighting.kb_brightness);
            sendKeyboard({
              kb_enabled: true,
              kb_brightness: targetBrightness,
            });
          }
        }}
      />

      {/* Dual-Engine Architecture Selector & FPS Control Bar */}
      {!isSingleColor && !isSingleZone && !isFourZone && (
        <KbEngineSelector
          engine={lighting.kb_engine || 'hardware'}
          fps={lighting.kb_fps || 30}
          onSelectHardware={() => {
            invoke('clear_custom_brfx_script').catch(() => {});
            sendKeyboard({ kb_engine: 'hardware' });
          }}
          onSelectStreamer={() => {
            invoke('clear_custom_brfx_script').catch(() => {});
            const savedStreamerEff = Number(localStorage.getItem('openrevo_active_streamer_eff')) || 101;
            const targetEff = (lighting.streamer_effect && lighting.streamer_effect >= 100 && lighting.streamer_effect < 200)
              ? lighting.streamer_effect
              : (savedStreamerEff >= 100 && savedStreamerEff < 200 ? savedStreamerEff : 101);
            sendKeyboard({ kb_engine: 'streamer', streamer_effect: targetEff });
          }}
          onSelectCustom={async () => {
            const s = scripts.find((item) => item.id === activeScriptId) || scripts[0];
            if (s?.code) {
              await invoke('apply_custom_brfx_script', {
                name: s.name || s.id,
                content: s.code,
              }).catch(() => {});
            }
            sendKeyboard({
              kb_engine: 'custom',
              streamer_effect: s?.builtinEffectId || 200,
              kb_speed: s ? Math.round(s.speed * 2) : 2,
              kb_direction: s?.direction ?? 0,
              kb_color: s?.color || '#00F0FF',
              custom_scale: s?.scale ?? 1.0,
              custom_reactive: s?.reactive ?? false,
              custom_script_id: s?.id || 'kamehameha',
            });
          }}
          onChangeFps={(val) => sendKeyboard({ kb_fps: val })}
        />
      )}

      {/* Dynamic Content: Custom BRFX Studio vs Built-in Hardware/Streamer Modes */}
      {!isSingleColor && !isSingleZone && !isFourZone && lighting.kb_engine === 'custom' ? (
        <>
          <BrfxSelectorBar
            scripts={scripts}
            activeScriptId={activeScriptId}
            onSelectScript={handleSelectScript}
            onRefresh={loadScriptsFromDisk}
            onViewSource={() => setIsSourceModalOpen(true)}
            onImport={handleImportScript}
            onExport={handleExportScript}
          />

          <BrfxTuningPanel
            values={activeTuningValues}
            baselineValues={baselineValuesRef.current[activeScriptId] || activeTuningValues}
            onChange={handleTuningChange}
            onReset={handleResetTuning}
          />

          {/* Global Brightness Slider for Custom Engine */}
          <div className="lighting-sub-panel">
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-secondary)' }}>
                <Sun className="w-3.5 h-3.5" style={{ color: 'var(--status-warn)' }} />
                全域键盘亮度
              </span>
              <span style={{ color: 'var(--color-brand)', fontWeight: 700 }}>
                {displayBrightness === 0 ? '关闭 (0)' : `${displayBrightness} 档`}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max={4}
              step="1"
              value={displayBrightness}
              onChange={(e) => handleBrightnessChange(Number(e.target.value))}
              style={{ width: '100%', accentColor: 'var(--color-brand)', cursor: 'pointer' }}
            />
          </div>
        </>
      ) : (
        <>
          {/* Mode Selector Grid */}
          {!isSingleColor && (
            <KeyboardModeGrid
              isSingleZone={isSingleZone}
              isFourZone={isFourZone}
              engine={lighting.kb_engine || 'hardware'}
              currentEffect={lighting.kb_effect}
              currentStreamerEffect={lighting.streamer_effect}
              onSelectEffect={(id) => sendKeyboard({ kb_effect: id })}
              onSelectStreamerEffect={(id) => {
                localStorage.setItem('openrevo_active_streamer_eff', String(id));
                sendKeyboard({ streamer_effect: id });
              }}
            />
          )}

          {/* Controls Section */}
          <KeyboardControlsBar
            isSingleColor={isSingleColor}
            isSingleZone={isSingleZone}
            isFourZone={isFourZone}
            direction={lighting.kb_direction ?? 0}
            onDirectionChange={(dir) => sendKeyboard({ kb_direction: dir })}
            displayBrightness={displayBrightness}
            onBrightnessChange={handleBrightnessChange}
            displaySpeed={displaySpeed}
            onSpeedChange={handleSpeedChange}
          />

          {/* Color Palette */}
          {isFourZone ? (
            <FourZoneColorPanel
              currentZones={currentZones}
              mainColor={lighting.kb_color}
              onZoneColorChange={handleZoneColorChange}
              onApplyZonePreset={applyZonePreset}
              onChangeMainColor={(hex) => sendKeyboard({ kb_color: hex })}
            />
          ) : !isSingleColor && (
            <LightColorRow
              label="定制主色调："
              color={lighting.kb_color}
              onChangeColor={(hex: string) => sendKeyboard({ kb_color: hex })}
              iconColor="var(--color-brand)"
            />
          )}
        </>
      )}

      {/* Source Code Modal */}
      {isSourceModalOpen && (
        <BrfxSourceModal
          script={scripts.find((s) => s.id === activeScriptId) || scripts[0]}
          onClose={() => setIsSourceModalOpen(false)}
          onOpenInNotepad={handleOpenInNotepad}
        />
      )}
    </div>
  );
};
