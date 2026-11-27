import React, { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HardwareState, PowerMode } from '../../types/hardware';
import { ModelCapabilities } from '../../plugins/plugin_manifest';
import { useTurboSubmode } from '../../hooks/useTurboSubmode';
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
  const [ocDefaults, setOcDefaults] = useState<{ core: number; mem: number }>({ core: 105, mem: 0 });

  useEffect(() => {
    invoke<any>('get_app_config')
      .then((cfg) => {
        if (!cfg) return;
        const core = Number(cfg.gpu_oc_core_mhz);
        const mem = Number(cfg.gpu_oc_mem_mhz);
        setOcDefaults({
          core: Number.isFinite(core) && core >= 0 ? core : 105,
          mem: Number.isFinite(mem) && mem >= 0 ? mem : 0,
        });
      })
      .catch(() => { });
  }, []);
  // 狂暴子模式（静音/高能）：只有能力位满足的机型才显示入口
  const turboSub = useTurboSubmode();

  // OC 状态：初始查配置与驱动 + 跟随 host 的 turbo-oc-changed / gpu-clock-offsets-changed
  useEffect(() => {
    let alive = true;
    invoke<any>('get_app_config')
      .then((cfg) => {
        if (!alive) return;
        if (cfg && typeof cfg.turbo_oc_enabled === 'boolean') {
          setGpuOc((s) => ({ ...s, on: cfg.turbo_oc_enabled }));
        }
      })
      .catch(() => { });

    let unlistenTurboOc: any = null;
    let unlistenOffsets: any = null;

    listen('turbo-oc-changed', (e: any) => {
      if (!alive) return;
      const p = (e && e.payload) || {};
      if (typeof p.enabled === 'boolean') {
        setGpuOc((s) => ({ ...s, on: p.enabled, busy: false, error: false }));
      }
    }).then((f) => { unlistenTurboOc = f; }).catch(() => { });

    listen('gpu-clock-offsets-changed', (e: any) => {
      if (!alive) return;
      const p = (e && e.payload) || {};
      const core = Number(p.core_offset_mhz) || 0;
      const mem = Number(p.mem_offset_mhz) || 0;
      if (status.power_mode === 3) {
        setGpuOc((s) => ({ ...s, on: core !== 0 || mem !== 0 }));
      }
    }).then((f) => { unlistenOffsets = f; }).catch(() => { });

    return () => {
      alive = false;
      try { if (unlistenTurboOc) unlistenTurboOc(); } catch { /* ignore */ }
      try { if (unlistenOffsets) unlistenOffsets(); } catch { /* ignore */ }
    };
  }, [status.power_mode]);

  // 静音/高能 分段按钮：先确保进入狂暴档，再应用子模式
  const applyTurboSeg = (silent: boolean) => {
    checkOemGuard(() => {
      if (status.power_mode !== 3) {
        onSetPowerMode(3);
      }
      void turboSub.apply(silent);
    });
  };

  // OC 开关：文字固定 OC，开=高亮，关=普通；只跟随当前激活的狂暴子模式按钮显示
  const renderOcChip = (restricted: boolean) => {
    if (status.power_mode !== 3 || restricted) return null;
    return (
      <button
        className={`gpu-oc-chip${gpuOc.on ? ' is-applied' : ''}${gpuOc.error ? ' is-error' : ''}`}
        title={gpuOc.on
          ? `GPU-разгон включён (ядро +${ocDefaults.core} МГц / память +${ocDefaults.mem} МГц), нажмите для выключения`
          : `GPU-разгон выключен, нажмите для включения (ядро +${ocDefaults.core} МГц / память +${ocDefaults.mem} МГц)`}
        onClick={async (e) => {
          e.stopPropagation();
          if (gpuOc.busy) return;
          const next = !gpuOc.on;
          setGpuOc((s) => ({ ...s, busy: true, error: false }));
          try {
            await invoke('set_turbo_oc_enabled', { enabled: next });
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
        return 'Офисный режим';
      case 2:
        return 'Режим Баланса';
      case 3:
        return 'Режим Турбо';
      case 4:
        return hasCustomMode ? 'Экспертная пользовательская настройка' : 'Режим Баланса';
      default:
        return 'Режим Баланса';
    }
  }, [status.power_mode, hasCustomMode]);

  return (
    <div className="m-card">
      <div className="m-card-header">
        <div className="m-header-left">
          <Zap size={13} style={{ color: 'var(--theme-pwr)' }} />
          <span>Режим производительности:</span>
          <span style={{ color: 'var(--theme-pwr)', fontWeight: 700 }}>{modeDesc}</span>
        </div>
        <div className="m-header-right" style={{ color: 'var(--theme-pwr)' }}>
          CPU: {status.cpu_temp > 0 ? `${status.cpu_temp}°C` : '--'} · Fan: {status.cpu_fan_rpm > 0 ? `${status.cpu_fan_rpm} RPM` : 'Тихий'}
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
          <span style={{ fontSize: '10.5px', fontWeight: 600 }}>Офис</span>
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
          <span style={{ fontSize: '10.5px', fontWeight: 600 }}>Баланс</span>
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
                    disabled={isRestricted || turboSub.busy}
                    title={isRestricted ? 'Недоступно от батареи или ограниченного питания — подключите штатный адаптер' : 'Тихий Турбо: тихий подрежим внутри Турбо'}
                    onClick={() => { if (!isRestricted) applyTurboSeg(true); }}
                  >
                    {status.power_mode === 3 && turboSub.silent && renderOcChip(isRestricted)}
                    <Snowflake size={14} />
                    <span style={{ fontSize: '9.5px', fontWeight: 600 }}>
                      {isRestricted ? 'Тихий' : 'Тихий Турбо'}
                    </span>
                  </button>
                  <button
                    className={`m-btn-action turbo-seg-item${status.power_mode === 3 && !turboSub.silent ? ' active' : ''}`}
                    disabled={isRestricted || turboSub.busy}
                    title={isRestricted ? 'Недоступно от батареи или ограниченного питания — подключите штатный адаптер' : 'Мощный Турбо: высокопроизводительный/предельный подрежим внутри Турбо'}
                    onClick={() => { if (!isRestricted) applyTurboSeg(false); }}
                  >
                    {status.power_mode === 3 && !turboSub.silent && renderOcChip(isRestricted)}
                    <Flame size={14} />
                    <span style={{ fontSize: '9.5px', fontWeight: 600 }}>
                      {isRestricted ? 'Мощный' : 'Мощный Турбо'}
                    </span>
                  </button>
                  {turboSub.error && (
                    <div role="alert" style={{ fontSize: '10px', color: 'var(--theme-pwr)' }}>{turboSub.error}</div>
                  )}
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
                  title={isRestricted ? 'Недоступно от батареи или ограниченного питания — подключите штатный адаптер' : 'Режим Турбо'}
                  onClick={() => {
                    if (isRestricted) return;
                    checkOemGuard(() => onSetPowerMode(3));
                  }}
                >
                  {renderOcChip(isRestricted)}
                  <Flame size={14} />
                  <span style={{ fontSize: '10.5px', fontWeight: 600 }}>
                    {isRestricted ? 'Турбо (нужен адаптер)' : 'Турбо'}
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
                      ? 'Недоступно от батареи или ограниченного питания — подключите штатный адаптер'
                      : status.power_mode === 4
                      ? 'Активирован Экспертный режим; повторный клик открывает большую панель тонкой настройки'
                      : 'Переключиться в Экспертный пользовательский режим'
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
                    {isRestricted ? 'Экспертный (нужен адаптер)' : 'Экспертная настройка'}
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
