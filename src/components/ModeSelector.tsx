import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { POWER_MODES, PowerMode } from '../types/hardware';
import { ModelCapabilities } from '../plugins/plugin_manifest';
import { Shield, Zap, Flame, Snowflake, Sliders, Settings2 } from 'lucide-react';

interface ModeSelectorProps {
  currentMode: PowerMode | number;
  onSelectMode: (mode: PowerMode) => void;
  disabled?: boolean;
  isDcOnline?: boolean;
  unlockBatteryLimit?: boolean;
  onOpenCustomTuning?: () => void;
  capabilities?: ModelCapabilities;
  isThinLight?: boolean;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const ModeSelector: React.FC<ModeSelectorProps> = ({
  currentMode,
  onSelectMode,
  disabled = false,
  isDcOnline = true,
  unlockBatteryLimit = false,
  onOpenCustomTuning,
  capabilities,
  isThinLight = false,
}) => {
  const isLight = isThinLight || capabilities?.form_factor === 'thin_light';
  const isUnadapted = capabilities?.is_adapted === false;
  const hasCustomMode = !isUnadapted && (capabilities?.power_modes ? capabilities.power_modes.includes('custom') : !isLight);
  const availableModes = POWER_MODES.filter((m) => hasCustomMode || m.id !== 4);

  // 狂暴子模式（静音/高能）：只有能力位满足的机型才拆成两段
  const [turboSub, setTurboSub] = useState<{ supported: boolean; silent: boolean }>({ supported: false, silent: false });
  const [gcuOc, setGcuOc] = useState<{ on: boolean; busy: boolean; error: boolean }>({ on: false, busy: false, error: false });

  // OC 状态：初始查一次 + 跟随 host 的 gpu-clock-offsets-changed（宿主/另一窗口复位时同步）
  useEffect(() => {
    let alive = true;
    const fromOffsets = (core: number, mem: number) => {
      if (!alive) return;
      setGcuOc((s) => ({ ...s, on: core !== 0 || mem !== 0 }));
    };
    invoke<any>('get_gpu_oc_state')
      .then((v) => {
        if (Array.isArray(v)) fromOffsets(Number(v[0]) || 0, Number(v[1]) || 0);
      })
      .catch(() => { });
    let unlisten: any = null;
    listen('gpu-clock-offsets-changed', (e: any) => {
      const p = (e && e.payload) || {};
      fromOffsets(Number(p.core_offset_mhz) || 0, Number(p.mem_offset_mhz) || 0);
    })
      .then((f) => { unlisten = f; })
      .catch(() => { });
    return () => {
      alive = false;
      try { if (unlisten) unlisten(); } catch { /* ignore */ }
    };
  }, []);

  useEffect(() => {
    let alive = true;
    const apply = (v: any) => {
      if (!alive || !v) return;
      if (typeof v.supported === 'boolean') {
        setTurboSub({ supported: v.supported, silent: !!v.silent });
      } else if (typeof v.silent === 'boolean') {
        setTurboSub((s) => ({ ...s, silent: v.silent }));
      }
    };
    invoke<any>('get_turbo_submode_state').then(apply).catch(() => { });
    let unlisten: any = null;
    let unlistenMode: any = null;
    listen('turbo-submode-changed', (e: any) => apply(e.payload))
      .then((f) => { unlisten = f; })
      .catch(() => { });
    // 物理按键切档后，宿主会按记忆重新下发子模式；稍后刷新一次状态
    listen('physical-mode-switched', (e: any) => {
      if (Number(e?.payload) !== 3) return;
      setTimeout(() => {
        if (!alive) return;
        invoke<any>('get_turbo_submode_state').then(apply).catch(() => { });
      }, 1000);
    })
      .then((f) => { unlistenMode = f; })
      .catch(() => { });
    return () => {
      alive = false;
      try { if (unlisten) unlisten(); } catch { /* ignore */ }
      try { if (unlistenMode) unlistenMode(); } catch { /* ignore */ }
    };
  }, []);

  const applyTurboSeg = async (silent: boolean, isRestricted: boolean) => {
    if (isRestricted) return;
    try {
      if (currentMode !== 3) {
        onSelectMode(3);
        await sleep(900);
      }
      const st: any = await invoke('set_turbo_submode', { silent });
      if (st && typeof st.supported === 'boolean') {
        setTurboSub({ supported: st.supported, silent: !!st.silent });
      }
    } catch (err) {
      console.warn('[ModeSelector] turbo submode switch failed:', err);
    }
  };

  const toggleGcuOc = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (gcuOc.busy) return;
    const next = !gcuOc.on;
    setGcuOc((s) => ({ ...s, busy: true, error: false }));
    try {
      await invoke('apply_gpu_oc', { coreMhz: next ? 105 : 0, memMhz: next ? 500 : 0 });
      setGcuOc({ on: next, busy: false, error: false });
    } catch (err) {
      console.warn('[ModeSelector] toggle gpu oc failed:', err);
      setGcuOc((s) => ({ ...s, busy: false, error: true }));
    }
  };

