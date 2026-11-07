/**
 * GpuModeView — 显卡模式页编排层（状态管理 + 子组件组装）
 *
 * 本文件职责：
 *   1. 持有所有 IPC 状态（gpuInfo / brightness / tuningState / calibrationState）
 *   2. 编排 invoke / listen 生命周期
 *   3. 将状态与回调注入各子组件插槽
 *
 * 插件化路径（Phase B/C）：
 *   - BrightnessControl / DisplayColorTuning / DisplayColorCalibration 均可作为
 *     独立插槽插拔，主插件仅保留 GpuModeCards（MUX 切换）作为核心能力。
 */
import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { DisplayTuningModal } from './display/DisplayTuningModal';
import { BrightnessControl } from './display/BrightnessControl';
import { DisplayColorTuning } from './display/DisplayColorTuning';
import { DisplayColorCalibration } from './display/DisplayColorCalibration';
import { MiniLedControl } from './display/MiniLedControl';
import { GpuModeCards, GpuModeInfo, PendingSwitch, NoticeState } from './gpu/GpuModeCards';
import { getModeTitle, getModeHexDisplay } from './gpu/gpuUtils';
import { DisplayTuningState, DisplayMonitor } from '../types/hardware';

// ─── Re-exported types (used by child components / plugin API) ────────────────

export interface ColorProfileItem {
  id: string;
  name: string;
  file_name: string;
  is_builtin: boolean;
  description: string;
}

export interface ColorCalibrationState {
  enabled: boolean;
  active_profile_id: string;
  active_profile_name: string;
  profiles: ColorProfileItem[];
  monitor_name: string;
  hdr_active: boolean;
  current_device_name?: string;
  monitors?: DisplayMonitor[];
}

export interface GpuCapabilities {
  supports_dgpu: boolean;
  supports_hybrid: boolean;
  supports_igpu_only: boolean;
  reason_igpu_unsupported?: string | null;
  platform: string;
  is_50_series: boolean;
  is_30_series: boolean;
  baseboard: string;
  gpu_name: string;
  is_safety_locked?: boolean;
  user_unlocked?: boolean;
  supports_hotswitch?: boolean;
  acpi_igpu_active?: boolean;
}

// ─── Component ────────────────────────────────────────────────────────────────

