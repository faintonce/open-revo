import React, { useState, useEffect, useMemo, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HardwareState } from '../../types/hardware';
import { Tv, Laptop, ChevronDown, Battery, Check, Sun } from 'lucide-react';
import { trText, useI18n } from '../../i18n/I18nContext';

interface DrawerDisplayCardProps {
  status: HardwareState;
  onSwitchRefreshRate: (hz: number, persist?: boolean) => void;
  checkOemGuard: (action: () => void) => void;
}

export const DrawerDisplayCard: React.FC<DrawerDisplayCardProps> = ({
  status,
  onSwitchRefreshRate,
  checkOemGuard,
}) => {
  useI18n();
  const [showHzMenu, setShowHzMenu] = useState(false);
  const [selectedMonIdx, setSelectedMonIdx] = useState(0);
  const [showMonMenu, setShowMonMenu] = useState(false);
  const [autoMinHz, setAutoMinHz] = useState(status.auto_min_refresh_on_battery ?? true);
  const [brightness, setBrightness] = useState(70);
  const brightnessTimerRef = useRef<any>(null);
  const isInteractingBrightnessRef = useRef<number>(0);

  // 响应外部状态变化
  useEffect(() => {
    if (typeof status.auto_min_refresh_on_battery === 'boolean') {
      setAutoMinHz(status.auto_min_refresh_on_battery);
    }
  }, [status.auto_min_refresh_on_battery]);

  const handleToggleAutoMinHz = async () => {
    const nextVal = !autoMinHz;
    setAutoMinHz(nextVal);
    try {
      await invoke('set_auto_min_refresh_on_battery', { enable: nextVal });
    } catch (e) {
      console.error('Failed to toggle auto min refresh:', e);
      setAutoMinHz(!nextVal);
    }
  };

  // 初始化拉取当前真实屏幕亮度 + 聚焦与周期同步
  useEffect(() => {
    const fetchBrightness = () => {
      // 若用户在 1.5 秒内曾手动滑动或点击调节，则跳过本次同步，防止滑块抖动
      if (Date.now() - isInteractingBrightnessRef.current < 1500) return;
      invoke<number>('get_display_brightness')
        .then((val) => {
          if (typeof val === 'number' && val >= 0 && val <= 100) {
            setBrightness(val);
          }
        })
        .catch((err) => console.warn('Failed to get display brightness:', err));
    };

    fetchBrightness();
    window.addEventListener('focus', fetchBrightness);
    const interval = setInterval(fetchBrightness, 3000);

    let unlistenBrightness: (() => void) | undefined;
    listen<number>('display-brightness-changed', (event) => {
      if (typeof event.payload === 'number' && Date.now() - isInteractingBrightnessRef.current > 600) {
        setBrightness(event.payload);
      }
    }).then((fn) => {
      unlistenBrightness = fn;
    });

    return () => {
      window.removeEventListener('focus', fetchBrightness);
      clearInterval(interval);
      if (unlistenBrightness) unlistenBrightness();
    };
  }, []);

  // 点击外部关闭下拉菜单
  useEffect(() => {
    const closeMenus = () => {
      setShowHzMenu(false);
      setShowMonMenu(false);
    };
    window.addEventListener('click', closeMenus);
    return () => window.removeEventListener('click', closeMenus);
  }, []);

  const handleBrightnessChange = (val: number) => {
    isInteractingBrightnessRef.current = Date.now();
    setBrightness(val);
    if (brightnessTimerRef.current) {
      clearTimeout(brightnessTimerRef.current);
    }
    brightnessTimerRef.current = setTimeout(() => {
      invoke('set_display_brightness', { brightness: val }).catch((err) =>
        console.warn('Set display brightness error:', err)
      );
    }, 50);
  };

  const handleBrightnessQuick = (val: number) => {
    isInteractingBrightnessRef.current = Date.now();
    setBrightness(val);
    if (brightnessTimerRef.current) {
      clearTimeout(brightnessTimerRef.current);
    }
    invoke('set_display_brightness', { brightness: val }).catch((err) =>
      console.warn('Set display brightness quick error:', err)
    );
  };

  // Multi-Monitor resolution & targeted refresh rate management
  const monitors = useMemo(() => status.monitors || [], [status.monitors]);
  const activeMon = monitors[selectedMonIdx] || monitors[0] || null;
  const currentHz = activeMon ? activeMon.current_hz : status.refresh_rate;

  // Calculate available refresh rates for currently selected monitor
  const rates = useMemo(() => {
    let list = activeMon?.supported_rates;
    if (!list || list.length === 0) {
      list = status.available_refresh_rates && status.available_refresh_rates.length > 0
        ? status.available_refresh_rates
        : [60, 144, 240];
    }
    const sorted = [...list].sort((a, b) => a - b);
    const minRate = sorted[0];
    const maxRate = sorted[sorted.length - 1];
    const intermediate = sorted.slice(1, -1);
    return { minRate, maxRate, intermediate, all: sorted };
  }, [activeMon, status.available_refresh_rates]);

  // Handle switching refresh rate targeting the currently selected monitor
  const handleSwitchRate = async (hz: number) => {
    checkOemGuard(async () => {
      if (activeMon) {
        try {
          await invoke('set_display_monitor_refresh_rate', {
            deviceName: activeMon.device_name,
            hz,
          });
          activeMon.current_hz = hz;
          if (activeMon.is_primary || monitors.length <= 1) {
            onSwitchRefreshRate(hz, false);
          }
        } catch (err) {
          console.warn('Set monitor refresh rate error:', err);
        }
      } else {
        onSwitchRefreshRate(hz, true);
      }
    });
  };

  return (
    <>
      {/* =========================================================
          第 3 排：屏幕刷新率 (多显示器自适应，四个按钮严格各占 1/4)
          ========================================================= */}
      <div className="m-card" style={{ position: 'relative', zIndex: showMonMenu ? 200 : 1 }}>
        <div className="m-card-header" style={{ position: 'relative', zIndex: 10 }}>
          <div className="m-header-left">
            <Tv size={13} style={{ color: 'var(--theme-hz)' }} />
            <span>{trText('屏幕刷新率:')}</span>
            <span style={{ color: 'var(--theme-hz)', fontWeight: 700 }}>{currentHz}Hz</span>
          </div>

          <div className="m-header-right" style={{ display: 'flex', alignItems: 'center' }}>
            {monitors.length <= 1 ? (
              <span style={{ color: 'var(--theme-hz)', fontSize: '10px' }}>
                {activeMon ? activeMon.friendly_name : trText('2.5K 屏')}
              </span>
            ) : (
              /* 多显示器微型下拉选择器 (纯黑不透底，杜绝折行溢出) */
              <div style={{ position: 'relative' }} onClick={(e) => e.stopPropagation()}>
                <button
                  className="m-mon-selector-btn"
                  onClick={() => setShowMonMenu(!showMonMenu)}
                  title="点击切换当前调控的显示器"
                >
                  <span style={{ display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
                    {activeMon?.is_internal ? <Laptop size={11} /> : <Tv size={11} />}
                    {activeMon?.friendly_name?.split(' ')[0] || trText('屏幕')} · {currentHz}Hz
                  </span>
                  <ChevronDown size={10} />
                </button>

                {showMonMenu && (
                  <div className="m-mon-dropdown-menu">
                    {monitors.map((m, idx) => (
                      <div
                        key={m.device_name}
                        className={`m-mon-dropdown-item ${idx === selectedMonIdx ? 'active' : ''}`}
                        onClick={() => {
                          setSelectedMonIdx(idx);
                          setShowMonMenu(false);
                        }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                          <span style={{ display: 'flex', alignItems: 'center' }}>
                            {m.is_internal ? <Laptop size={12} /> : <Tv size={12} />}
                          </span>
                          <span style={{ fontWeight: 600 }}>{m.friendly_name}</span>
                        </div>
                        <span className="m-mon-badge-hz">{m.current_hz}Hz</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>

        <div className="m-grid-4">
          {/* 1/4: 拔电自动低刷 */}
          <button
            className="m-btn-hz"
            style={
              autoMinHz
                ? {
                  background: 'var(--theme-hz-dim)',
                  borderColor: 'var(--theme-hz-border)',
                  color: 'var(--theme-hz)',
                }
                : {}
            }
            onClick={handleToggleAutoMinHz}
            title="拔掉充电器时自动将内置屏幕切换至最低支持刷新率以延长续航 (外接屏不受影响)"
          >
            <Battery size={12} />
            <span>{trText('拔电低刷')}</span>
          </button>

          {/* 2/4: 最低档 */}
          <button
            className="m-btn-hz"
            style={
              currentHz === rates.minRate
                ? {
                  background: 'var(--theme-hz-dim)',
                  borderColor: 'var(--theme-hz-border)',
                  color: 'var(--theme-hz)',
                  boxShadow: '0 0 10px var(--theme-hz-glow)',
                }
                : {}
            }
            onClick={() => handleSwitchRate(rates.minRate)}
          >
            {rates.minRate}Hz
          </button>

          {/* 3/4: 最高档 */}
          <button
            className="m-btn-hz"
            style={
              currentHz === rates.maxRate
                ? {
                  background: 'var(--theme-hz-dim)',
                  borderColor: 'var(--theme-hz-border)',
                  color: 'var(--theme-hz)',
                  boxShadow: '0 0 10px var(--theme-hz-glow)',
                }
                : {}
            }
            onClick={() => handleSwitchRate(rates.maxRate)}
          >
            {rates.maxRate}Hz
          </button>

          {/* 4/4: 更多档位下拉 (容器 1/4，内部按钮 100% 撑满) */}
          <div className="m-hz-dropdown-wrap" style={{ zIndex: showHzMenu ? 50 : 1 }} onClick={(e) => e.stopPropagation()}>
            <button
              className="m-btn-hz"
              style={
                rates.intermediate.includes(currentHz)
                  ? {
                    background: 'var(--theme-hz-dim)',
                    borderColor: 'var(--theme-hz-border)',
                    color: 'var(--theme-hz)',
                  }
                  : {}
              }
              onClick={() => setShowHzMenu(!showHzMenu)}
            >
              <span>{rates.intermediate.includes(currentHz) ? `${currentHz}Hz` : trText('更多')}</span>
              <ChevronDown size={10} />
            </button>

            {showHzMenu && (
              <div className="m-hz-dropdown-menu">
                {(rates.intermediate.length > 0 ? rates.intermediate : [30, 48, 100, 144]).map((hz) => (
                  <div
                    key={hz}
                    className="m-hz-dropdown-item"
                    onClick={() => {
                      handleSwitchRate(hz);
                      setShowHzMenu(false);
                    }}
                  >
                    <span>{hz}Hz</span>
                    {currentHz === hz && <Check size={11} style={{ color: 'var(--theme-hz)' }} />}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* =========================================================
          第 4 排：屏幕亮度
          ========================================================= */}
      <div className="m-card">
        <div className="m-card-header">
          <div className="m-header-left">
            <Sun size={13} style={{ color: 'var(--theme-lux)' }} />
            <span>{trText('屏幕亮度:')}</span>
            <span style={{ color: 'var(--theme-lux)', fontWeight: 700 }}>{brightness}%</span>
          </div>
          <div style={{ display: 'flex', gap: '5px' }}>
            <div className="m-chip" onClick={() => handleBrightnessQuick(25)}>{trText('暗光 25%')}</div>
            <div className="m-chip" onClick={() => handleBrightnessQuick(60)}>{trText('室内 60%')}</div>
            <div className="m-chip" onClick={() => handleBrightnessQuick(100)}>{trText('户外 100%')}</div>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '9px' }}>
          <Sun size={12} style={{ color: 'var(--text-muted)', flexShrink: 0 }} />
          <input
            type="range"
            min={0}
            max={100}
            value={brightness}
            onChange={(e) => handleBrightnessChange(Number(e.target.value))}
            style={{ accentColor: 'var(--theme-lux)' }}
          />
          <Sun size={14} style={{ color: 'var(--theme-lux)', flexShrink: 0 }} />
        </div>
      </div>
    </>
  );
};
