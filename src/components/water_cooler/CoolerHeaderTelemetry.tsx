import React from 'react';
import {
  Droplets,
  RotateCcw,
  Bluetooth,
  Loader2,
  Zap,
} from 'lucide-react';
import { CoolerTelemetry } from './coolerTypes';

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
  return (
    <>
      {/* 1. 顶部连接状态与操作按钮组 */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexShrink: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontSize: '14px', fontWeight: 700, color: 'var(--text-primary)' }}>Внешний блок СЖО</span>
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
            Прямое нативное соединение BLE
          </span>
          {coolerData.mac_address ? (
            <button
              type="button"
              onClick={onOpenDeviceSelector}
              className="cooler-mac-badge-btn"
              title="Нажмите, чтобы открыть список блоков СЖО и привязать свой, исключив подключение к соседнему устройству"
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
              title="Нажмите, чтобы найти и выбрать свой блок СЖО"
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
              <span>Выбрать блок СЖО</span>
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
            <span>{coolerData.connected ? 'Подключено напрямую, под управлением' : 'Не подключено'}</span>
          </div>

          {coolerData.connected ? (
            <>
              <button
                type="button"
                onClick={onFillWater}
                className="cooler-prime-btn"
                title="8 импульсных пусков/остановок насоса для удаления микропузырей из контура СЖО"
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
                <span>{coolerData.is_priming ? 'Идёт заправка и удаление воздуха...' : 'Заправка и удаление воздуха'}</span>
              </button>
              <button
                type="button"
                onClick={onResetToDefault}
                className="cooler-curve-default-btn"
                title="Восстановить рекомендованную заводом умную адаптивную кривую охлаждения и заводские параметры"
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
                <span>Кривая по умолчанию</span>
              </button>
              <button
                type="button"
                onClick={onDisconnectCooler}
                className="cooler-disconnect-btn"
                title="Безопасно останавливает насос и вентилятор радиатора, разрывая Bluetooth-соединение"
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
                <span>Отключить СЖО</span>
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
                  {isCoolerConnecting ? 'Прямое подключение...' : 'Подключить сразу'}
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
                <span>{isPairing ? '⟳ Сканирование и сопряжение...' : 'Поиск и сопряжение'}</span>
              </button>
            </>
          )}
        </div>
      </div>

      {/* 2. 4 核心遥测指标卡片 */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '7px', flexShrink: 0 }}>
        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>Текущее состояние потока</span>
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
              ? (coolerData.is_flow_ok || coolerData.meter_state === 2 || coolerData.meter_state === 1 ? 'Обычный (Flowing)' : coolerData.meter_state === 3 ? 'Слабый поток / устанавливается' : 'Нет потока')
              : '--'}
          </div>
        </div>

        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid rgba(56, 189, 248, 0.25)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>Сводная температура ядра max(CPU, GPU)</span>
          <div className="metric-value" style={{ fontSize: '13px', fontWeight: 800, color: 'var(--status-info)' }}>
            {liveMaxTemp} °C
            <span style={{ fontSize: '9px', color: 'var(--text-dim)', fontWeight: 500, marginLeft: '4px' }}>
              (CPU {cpuTemp}° / GPU {gpuTemp}°)
            </span>
          </div>
        </div>

        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>Рабочие режимы насоса / ступени</span>
          <div className="metric-value" style={{ fontSize: '13px', fontWeight: 800, color: 'var(--status-info)' }}>
            {coolerData.connected ? (
              (() => {
                const tier = coolerData.pump_tier ?? (coolerData.pump_voltage >= 11 ? 3 : coolerData.pump_voltage >= 7 ? 2 : coolerData.pump_voltage === 0 ? 0 : 2);
                if (tier === 0 || coolerData.pump_voltage === 0) {
                  return <span style={{ color: 'var(--text-muted)' }}>Остановлено (0%)</span>;
                }
                if (tier === 3 || coolerData.pump_voltage >= 11) {
                  return <span style={{ color: 'var(--color-beast)' }}>Предел Турбо (100%)</span>;
                }
                if (tier === 1) {
                  return <span style={{ color: 'var(--status-info)' }}>Тихий цикл (35%)</span>;
                }
                return <span style={{ color: 'var(--status-info)' }}>Стандартный цикл (45%)</span>;
              })()
            ) : '--'}
          </div>
        </div>

        <div className="metric-pill" style={{ background: 'var(--surface-sunken)', padding: '5px 9px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
          <span className="metric-label" style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>Обороты вентилятора радиатора в реальном времени</span>
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
                {coolerData.fan_duty === 0 ? '0 RPM — полная остановка (тишина)' : `${Math.round(coolerData.fan_duty * 24)} RPM`}
              </span>
            )}
          </div>
        </div>
      </div>
    </>
  );
};
