import React, { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Fan,
  Monitor,
  ZapOff,
  ChevronDown,
  Laptop,
  Tv,
} from 'lucide-react';
import { HardwareState } from '../../types/hardware';

import { ModelCapabilities } from '../../plugins/plugin_manifest';

interface QuickActionCardsProps {
  status: HardwareState;
  capabilities?: ModelCapabilities;
  onToggleFanBoost: (targetRpm?: number) => void;
  onToggleBatteryLimit?: () => void;
  onSwitchRefreshRate: (hz: number, invokeBackend?: boolean) => void;
  checkOemGuard: (action: () => void) => void;
}

export const QuickActionCards: React.FC<QuickActionCardsProps> = ({
  status,
  capabilities,
  onToggleFanBoost,
  onSwitchRefreshRate,
  checkOemGuard,
}) => {
  const [autoMinRefresh, setAutoMinRefresh] = useState(status.auto_min_refresh_on_battery ?? false);
  const [bldRpm, setBldRpm] = useState<number>(() => {
    const saved = localStorage.getItem('openrevo_bld_target_rpm');
    if (saved) {
      const parsed = Number(saved);
      if (!isNaN(parsed) && parsed >= 2200 && parsed <= 5800) {
        return parsed;
      }
    }
    return 5800;
  });
  const bldDebounceRef = React.useRef<any>(null);

  const isBldFanLock = useMemo(() => {
    return capabilities?.fan_boost_mode === 'direct_rpm_clamp';
  }, [capabilities?.fan_boost_mode]);

  const handleBldSwitchToggle = () => {
    checkOemGuard(() => {
      onToggleFanBoost(bldRpm);
    });
  };

  const handleBldRpmChange = (newVal: number) => {
    setBldRpm(newVal);
    localStorage.setItem('openrevo_bld_target_rpm', String(newVal));
    if (status.fan_boost) {
      if (bldDebounceRef.current) clearTimeout(bldDebounceRef.current);
      bldDebounceRef.current = setTimeout(async () => {
        try {
          await invoke('set_bld_fan_speed_cap', { rpm: newVal });
        } catch (err) {
          console.warn('BLD debounce set fan speed failed:', err);
        }
      }, 150);
    }
  };

  useEffect(() => {
    return () => {
      if (bldDebounceRef.current) clearTimeout(bldDebounceRef.current);
    };
  }, []);

  useEffect(() => {
    if (typeof status.auto_min_refresh_on_battery === 'boolean') {
      setAutoMinRefresh(status.auto_min_refresh_on_battery);
    }
  }, [status.auto_min_refresh_on_battery]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<number>('refresh-rate-changed', (event) => {
      onSwitchRefreshRate(event.payload, false);
    }).then((fn) => { unlisten = fn; });
    return () => {
      if (unlisten) unlisten();
    };
  }, [onSwitchRefreshRate]);

  const handleToggleAutoMinRefresh = async () => {
    const nextVal = !autoMinRefresh;
    setAutoMinRefresh(nextVal);
    try {
      await invoke('set_auto_min_refresh_on_battery', { enable: nextVal });
    } catch (e) {
      console.error('Failed to toggle auto min refresh:', e);
      setAutoMinRefresh(!nextVal);
    }
  };

  const monitors = useMemo(() => status.monitors || [], [status.monitors]);
  const [selectedMonIdx, setSelectedMonIdx] = useState(0);
  const [showMonMenu, setShowMonMenu] = useState(false);

  // Close dropdown on outside click
  useEffect(() => {
    const handleOutsideClick = () => setShowMonMenu(false);
    window.addEventListener('click', handleOutsideClick);
    return () => window.removeEventListener('click', handleOutsideClick);
  }, []);

  const activeMon = monitors[selectedMonIdx] || monitors[0] || null;
  const currentHz = activeMon ? activeMon.current_hz : status.refresh_rate;
  const supportedRates = useMemo(() => {
    if (activeMon?.supported_rates && activeMon.supported_rates.length > 0) {
      return [...activeMon.supported_rates].sort((a, b) => a - b);
    }
    return status.available_refresh_rates && status.available_refresh_rates.length > 0
      ? status.available_refresh_rates
      : [60, 144, 240];
  }, [activeMon, status.available_refresh_rates]);

  const handleSetMonitorRate = async (deviceName: string, hz: number) => {
    try {
      await invoke('set_display_monitor_refresh_rate', { deviceName, hz });
      const target = monitors.find((m) => m.device_name === deviceName);
      if (target) {
        target.current_hz = hz;
        if (target.is_primary || monitors.length <= 1) {
          onSwitchRefreshRate(hz, false);
        }
      }
    } catch (err) {
      console.warn('Set monitor rate failed:', err);
    }
  };

  return (
    <div style={{ display: 'flex', gap: '12px', alignItems: 'stretch', position: 'relative', zIndex: showMonMenu ? 200 : 1 }}>
      {/* 1. 一键强冷 (同方物理开关 / 宝龙达转速温度锁定滑块) */}
      {status.support_flags?.has_fan_boost !== false && (
        isBldFanLock ? (
          <div
            className="glass-card bld-fan-card"
            title="Блокировка вентилятора для платформ Baolongda: свободная установка целевых оборотов в диапазоне 0~5800 RPM; при включении обороты фиксируются, при выключении возвращается автоматическое управление температурой BIOS"
          >
            <div className="bld-fan-header">
              <div className="bld-fan-title-group">
                <Fan className={`w-3.5 h-3.5 flex-shrink-0 ${status.fan_boost ? 'text-cyan-400 animate-spin' : 'text-cyan-400'}`} />
                <span className="bld-fan-title">Фиксация оборотов форс-охлаждения</span>
                <span className={`bld-fan-rpm-badge ${status.fan_boost ? '' : 'inactive'}`}>
                  {bldRpm} RPM
                </span>
              </div>
              <button
                onClick={handleBldSwitchToggle}
                className={`bld-fan-switch ${status.fan_boost ? 'active' : ''}`}
                title={status.fan_boost ? 'Нажмите, чтобы снять блокировку оборотов и вернуть заводское автоматическое управление температурой' : 'Нажмите, чтобы включить блокировку вентилятора'}
              >
                {status.fan_boost ? 'Заблокировано' : 'Авто'}
              </button>
            </div>
            <div className="bld-fan-slider-row">
              <span className="bld-fan-limit-hint">2200</span>
              <input
                type="range"
                min="2200"
                max="5800"
                step="100"
                value={bldRpm}
                onChange={(e) => handleBldRpmChange(Number(e.target.value))}
                className="bld-fan-range"
                title={`Целевые обороты: ${bldRpm} RPM`}
              />
              <span className="bld-fan-limit-hint">5800</span>
            </div>
          </div>
        ) : (
          <div
            className="glass-card"
            style={{
              minWidth: '180px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: '12px',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Fan className="w-5 h-5 text-cyan-400 flex-shrink-0" />
              <div>
                <div style={{ fontSize: '12px', fontWeight: 600 }}>Форс-охлаждение</div>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)' }}>Максимальные обороты для предельного охлаждения</div>
              </div>
            </div>
            <button
              onClick={() => checkOemGuard(onToggleFanBoost)}
              className={`toggle-btn ${status.fan_boost ? 'active-cyan' : ''}`}
            >
              {status.fan_boost ? 'Включено' : 'Выключить'}
            </button>
          </div>
        )
      )}

      {/* 2. 拔电自动低刷 */}
      <div
        className="glass-card"
        title="При включении автоматически снижает частоту обновления экрана до минимальной поддерживаемой панелью при отключении адаптера, максимально экономя батарею; если на батарее частота была изменена вручную, сохранится ручное значение"
        style={{
          minWidth: '180px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '12px',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <ZapOff className={`w-5 h-5 ${autoMinRefresh ? 'text-indigo-400' : 'text-slate-400'} flex-shrink-0`} />
          <div>
            <div style={{ fontSize: '12px', fontWeight: 600 }}>Автопонижение частоты обновления от батареи</div>
            <div style={{ fontSize: '10px', color: 'var(--text-dim)' }}>Максимум автономности от батареи</div>
          </div>
        </div>
        <button
          onClick={handleToggleAutoMinRefresh}
          className={`toggle-btn ${autoMinRefresh ? 'active-indigo' : ''}`}
        >
          {autoMinRefresh ? 'Включено' : 'Выключить'}
        </button>
      </div>

      {/* 4. 屏幕刷新率卡片 (固定高度单行，多屏纯黑下拉，杜绝撑大容器) */}
      <div
        className="glass-card"
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '12px',
          position: 'relative',
          zIndex: showMonMenu ? 250 : 1,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', minWidth: '130px', flexShrink: 0 }}>
          <Monitor className="w-5 h-5 text-indigo-400 flex-shrink-0" />
          <div>
            <div style={{ fontSize: '12px', fontWeight: 600 }}>Частота обновления экрана</div>
            {monitors.length <= 1 ? (
              <div style={{ fontSize: '10px', color: 'var(--text-dim)' }}>
                {activeMon ? activeMon.friendly_name : 'Экран 2.5K'}
              </div>
            ) : (
              /* 多屏微型下拉选择器 */
              <div style={{ position: 'relative' }} onClick={(e) => e.stopPropagation()}>
                <button
                  onClick={() => setShowMonMenu(!showMonMenu)}
                  title="Нажмите, чтобы выбрать управляемый монитор"
                  className="monitor-select-btn"
                  style={{ marginTop: '2px' }}
                >
                  <span style={{ display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
                    {activeMon?.is_internal ? <Laptop size={11} /> : <Tv size={11} />}
                    {activeMon?.friendly_name?.split(' ')[0] || 'Экран'}
                  </span>
                  <ChevronDown size={10} />
                </button>

                {showMonMenu && (
                  <div className="monitor-select-dropdown">
                    {monitors.map((m, idx) => (
                      <div
                        key={m.device_name}
                        onClick={() => {
                          setSelectedMonIdx(idx);
                          setShowMonMenu(false);
                        }}
                        className={`monitor-select-item ${idx === selectedMonIdx ? 'active' : ''}`}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                          <span style={{ display: 'flex', alignItems: 'center' }}>
                            {m.is_internal ? <Laptop size={12} /> : <Tv size={12} />}
                          </span>
                          <span style={{ fontWeight: 600 }}>{m.friendly_name}</span>
                        </div>
                        <span
                          style={{
                            fontSize: '9.5px',
                            fontFamily: 'monospace',
                            background: 'var(--surface-badge)',
                            padding: '1px 5px',
                            borderRadius: '3px',
                          }}
                        >
                          {m.current_hz}Hz
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>

        <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap', justifyContent: 'flex-end' }}>
          {supportedRates.map((hz) => (
            <button
              key={hz}
              onClick={() => checkOemGuard(() => activeMon ? handleSetMonitorRate(activeMon.device_name, hz) : onSwitchRefreshRate(hz, true))}
              className={`toggle-btn ${currentHz === hz ? 'active-indigo' : ''}`}
              style={{ minWidth: '44px', padding: '3px 8px' }}
            >
              {hz}Hz
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};
