import React from 'react';
import {
  Droplets,
  RotateCcw,
  Bluetooth,
  Loader2,
  Zap,
} from 'lucide-react';
import { CoolerTelemetry } from './coolerTypes';
import { trText, useI18n } from '../../i18n/I18nContext';

interface CoolerHeaderTelemetryProps {
  coolerData: CoolerTelemetry;
  isCoolerConnecting: boolean;
  isPairing: boolean;
  liveMaxTemp: number;
  cpuTemp: number;
  gpuTemp: number;
  onConnectCooler: () => void;
  onPairCooler: () => void;
  onDisconnectCooler: () => void;
  onFillWater: () => void;
  onResetToDefault: () => void;
  onOpenDeviceSelector?: () => void;
}

export const CoolerHeaderTelemetry: React.FC<CoolerHeaderTelemetryProps> = ({
  coolerData,
  isCoolerConnecting,
  isPairing,
  liveMaxTemp,
  cpuTemp,
  gpuTemp,
  onConnectCooler,
  onPairCooler,
  onDisconnectCooler,
  onFillWater,
  onResetToDefault,
  onOpenDeviceSelector,
}) => {
  useI18n();
  return (
    <>
      {/* 1. 顶部连接状态与操作按钮组 */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexShrink: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontSize: '14px', fontWeight: 700, color: 'var(--text-primary)' }}>{trText('外置水冷魔盒')}</span>
          <span
            className="cooler-ble-badge"
            style={{
              fontSize: '9px',
              color: 'var(--status-info)',
              background: 'var(--status-info-dim)',
              border: '1px solid rgba(56, 189, 248, 0.3)',
              padding: '1px 6px',
              borderRadius: 'var(--radius-xs)',
              fontWeight: 600,
            }}
          >
            BLE 原生直连
          </span>
          {coolerData.mac_address ? (
            <button
              type="button"
              onClick={onOpenDeviceSelector}
              className="cooler-mac-badge-btn"
              title="点击打开水冷魔盒列表，绑定专属水冷箱以防止邻机串连"
              style={{
                fontSize: '9px',
                color: 'var(--text-secondary)',
                background: 'var(--surface-badge)',
                border: '1px solid var(--border-subtle)',
                padding: '2px 7px',
                borderRadius: 'var(--radius-xs)',
                fontFamily: 'var(--font-mono)',
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                cursor: 'pointer',
                transition: 'all 0.15s ease',
              }}
            >
              <span>MAC: {coolerData.mac_address}</span>
              <Bluetooth size={10} style={{ color: 'var(--status-info)' }} />
            </button>
          ) : (
            <button
              type="button"
              onClick={onOpenDeviceSelector}
              className="cooler-mac-badge-btn"
              title="点击搜索并选择专属水冷箱"
              style={{
                fontSize: '9px',
                color: 'var(--status-info)',
                background: 'var(--status-info-dim)',
                border: '1px dashed rgba(56, 189, 248, 0.4)',
                padding: '2px 7px',
                borderRadius: 'var(--radius-xs)',
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                cursor: 'pointer',
              }}
            >
              <Bluetooth size={10} />
              <span>{trText('选择水冷魔盒')}</span>
            </button>
          )}
        </div>

        <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
          <div
            className={`cooler-status-badge ${coolerData.connected ? 'connected' : ''}`}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              padding: '3px 8px',
              borderRadius: 'var(--radius-xs)',
              background: coolerData.connected ? 'var(--status-ok-dim)' : 'var(--surface-badge)',
              border: `1px solid ${coolerData.connected ? 'rgba(16, 185, 129, 0.35)' : 'var(--border-subtle)'}`,
              fontSize: '10px',
              fontWeight: 700,
              color: coolerData.connected ? 'var(--status-ok)' : 'var(--text-muted)',
            }}
          >
            <span
              className="cooler-status-dot"
              style={{
                width: '6px',
                height: '6px',
                borderRadius: 'var(--radius-full)',
                background: coolerData.connected ? 'var(--status-ok)' : 'var(--text-dim)',
                boxShadow: coolerData.connected ? '0 0 6px var(--status-ok)' : 'none',
              }}
            />
            <span>{coolerData.connected ? '已直连受控' : trText('未连接')}</span>
          </div>

          {coolerData.connected ? (
            <>
              <button
                type="button"
                onClick={onFillWater}
                className="cooler-prime-btn"
                title="脉冲启停水泵 8 次，排除水冷管道中的微小气泡"
                style={{
                  background: coolerData.is_priming ? 'var(--color-beast)' : 'var(--surface-pill)',
                  border: '1px solid var(--border-medium)',
                  color: coolerData.is_priming ? '#fff' : 'var(--text-secondary)',
                  padding: '3.5px 9px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '10px',
                  fontWeight: 600,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  transition: 'all var(--transition-fast)',
                }}
              >
                <Droplets className="w-3 h-3" />
                <span>{coolerData.is_priming ? '排气注水中...' : trText('注水排气')}</span>
              </button>
              <button
                type="button"
                onClick={onResetToDefault}
                className="cooler-curve-default-btn"
                title="恢复出厂推荐的智慧自适应温控曲线与出厂策略"
                style={{
                  background: 'var(--surface-pill)',
                  border: '1px solid var(--border-medium)',
                  color: 'var(--text-secondary)',
                  padding: '3.5px 9px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '10px',
                  fontWeight: 600,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  transition: 'all var(--transition-fast)',
                }}
              >
                <RotateCcw className="w-3 h-3" />
                <span>{trText('默认曲线')}</span>
              </button>
              <button
                type="button"
                onClick={onDisconnectCooler}
                className="cooler-disconnect-btn"
                title="安全停机水泵与冷排风扇，并断开蓝牙直连"
                style={{
                  background: 'var(--status-error-dim)',
                  border: '1px solid rgba(239, 68, 68, 0.28)',
                  color: 'var(--status-error)',
                  padding: '3.5px 9px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '10px',
                  fontWeight: 600,
                  cursor: 'pointer',
                  transition: 'all var(--transition-fast)',
                }}
              >
                <span>{trText('断开水冷')}</span>
              </button>
            </>
          ) : (
            <>
              <button
                type="button"
                onClick={onConnectCooler}
                disabled={isCoolerConnecting || isPairing}
                className="cooler-connect-btn"
                style={{
                  background: 'var(--status-info)',
                  border: '1px solid var(--status-info)',
                  color: '#ffffff',
                  padding: '4px 10px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '10px',
                  fontWeight: 700,
                  cursor: isCoolerConnecting ? 'wait' : 'pointer',
                  transition: 'all var(--transition-fast)',
                }}
              >
                <span style={{ display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
                  {isCoolerConnecting ? <Loader2 size={11} className="animate-spin" /> : <Zap size={11} />}
                  {isCoolerConnecting ? '正在直连...' : trText('立即直连')}
                </span>
              </button>
              <button
                type="button"
                onClick={onPairCooler}
                disabled={isPairing || isCoolerConnecting}
                className="cooler-pair-btn"
                style={{
                  background: 'var(--surface-pill)',
                  border: '1px solid var(--border-medium)',
                  color: 'var(--text-secondary)',
                  padding: '4px 10px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '10px',
                  fontWeight: 600,
                  cursor: isPairing ? 'wait' : 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  transition: 'all var(--transition-fast)',
                }}
              >
                <Bluetooth className="w-3 h-3" />
                <span>{isPairing ? '⟳ 扫描配对中...' : trText('搜索配对')}</span>
              </button>
            </>
          )}
        </div>
      </div>

      {/* 2. 4 核心遥测指标卡片 */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '7px', flexShrink: 0 }}>
        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>{trText('当前水流状态')}</span>
          <div
            className="metric-value"
            style={{
              fontSize: '13px',
              fontWeight: 800,
              color: coolerData.connected
                ? (coolerData.is_flow_ok ? 'var(--status-ok)' : 'var(--status-warn)')
                : 'var(--text-muted)',
            }}
          >
            {coolerData.connected
              ? (coolerData.is_flow_ok || coolerData.meter_state === 2 || coolerData.meter_state === 1 ? '正常 (Flowing)' : coolerData.meter_state === 3 ? '微流/建立中' : trText('无流量'))
              : '--'}
          </div>
        </div>

        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid rgba(56, 189, 248, 0.25)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>{trText('核心综合温度 max(CPU, GPU)')}</span>
          <div className="metric-value" style={{ fontSize: '13px', fontWeight: 800, color: 'var(--status-info)' }}>
            {liveMaxTemp} °C
            <span style={{ fontSize: '9px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '4px' }}>
              (CPU {cpuTemp}° / GPU {gpuTemp}°)
            </span>
          </div>
        </div>

        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>{trText('水泵运行工况 / 档位')}</span>
          <div className="metric-value" style={{ fontSize: '13px', fontWeight: 800, color: 'var(--status-info)' }}>
            {coolerData.connected ? (
              (() => {
                const tier = coolerData.pump_tier ?? (coolerData.pump_voltage >= 11 ? 3 : coolerData.pump_voltage >= 7 ? 2 : coolerData.pump_voltage === 0 ? 0 : 2);
                if (tier === 0 || coolerData.pump_voltage === 0) {
                  return <span style={{ color: 'var(--text-muted)' }}>{trText('已停机 (0%)')}</span>;
                }
                if (tier === 3 || coolerData.pump_voltage >= 11) {
                  return <span style={{ color: 'var(--color-beast)' }}>{trText('极限狂暴 (100%)')}</span>;
                }
                if (tier === 1) {
                  return <span style={{ color: 'var(--status-info)' }}>{trText('静音巡航 (35%)')}</span>;
                }
                return <span style={{ color: 'var(--status-info)' }}>{trText('标准巡航 (45%)')}</span>;
              })()
            ) : '--'}
          </div>
        </div>

        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>{trText('冷排风扇实时转速')}</span>
          <div className="metric-value" style={{ fontSize: '13px', fontWeight: 800, color: 'var(--color-brand)', display: 'flex', alignItems: 'center', gap: '5px' }}>
            <span>{coolerData.connected ? `${coolerData.fan_duty}%` : '--'}</span>
            {coolerData.connected && (
              <span
                style={{
                  fontSize: '9px',
                  fontWeight: 700,
                  padding: '1px 4px',
                  borderRadius: 'var(--radius-xs)',
                  background: coolerData.fan_duty === 0 ? 'var(--status-ok-dim)' : 'var(--border-glow)',
                  color: coolerData.fan_duty === 0 ? 'var(--status-ok)' : 'var(--color-brand)',
                }}
              >
                {coolerData.fan_duty === 0 ? '0 RPM 停转 (静音)' : `${Math.round(coolerData.fan_duty * 24)} RPM`}
              </span>
            )}
          </div>
        </div>
      </div>
    </>
  );
};
