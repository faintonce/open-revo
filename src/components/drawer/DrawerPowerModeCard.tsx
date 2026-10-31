import React, { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HardwareState, PowerMode } from '../../types/hardware';
import { ModelCapabilities } from '../../plugins/plugin_manifest';
import { Zap, Shield, Flame, Sliders, Snowflake } from 'lucide-react';

interface DrawerPowerModeCardProps {
  status: HardwareState;
  capabilities?: ModelCapabilities;
  onSetPowerMode: (mode: PowerMode) => void;
  checkOemGuard: (action: () => void) => void;
  onExpandDashboard: (tab?: 'overview' | 'gpu' | 'tuning' | 'lighting' | 'water' | 'system' | 'sponsor' | 'custom_tuning') => void;
}

export const DrawerPowerModeCard: React.FC<DrawerPowerModeCardProps> = ({
  status,
  capabilities,
  onSetPowerMode,
  checkOemGuard,
  onExpandDashboard,
}) => {
  const isThinLight = capabilities?.form_factor === 'thin_light' || capabilities?.discrete_gpu === false;
  const hasCustomMode = capabilities?.power_modes ? capabilities.power_modes.includes('custom') : !isThinLight;
  const [gpuOc, setGpuOc] = useState<{ on: boolean; busy: boolean; error: boolean }>({ on: false, busy: false, error: false });
  // 狂暴子模式（静音/高能）：只有能力位满足的机型才显示入口
  const [turboSub, setTurboSub] = useState<{ supported: boolean; silent: boolean }>({ supported: false, silent: false });

  // OC 状态：初始查一次 + 跟随 gpu-clock-offsets-changed（大窗口/宿主复位时同步）
  useEffect(() => {
    let alive = true;
    const fromOffsets = (core: number, mem: number) => {
      if (!alive) return;
      setGpuOc((s) => ({ ...s, on: core !== 0 || mem !== 0 }));
    };
    invoke<any>('get_gpu_oc_state')
      .then((v) => { if (Array.isArray(v)) fromOffsets(Number(v[0]) || 0, Number(v[1]) || 0); })
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

  // 静音/高能 分段按钮：先确保进入狂暴档，再切换子模式
  const applyTurboSeg = async (silent: boolean) => {
    try {
      if (status.power_mode !== 3) {
        checkOemGuard(() => onSetPowerMode(3));
        await new Promise((r) => setTimeout(r, 900));
      }
      const st: any = await invoke('set_turbo_submode', { silent });
      if (st && typeof st.supported === 'boolean') {
        setTurboSub({ supported: st.supported, silent: !!st.silent });
      }
    } catch (err) {
      console.warn('[DrawerPowerModeCard] turbo submode switch failed:', err);
    }
  };

  // OC 开关：文字固定 OC，开=高亮，关=普通；只跟随当前激活的狂暴子模式按钮显示
  const renderOcChip = (restricted: boolean) => {
    if (status.power_mode !== 3 || restricted) return null;
    return (
      <button
        className={`gpu-oc-chip${gpuOc.on ? ' is-applied' : ''}${gpuOc.error ? ' is-error' : ''}`}
        title={gpuOc.on
          ? 'GPU OC 已开启（核心 +105 MHz / 显存 +500 MHz），点击关闭'
          : 'GPU OC 未开启，点击开启（核心 +105 MHz / 显存 +500 MHz）'}
        onClick={async (e) => {
          e.stopPropagation();
          if (gpuOc.busy) return;
          const next = !gpuOc.on;
          setGpuOc((s) => ({ ...s, busy: true, error: false }));
          try {
            await invoke('apply_gpu_oc', { coreMhz: next ? 105 : 0, memMhz: next ? 500 : 0 });
            setGpuOc({ on: next, busy: false, error: false });
          } catch (err) {
            console.warn('[DrawerPowerModeCard] toggle gpu oc failed:', err);
            setGpuOc((s) => ({ ...s, busy: false, error: true }));
          }
        }}
      >
        OC
      </button>
    );
  };

  // Current power mode description
  const modeDesc = useMemo(() => {
    switch (status.power_mode) {
      case 1:
        return '办公模式';
      case 2:
        return '均衡模式';
      case 3:
        return '狂暴模式';
      case 4:
        return hasCustomMode ? '极客自定义调校' : '均衡模式';
      default:
        return '均衡模式';
    }
  }, [status.power_mode, hasCustomMode]);

  return (
    <div className="m-card">
      <div className="m-card-header">
        <div className="m-header-left">
          <Zap size={13} style={{ color: 'var(--theme-pwr)' }} />
          <span>性能模式:</span>
          <span style={{ color: 'var(--theme-pwr)', fontWeight: 700 }}>{modeDesc}</span>
        </div>
        <div className="m-header-right" style={{ color: 'var(--theme-pwr)' }}>
          CPU: {status.cpu_temp > 0 ? `${status.cpu_temp}°C` : '--'} · Fan: {status.cpu_fan_rpm > 0 ? `${status.cpu_fan_rpm} RPM` : '静音'}
        </div>
      </div>

      <div className={turboSub.supported ? 'm-pwr-row' : (hasCustomMode ? 'm-grid-4' : 'm-grid-3')}>
        <button
          className="m-btn-action"
          style={
            status.power_mode === 1
              ? {
                background: 'var(--theme-pwr-dim)',
                borderColor: 'var(--theme-pwr-border)',
                color: 'var(--theme-pwr)',
                boxShadow: '0 0 12px var(--theme-pwr-glow)',
              }
              : {}
          }
          onClick={() => checkOemGuard(() => onSetPowerMode(1))}
        >
          <Shield size={14} />
          <span style={{ fontSize: '10.5px', fontWeight: 600 }}>办公</span>
        </button>

        <button
          className="m-btn-action"
          style={
            status.power_mode === 2
              ? {
                background: 'var(--theme-pwr-dim)',
                borderColor: 'var(--theme-pwr-border)',
                color: 'var(--theme-pwr)',
                boxShadow: '0 0 12px var(--theme-pwr-glow)',
              }
              : {}
          }
          onClick={() => checkOemGuard(() => onSetPowerMode(2))}
        >
          <Zap size={14} />
          <span style={{ fontSize: '10.5px', fontWeight: 600 }}>均衡</span>
        </button>

        {(() => {
          const isRestricted = !isThinLight && status.is_dc_online === false && !status.unlock_battery_power_limit;
          return (
            <>
            <div className="gpu-oc-wrap">
              {turboSub.supported ? (
                <div className="turbo-seg">
                  <button
                    className={`m-btn-action turbo-seg-item${status.power_mode === 3 && turboSub.silent ? ' active' : ''}`}
                    disabled={isRestricted}
                    title={isRestricted ? '电池供电或受限供电下不可用，需接入原装电源适配器' : '静音狂暴：狂暴档内的安静子模式'}
                    onClick={() => { if (!isRestricted) applyTurboSeg(true); }}
                  >
                    {status.power_mode === 3 && turboSub.silent && renderOcChip(isRestricted)}
                    <Snowflake size={14} />
                    <span style={{ fontSize: '9.5px', fontWeight: 600 }}>
                      {isRestricted ? '静音' : '静音狂暴'}
                    </span>
                  </button>
                  <button
                    className={`m-btn-action turbo-seg-item${status.power_mode === 3 && !turboSub.silent ? ' active' : ''}`}
                    disabled={isRestricted}
                    title={isRestricted ? '电池供电或受限供电下不可用，需接入原装电源适配器' : '高能狂暴：狂暴档内的高能/极限子模式'}
                    onClick={() => { if (!isRestricted) applyTurboSeg(false); }}
                  >
                    {status.power_mode === 3 && !turboSub.silent && renderOcChip(isRestricted)}
                    <Flame size={14} />
                    <span style={{ fontSize: '9.5px', fontWeight: 600 }}>
                      {isRestricted ? '高能' : '高能狂暴'}
                    </span>
                  </button>
                </div>
              ) : (
                <button
                  className="m-btn-action"
                  disabled={isRestricted}
                  style={{
                    ...(status.power_mode === 3
                      ? {
                        background: 'var(--theme-pwr-dim)',
                        borderColor: 'var(--theme-pwr-border)',
                        color: 'var(--theme-pwr)',
                        boxShadow: '0 0 12px var(--theme-pwr-glow)',
                      }
                      : {}),
                    ...(isRestricted
                      ? {
                        opacity: 0.45,
                        cursor: 'not-allowed',
                        filter: 'grayscale(0.6)',
                      }
                      : {}),
                  }}
                  title={isRestricted ? '电池供电或受限供电下不可用，需接入原装电源适配器' : '狂暴模式'}
                  onClick={() => {
                    if (isRestricted) return;
                    checkOemGuard(() => onSetPowerMode(3));
                  }}
                >
                  {renderOcChip(isRestricted)}
                  <Flame size={14} />
                  <span style={{ fontSize: '10.5px', fontWeight: 600 }}>
                    {isRestricted ? '狂暴 (需电源)' : '狂暴'}
                  </span>
                </button>
              )}
                </div>

              {hasCustomMode && (
                <button
                  className="m-btn-action"
                  disabled={isRestricted}
                  style={{
                    ...(status.power_mode === 4
                      ? {
                        background: 'var(--theme-pwr-dim)',
                        borderColor: 'var(--theme-pwr-border)',
                        color: 'var(--theme-pwr)',
                        boxShadow: '0 0 12px var(--theme-pwr-glow)',
                      }
                      : {}),
                    ...(isRestricted
                      ? {
                        opacity: 0.45,
                        cursor: 'not-allowed',
                        filter: 'grayscale(0.6)',
                      }
                      : {}),
                  }}
                  title={
                    isRestricted
                      ? '电池供电或受限供电下不可用，需接入原装电源适配器'
                      : status.power_mode === 4
                      ? '已激活极客模式，再次点击打开高级调校大面板'
                      : '切换为极客自定义模式'
                  }
                  onClick={() => {
                    if (isRestricted) return;
                    checkOemGuard(() => {
                      if (status.power_mode === 4) {
                        onExpandDashboard('custom_tuning');
                      } else {
                        onSetPowerMode(4);
                      }
                    });
                  }}
                >
                  <Sliders size={14} />
                  <span style={{ fontSize: '10.5px', fontWeight: 600 }}>
                    {isRestricted ? '极客 (需电源)' : '极客调校'}
                  </span>
                </button>
              )}
            </>
          );
        })()}
      </div>
    </div>
  );
};