  const gcuOcChip = () => (
    <div
      className={`mode-oc-chip${gcuOc.on ? ' is-applied' : ''}${gcuOc.error ? ' is-error' : ''}`}
      title={gcuOc.on
        ? 'GPU OC 已开启（核心 +105 MHz / 显存 +500 MHz），点击关闭'
        : 'GPU OC 未开启，点击开启（核心 +105 MHz / 显存 +500 MHz）'}
      onClick={toggleGcuOc}
    >
      {gcuOc.busy ? 'OC...' : gcuOc.error ? 'GPU OC !' : gcuOc.on ? 'GPU OC ON' : 'GPU OC OFF'}
    </div>
  );

  const getIcon = (id: PowerMode) => {
    switch (id) {
      case 1:
        return <Shield className="w-4 h-4 text-emerald-400" />;
      case 2:
        return <Zap className="w-4 h-4 text-blue-400" />;
      case 3:
        return <Flame className="w-4 h-4 text-rose-400" />;
      case 4:
        return <Sliders className="w-4 h-4 text-purple-400" />;
      default:
        return <Zap className="w-4 h-4" />;
    }
  };

  const getActiveClass = (id: PowerMode) => {
    if (currentMode !== id) return '';
    switch (id) {
      case 1:
        return 'active-office';
      case 2:
        return 'active-balance';
      case 3:
        return 'active-beast';
      case 4:
        return 'active-custom';
      default:
        return 'active-balance';
    }
  };

