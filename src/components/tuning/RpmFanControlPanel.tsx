import React, { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CustomProfileConfig, RpmCurvePoint, RpmFanCurves, RpmFanControlStatus, RpmSmoothing } from '../../types/hardware';
import { RpmFanCurveEditor } from './RpmFanCurveEditor';
import { RevSwitch } from '../ui/RevSwitch';

interface RpmFanControlPanelProps {
  fan0: number;
  fan1: number;
  policyVersion?: string;
  onPolicySaved?: (profile: CustomProfileConfig) => void;
}

export const RpmFanControlPanel: React.FC<RpmFanControlPanelProps> = ({ fan0, fan1, policyVersion, onPolicySaved }) => {
  const [control, setControl] = useState<RpmFanControlStatus>({ mode: 'auto', requested_mode: 'auto', target_rpm: [0, 0], resume_pending: false, recovery_remaining_secs: 0 });
  const [targets, setTargets] = useState<[number, number]>([2200, 2200]);
  const [curves, setCurves] = useState<RpmFanCurves | null>(null);
  const [tab, setTab] = useState<'fan0' | 'fan1'>('fan0');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const mounted = useRef(true);
  const targetsInitialized = useRef(false);
  const refresh = async () => {
    const state = await invoke<RpmFanControlStatus>('get_fan_control_state');
    if (mounted.current) {
      setControl(state);
      if (!targetsInitialized.current && state.rpm_limits) {
        const limits = state.rpm_limits;
        setTargets(previous => previous.map((value, index) => Math.max(limits.minimum[index],
          Math.min(limits.maximum, state.requested_mode === 'manual' ? state.target_rpm[index] : value))) as [number, number]);
        targetsInitialized.current = true;
      }
    }
  };
  useEffect(() => {
    let alive = true;
    invoke<RpmFanCurves>('get_rpm_fan_curves').then(value => { if (alive) setCurves(value); })
      .catch(error => { if (alive) setError(String(error)); });
    return () => { alive = false; };
  }, [policyVersion]);
  useEffect(() => {
    mounted.current = true;
    refresh().catch(e => { if (mounted.current) setError(String(e)); });
    const poll = setInterval(() => refresh().catch(e => { if (mounted.current) setError(String(e)); }), 1500);
    return () => { mounted.current = false; clearInterval(poll); clearTimeout(timer.current); };
  }, []);
  const run = async (command: string, args?: Record<string, unknown>) => {
    clearTimeout(timer.current); setBusy(true); setError('');
    try { await invoke(command, args); await refresh(); return true; }
    catch (e) { if (mounted.current) setError(String(e)); return false; }
    finally {
      if (onPolicySaved && mounted.current) {
        try {
          const profile = await invoke<CustomProfileConfig>('get_custom_profile');
          if (mounted.current) onPolicySaved(profile);
        } catch (error) { if (mounted.current) setError(String(error)); }
      }
      if (mounted.current) setBusy(false);
    }
  };
  const curveSelected = control.requested_mode === 'curve';
  const limits = control.rpm_limits;
  const linkedMinimum = limits ? Math.max(...limits.minimum) : 0;
  const curveMinimum = limits ? curves?.linked ? linkedMinimum : limits.minimum[tab === 'fan0' ? 0 : 1] : 0;
  const saveLive = (next: RpmFanCurves) => {
    setCurves(next); clearTimeout(timer.current);
    if (curveSelected) timer.current = setTimeout(() => run('set_rpm_fan_curves', { curves: next }), 300);
  };
  const changeCurve = (points: RpmCurvePoint[]) => {
    if (!curves) return;
    saveLive(curves.linked ? { ...curves, fan0: points, fan1: points } : { ...curves, [tab]: points });
  };
  const changeSmoothing = async (patch: Partial<RpmSmoothing>) => {
    if (!curves || busy) return;
    const previous = curves;
    const next = { ...curves, smoothing: { ...curves.smoothing, ...patch } };
    setCurves(next);
    const saved = curveSelected
      ? await run('set_rpm_fan_curves', { curves: next })
      : await run('set_rpm_fan_smoothing', { smoothing: next.smoothing });
    if (!saved && mounted.current) setCurves(previous);
  };
  const modeLabel = control.mode === 'curve'
    ? `曲线运行中 · CPU 封装 ${control.cpu_temperature ?? '--'}°C · 目标 ${control.target_rpm.join(' / ')} RPM`
    : control.mode === 'manual' ? `固定目标 ${control.target_rpm.join(' / ')} RPM`
    : control.resume_pending ? `保护临时托管 · CPU 封装 ${control.cpu_temperature ?? '--'}°C · 条件稳定后恢复曲线（至少 ${control.recovery_remaining_secs} 秒）`
    : control.mode === 'auto' ? '原厂自动温控' : '正在恢复原厂自动温控';
  return <div className="tuning-section rpm-fan-panel">
    <div className="tuning-card">
      <div className="tuning-header"><span className="tuning-header-title">双风扇目标转速</span></div>
      <div className="sys-row-desc">风扇 1：{fan0} RPM · 风扇 2：{fan1} RPM</div>
      <div className="rpm-fan-actions">
        <button className={`toggle-btn ${control.requested_mode === 'auto' ? 'active-indigo' : ''}`} disabled={busy} onClick={() => run('restore_fan_auto')}>原厂自动</button>
        <button className={`toggle-btn ${curveSelected ? 'active-indigo' : ''}`} disabled={busy || !curves || !limits} onClick={() => curves && run('set_rpm_fan_curves', { curves })}>转速曲线</button>
      </div>
      {limits && [0, 1].map(index => <label className="sys-row" key={index}>
        <span className="sys-row-label">风扇 {index + 1} 固定目标：{targets[index]} RPM</span>
        <input aria-label={`风扇 ${index + 1} 固定目标转速`} type="range" min={limits.minimum[index]} max={limits.maximum} step={limits.step} value={targets[index]} disabled={busy}
          onChange={e => setTargets(previous => previous.map((v, i) => i === index ? Number(e.target.value) : v) as [number, number])} />
      </label>)}
      <button className={`toggle-btn ${control.requested_mode === 'manual' ? 'active-indigo' : ''}`} disabled={busy || !limits} onClick={() => run('set_manual_fan_rpm', { cpuRpm: targets[0], gpuRpm: targets[1] })}>应用固定转速</button>
      <div className="sys-row-desc">{modeLabel}</div>
      {control.smoothing_active && <div className="sys-row-desc">平滑过渡中 · 当前下发 {control.applied_rpm?.join(' / ') ?? '--'} RPM</div>}
      {control.applied_rpm && control.applied_rpm.some((rpm,i) => rpm > (control.transition_rpm ?? control.target_rpm)[i]) && <div className="sys-row-desc">低速起转提升中 · 当前下发 {control.applied_rpm.join(' / ')} RPM</div>}
      {limits && <div className="sys-row-desc">主风扇 {limits.minimum[0]}–{limits.maximum} RPM，副风扇 {limits.minimum[1]}–{limits.maximum} RPM；联动最低 {linkedMinimum} RPM。低速启用时先以 {limits.startup_rpm} RPM 起转 {limits.startup_secs} 秒。</div>}
      <div className="sys-row-desc">持续停转、起转失败或反复停转时交回原厂温控。</div>
      <div className="sys-row-desc">达到 90°C、读温或通信异常时暂交固件；温度 ≤85°C 且传感器、通信连续正常 5 秒后自动恢复曲线。主动选择原厂自动会取消恢复。</div>
      {(error || control.error) && <div className={error ? 'rpm-fan-error' : 'sys-row-desc'} role="status">{error || control.error}</div>}
    </div>
    {curves && <div className="tuning-card">
      <div className="tuning-header">
        <span className="tuning-header-title">软件平滑过渡</span>
        <RevSwitch checked={curves.smoothing.enabled} disabled={busy} onChange={enabled => { void changeSmoothing({ enabled }); }} size="sm" />
      </div>
      <div className="rpm-smoothing-options">
        <label>升速速度<select className="rev-select" value={curves.smoothing.rise_rpm_per_sec} disabled={busy || !curves.smoothing.enabled}
          onChange={e => { void changeSmoothing({ rise_rpm_per_sec: Number(e.target.value) }); }}>
          {[...new Set([500,1000,1500,2000,3000,5000,curves.smoothing.rise_rpm_per_sec])].sort((a,b) => a-b).map(rate => <option key={rate} value={rate}>{rate} RPM/秒</option>)}
        </select></label>
        <label>降速速度<select className="rev-select" value={curves.smoothing.fall_rpm_per_sec} disabled={busy || !curves.smoothing.enabled}
          onChange={e => { void changeSmoothing({ fall_rpm_per_sec: Number(e.target.value) }); }}>
          {[...new Set([100,200,300,500,1000,2000,curves.smoothing.fall_rpm_per_sec])].sort((a,b) => a-b).map(rate => <option key={rate} value={rate}>{rate} RPM/秒</option>)}
        </select></label>
        <label>降速等待<select className="rev-select" value={curves.smoothing.fall_delay_secs} disabled={busy || !curves.smoothing.enabled}
          onChange={e => { void changeSmoothing({ fall_delay_secs: Number(e.target.value) }); }}>
          {[...new Set([0,2,3,5,10,curves.smoothing.fall_delay_secs])].sort((a,b) => a-b).map(seconds => <option key={seconds} value={seconds}>{seconds} 秒</option>)}
        </select></label>
      </div>
      <div className="sys-row-desc">设置自动保存，适用于曲线和固定转速。温度短暂回落时先等待，再缓慢降速；达到 85°C 时立即升速并暂停降速。起转和停转保护优先执行。</div>
    </div>}
    {curves && limits && <>
      <div className="rpm-fan-actions">
        <button className={`toggle-btn ${tab === 'fan0' ? 'active-indigo' : ''}`} disabled={busy} onClick={() => setTab('fan0')}>风扇 1 曲线</button>
        <button className={`toggle-btn ${tab === 'fan1' ? 'active-indigo' : ''}`} disabled={busy} onClick={() => setTab('fan1')}>风扇 2 曲线</button>
        <label className="rpm-curve-link"><input type="checkbox" checked={curves.linked} disabled={busy}
          onChange={e => { const points = curves[tab].map(p => ({ ...p, rpm: Math.max(linkedMinimum, p.rpm) })); saveLive(e.target.checked ? { ...curves, fan0: points, fan1: points, linked: true } : { ...curves, linked: false }); }} />双风扇联动</label>
      </div>
      <RpmFanCurveEditor curve={curves[tab]} onChange={changeCurve} disabled={busy} minRpm={curveMinimum} maxRpm={limits.maximum} rpmStep={limits.step} />
      <button className="toggle-btn" disabled={busy} onClick={() => run('set_rpm_fan_curves', { curves })}>保存并启用转速曲线</button>
    </>}
  </div>;
};
