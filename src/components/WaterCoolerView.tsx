import { subscribeVisiblePolling } from './mainVisibility';
import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertTriangle, Info, Check, X } from 'lucide-react';
import { HardwareState } from '../types/hardware';
import {
  CoolerTelemetry,
  CoolerCurvePoint,
  WaterCoolerDevice,
  DEFAULT_COOLER_CURVE,
} from './water_cooler/coolerTypes';
import { CoolerHeaderTelemetry } from './water_cooler/CoolerHeaderTelemetry';
import { CoolerPresetCards } from './water_cooler/CoolerPresetCards';
import { CoolerCurvePanel } from './water_cooler/CoolerCurvePanel';
import { CoolerManualRgbPanel } from './water_cooler/CoolerManualRgbPanel';
import { CoolerDeviceSelectorModal } from './water_cooler/CoolerDeviceSelectorModal';

export type { CoolerTelemetry, CoolerCurvePoint, WaterCoolerDevice };

export interface WaterCoolerViewProps {
  status: HardwareState;
  onConnectionChange?: (connected: boolean) => void;
}

export const WaterCoolerView: React.FC<WaterCoolerViewProps> = ({
  status,
  onConnectionChange,
}) => {
  const [coolerData, setCoolerData] = useState<CoolerTelemetry>({
    connected: false,
    is_flow_ok: true,
    meter_state: 1,
    pump_voltage: 7,
    pump_tier: 2,
    fan_duty: 40,
    pump_rpm: 0,
    running: false,
    safety_tripped: false,
    is_priming: false,
    led_on: true,
    led_r: 0,
    led_g: 128,
    led_b: 0,
    led_mode: 2,
    strategy: 'smart',
    curve_points: DEFAULT_COOLER_CURVE,
    pump_start_temp: 55,
    pump_turbo_temp: 90,
  });

  const [curveTemplate, setCurveTemplate] = useState<'standard' | 'quiet' | 'aggressive' | 'linear'>('standard');
  const [isCoolerConnecting, setIsCoolerConnecting] = useState(false);
  const [isPairing, setIsPairing] = useState(false);
  const [isDeviceSelectorOpen, setIsDeviceSelectorOpen] = useState(false);
  const [pairMsg, setPairMsg] = useState<string | null>(null);
  const isDraggingPumpRef = useRef(false);
  const strategyDebounceTimer = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (strategyDebounceTimer.current) {
        window.clearTimeout(strategyDebounceTimer.current);
      }
    };
  }, []);

  // Active polling while mounted
  useEffect(() => {
    return subscribeVisiblePolling(() => invoke<CoolerTelemetry>('get_water_cooler_status'), (res) => {
      if (!res) return;
      setCoolerData((prev) => ({
        ...res,
        pump_start_temp: isDraggingPumpRef.current ? prev.pump_start_temp : (res.pump_start_temp ?? prev.pump_start_temp),
        pump_turbo_temp: isDraggingPumpRef.current ? prev.pump_turbo_temp : (res.pump_turbo_temp ?? prev.pump_turbo_temp),
      }));
      onConnectionChange?.(res.connected);
    }, 1500);
  }, [onConnectionChange]);

  const handleUpdateCooler = async (pumpV?: number, fanD?: number) => {
    try {
      const nextV = pumpV !== undefined ? pumpV : (coolerData.pump_tier ?? coolerData.pump_voltage);
      const nextD = fanD !== undefined ? fanD : coolerData.fan_duty;

      console.warn('[COOLER_UI] Click -> Requested pumpV:', pumpV, 'effective nextV:', nextV, 'fanD:', nextD);

      // 1. Instant 0ms Optimistic UI update: pump_tier MUST be updated together so buttons follow immediately!
      setCoolerData((prev) => ({
        ...prev,
        strategy: 'custom',
        pump_voltage: nextV,
        pump_tier: nextV,
        fan_duty: nextD,
      }));

      // 2. Transmit to backend with dual-key casing (pumpV & pump_v, fanD & fan_d) for 100% deserialization compatibility
      const res = await invoke<CoolerTelemetry>('set_water_cooler_speed', {
        pumpV: nextV,
        pump_v: nextV,
        fanD: nextD,
        fan_d: nextD,
      });
      console.warn('[COOLER_UI] IPC Returned telemetry -> tier:', res?.pump_tier, 'volt:', res?.pump_voltage);
      if (res) {
        setCoolerData(res);
      }
    } catch (e) {
      console.error('Failed to set cooler speed:', e);
    }
  };

  const handleUpdateCoolerStrategy = (strategy: string, curve?: CoolerCurvePoint[]) => {
    const targetTier = strategy === 'performance' ? 3 : (strategy === 'quiet' || strategy === 'balance' ? 2 : coolerData.pump_tier ?? 2);
    const targetFan = strategy === 'performance' ? 90 : (strategy === 'balance' ? 65 : (strategy === 'quiet' ? 40 : coolerData.fan_duty));

    // 1. Immediate optimistic UI feedback for instantaneous responsiveness
    setCoolerData((prev) => ({
      ...prev,
      strategy,
      pump_tier: strategy === 'smart' ? prev.pump_tier : targetTier,
      pump_voltage: strategy === 'smart' ? prev.pump_voltage : targetTier,
      fan_duty: strategy === 'smart' ? prev.fan_duty : targetFan,
      curve_points: curve || prev.curve_points,
    }));

    // 2. Debounce IPC to absorb rapid user clicks and protect water cooler MCU
    if (strategyDebounceTimer.current) {
      window.clearTimeout(strategyDebounceTimer.current);
    }
    strategyDebounceTimer.current = window.setTimeout(async () => {
      try {
        const res = await invoke<CoolerTelemetry>('set_water_cooler_strategy', {
          strategy,
          curve: curve || null,
        });
        if (res) {
          setCoolerData(res);
        }
      } catch (e) {
        console.error('Failed to set cooler strategy:', e);
      }
    }, 120);
  };

  const handleConnectCooler = async () => {
    setIsCoolerConnecting(true);
    try {
      const isCustom = coolerData.strategy === 'custom';
      const effectivePump = coolerData.pump_tier ?? coolerData.pump_voltage;
      const res = await invoke<CoolerTelemetry>('start_water_cooler', {
        pumpV: isCustom ? effectivePump : null,
        pump_v: isCustom ? effectivePump : null,
        fanD: isCustom ? coolerData.fan_duty : null,
        fan_d: isCustom ? coolerData.fan_duty : null,
      });
      if (res) {
        setCoolerData(res);
      }
      setTimeout(async () => {
        try {
          const fresh = await invoke<CoolerTelemetry>('get_water_cooler_status');
          if (fresh) setCoolerData(fresh);
        } catch (_) {}
      }, 1200);
    } catch (e) {
      console.error('Failed to connect cooler:', e);
      setPairMsg('连接水冷箱超时，请确认水冷魔盒通电并在蓝牙覆盖范围内');
      setTimeout(() => setPairMsg(null), 4000);
    } finally {
      setTimeout(() => setIsCoolerConnecting(false), 800);
    }
  };

  const handleUpdateCoolerLed = async (on?: boolean, r?: number, g?: number, b?: number, mode?: number) => {
    try {
      const nextOn = on !== undefined ? on : coolerData.led_on;
      let nextR = r !== undefined ? r : coolerData.led_r;
      let nextG = g !== undefined ? g : coolerData.led_g;
      let nextB = b !== undefined ? b : coolerData.led_b;
      let nextMode = mode !== undefined ? mode : coolerData.led_mode;
      if (nextMode === 0) nextMode = 2; // Default to Mode 2 Colorful Flow
      if (nextOn && nextR === 0 && nextG === 0 && nextB === 0) {
        // Fallback to official Emerald Green baseline
        nextR = 0;
        nextG = 128;
        nextB = 0;
      }

      setCoolerData((prev) => ({
        ...prev,
        led_on: nextOn,
        led_r: nextR,
        led_g: nextG,
        led_b: nextB,
        led_mode: nextMode,
      }));

      const res = await invoke<CoolerTelemetry>('set_water_cooler_led', {
        on: nextOn,
        r: nextR,
        g: nextG,
        b: nextB,
        mode: nextMode,
      });
      if (res) {
        setCoolerData(res);
      }
    } catch (e) {
      console.error('Failed to set cooler LED:', e);
    }
  };

  const handleFillWater = async () => {
    try {
      await invoke('fill_water_cooler');
      setCoolerData((prev) => ({ ...prev, is_priming: true }));
      setPairMsg('已启动注水排气流程（水泵将脉冲启停 8 次排除管路气泡）');
      setTimeout(() => setPairMsg(null), 4000);
    } catch (e) {
      console.error('Failed to start fill water:', e);
    }
  };

  const handleDisconnectCooler = async () => {
    setIsCoolerConnecting(true);
    try {
      const res = await invoke<CoolerTelemetry>('stop_water_cooler');
      if (res) {
        setCoolerData(res);
      }
      setPairMsg('已安全断开水冷箱连接，水泵与冷排已停机');
      setTimeout(() => setPairMsg(null), 3500);
    } catch (e) {
      console.error('Failed to disconnect cooler:', e);
    } finally {
      setIsCoolerConnecting(false);
    }
  };

  const handleResetCooler = async () => {
    try {
      const res = await invoke<CoolerTelemetry>('reset_water_cooler');
      if (res) {
        setCoolerData(res);
        setPairMsg('已恢复出厂策略：水泵标准巡航 / 冷排 75% / 幻彩流光 RGB');
        setTimeout(() => setPairMsg(null), 3500);
      }
    } catch (e) {
      console.error('Failed to reset cooler:', e);
    }
  };

  const handlePairCooler = async () => {
    setIsPairing(true);
    setPairMsg(null);
    try {
      const res = await invoke<CoolerTelemetry>('pair_water_cooler');
      if (res) {
        setCoolerData(res);
        setPairMsg(`配对成功！已连接水冷箱: ${res.mac_address || '已更新'}`);
        setTimeout(() => setPairMsg(null), 4000);
      }
    } catch (e: any) {
      console.error('Failed to pair cooler:', e);
      setPairMsg(`配对失败: ${typeof e === 'string' ? e : '未找到水冷箱，请确认其通电并在附近'}`);
      setTimeout(() => setPairMsg(null), 5000);
    } finally {
      setIsPairing(false);
    }
  };

  const handleSelectDevice = async (device: WaterCoolerDevice) => {
    try {
      const res = await invoke<CoolerTelemetry>('select_water_cooler', { mac: device.mac_address });
      if (res) {
        setCoolerData(res);
      }
      setPairMsg(`已绑定专属水冷箱: ${device.name} (${device.mac_address})`);
      setTimeout(() => setPairMsg(null), 4000);
    } catch (e: any) {
      console.error('Failed to select water cooler device:', e);
      throw e;
    }
  };

  const handleUpdatePumpThresholds = async (startTemp: number, turboTemp: number, committed: boolean) => {
    if (!committed) {
      isDraggingPumpRef.current = true;
      return;
    }

    isDraggingPumpRef.current = false;
    setCoolerData((prev) => ({
      ...prev,
      pump_start_temp: startTemp,
      pump_turbo_temp: turboTemp,
    }));

    try {
      const res = await invoke<CoolerTelemetry>('set_water_cooler_pump_thresholds', {
        startTemp,
        turboTemp,
      });
      if (res) {
        setCoolerData(res);
      }
    } catch (e) {
      console.error('Failed to set cooler pump thresholds:', e);
    }
  };

  const activeCurve = coolerData.curve_points && coolerData.curve_points.length >= 2
    ? coolerData.curve_points
    : DEFAULT_COOLER_CURVE;

  const liveMaxTemp = Math.max(status.cpu_temp || 0, status.gpu_temp || 0) || 45;

  const getInterpolatedCooler = (pts: CoolerCurvePoint[], t: number) => {
    if (!pts || pts.length === 0) return { duty: 0, volt: 7 };
    const sorted = [...pts].sort((a, b) => a.temp - b.temp);
    if (t <= sorted[0].temp) return { duty: sorted[0].fan_duty, volt: sorted[0].pump_voltage };
    for (let i = 0; i < sorted.length - 1; i++) {
      const p1 = sorted[i];
      const p2 = sorted[i + 1];
      if (t >= p1.temp && t <= p2.temp) {
        const range = p2.temp - p1.temp || 1;
        const ratio = (t - p1.temp) / range;
        const duty = Math.round(p1.fan_duty + ratio * (p2.fan_duty - p1.fan_duty));
        const volt = ratio >= 0.5 ? p2.pump_voltage : p1.pump_voltage;
        return { duty: Math.max(0, Math.min(100, duty)), volt };
      }
    }
    const last = sorted[sorted.length - 1];
    return { duty: last.fan_duty, volt: last.pump_voltage };
  };

  const liveInterpolated = getInterpolatedCooler(activeCurve, liveMaxTemp);

  const applyCoolerTemplate = (type: 'standard' | 'quiet' | 'aggressive' | 'linear') => {
    setCurveTemplate(type);
    let pts: CoolerCurvePoint[] = [];
    if (type === 'standard') {
      pts = [
        { temp: 40, fan_duty: 0, pump_voltage: 0 },
        { temp: 55, fan_duty: 0, pump_voltage: 2 },
        { temp: 68, fan_duty: 45, pump_voltage: 2 },
        { temp: 78, fan_duty: 70, pump_voltage: 2 },
        { temp: 88, fan_duty: 100, pump_voltage: 3 },
      ];
    } else if (type === 'quiet') {
      pts = [
        { temp: 40, fan_duty: 0, pump_voltage: 0 },
        { temp: 65, fan_duty: 0, pump_voltage: 2 },
        { temp: 75, fan_duty: 40, pump_voltage: 2 },
        { temp: 84, fan_duty: 70, pump_voltage: 2 },
        { temp: 92, fan_duty: 100, pump_voltage: 3 },
      ];
    } else if (type === 'aggressive') {
      pts = [
        { temp: 30, fan_duty: 0, pump_voltage: 0 },
        { temp: 45, fan_duty: 30, pump_voltage: 2 },
        { temp: 60, fan_duty: 60, pump_voltage: 2 },
        { temp: 72, fan_duty: 85, pump_voltage: 3 },
        { temp: 82, fan_duty: 100, pump_voltage: 3 },
      ];
    } else if (type === 'linear') {
      pts = [
        { temp: 35, fan_duty: 0, pump_voltage: 0 },
        { temp: 50, fan_duty: 25, pump_voltage: 2 },
        { temp: 65, fan_duty: 50, pump_voltage: 2 },
        { temp: 80, fan_duty: 75, pump_voltage: 2 },
        { temp: 95, fan_duty: 100, pump_voltage: 3 },
      ];
    }
    handleUpdateCoolerStrategy('smart', pts);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '7px', height: '100%' }}>
      {/* 浮动 Toast 气泡 */}
      {pairMsg && (
        <div
          style={{
            position: 'absolute',
            top: '12px',
            right: '16px',
            zIndex: 30,
            background: pairMsg.includes('失败')
              ? 'rgba(159, 18, 57, 0.95)'
              : pairMsg.includes('断开')
                ? 'var(--surface-overlay)'
                : 'rgba(6, 78, 59, 0.95)',
            border: `1px solid ${pairMsg.includes('失败') ? 'var(--status-error)' : pairMsg.includes('断开') ? 'var(--border-medium)' : 'var(--status-ok)'}`,
            color: 'var(--text-primary)',
            padding: '8px 14px',
            borderRadius: 'var(--radius-md)',
            fontSize: '11px',
            fontWeight: 600,
            boxShadow: '0 8px 24px rgba(0, 0, 0, 0.55)',
            backdropFilter: 'blur(10px)',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            maxWidth: '420px',
          }}
        >
          <span style={{ display: 'flex', alignItems: 'center' }}>
            {pairMsg.includes('失败') ? (
              <AlertTriangle size={14} className="text-rose-400" />
            ) : pairMsg.includes('断开') ? (
              <Info size={14} className="text-slate-300" />
            ) : (
              <Check size={14} className="text-emerald-400" />
            )}
          </span>
          <span style={{ flex: 1, lineHeight: 1.4 }}>{pairMsg}</span>
          <button
            type="button"
            onClick={() => setPairMsg(null)}
            style={{ background: 'transparent', border: 'none', color: 'var(--text-primary)', cursor: 'pointer', display: 'flex', alignItems: 'center', padding: '0 2px' }}
          >
            <X size={13} />
          </button>
        </div>
      )}

      {/* 水冷统一大卡片 */}
      <div
        className="glass-card"
        style={{
          padding: '9px 12px',
          display: 'flex',
          flexDirection: 'column',
          gap: '7px',
          height: '100%',
          justifyContent: 'space-between',
          borderRadius: 'var(--radius-lg)',
        }}
      >
        {/* 1. 顶部连接状态与 4 核心遥测指标卡片 */}
        <CoolerHeaderTelemetry
          coolerData={coolerData}
          isCoolerConnecting={isCoolerConnecting}
          isPairing={isPairing}
          liveMaxTemp={liveMaxTemp}
          cpuTemp={status.cpu_temp}
          gpuTemp={status.gpu_temp}
          onConnectCooler={handleConnectCooler}
          onPairCooler={handlePairCooler}
          onDisconnectCooler={handleDisconnectCooler}
          onFillWater={handleFillWater}
          onResetToDefault={async () => {
            applyCoolerTemplate('standard');
            await handleResetCooler();
          }}
          onOpenDeviceSelector={() => setIsDeviceSelectorOpen(true)}
        />

        {/* 2. 散热预设策略卡片栏 (智慧自适应 / 静音 / 均衡 / 极速狂暴) */}
        <CoolerPresetCards
          strategy={coolerData.strategy}
          liveMaxTemp={liveMaxTemp}
          liveInterpolated={liveInterpolated}
          onUpdateStrategy={(strat) => handleUpdateCoolerStrategy(strat)}
        />

        {/* 3. 核心温控曲线面板 (SVG 画布、节点拖拽与曲线模板) */}
        <CoolerCurvePanel
          strategy={coolerData.strategy}
          activeCurve={activeCurve}
          liveMaxTemp={liveMaxTemp}
          liveInterpolated={liveInterpolated}
          curveTemplate={curveTemplate}
          pumpStartTemp={coolerData.pump_start_temp ?? 55}
          pumpTurboTemp={coolerData.pump_turbo_temp ?? 90}
          onApplyTemplate={applyCoolerTemplate}
          onCurveChange={(newPts) => {
            setCoolerData((prev) => ({ ...prev, curve_points: newPts }));
            handleUpdateCoolerStrategy('smart', newPts);
          }}
          onPumpThresholdsChange={handleUpdatePumpThresholds}
        />

        {/* 4. 手动精调 (水泵电压 / 冷排风扇转速) & 水箱 RGB 控制 */}
        <CoolerManualRgbPanel
          coolerData={coolerData}
          onUpdateCooler={handleUpdateCooler}
          onUpdateCoolerLed={handleUpdateCoolerLed}
        />
      </div>

      {/* 5. 专属水冷箱设备选择与防串连绑定弹窗 */}
      <CoolerDeviceSelectorModal
        isOpen={isDeviceSelectorOpen}
        onClose={() => setIsDeviceSelectorOpen(false)}
        currentMac={coolerData.mac_address}
        onSelectDevice={handleSelectDevice}
      />
    </div>
  );
};
