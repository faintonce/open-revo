import React from 'react';
import { Cpu, Activity, Fan, Sparkles } from 'lucide-react';
import { HardwareState } from '../../types/hardware';
import { PowerWattBadge } from '../tuning/HardwareTelemetryBar';

import { ModelCapabilities } from '../../plugins/plugin_manifest';

function formatCpuShort(brand?: string): string {
  if (!brand) return 'CPU';
  let s = brand
    .replace(/\(R\)/gi, '')
    .replace(/\(TM\)/gi, '')
    .replace(/\bCPU\b/gi, '')
    .replace(/\bProcessor\b/gi, '')
    .trim();
  s = s.replace(/\s+/g, ' ');
  return s || 'CPU';
}

function formatGpuShort(name?: string, isPureIgpu?: boolean): string {
  if (!name) return isPureIgpu ? 'Core Graphics (iGPU)' : 'GPU';
  let s = name
    .replace(/NVIDIA\s+GeForce\s+/gi, '')
    .replace(/\(R\)|\(TM\)/gi, '')
    .replace(/\s+Laptop\s+GPU/gi, '')
    .trim();
  return s || 'GPU';
}

interface HardwareStatCardsProps {
  status: HardwareState;
  capabilities?: ModelCapabilities;
}

export const HardwareStatCards: React.FC<HardwareStatCardsProps> = ({ status, capabilities }) => {
  const isPureIgpu = capabilities ? capabilities.discrete_gpu === false : !status.gpu_name;

  const isApuOrIgpu = isPureIgpu;
  const fanControl = status.fan_control;
  const fanLabel = fanControl
    ? fanControl.resume_pending ? '保护·待恢复'
      : fanControl.mode === 'curve' ? '转速曲线'
      : fanControl.mode === 'manual' ? '固定转速'
      : fanControl.mode === 'auto' ? '原厂自动' : '恢复自动中'
    : status.fan_boost ? '一键强冷' : '自动温控';

  return (
    <div className="telemetry-cards-grid" data-slot="telemetry-grid">
      {/* 1. CPU Card */}
      <div
        className="glass-card telemetry-card telemetry-card-cpu"
        data-slot="cpu"
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          minHeight: '116px',
          padding: '13px 14px',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, color: '#a5b4fc', letterSpacing: '0.02em' }}>
            CPU 处理器状态
          </span>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <PowerWattBadge watts={status.cpu_power_w} type="cpu" />
            <Cpu className="w-4 h-4 text-indigo-400 flex-shrink-0" />
          </div>
        </div>

        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', margin: '8px 0 3px 0' }}>
            <div style={{ display: 'flex', alignItems: 'baseline' }}>
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: '24px', fontWeight: 800, color: status.cpu_temp > 85 ? '#f43f5e' : '#fff', lineHeight: 1 }}>
                {status.cpu_temp > 0 ? status.cpu_temp : '--'}
              </span>
              <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>°C</span>
            </div>

            <div style={{ width: '1px', height: '18px', background: 'rgba(255, 255, 255, 0.1)', margin: '0 4px', alignSelf: 'center' }} />

            <div style={{ display: 'flex', alignItems: 'baseline' }}>
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: '19px', fontWeight: 700, color: '#818cf8', lineHeight: 1 }}>
                {status.cpu_freq > 0 ? status.cpu_freq : 2200}
              </span>
              <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>MHz</span>
            </div>
          </div>

          {/* 负载细微进度条 */}
          <div style={{ height: '3px', width: '100%', background: 'rgba(255, 255, 255, 0.07)', borderRadius: '2px', overflow: 'hidden', marginTop: '5px', marginBottom: '7px' }}>
            <div
              style={{
                height: '100%',
                borderRadius: '2px',
                width: `${Math.min(100, Math.max(0, status.cpu_usage || 0))}%`,
                background: 'linear-gradient(90deg, #6366f1, #818cf8)',
              }}
            />
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', fontSize: '10.5px', color: 'var(--text-dim)' }}>
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', minWidth: 0, flex: 1 }}>
            {formatCpuShort(status.cpu_name)}
          </span>
          <span
            style={{
              background: 'rgba(255, 255, 255, 0.07)',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              padding: '2px 8px',
              borderRadius: '4px',
              fontFamily: 'var(--font-mono)',
              fontSize: '11.5px',
              fontWeight: 600,
              color: '#e2e8f0',
              whiteSpace: 'nowrap',
              flexShrink: 0,
            }}
          >
            {status.cpu_usage || 0}%
          </span>
        </div>
      </div>

      {/* 2. GPU Card */}
      <div
        className="glass-card telemetry-card telemetry-card-gpu"
        data-slot="gpu"
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          minHeight: '116px',
          padding: '13px 14px',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, color: isPureIgpu ? '#38bdf8' : '#6ee7b7', letterSpacing: '0.02em' }}>
            {isPureIgpu ? 'GPU 集成核心' : 'GPU 独立显卡'}
          </span>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <PowerWattBadge watts={status.gpu_power_w} available={status.gpu_power_available} type="gpu" isApuOrIgpu={isApuOrIgpu} />
            <Activity className={`w-4 h-4 ${isPureIgpu ? 'text-sky-400' : 'text-emerald-400'} flex-shrink-0`} />
          </div>
        </div>

        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', margin: '8px 0 3px 0' }}>
            <div style={{ display: 'flex', alignItems: 'baseline' }}>
              <span title={status.gpu_temp_unavailable_reason || undefined} style={{ fontFamily: 'var(--font-mono)', fontSize: '24px', fontWeight: 800, color: status.gpu_temp > 80 ? '#f43f5e' : '#fff', lineHeight: 1 }}>
                {status.gpu_temp > 0 ? status.gpu_temp : '--'}
              </span>
              <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>°C</span>
            </div>

            <div style={{ width: '1px', height: '18px', background: 'rgba(255, 255, 255, 0.1)', margin: '0 4px', alignSelf: 'center' }} />

            <div style={{ display: 'flex', alignItems: 'baseline' }}>
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: '19px', fontWeight: 700, color: isPureIgpu ? '#38bdf8' : '#34d399', lineHeight: 1 }}>
                {status.gpu_core_freq > 0 ? status.gpu_core_freq : '--'}
              </span>
              <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>MHz</span>
            </div>
          </div>

          {/* 负载细微进度条 */}
          <div style={{ height: '3px', width: '100%', background: 'rgba(255, 255, 255, 0.07)', borderRadius: '2px', overflow: 'hidden', marginTop: '5px', marginBottom: '7px' }}>
            <div
              style={{
                height: '100%',
                borderRadius: '2px',
                width: `${Math.min(100, Math.max(0, status.gpu_usage || 0))}%`,
                background: isPureIgpu ? 'linear-gradient(90deg, #0284c7, #38bdf8)' : 'linear-gradient(90deg, #10b981, #34d399)',
              }}
            />
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', fontSize: '10.5px', color: 'var(--text-dim)' }}>
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', minWidth: 0, flex: 1 }}>
            {formatGpuShort(status.gpu_name, isPureIgpu)}
          </span>
          <span
            style={{
              background: 'rgba(255, 255, 255, 0.07)',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              padding: '2px 8px',
              borderRadius: '4px',
              fontFamily: 'var(--font-mono)',
              fontSize: '11.5px',
              fontWeight: 600,
              color: '#e2e8f0',
              whiteSpace: 'nowrap',
              flexShrink: 0,
            }}
          >
            {status.gpu_usage || 0}%
          </span>
        </div>
      </div>

      {/* 3. Fan RPM Card */}
      <div
        className="glass-card telemetry-card telemetry-card-fan"
        data-slot="fan"
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          minHeight: '116px',
          padding: '13px 14px',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, color: '#7dd3fc', letterSpacing: '0.02em' }}>
            散热系统状态
          </span>
          <Fan className={`w-4 h-4 flex-shrink-0 ${status.fan_boost ? 'text-cyan-400 animate-spin' : 'text-slate-400'}`} />
        </div>

        <div>
          {/* 单风扇 vs 双风扇自适应展示 */}
          {status.gpu_fan_rpm > 0 || !isPureIgpu ? (
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', margin: '6px 0 2px 0' }}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                <span style={{ fontSize: '9.5px', color: 'var(--text-dim)', fontWeight: 600, letterSpacing: '0.03em' }}>
                  {isPureIgpu ? '主散热扇' : 'CPU 扇'}
                </span>
                <div style={{ fontFamily: 'var(--font-mono)', fontSize: '18px', fontWeight: 700, color: '#fff', display: 'flex', alignItems: 'baseline' }}>
                  <span>{status.cpu_fan_rpm > 0 ? status.cpu_fan_rpm : '--'}</span>
                  <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>RPM</span>
                </div>
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', borderLeft: '1px solid rgba(255,255,255,0.08)', paddingLeft: '8px' }}>
                <span style={{ fontSize: '9.5px', color: 'var(--text-dim)', fontWeight: 600, letterSpacing: '0.03em' }}>
                  {isPureIgpu ? '副散热扇' : 'GPU 扇'}
                </span>
                <div style={{ fontFamily: 'var(--font-mono)', fontSize: '18px', fontWeight: 700, color: '#a5f3fc', display: 'flex', alignItems: 'baseline' }}>
                  <span>{status.gpu_fan_rpm > 0 ? status.gpu_fan_rpm : '--'}</span>
                  <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>RPM</span>
                </div>
              </div>
            </div>
          ) : (
            <div style={{ margin: '6px 0 2px 0' }}>
              <span style={{ fontSize: '9.5px', color: 'var(--text-dim)', fontWeight: 600, letterSpacing: '0.03em' }}>
                散热风扇
              </span>
              <div style={{ fontFamily: 'var(--font-mono)', fontSize: '20px', fontWeight: 700, color: '#fff', display: 'flex', alignItems: 'baseline' }}>
                <span>{status.cpu_fan_rpm > 0 ? status.cpu_fan_rpm : '--'}</span>
                <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '4px' }}>RPM</span>
              </div>
            </div>
          )}

          {/* 转速负荷细微进度条 */}
          <div style={{ height: '3px', width: '100%', background: 'rgba(255, 255, 255, 0.07)', borderRadius: '2px', overflow: 'hidden', marginTop: '5px', marginBottom: '7px' }}>
            <div
              style={{
                height: '100%',
                borderRadius: '2px',
                width: `${Math.min(100, Math.max(10, ((Math.max(status.cpu_fan_rpm, status.gpu_fan_rpm) || 1500) / 5000) * 100))}%`,
                background: status.fan_boost ? 'linear-gradient(90deg, #06b6d4, #f43f5e)' : 'linear-gradient(90deg, #0284c7, #38bdf8)',
              }}
            />
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', fontSize: '10.5px', color: 'var(--text-dim)' }}>
          <span>{isPureIgpu ? (status.gpu_fan_rpm > 0 ? '双涡轮高效散热' : '智能高效散热') : '双风扇散热'}</span>
          <span className="fan-control-badge" data-mode={fanControl?.resume_pending ? 'protected' : fanControl?.mode ?? (status.fan_boost ? 'boost' : 'auto')}
            title={fanControl?.error ?? (fanControl?.mode === 'curve' || fanControl?.mode === 'manual' ? `目标 ${fanControl.target_rpm.join(' / ')} RPM` : '由固件按当前性能模式自动调速')}>
            {fanLabel}
          </span>
        </div>
      </div>

      {/* 4. VRAM & Clocks Card */}
      <div
        className="glass-card telemetry-card telemetry-card-vram"
        data-slot="vram"
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          minHeight: '116px',
          padding: '13px 14px',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, color: '#d8b4fe', letterSpacing: '0.02em' }}>
            {isPureIgpu ? 'UMA 共享显存' : 'VRAM 显存容量'}
          </span>
          <Sparkles className="w-4 h-4 text-purple-400 flex-shrink-0" />
        </div>

        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', margin: '8px 0 3px 0' }}>
            <div style={{ display: 'flex', alignItems: 'baseline' }}>
              {status.gpu_vram_mb > 0 && status.gpu_vram_mb < 1024 ? (
                <>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: '21px', fontWeight: 800, color: '#fff', lineHeight: 1 }}>
                    {status.gpu_vram_used_mb > 0 ? status.gpu_vram_used_mb : 256}
                  </span>
                  <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>
                    / {status.gpu_vram_mb} MB
                  </span>
                </>
              ) : (
                <>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: '21px', fontWeight: 800, color: '#fff', lineHeight: 1 }}>
                    {status.gpu_vram_used_mb > 0 ? (status.gpu_vram_used_mb / 1024).toFixed(1) : '1.2'}
                  </span>
                  <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>
                    / {status.gpu_vram_mb > 0
                      ? (status.gpu_vram_mb % 1024 === 0
                        ? `${Math.round(status.gpu_vram_mb / 1024)}`
                        : (status.gpu_vram_mb / 1024).toFixed(1))
                      : '16'} GB
                  </span>
                </>
              )}
            </div>

            <div style={{ width: '1px', height: '18px', background: 'rgba(255, 255, 255, 0.1)', margin: '0 4px', alignSelf: 'center' }} />

            <div style={{ display: 'flex', alignItems: 'baseline' }}>
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: '19px', fontWeight: 700, color: '#c084fc', lineHeight: 1 }}>
                {status.gpu_mem_freq > 0 ? status.gpu_mem_freq : 405}
              </span>
              <span style={{ fontSize: '10.5px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '2px' }}>MHz</span>
            </div>
          </div>

          {/* 显存负载细微进度条 */}
          <div style={{ height: '3px', width: '100%', background: 'rgba(255, 255, 255, 0.07)', borderRadius: '2px', overflow: 'hidden', marginTop: '5px', marginBottom: '7px' }}>
            <div
              style={{
                height: '100%',
                borderRadius: '2px',
                width: `${status.gpu_vram_mb > 0 && status.gpu_vram_used_mb > 0 ? Math.min(100, Math.round((status.gpu_vram_used_mb / status.gpu_vram_mb) * 100)) : 8}%`,
                background: 'linear-gradient(90deg, #9333ea, #c084fc)',
              }}
            />
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', fontSize: '10.5px', color: 'var(--text-dim)' }}>
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', minWidth: 0, flex: 1 }}>
            {!isPureIgpu
              ? '专用显存 (VRAM)'
              : '共享显存 (UMA)'}
          </span>
          <span
            style={{
              background: 'rgba(255, 255, 255, 0.07)',
              border: '1px solid rgba(255, 255, 255, 0.12)',
              padding: '2px 8px',
              borderRadius: '4px',
              fontFamily: 'var(--font-mono)',
              fontSize: '11.5px',
              fontWeight: 600,
              color: '#e2e8f0',
              whiteSpace: 'nowrap',
              flexShrink: 0,
            }}
          >
            {status.gpu_vram_mb > 0 && status.gpu_vram_used_mb > 0 ? Math.round((status.gpu_vram_used_mb / status.gpu_vram_mb) * 100) : 8}%
          </span>
        </div>
      </div>
    </div>
  );
};