export const GpuModeView: React.FC = () => {
  // ── GPU mode state ─────────────────────────────────────────────────────────
  const [gpuInfo, setGpuInfo] = useState<GpuModeInfo>({
    current_mode: 'dgpu',
    has_igpu_present: true,
    has_dgpu: true,
    gpu_name: '',
    platform: 'intel',
    cpu_name: '',
    hardware_read: false,
    error: null,
  });
  const [loading, setLoading] = useState(false);
  const [rebooting, setRebooting] = useState(false);
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const [pendingSwitch, setPendingSwitch] = useState<PendingSwitch | null>(null);

  const fetchGpuInfo = () => {
    invoke<GpuModeInfo>('get_gpu_mode_info')
      .then((res) => {
        if (res) {
          setGpuInfo(res);
          if (res.error) setNotice({ type: 'warning', text: `硬件读取提示: ${res.error}` });
        }
      })
      .catch((e) => {
        console.error('Failed to get GPU mode info:', e);
        setNotice({ type: 'error', text: `读取显卡硬件状态失败: ${e}` });
      });
  };

  // ── Brightness state ───────────────────────────────────────────────────────
  const [brightness, setBrightness] = useState<number>(70);
  const brightnessTimerRef = useRef<any>(null);
  const isInteractingBrightnessRef = useRef<number>(0);

  const handleBrightnessChange = (val: number) => {
    isInteractingBrightnessRef.current = Date.now();
    setBrightness(val);
    if (brightnessTimerRef.current) clearTimeout(brightnessTimerRef.current);
    brightnessTimerRef.current = setTimeout(() => {
      invoke('set_display_brightness', { brightness: val }).catch((err) =>
        console.warn('Set display brightness error:', err)
      );
    }, 50);
  };

  const handleBrightnessQuick = (val: number) => {
    isInteractingBrightnessRef.current = Date.now();
    setBrightness(val);
    if (brightnessTimerRef.current) clearTimeout(brightnessTimerRef.current);
    invoke('set_display_brightness', { brightness: val }).catch((err) =>
      console.warn('Set display brightness quick error:', err)
    );
  };

  // ── Display tuning state ───────────────────────────────────────────────────
  const [tuningState, setTuningState] = useState<DisplayTuningState | null>(null);
  const [isTuningModalOpen, setIsTuningModalOpen] = useState(false);

  const fetchTuningState = () => {
    invoke<DisplayTuningState>('get_display_tuning_state')
      .then((res) => { if (res) setTuningState(res); })
      .catch((e) => console.warn('Failed to get display tuning state:', e));
  };

  const handleToggleTuning = async () => {
    const nextVal = !tuningState?.enabled;
    setTuningState((prev) =>
      prev
        ? { ...prev, enabled: nextVal }
        : {
            enabled: nextVal,
            active_preset_id: 'preset_vibrant',
            active_preset: {
              id: 'preset_vibrant', name: '多彩鲜艳 (Vibrant)',
              color_temp_k: 6500, gain_r: 1.0, gain_g: 1.0, gain_b: 1.0,
              saturation: 1.5, contrast: 1.0, gamma: 1.0, is_builtin: true,
            },
            presets: [],
          }
    );
    try {
      await invoke('set_display_tuning_enabled', { enable: nextVal });
      fetchTuningState();
    } catch (e) {
      console.error('Failed to toggle display tuning:', e);
      fetchTuningState();
    }
  };

  // ── Color calibration state ────────────────────────────────────────────────
  const [calibrationState, setCalibrationState] = useState<ColorCalibrationState | null>(null);
  const [isCalibrating, setIsCalibrating] = useState(false);
  const lastSelectedIdRef = useRef<string>('std_srgb');
  const activeDeviceNameRef = useRef<string | null | undefined>(null);

  // ── MiniLED Local Dimming state ───────────────────────────────────────────
  const [isMiniLedSupported, setIsMiniLedSupported] = useState<boolean>(false);
  const [miniLedEnabled, setMiniLedEnabled] = useState<boolean>(false);
  const [isMiniLedToggling, setIsMiniLedToggling] = useState<boolean>(false);

  const handleToggleMiniLed = async (nextVal: boolean) => {
    setIsMiniLedToggling(true);
    try {
      const res = await invoke<boolean>('set_miniled_status', { enabled: nextVal });
      setMiniLedEnabled(res);
      setNotice(null);
    } catch (err: any) {
      console.error('Failed to toggle MiniLED:', err);
      setNotice({
        type: 'error',
        text: `切换 MiniLED 分区背光失败: ${err?.message || err}`,
      });
    } finally {
      setIsMiniLedToggling(false);
    }
  };

  const fetchCalibrationState = (targetDevice?: string) => {
    const dev = targetDevice || activeDeviceNameRef.current || calibrationState?.current_device_name;
    invoke<ColorCalibrationState>('get_color_calibration_state', { deviceName: dev || null })
      .then((res) => {
        if (res) {
          setCalibrationState(res);
          activeDeviceNameRef.current = res.current_device_name;
          if (res.active_profile_id && res.active_profile_id !== 'default_native') {
            lastSelectedIdRef.current = res.active_profile_id;
          }
        }
      })
      .catch((e) => console.warn('Failed to get color calibration state:', e));
  };

  const handleSelectMonitor = (deviceName: string) => {
    activeDeviceNameRef.current = deviceName;
    fetchCalibrationState(deviceName);
  };

  const handleSelectCalibration = (profileId: string) => {
    if (profileId !== 'default_native') {
      lastSelectedIdRef.current = profileId;
    }
  };

  const handleApplyCalibration = async (profileId: string) => {
    if (isCalibrating) return;
    setIsCalibrating(true);
    if (profileId !== 'default_native') lastSelectedIdRef.current = profileId;
    const targetDev = activeDeviceNameRef.current || calibrationState?.current_device_name;
    setCalibrationState((prev) => {
      if (!prev) return null;
      const target = prev.profiles.find((p) => p.id === profileId);
      return {
        ...prev,
        active_profile_id: profileId,
        active_profile_name: target ? target.name : prev.active_profile_name,
        enabled: profileId !== 'default_native',
      };
    });
    try {
      const res = await invoke<ColorCalibrationState>('apply_color_profile', {
        profileId,
        deviceName: targetDev || null,
      });
      if (res) {
        setCalibrationState(res);
        activeDeviceNameRef.current = res.current_device_name;
        if (res.active_profile_id && res.active_profile_id !== 'default_native') {
          lastSelectedIdRef.current = res.active_profile_id;
        }
      }
    } catch (e) {
      console.error('Failed to apply color profile:', e);
      fetchCalibrationState(targetDev || undefined);
    } finally {
      setIsCalibrating(false);
    }
  };

  const handleImportCalibrationFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const targetDev = calibrationState?.current_device_name;
    try {
      const arrayBuffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(arrayBuffer));
      const res = await invoke<ColorCalibrationState>('import_custom_color_profile', {
        fileName: file.name,
        fileBytes: bytes,
        deviceName: targetDev || null,
      });
      if (res) setCalibrationState(res);
    } catch (err) {
      console.error('Failed to import ICC file:', err);
    }
  };

  const handleDeleteCalibrationProfile = async () => {
    if (!calibrationState?.active_profile_id) return;
    const currentItem = calibrationState.profiles.find((p) => p.id === calibrationState.active_profile_id);
    if (!currentItem || currentItem.is_builtin) return;
    const targetDev = calibrationState?.current_device_name;
    try {
      const res = await invoke<ColorCalibrationState>('delete_custom_color_profile', {
        profileId: currentItem.id,
        deviceName: targetDev || null,
      });
      if (res) setCalibrationState(res);
    } catch (err) {
      console.error('Failed to delete ICC file:', err);
    }
  };

  // ── GPU mode switch handlers ────────────────────────────────────────────────
  const hasDgpu = gpuInfo.has_dgpu === true && gpuInfo.capabilities?.supports_dgpu !== false;
  const isTriModeSupported = gpuInfo.capabilities ? gpuInfo.capabilities.supports_igpu_only : true;

  const handleCardClick = (targetMode: 'dgpu' | 'hybrid' | 'igpu') => {
    if (loading || rebooting) return;
    if (!hasDgpu) {
      setNotice({
        type: targetMode !== 'igpu' ? 'warning' : 'info',
        text: targetMode !== 'igpu'
          ? `当前机型为纯核显架构轻薄本（未配备独立显卡与 MUX 切换芯片），不支持切换为【${getModeTitle(targetMode)}】。`
          : '当前机型为原生纯核显架构，硬件已永久处于【核显模式】，无需切换。',
      });
      return;
    }
    if (targetMode === 'igpu' && !isTriModeSupported) {
      setNotice({ type: 'warning', text: gpuInfo.capabilities?.reason_igpu_unsupported || '当前机型模具为双显双模硬件架构，主板无物理核显直连开关，已启用安全保护以防止黑屏变砖。' });
      return;
    }
    if (gpuInfo.current_mode === targetMode) {
      setNotice({ type: 'info', text: `当前主板硬件已处于【${getModeTitle(targetMode)}】(${getModeHexDisplay(targetMode, gpuInfo.platform)})，无需重复切换。` });
      return;
    }
    setPendingSwitch({
      targetMode,
      targetTitle: getModeTitle(targetMode),
      targetHex: getModeHexDisplay(targetMode, gpuInfo.platform),
      currentTitle: getModeTitle(gpuInfo.current_mode),
      currentHex: getModeHexDisplay(gpuInfo.current_mode, gpuInfo.platform),
    });
  };

  const handleConfirmSwitch = async (restartImmediately: boolean) => {
    if (!pendingSwitch) return;
    const { targetMode: target, targetTitle: title } = pendingSwitch;
    setLoading(true);
    setNotice(null);
    if (restartImmediately) setRebooting(true);
    try {
      const res = await invoke<GpuModeInfo>('set_gpu_mode', { mode: target, restart: restartImmediately });
      if (res) setGpuInfo(res);
      setPendingSwitch(null);
      setNotice({
        type: 'success',
        text: restartImmediately
          ? `硬件写入成功！正在平稳安全重启电脑以生效【${title}】...`
          : `主板 UEFI NVRAM 已安全写入【${title}】配置，将在下次电脑重启/开机时由 BIOS 硬件生效。`,
      });
    } catch (e: any) {
      console.error('Failed to set GPU mode:', e);
      setNotice({ type: 'error', text: `显卡模式切换异常: ${e?.message || e} (请确认已通过管理员权限运行 OpenRevo)` });
      setRebooting(false);
      setPendingSwitch(null);
    } finally {
      setLoading(false);
    }
  };

  const handleUnlockGeekIgpu = async (unlock: boolean) => {
    try {
      setLoading(true);
      const res = await invoke<GpuModeInfo>('set_experimental_igpu_unlock', { unlock });
      if (res) {
        setGpuInfo(res);
        setNotice({
          type: unlock ? 'warning' : 'info',
          text: unlock
            ? '已启用极客自主免责解锁！若电脑在切换核显模式后出现黑屏，请长按电源键强制关机并拔掉电源复位 BIOS。'
            : '已恢复双显双模硬件安全保护锁定。',
        });
      }
    } catch (err: any) {
      setNotice({ type: 'error', text: `设置极客解锁失败: ${err?.message || err}` });
    } finally {
      setLoading(false);
    }
  };

  // ── Lifecycle ──────────────────────────────────────────────────────────────
  useEffect(() => {
    fetchGpuInfo();
    fetchTuningState();
    fetchCalibrationState();

    invoke<boolean>('get_miniled_support')
      .then((supported) => {
        setIsMiniLedSupported(supported);
        if (supported) {
          invoke<boolean>('get_miniled_status')
            .then(setMiniLedEnabled)
            .catch(() => {});
        }
      })
      .catch((e) => console.warn('Failed to query miniled support:', e));

    const fetchBrightness = () => {
      if (Date.now() - isInteractingBrightnessRef.current < 1500) return;
      invoke<number>('get_display_brightness')
        .then((val) => { if (typeof val === 'number' && val >= 0 && val <= 100) setBrightness(val); })
        .catch(() => {});
    };
    const handleFocusSync = () => {
      fetchBrightness();
      fetchCalibrationState(activeDeviceNameRef.current || undefined);
    };
    fetchBrightness();
    window.addEventListener('focus', handleFocusSync);
    const brightnessInterval = setInterval(fetchBrightness, 3000);

    let unlistenBrightness: (() => void) | undefined;
    listen<number>('display-brightness-changed', (event) => {
      if (typeof event.payload === 'number' && Date.now() - isInteractingBrightnessRef.current > 600) {
        setBrightness(event.payload);
      }
    }).then((fn) => { unlistenBrightness = fn; });

    let unlistenColor: (() => void) | undefined;
    listen('color-calibration-changed', () => {
      fetchCalibrationState(activeDeviceNameRef.current || undefined);
    }).then((fn) => { unlistenColor = fn; });

    return () => {
      window.removeEventListener('focus', handleFocusSync);
      clearInterval(brightnessInterval);
      if (unlistenBrightness) unlistenBrightness();
      if (unlistenColor) unlistenColor();
    };
  }, []);

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
      {/* Slot 1: GPU mode selector + confirmation modal */}
      <GpuModeCards
        gpuInfo={gpuInfo}
        loading={loading}
        rebooting={rebooting}
        notice={notice}
        pendingSwitch={pendingSwitch}
        onCardClick={handleCardClick}
        onConfirmSwitch={handleConfirmSwitch}
        onCancelSwitch={() => setPendingSwitch(null)}
        onDismissNotice={() => setNotice(null)}
        onUnlockGeekIgpu={handleUnlockGeekIgpu}
      />

      {/* Slot 2: Screen brightness control */}
      <BrightnessControl
        brightness={brightness}
        onChange={handleBrightnessChange}
        onQuick={handleBrightnessQuick}
      />

      {/* Slot 3: Display color tuning (Display Tuning Engine) */}
      <DisplayColorTuning
        tuningState={tuningState}
        onToggle={handleToggleTuning}
        onOpenModal={() => setIsTuningModalOpen(true)}
      />

      {/* Slot 4: ICC color calibration */}
      <DisplayColorCalibration
        calibrationState={calibrationState}
        isCalibrating={isCalibrating}
        onSelect={handleSelectCalibration}
        onApply={handleApplyCalibration}
        onImport={handleImportCalibrationFile}
        onDelete={handleDeleteCalibrationProfile}
        onSelectMonitor={handleSelectMonitor}
      />

      {/* Slot 5: MiniLED Local Dimming (dynamically displayed when hardware supported) */}
      {isMiniLedSupported && (
        <MiniLedControl
          enabled={miniLedEnabled}
          isToggling={isMiniLedToggling}
          onToggle={handleToggleMiniLed}
        />
      )}

      {/* Display Tuning overlay modal */}
      {isTuningModalOpen && (
        <DisplayTuningModal
          onClose={() => setIsTuningModalOpen(false)}
          onUpdated={fetchTuningState}
        />
      )}
    </div>
  );
};
