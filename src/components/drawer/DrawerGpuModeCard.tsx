import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HardwareState } from '../../types/hardware';
import { Cpu, Repeat, Crosshair, Lock } from 'lucide-react';
import { GpuCapabilities } from '../GpuModeView';

interface DrawerGpuModeCardProps {
  status: HardwareState;
  onExpandDashboard: (tab?: 'overview' | 'gpu' | 'tuning' | 'lighting' | 'water' | 'system' | 'sponsor' | 'custom_tuning') => void;
}

export const DrawerGpuModeCard: React.FC<DrawerGpuModeCardProps> = ({
  status,
  onExpandDashboard,
}) => {
  // GPU architecture & mode state
  const [gpuModeInfo, setGpuModeInfo] = useState<{
    current_mode: string;
    has_dgpu: boolean;
    has_igpu_present: boolean;
    gpu_name: string;
    capabilities?: GpuCapabilities;
  } | null>(null);

  useEffect(() => {
    const fetchGpuMode = () => {
      invoke<any>('get_gpu_mode_info')
        .then((res) => {
          if (res) setGpuModeInfo(res);
        })
        .catch(() => {});
    };

    fetchGpuMode();
    window.addEventListener('focus', fetchGpuMode);

    let unlistenGpu: (() => void) | undefined;
    listen<any>('gpu-mode-changed', (e) => {
      if (e.payload) setGpuModeInfo(e.payload);
    }).then((fn) => { unlistenGpu = fn; });

    return () => {
      window.removeEventListener('focus', fetchGpuMode);
      if (unlistenGpu) unlistenGpu();
    };
  }, []);

  const isSafetyLocked = gpuModeInfo?.capabilities?.is_safety_locked === true;
  const isUserUnlocked = gpuModeInfo?.capabilities?.user_unlocked === true;
  const isTriMode = isSafetyLocked
    ? isUserUnlocked
    : (gpuModeInfo?.capabilities ? gpuModeInfo.capabilities.supports_igpu_only : true);

  // 双显双模机型保护锁定（有独立显卡但非三模硬件）
  const isIgpuLocked = gpuModeInfo?.has_dgpu !== false && !isTriMode;

  return (
    <div className="m-card">
      <div className="m-card-header">
        <div className="m-header-left">
          <Cpu size={13} style={{ color: 'var(--theme-gpu)' }} />
          <span>显卡模式:</span>
          <span style={{ color: 'var(--theme-gpu)', fontWeight: 700 }}>
            {gpuModeInfo?.has_dgpu === false
              ? '核显模式 (纯APU架构)'
              : gpuModeInfo?.current_mode === 'dgpu'
              ? '独显直连 (dGPU)'
              : gpuModeInfo?.current_mode === 'hybrid'
              ? '混合输出 (Dynamic)'
              : gpuModeInfo?.current_mode === 'igpu'
              ? '核显模式 (iGPU)'
              : '检测中...'}
          </span>
        </div>
        <div className="m-header-right" style={{ color: 'var(--theme-gpu)' }}>
          GPU: {status.gpu_temp > 0 ? `${status.gpu_temp}°C` : (status.cpu_temp > 0 ? `${status.cpu_temp}°C` : '--')} · Fan: {status.gpu_fan_rpm > 0 ? `${status.gpu_fan_rpm} RPM` : '静音'}
        </div>
      </div>

      <div className="m-grid-3">
        <button
          className="m-btn-action"
          disabled={isIgpuLocked}
          style={{
            ...((gpuModeInfo?.current_mode === 'igpu' || gpuModeInfo?.has_dgpu === false) && !isIgpuLocked
              ? {
                  background: 'var(--theme-gpu-dim)',
                  borderColor: 'var(--theme-gpu-border)',
                  color: 'var(--theme-gpu)',
                  boxShadow: '0 0 12px var(--theme-gpu-glow)',
                }
              : {}),
            ...(isIgpuLocked
              ? { opacity: 0.38, cursor: 'not-allowed', filter: 'grayscale(0.6)' }
              : {}),
          }}
          title={
            gpuModeInfo?.has_dgpu === false
              ? '纯核显轻薄本架构，原生运行于核显模式'
              : isIgpuLocked
              ? (gpuModeInfo?.capabilities?.reason_igpu_unsupported || '当前机型为双模架构，无物理核显直连走线，已锁定保护防黑屏')
              : '点击打开高级显卡切换面板'
          }
          onClick={() => {
            if (!isIgpuLocked) onExpandDashboard('gpu');
          }}
        >
          {isIgpuLocked ? <Lock size={14} /> : <Cpu size={14} />}
          <span style={{ fontSize: '10.5px', fontWeight: 600 }}>
            {gpuModeInfo?.has_dgpu === false ? '纯集显 (生效)' : isIgpuLocked ? '双模锁定' : '纯集显'}
          </span>
        </button>

        <button
          className="m-btn-action"
          disabled={gpuModeInfo?.has_dgpu === false}
          style={{
            ...(gpuModeInfo?.current_mode === 'hybrid' && gpuModeInfo?.has_dgpu !== false
              ? {
                  background: 'var(--theme-gpu-dim)',
                  borderColor: 'var(--theme-gpu-border)',
                  color: 'var(--theme-gpu)',
                  boxShadow: '0 0 12px var(--theme-gpu-glow)',
                }
              : {}),
            ...(gpuModeInfo?.has_dgpu === false
              ? { opacity: 0.35, cursor: 'not-allowed' }
              : {}),
          }}
          title={gpuModeInfo?.has_dgpu === false ? '纯核显架构无独立显卡，不可用' : '点击打开高级显卡切换面板'}
          onClick={() => {
            if (gpuModeInfo?.has_dgpu !== false) onExpandDashboard('gpu');
          }}
        >
          <Repeat size={14} />
          <span style={{ fontSize: '10.5px', fontWeight: 600 }}>标准混合</span>
        </button>

        <button
          className="m-btn-action"
          disabled={gpuModeInfo?.has_dgpu === false}
          style={{
            ...(gpuModeInfo?.current_mode === 'dgpu' && gpuModeInfo?.has_dgpu !== false
              ? {
                  background: 'var(--theme-gpu-dim)',
                  borderColor: 'var(--theme-gpu-border)',
                  color: 'var(--theme-gpu)',
                  boxShadow: '0 0 12px var(--theme-gpu-glow)',
                }
              : {}),
            ...(gpuModeInfo?.has_dgpu === false
              ? { opacity: 0.35, cursor: 'not-allowed' }
              : {}),
          }}
          title={gpuModeInfo?.has_dgpu === false ? '纯核显架构无独立显卡，不可用' : '点击打开高级显卡切换面板'}
          onClick={() => {
            if (gpuModeInfo?.has_dgpu !== false) onExpandDashboard('gpu');
          }}
        >
          <Crosshair size={14} />
          <span style={{ fontSize: '10.5px', fontWeight: 600 }}>独显直连</span>
        </button>
      </div>
    </div>
  );
};