  return (
    <div style={{ display: 'grid', gridTemplateColumns: `repeat(${availableModes.length}, 1fr)`, gap: '8px' }}>
      {availableModes.map((mode) => {
        const isActive = currentMode === mode.id;
        const isCustom = mode.id === 4;
        const isRestricted = !isLight && (mode.id === 3 || mode.id === 4) && isDcOnline === false && !unlockBatteryLimit;

        // 狂暴档：支持子模式 -> 拆成「静音狂暴 / 高能狂暴」两段
        if (mode.id === 3 && turboSub.supported) {
          const silentActive = isActive && turboSub.silent;
          const extremeActive = isActive && !turboSub.silent;
          return (
            <div key={mode.id} className="mode-seg-wrap">
              <div className="mode-seg">
                <button
                  disabled={disabled || isRestricted}
                  onClick={() => applyTurboSeg(true, isRestricted)}
                  className={`mode-btn mode-seg-btn mode-beast ${silentActive ? 'active-beast' : ''}`}
                  title={isRestricted ? '电池供电或受限供电下不可用，需接入原装电源适配器' : '静音狂暴：狂暴档内的安静子模式'}
                  style={isRestricted ? { opacity: 0.45, cursor: 'not-allowed', filter: 'grayscale(0.6)' } : undefined}
                >
                  {silentActive && !isRestricted && gcuOcChip()}
                  <div style={{ marginBottom: '2px' }}><Snowflake className="w-4 h-4 text-cyan-300" /></div>
                  <div className="mode-title">静音狂暴</div>
                  <div className="mode-sub">{isRestricted ? '需圆口DC' : silentActive ? '● 当前' : '静音'}</div>
                </button>
                <button
                  disabled={disabled || isRestricted}
                  onClick={() => applyTurboSeg(false, isRestricted)}
                  className={`mode-btn mode-seg-btn mode-beast ${extremeActive ? 'active-beast' : ''}`}
                  title={isRestricted ? '电池供电或受限供电下不可用，需接入原装电源适配器' : '高能狂暴：狂暴档内的高能/极限子模式'}
                  style={isRestricted ? { opacity: 0.45, cursor: 'not-allowed', filter: 'grayscale(0.6)' } : undefined}
                >
                  {extremeActive && !isRestricted && gcuOcChip()}
                  <div style={{ marginBottom: '2px' }}><Flame className="w-4 h-4 text-rose-400" /></div>
                  <div className="mode-title">高能狂暴</div>
                  <div className="mode-sub">{isRestricted ? '需圆口DC' : extremeActive ? '● 当前' : '高能'}</div>
                </button>
              </div>
            </div>
          );
        }

        return (
          <button
            key={mode.id}
            disabled={disabled || isRestricted}
            onClick={() => {
              if (isRestricted) return;
              if (isActive && isCustom && onOpenCustomTuning) {
                onOpenCustomTuning();
              } else {
                onSelectMode(mode.id);
              }
            }}
            className={`mode-btn ${mode.id === 3 ? 'mode-beast' : ''} ${getActiveClass(mode.id)}`}
            title={
              isRestricted
                ? '电池供电或受限供电下不可用，需接入原装电源适配器'
                : isCustom
                ? (isLight ? '点击直接打开自定义风扇曲线调校' : (isActive ? '已激活自定义模式，点击可直接打开硬件参数调校' : `${mode.nameEn} - ${mode.desc}`))
                : `${mode.nameEn} - ${mode.desc}`
            }
            style={{
              position: 'relative',
              ...(isRestricted
                ? {
                    opacity: 0.45,
                    cursor: 'not-allowed',
                    filter: 'grayscale(0.6)',
                  }
                : {}),
            }}
          >
            {/* 狂暴档 GCU OC 快捷按钮（左上角） */}
            {mode.id === 3 && isActive && !isRestricted && gcuOcChip()}

            {/* Corner Badge for Custom Mode Tuning */}
            {isCustom && (isActive || isLight) && onOpenCustomTuning && !isRestricted && (
              <div
                onClick={(e) => {
                  e.stopPropagation();
                  onOpenCustomTuning();
                }}
                style={{
                  position: 'absolute',
                  top: '4px',
                  right: '5px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '2px',
                  padding: '1px 5px',
                  borderRadius: '4px',
                  background: 'rgba(168, 85, 247, 0.35)',
                  border: '1px solid rgba(192, 132, 252, 0.6)',
                  color: '#e9d5ff',
                  fontSize: '9px',
                  fontWeight: 700,
                  cursor: 'pointer',
                  boxShadow: '0 0 6px rgba(168, 85, 247, 0.4)',
                }}
                title="打开自定义硬件参数调校"
              >
                <Settings2 className="w-2.5 h-2.5" />
                <span>调校</span>
              </div>
            )}

            <div style={{ marginBottom: '2px' }}>{getIcon(mode.id)}</div>
            <div className="mode-title">{mode.name.replace('模式', '')}</div>
            <div className="mode-sub">
              {isRestricted
                ? '需圆口DC'
                : isActive
                ? isCustom
                  ? '● 极客调校'
                  : '● 当前'
                : mode.id === 1
                ? '静音'
                : mode.id === 2
                ? '日常'
                : mode.id === 3
                ? '狂暴'
                : '极客'}
            </div>
          </button>
        );
      })}
    </div>
  );
};
