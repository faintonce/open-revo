import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Zap, Battery, Plug, ShieldCheck, Globe, Ban, Lock, Clock, Moon, Snowflake, Info } from 'lucide-react';
import { RevBadge, RevSwitch } from './ui';

interface PowerSettingsInfo {
  battery_percent: number;
  is_ac_online: boolean;
  is_charging?: boolean;
  battery_mode: string; // "long_life" (100) | "balance" (80) | "station" (60)
  battery_limit: number;
  battery_hardware_limit_enabled: boolean;
  usb_charge_enabled: boolean;
  ac_recovery_enabled: boolean;
  high_perf_scheme: boolean;

  // Modern Standby & Sleep Guard
  sleep_guard_enabled: boolean;
  sleep_disconnect_net: boolean;
  sleep_block_away: boolean;
  sleep_ignore_sysreq: boolean;
  sleep_disable_wake_timers: boolean;
  sleep_deep_sleep: boolean;
  sleep_hibernate_30m: boolean;
  sleep_auto_office: boolean;
}

export const PowerTuningView: React.FC = () => {
  const [powerInfo, setPowerInfo] = useState<PowerSettingsInfo>({
    battery_percent: 100,
    is_ac_online: true,
    is_charging: false,
    battery_mode: 'balance',
    battery_limit: 80,
    battery_hardware_limit_enabled: false,
    usb_charge_enabled: false,
    ac_recovery_enabled: false,
    high_perf_scheme: true,
    sleep_guard_enabled: false,
    sleep_disconnect_net: false,
    sleep_block_away: false,
    sleep_ignore_sysreq: false,
    sleep_disable_wake_timers: false,
    sleep_deep_sleep: false,
    sleep_hibernate_30m: false,
    sleep_auto_office: false,
  });

  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<PowerSettingsInfo>('get_power_settings')
      .then((res) => {
        if (res) setPowerInfo(res);
      })
      .catch((e) => console.error('Failed to get power settings:', e));

    const timer = setInterval(() => {
      invoke<PowerSettingsInfo>('get_power_settings')
        .then((res) => {
          if (res) setPowerInfo(res);
        })
        .catch(() => { });
    }, 5000);
    return () => clearInterval(timer);
  }, []);

  const handleSelectBatteryMode = async (mode: string) => {
    if (loading) return;
    setLoading(true);
    setPowerInfo((prev) => ({ ...prev, battery_mode: mode }));
    try {
      const res = await invoke<PowerSettingsInfo>('set_battery_mode', { mode });
      if (res) setPowerInfo(res);
    } catch (e) {
      console.error('Failed to set battery mode:', e);
    } finally {
      setLoading(false);
    }
  };

  const handleToggleOption = async (option: 'usb_charge' | 'ac_recovery' | 'high_perf') => {
    if (loading) return;
    setLoading(true);

    const keyMap = {
      usb_charge: 'usb_charge_enabled',
      ac_recovery: 'ac_recovery_enabled',
      high_perf: 'high_perf_scheme',
    } as const;

    const nextVal = !powerInfo[keyMap[option]];
    setPowerInfo((prev) => ({ ...prev, [keyMap[option]]: nextVal }));

    try {
      const res = await invoke<PowerSettingsInfo>('set_power_option', { option, enable: nextVal });
      if (res) setPowerInfo(res);
    } catch (e) {
      console.error(`Failed to toggle ${option}:`, e);
    } finally {
      setLoading(false);
    }
  };

  const handleToggleSleepGuard = async (
    option: 'disconnect_net' | 'block_away' | 'ignore_sysreq' | 'disable_wake_timers' | 'deep_sleep' | 'hibernate_30m'
  ) => {
    if (loading) return;
    setLoading(true);

    const keyMap = {
      disconnect_net: 'sleep_disconnect_net',
      block_away: 'sleep_block_away',
      ignore_sysreq: 'sleep_ignore_sysreq',
      disable_wake_timers: 'sleep_disable_wake_timers',
      deep_sleep: 'sleep_deep_sleep',
      hibernate_30m: 'sleep_hibernate_30m',
    } as const;

    const nextVal = !powerInfo[keyMap[option]];
    setPowerInfo((prev) => ({ ...prev, [keyMap[option]]: nextVal }));

    try {
      const res = await invoke<PowerSettingsInfo>('set_sleep_guard_option', { option, enable: nextVal });
      if (res) setPowerInfo(res);
    } catch (e) {
      console.error(`Failed to toggle sleep guard option ${option}:`, e);
    } finally {
      setLoading(false);
    }
  };

  const handleToggleMasterGuard = async () => {
    if (loading) return;
    setLoading(true);
    const nextVal = !powerInfo.sleep_guard_enabled;
    setPowerInfo((prev) => ({
      ...prev,
      sleep_guard_enabled: nextVal,
      sleep_disconnect_net: nextVal,
      sleep_block_away: nextVal,
      sleep_ignore_sysreq: nextVal,
      sleep_disable_wake_timers: nextVal,
      sleep_deep_sleep: nextVal,
      sleep_hibernate_30m: nextVal,
    }));

    try {
      const res = await invoke<PowerSettingsInfo>('set_master_sleep_guard', { enable: nextVal });
      if (res) setPowerInfo(res);
    } catch (e) {
      console.error('Failed to set master sleep guard:', e);
    } finally {
      setLoading(false);
    }
  };

  const handleToggleAutoOffice = async () => {
    if (loading) return;
    setLoading(true);
    const nextVal = !powerInfo.sleep_auto_office;
    setPowerInfo((prev) => ({ ...prev, sleep_auto_office: nextVal }));

    try {
      const res = await invoke<PowerSettingsInfo>('set_sleep_auto_office', { enable: nextVal });
      if (res) setPowerInfo(res);
    } catch (e) {
      console.error('Failed to set auto office on sleep:', e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="power-view-container">
      {/* 1. Top Header Bar with Auto Office Checkbox */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexShrink: 0, gap: '12px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <div className="power-header-zap-badge">
            <Zap className="w-4 h-4 power-header-zap-icon" />
          </div>
          <div>
            <div style={{ fontSize: '13.5px', fontWeight: 700, color: 'var(--text-primary)', letterSpacing: '0.02em', whiteSpace: 'nowrap' }}>
              电源与电池管理 (Power & Battery Management)
            </div>
            <div style={{ fontSize: '10.5px', color: 'var(--text-dim)', whiteSpace: 'nowrap' }}>
              主板硬件充放电控制 · 电池健康长寿模式 · 关机反向供电
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {/* Top Middle: 睡眠时自动进入办公模式 */}
          <div
            onClick={handleToggleAutoOffice}
            className={`sleep-office-mode-card ${powerInfo.sleep_auto_office ? 'active' : ''}`}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1px' }}>
              <div style={{ fontSize: '11.5px', fontWeight: 700, color: 'var(--text-primary)', whiteSpace: 'nowrap', lineHeight: '1.25' }}>
                睡眠时自动进入办公模式
              </div>
              <div style={{ fontSize: '9.5px', color: 'var(--text-dim)', whiteSpace: 'nowrap', lineHeight: '1.25' }}>
                合盖前自动降频限功耗 · 防狂暴模式带热入睡
              </div>
            </div>
            <RevSwitch
              checked={powerInfo.sleep_auto_office}
              onChange={handleToggleAutoOffice}
              size="sm"
              activeColor="var(--accent-pwr)"
            />
          </div>
        </div>
      </div>

      {/* 2. Modern Standby & Sleep Guard (Sleep Guard) */}
      <div className="glass-card sleep-guard-card">
        {/* Master Banner */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            paddingBottom: '7px',
            borderBottom: '1px solid var(--border-subtle)',
            cursor: 'pointer',
          }}
          onClick={handleToggleMasterGuard}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '9px' }}>
            <div className="sleep-guard-icon-badge">
              <ShieldCheck className="w-4 h-4" />
            </div>
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span style={{ fontSize: '12px', fontWeight: 800, color: 'var(--text-primary)' }}>
                  智能睡眠守护 / 背包防烫模式 (Anti-Overheat Sleep Guard)
                </span>
                <RevBadge variant="accent">推荐全开</RevBadge>
              </div>
              <div style={{ fontSize: '9.5px', color: 'var(--text-muted)', marginTop: '1px' }}>
                封杀 Windows 现代待机后台偷跑；杜绝密闭背包发烫、电池偷耗以及合盖睡死黑屏无法唤醒。
              </div>
            </div>
          </div>

          <RevSwitch
            checked={powerInfo.sleep_guard_enabled}
            onChange={handleToggleMasterGuard}
            size="md"
            activeColor="var(--accent-beast)"
          />
        </div>

        {/* 6 Core Guard Policies (3x2 Grid) */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '6px 9px' }}>
          {/* Policy 1: 待机切断网络 */}
          <div
            className="sleep-policy-item"
            style={{ cursor: 'pointer' }}
            onClick={() => handleToggleSleepGuard('disconnect_net')}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1px', overflow: 'hidden' }}>
              <div style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '4px', whiteSpace: 'nowrap' }}>
                <Globe className="w-3 h-3 text-cyan-400" />
                <span>待机断开网络连接</span>
              </div>
              <div style={{ fontSize: '9px', color: 'var(--text-dim)', whiteSpace: 'nowrap', textOverflow: 'ellipsis', overflow: 'hidden' }}>
                切断休眠 Wi-Fi，阻止系统后台联网唤醒
              </div>
            </div>
            <RevSwitch
              checked={powerInfo.sleep_disconnect_net}
              onChange={() => handleToggleSleepGuard('disconnect_net')}
              size="sm"
              activeColor="var(--status-ok)"
            />
          </div>

          {/* Policy 2: 封杀离开模式伪睡眠 */}
          <div
            className="sleep-policy-item"
            style={{ cursor: 'pointer' }}
            onClick={() => handleToggleSleepGuard('block_away')}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1px', overflow: 'hidden' }}>
              <div style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '4px', whiteSpace: 'nowrap' }}>
                <Ban className="w-3 h-3 text-rose-400" />
                <span>封杀离开模式伪睡眠</span>
              </div>
              <div style={{ fontSize: '9px', color: 'var(--text-dim)', whiteSpace: 'nowrap', textOverflow: 'ellipsis', overflow: 'hidden' }}>
                禁止流氓软件以“离开模式”偷跑发热
              </div>
            </div>
            <RevSwitch
              checked={powerInfo.sleep_block_away}
              onChange={() => handleToggleSleepGuard('block_away')}
              size="sm"
              activeColor="var(--status-ok)"
            />
          </div>

          {/* Policy 3: 忽略系统所需策略 */}
          <div
            className="sleep-policy-item"
            style={{ cursor: 'pointer' }}
            onClick={() => handleToggleSleepGuard('ignore_sysreq')}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1px', overflow: 'hidden' }}>
              <div style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '4px', whiteSpace: 'nowrap' }}>
                <Lock className="w-3 h-3 text-amber-400" />
                <span>忽略系统所需策略</span>
              </div>
              <div style={{ fontSize: '9px', color: 'var(--text-dim)', whiteSpace: 'nowrap', textOverflow: 'ellipsis', overflow: 'hidden' }}>
                强制无视第三方进程的阻止待机声明
              </div>
            </div>
            <RevSwitch
              checked={powerInfo.sleep_ignore_sysreq}
              onChange={() => handleToggleSleepGuard('ignore_sysreq')}
              size="sm"
              activeColor="var(--status-ok)"
            />
          </div>

          {/* Policy 4: 禁用 RTC 唤醒定时器 */}
          <div
            className="sleep-policy-item"
            style={{ cursor: 'pointer' }}
            onClick={() => handleToggleSleepGuard('disable_wake_timers')}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1px', overflow: 'hidden' }}>
              <div style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '4px', whiteSpace: 'nowrap' }}>
                <Clock className="w-3 h-3 text-indigo-400" />
                <span>禁用 RTC 唤醒定时器</span>
              </div>
              <div style={{ fontSize: '9px', color: 'var(--text-dim)', whiteSpace: 'nowrap', textOverflow: 'ellipsis', overflow: 'hidden' }}>
                杜绝系统任务夜间或包内自启“焖烧”
              </div>
            </div>
            <RevSwitch
              checked={powerInfo.sleep_disable_wake_timers}
              onChange={() => handleToggleSleepGuard('disable_wake_timers')}
              size="sm"
              activeColor="var(--status-ok)"
            />
          </div>

          {/* Policy 5: 启用硬件深度睡眠 */}
          <div
            className="sleep-policy-item"
            style={{ cursor: 'pointer' }}
            onClick={() => handleToggleSleepGuard('deep_sleep')}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1px', overflow: 'hidden' }}>
              <div style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '4px', whiteSpace: 'nowrap' }}>
                <Moon className="w-3 h-3 text-purple-400" />
                <span>启用 ACPI 深度睡眠</span>
              </div>
              <div style={{ fontSize: '9px', color: 'var(--text-dim)', whiteSpace: 'nowrap', textOverflow: 'ellipsis', overflow: 'hidden' }}>
                允许 CPU 进入极低功耗 C10/D3cold 状态
              </div>
            </div>
            <RevSwitch
              checked={powerInfo.sleep_deep_sleep}
              onChange={() => handleToggleSleepGuard('deep_sleep')}
              size="sm"
              activeColor="var(--status-ok)"
            />
          </div>

          {/* Policy 6: 待机超时落锁休眠 */}
          <div
            className="sleep-policy-item"
            style={{ cursor: 'pointer' }}
            onClick={() => handleToggleSleepGuard('hibernate_30m')}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1px', overflow: 'hidden' }}>
              <div style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '4px', whiteSpace: 'nowrap' }}>
                <Snowflake className="w-3 h-3 text-sky-400" />
                <span>30 分钟超时转入休眠</span>
              </div>
              <div style={{ fontSize: '9px', color: 'var(--text-dim)', whiteSpace: 'nowrap', textOverflow: 'ellipsis', overflow: 'hidden' }}>
                待机半小时未唤醒自动存盘切断供电
              </div>
            </div>
            <RevSwitch
              checked={powerInfo.sleep_hibernate_30m}
              onChange={() => handleToggleSleepGuard('hibernate_30m')}
              size="sm"
              activeColor="var(--status-ok)"
            />
          </div>
        </div>
      </div>

      {/* 3. Section 2: 硬件电源与供电选项 (3 联横向卡片) */}
      <div className="glass-card" style={{ padding: '8px 13px', display: 'flex', flexDirection: 'column', gap: '7px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ fontSize: '11.5px', fontWeight: 700, color: 'var(--text-muted)', letterSpacing: '0.04em' }}>
            电源与供电选项
          </div>
          <div style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>
            主板硬件 EC 寄存器底层配置
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '9px' }}>
          {/* Option 1: 关机 USB 供电 */}
          <div
            className="power-subtle-card"
            style={{
              padding: '7px 10px',
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'space-between',
              gap: '5px',
              cursor: 'pointer',
            }}
            onClick={() => handleToggleOption('usb_charge')}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '6px' }}>
              <span style={{ fontSize: '11px', fontWeight: 700, color: 'var(--text-primary)' }}>关机下使用USB给电</span>
              <RevSwitch
                checked={powerInfo.usb_charge_enabled}
                onChange={() => handleToggleOption('usb_charge')}
                size="sm"
                activeColor="var(--accent-pwr)"
              />
            </div>
            <div style={{ fontSize: '9.5px', color: 'var(--text-dim)', lineHeight: '1.35' }}>
              可在关机或休眠状态下支持USB端口持续供电，方便为手机或外设应急充电。
            </div>
          </div>

          {/* Option 2: 来电自动开机 */}
          <div
            className="power-subtle-card"
            style={{
              padding: '7px 10px',
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'space-between',
              gap: '5px',
              cursor: 'pointer',
            }}
            onClick={() => handleToggleOption('ac_recovery')}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '6px' }}>
              <span style={{ fontSize: '11px', fontWeight: 700, color: 'var(--text-primary)' }}>来电自动开机 (AC Recovery)</span>
              <RevSwitch
                checked={powerInfo.ac_recovery_enabled}
                onChange={() => handleToggleOption('ac_recovery')}
                size="sm"
                activeColor="var(--accent-pwr)"
              />
            </div>
            <div style={{ fontSize: '9.5px', color: 'var(--text-dim)', lineHeight: '1.35' }}>
              开启后在关机状态下，接通外部电源适配器瞬间主板自动开机启动系统。
            </div>
          </div>

          {/* Option 3: 高性能电源模式 */}
          <div
            className="power-subtle-card"
            style={{
              padding: '7px 10px',
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'space-between',
              gap: '5px',
              position: 'relative',
              cursor: 'pointer',
            }}
            onClick={() => handleToggleOption('high_perf')}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '6px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                <span style={{ fontSize: '11px', fontWeight: 700, color: 'var(--text-primary)' }}>高性能电源模式</span>
                <div
                  className="tooltip-container"
                  title="根据选择的模式自动调整 Windows 的电源模式 (已接通电源)：&#10;• 办公模式 → 最佳能效&#10;• 均衡模式 → 平衡&#10;• 狂暴模式 (如有) → 最佳性能&#10;• 自定义模式 (如有) → 平衡"
                  style={{ cursor: 'help', display: 'flex', alignItems: 'center' }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <Info className="w-3 h-3 text-sky-400" />
                </div>
              </div>
              <RevSwitch
                checked={powerInfo.high_perf_scheme}
                onChange={() => handleToggleOption('high_perf')}
                size="sm"
                activeColor="var(--accent-pwr)"
              />
            </div>
            <div style={{ fontSize: '9.5px', color: 'var(--text-dim)', lineHeight: '1.35' }}>
              开启后根据四大模式自动调整 Windows 系统电源模式；关闭后切换模式不影响系统电源模式。
            </div>
          </div>
        </div>
      </div>

      {/* 4. Section 3: 电池健康 */}
      <div className="glass-card" style={{ padding: '8px 14px', display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ fontSize: '11.5px', fontWeight: 700, color: 'var(--text-muted)', letterSpacing: '0.04em' }}>
            电池健康
          </div>
          <div style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>
            主板硬件级充电控制 · 电池健康长寿模式
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1.3fr 1fr', gap: '14px', alignItems: 'center' }}>
          {/* Left: 3 Radio Modes with Exact OEM Copy */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {/* Mode 1: 长效模式 */}
            <div
              onClick={() => handleSelectBatteryMode('long_life')}
              className={`battery-mode-card battery-mode-long-life ${powerInfo.battery_mode === 'long_life' ? 'active' : ''}`}
            >
              <div className="battery-mode-radio-circle">
                {powerInfo.battery_mode === 'long_life' && <div className="battery-mode-radio-dot" />}
              </div>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <span style={{ fontSize: '11.5px', fontWeight: 700, color: 'var(--text-primary)' }}>长效模式</span>
                </div>
                <div style={{ fontSize: '9.5px', color: 'var(--text-dim)', lineHeight: '1.35', marginTop: '1px' }}>
                  适合很长时间使用电池工作的环境。
                </div>
              </div>
            </div>

            {/* Mode 2: 平衡模式 */}
            <div
              onClick={() => handleSelectBatteryMode('balance')}
              className={`battery-mode-card battery-mode-balance ${powerInfo.battery_mode === 'balance' ? 'active' : ''}`}
            >
              <div className="battery-mode-radio-circle">
                {powerInfo.battery_mode === 'balance' && <div className="battery-mode-radio-dot" />}
              </div>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <span style={{ fontSize: '11.5px', fontWeight: 700, color: 'var(--text-primary)' }}>平衡模式</span>
                </div>
                <div style={{ fontSize: '9.5px', color: 'var(--text-dim)', lineHeight: '1.35', marginTop: '1px' }}>
                  偶而会携带电脑工作，且不需要多余靠电池供电。
                </div>
              </div>
            </div>

            {/* Mode 3: 工作站模式 */}
            <div
              onClick={() => handleSelectBatteryMode('station')}
              className={`battery-mode-card battery-mode-station ${powerInfo.battery_mode === 'station' ? 'active' : ''}`}
            >
              <div className="battery-mode-radio-circle">
                {powerInfo.battery_mode === 'station' && <div className="battery-mode-radio-dot" />}
              </div>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <span style={{ fontSize: '11.5px', fontWeight: 700, color: 'var(--text-primary)' }}>工作站模式</span>
                </div>
                <div style={{ fontSize: '9.5px', color: 'var(--text-dim)', lineHeight: '1.35', marginTop: '1px' }}>
                  如果很容易接续适配器工作，无携带电脑外出工作的需求，强烈建议使用此模式，延长电池的寿命。
                </div>
              </div>
            </div>
          </div>

          {/* Right: Real-time Animated Battery Gauge */}
          <div className="battery-gauge-box">
            {/* Battery Shell */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
              <div className="battery-gauge-shell">
                <div
                  className="battery-gauge-bar"
                  style={{
                    height: '100%',
                    width: `${Math.max(5, Math.min(100, powerInfo.battery_percent))}%`,
                    borderRadius: 'var(--radius-xs)',
                    background:
                      powerInfo.battery_percent <= 20
                        ? 'linear-gradient(90deg, var(--status-error) 0%, #fb7185 100%)'
                        : powerInfo.battery_percent <= 50
                        ? 'linear-gradient(90deg, var(--status-warn) 0%, #fbbf24 100%)'
                        : 'linear-gradient(90deg, var(--status-ok) 0%, var(--accent-gpu) 100%)',
                    boxShadow:
                      powerInfo.battery_percent <= 20
                        ? '0 0 12px var(--status-error-dim)'
                        : '0 0 12px var(--status-ok-dim)',
                    transition: 'width 0.4s ease, background 0.4s ease',
                  }}
                />
              </div>
              <div
                style={{
                  width: '5px',
                  height: '16px',
                  background: 'var(--border-highlight)',
                  borderRadius: '0 3px 3px 0',
                }}
              />
            </div>

            {/* Reading */}
            <div style={{ display: 'flex', alignItems: 'baseline', gap: '2px' }}>
              <span
                style={{
                  fontSize: '30px',
                  fontWeight: 900,
                  color: 'var(--text-primary)',
                  fontFamily: 'Consolas, monospace',
                  letterSpacing: '-0.02em',
                }}
              >
                {powerInfo.battery_percent}
              </span>
              <span style={{ fontSize: '13px', fontWeight: 800, color: 'var(--status-ok)' }}>%</span>
            </div>

            <div
              style={{
                fontSize: '10.5px',
                color: powerInfo.is_ac_online ? 'var(--status-ok)' : 'var(--status-warn)',
                display: 'flex',
                alignItems: 'center',
                gap: '5px',
                fontWeight: 600,
              }}
            >
              {powerInfo.is_ac_online ? <Plug className="w-3.5 h-3.5" /> : <Battery className="w-3.5 h-3.5" />}
              <span>{powerInfo.is_ac_online ? '已连接电源适配器' : '正在使用电池供电'}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
