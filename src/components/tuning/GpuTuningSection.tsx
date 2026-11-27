import React from 'react';
import { Flame, AlertTriangle, Zap } from 'lucide-react';
import { HardwareState, GpuTuningCapabilities } from '../../types/hardware';
import { RevSwitch, RevSwitchProps } from '../ui/RevSwitch';
import { GpuLegacyBanner } from './GpuLegacyBanner';
import { GpuPresetReferenceCard } from './GpuPresetReferenceCard';

// Backward compatibility alias for legacy imports
export type ToggleSwitchProps = RevSwitchProps;
export const ToggleSwitch: React.FC<RevSwitchProps> = RevSwitch;

export interface GpuTuningSectionProps {
  presets?: HardwareState['power_presets'];
  capabilities?: GpuTuningCapabilities | null;
  gpuSuperPerf: boolean;
  gpuCtgpEnabled?: boolean;
  gpuCtgpWatts?: number;
  gpuDbEnabled: boolean;
  gpuDbWatts: number;
  gpuCoreOffsetMhz: number;
  gpuMemOffsetMhz: number;
  gpuOffsetStep?: number;
  gpuCoreOffsetMax?: number;
  gpuMemOffsetMax?: number;
  onToggleSuperPerf: (enabled: boolean) => void;
  onToggleCtgp?: (enabled: boolean) => void;
  onChangeCtgpWatts?: (watts: number) => void;
  onToggleDb: (enabled: boolean) => void;
  onChangeDbWatts: (watts: number) => void;
  onChangeCoreOffset: (mhz: number) => void;
  onChangeMemOffset: (mhz: number) => void;
  gpuPowerTuningMode?: 'discrete_manual' | 'mode_locked' | 'none' | string;
}

export const GpuTuningSection: React.FC<GpuTuningSectionProps> = ({
  presets,
  capabilities,
  gpuSuperPerf,
  gpuCtgpEnabled = false,
  gpuCtgpWatts,
  gpuDbEnabled,
  gpuDbWatts,
  gpuCoreOffsetMhz,
  gpuMemOffsetMhz,
  gpuOffsetStep = 5,
  gpuCoreOffsetMax = 250,
  gpuMemOffsetMax = 1000,
  gpuPowerTuningMode = 'discrete_manual',
  onToggleSuperPerf,
  onChangeCtgpWatts,
  onToggleDb,
  onChangeDbWatts,
  onChangeCoreOffset,
  onChangeMemOffset,
}) => {
  const isPending = Boolean(capabilities?.is_pending || (!capabilities?.base_tgp_max_watts && capabilities?.supports_gpu_tuning));
  const isLegacyUnsupported = !isPending && Boolean(capabilities && !capabilities.supports_gpu_tuning);
  const isGpuDisabled = isLegacyUnsupported || gpuPowerTuningMode === 'none';
  const isModeLocked = gpuPowerTuningMode === 'mode_locked';
  const isNonFullPower = !isModeLocked && (capabilities?.supports_ctgp ?? false);

  // Derive effective values directly from hardware capabilities truth
  const floorWatts = capabilities?.ctgp_floor_watts ?? (presets?.base_tgp || 0);
  const baseMaxWatts = capabilities?.base_tgp_max_watts ?? (presets?.base_tgp || 0);

  const effectiveSuperPerf = isModeLocked ? true : gpuSuperPerf;

  // When Super Perf is ON, Base TGP is locked to hardware max
  // When Super Perf is OFF, Base TGP is user-configured if enabled, else baseMaxWatts
  const rawBase = effectiveSuperPerf
    ? baseMaxWatts
    : (gpuCtgpEnabled ? (gpuCtgpWatts ?? baseMaxWatts) : baseMaxWatts);
  const currentBaseWatts = isPending ? 0 : Math.max(floorWatts, Math.min(baseMaxWatts, rawBase));

  const maxDbCap = capabilities?.max_db_watts ?? (presets?.max_db || 0);

  // When Super Perf is ON, DB is locked ON at max cap
  // When Super Perf is OFF, DB follows user toggle and slider
  const effectiveDbEnabled = effectiveSuperPerf ? true : gpuDbEnabled;
  const currentDbWatts = isPending
    ? 0
    : effectiveDbEnabled
      ? (effectiveSuperPerf ? maxDbCap : Math.min(gpuDbWatts, maxDbCap))
      : 0;

  const totalCalculatedTgp = currentBaseWatts + currentDbWatts;
  const warnCoreThreshold = Math.max(150, Math.round(gpuCoreOffsetMax * 0.6));
  const warnMemThreshold = Math.max(500, Math.round(gpuMemOffsetMax * 0.5));

  const isCoreWarning = gpuCoreOffsetMhz > warnCoreThreshold;
  const isMemWarning = gpuMemOffsetMhz > warnMemThreshold;

  return (
    <div className="tuning-card">
      {/* Header */}
      <div className="tuning-header">
        <div className="tuning-header-title" style={{ color: 'var(--status-ok)' }}>
          <Flame className="w-4 h-4 shrink-0" />
          <span>Раскрытие и разгон GPU</span>
        </div>
        {capabilities?.gpu_name && (
          <span
            style={{
              fontSize: '10px',
              color: 'var(--text-dim)',
              flex: 1,
              minWidth: 0,
              textAlign: 'right',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
            title={capabilities.gpu_name}
          >
            {capabilities.gpu_name}
          </span>
        )}
      </div>

      {/* 10/20-Series Legacy Unsupported Banner */}
      {isLegacyUnsupported && <GpuLegacyBanner gpuName={capabilities?.gpu_name} />}

      {/* Non-dGPU / Thin & Light Unsupported Banner */}
      {gpuPowerTuningMode === 'none' && (
        <div
          className="sys-sub-panel"
          style={{
            padding: '8px 12px',
            marginBottom: '8px',
            borderRadius: 'var(--radius-sm)',
            fontSize: '11px',
            color: 'var(--text-dim)',
            lineHeight: 1.4,
          }}
        >
          В текущей модели нет dGPU либо платформа не поддерживает отдельную регулировку мощности GPU — разгон и функция раскрытия cTGP отключены.
        </div>
      )}

      {/* Main Controls Wrapper (Grayed out on non-dGPU or 10/20 series) */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '10px',
          opacity: isGpuDisabled ? 0.35 : 1,
          pointerEvents: isGpuDisabled ? 'none' : 'auto',
          filter: isGpuDisabled ? 'grayscale(0.7)' : 'none',
        }}
      >
        {/* Super Performance Toggle */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            background: effectiveSuperPerf ? 'var(--status-ok-dim)' : 'rgba(0,0,0,0.25)',
            padding: '8px 12px',
            borderRadius: 'var(--radius-md)',
            border: effectiveSuperPerf ? '1px solid rgba(16, 185, 129, 0.4)' : '1px solid var(--border-subtle)',
            transition: 'all var(--transition-fast)',
            opacity: isModeLocked ? 0.65 : 1,
            cursor: isModeLocked ? 'not-allowed' : 'default',
          }}
        >
          <span
            style={{
              fontWeight: 700,
              color: effectiveSuperPerf ? 'var(--status-ok)' : 'var(--text-primary)',
              fontSize: '12px',
              whiteSpace: 'nowrap',
              flexShrink: 0,
            }}
          >
            Супер-производительность (Super Perf)
          </span>
          <RevSwitch
            checked={effectiveSuperPerf}
            onChange={onToggleSuperPerf}
            disabled={isModeLocked}
            activeColor="var(--status-ok)"
          />
        </div>

        {/* GPU Core Clock Offset */}
        <div className="tuning-row" style={{ opacity: effectiveSuperPerf ? 1 : 0.45, transition: 'opacity var(--transition-fast)' }}>
          <div className="tuning-row-header">
            <span className="tuning-row-label">Смещение частоты ядра:</span>
            <span
              style={{
                fontWeight: 700,
                fontSize: '11px',
                whiteSpace: 'nowrap',
                color: gpuSuperPerf
                  ? isCoreWarning
                    ? 'var(--status-error)'
                    : 'var(--status-info)'
                  : 'var(--text-dim)',
              }}
            >
              {gpuSuperPerf ? (gpuCoreOffsetMhz > 0 ? `+${gpuCoreOffsetMhz}` : gpuCoreOffsetMhz) : 0} MHz
            </span>
          </div>
          <div style={{ padding: '2px 0 6px 0' }}>
            <input
              type="range"
              min="0"
              max={gpuCoreOffsetMax}
              step={gpuOffsetStep}
              disabled={!gpuSuperPerf}
              value={gpuSuperPerf ? gpuCoreOffsetMhz : 0}
              onChange={(e) => onChangeCoreOffset(parseInt(e.target.value, 10))}
              style={{
                width: '100%',
                accentColor: isCoreWarning ? 'var(--status-error)' : 'var(--status-info)',
                height: '6px',
                cursor: gpuSuperPerf ? 'pointer' : 'not-allowed',
              }}
            />
          </div>
        </div>

        {/* GPU Memory Clock Offset */}
        <div className="tuning-row" style={{ opacity: gpuSuperPerf ? 1 : 0.45, transition: 'opacity var(--transition-fast)' }}>
          <div className="tuning-row-header">
            <span className="tuning-row-label">Смещение частоты памяти:</span>
            <span
              style={{
                fontWeight: 700,
                fontSize: '11px',
                whiteSpace: 'nowrap',
                color: gpuSuperPerf
                  ? isMemWarning
                    ? 'var(--status-error)'
                    : gpuMemOffsetMhz < 0
                      ? 'var(--status-info)'
                      : 'var(--color-brand)'
                  : 'var(--text-dim)',
              }}
            >
              {gpuSuperPerf ? (gpuMemOffsetMhz > 0 ? `+${gpuMemOffsetMhz}` : gpuMemOffsetMhz) : 0} MHz
            </span>
          </div>
          <div style={{ padding: '2px 0 6px 0' }}>
            <input
              type="range"
              min={-gpuMemOffsetMax}
              max={gpuMemOffsetMax}
              step={gpuOffsetStep}
              disabled={!gpuSuperPerf}
              value={gpuSuperPerf ? gpuMemOffsetMhz : 0}
              onChange={(e) => onChangeMemOffset(parseInt(e.target.value, 10))}
              style={{
                width: '100%',
                accentColor: isMemWarning
                  ? 'var(--status-error)'
                  : gpuMemOffsetMhz < 0
                    ? 'var(--status-info)'
                    : 'var(--color-brand)',
                height: '6px',
                cursor: gpuSuperPerf ? 'pointer' : 'not-allowed',
              }}
            />
          </div>
        </div>

        {/* Overclocking Safety Warning */}
        {gpuSuperPerf && (isCoreWarning || isMemWarning) && (
          <div
            style={{
              background: 'var(--status-error-dim)',
              border: '1px solid rgba(239, 68, 68, 0.35)',
              borderRadius: 'var(--radius-md)',
              padding: '8px 10px',
              display: 'flex',
              alignItems: 'flex-start',
              gap: '8px',
            }}
          >
            <AlertTriangle className="w-4 h-4 shrink-0 mt-0.5" style={{ color: 'var(--status-warn)' }} />
            <div style={{ fontSize: '10.5px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>
              <div style={{ fontWeight: 700, color: 'var(--status-error)' }}>⚠️ Предупреждение о рисках разгона</div>
              <div>В этом диапазоне разгона возможны BSOD, чёрный экран или сбои драйвера — действуйте осторожно и следите за охлаждением!</div>
            </div>
          </div>
        )}

        {/* Mode Locked GPU Power Contract Card */}
        {isModeLocked && (
          <div
            style={{
              background: 'var(--surface-badge)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-md)',
              padding: '8px 10px',
              display: 'flex',
              flexDirection: 'column',
              gap: '4px',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', fontWeight: 600, color: 'var(--status-ok)' }}>
              <Zap className="w-3.5 h-3.5 shrink-0" />
              <span>Связка мощности платформы: заблокировано заводским режимом</span>
            </div>
            <div style={{ fontSize: '10px', color: 'var(--text-dim)', lineHeight: 1.45 }}>
              Потребление GPU в текущей платформе управляется заводскими режимами производительности (в Турбо автоматически раскрываются {presets?.base_tgp && presets?.max_db ? `${presets.base_tgp + presets.max_db} Вт на полную, включая ${presets.max_db}W Dynamic Boost` : 'Полная мощность'}; офисный — 30 Вт; Баланс — 120 Вт), разгон ядра и видеопамяти работает штатно.
            </div>
          </div>
        )}

        {/* Non-Full-Power Cards: cTGP Base TGP Slider */}
        {isNonFullPower && (
          <div className="tuning-row" style={{ opacity: (gpuSuperPerf || isPending) ? 0.45 : 1, transition: 'opacity var(--transition-fast)' }}>
            <div className="tuning-row-header">
              <span className="tuning-row-label">Потребление cTGP:</span>
              <span style={{ fontWeight: 700, color: (gpuSuperPerf || isPending) ? 'var(--text-dim)' : 'var(--status-info)', fontSize: '11px', whiteSpace: 'nowrap' }}>
                {isPending ? '--' : `${currentBaseWatts} W`}
              </span>
            </div>
            <div style={{ padding: '2px 0 6px 0' }}>
              <input
                type="range"
                min={isPending ? 0 : floorWatts}
                max={isPending ? 1 : baseMaxWatts}
                step="1"
                disabled={gpuSuperPerf || isPending}
                value={isPending ? 1 : currentBaseWatts}
                onChange={(e) => onChangeCtgpWatts && onChangeCtgpWatts(parseInt(e.target.value, 10))}
                style={{
                  width: '100%',
                  accentColor: 'var(--status-info)',
                  height: '6px',
                  cursor: (gpuSuperPerf || isPending) ? 'not-allowed' : 'pointer',
                }}
              />
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '9px', color: 'var(--text-dim)', marginTop: '2px', whiteSpace: 'nowrap' }}>
                <span>{isPending ? '--' : `${floorWatts} W`}</span>
                <span>{isPending ? '--' : `${baseMaxWatts} W`}</span>
              </div>
            </div>
          </div>
        )}

        {/* Dynamic Boost */}
        {!isModeLocked && (
          <div className="tuning-row" style={{ opacity: (gpuSuperPerf || isPending) ? 0.45 : 1, transition: 'opacity var(--transition-fast)' }}>
            <div className="tuning-row-header">
              <span className="tuning-row-label">Dynamic Boost:</span>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <span style={{ fontWeight: 700, color: (effectiveDbEnabled && !isPending) ? 'var(--status-ok)' : 'var(--text-dim)', fontSize: '11px', whiteSpace: 'nowrap' }}>
                  {isPending ? '--' : (effectiveDbEnabled ? `+${currentDbWatts} W` : 'Выключить')}
                </span>
                <RevSwitch
                  checked={effectiveDbEnabled}
                  onChange={onToggleDb}
                  disabled={gpuSuperPerf || isPending}
                  activeColor="var(--status-ok)"
                />
              </div>
            </div>
            <div style={{ padding: '2px 0 6px 0' }}>
              <input
                type="range"
                min={isPending ? 0 : 5}
                max={isPending ? 1 : maxDbCap}
                step="1"
                disabled={gpuSuperPerf || !gpuDbEnabled || isPending}
                value={isPending ? 1 : (gpuSuperPerf ? maxDbCap : Math.min(gpuDbWatts, maxDbCap))}
                onChange={(e) => onChangeDbWatts(parseInt(e.target.value, 10))}
                style={{
                  width: '100%',
                  accentColor: 'var(--status-ok)',
                  height: '6px',
                  cursor: (gpuSuperPerf || !gpuDbEnabled || isPending) ? 'not-allowed' : 'pointer',
                }}
              />
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '9px', color: 'var(--text-dim)', marginTop: '2px', whiteSpace: 'nowrap' }}>
                <span>{isPending ? '--' : '+5 W'}</span>
                <span>{isPending ? '--' : `+${maxDbCap} W`}</span>
              </div>
            </div>
          </div>
        )}

        {/* Real-time Combined TGP Badge */}
        {!isLegacyUnsupported && (
          <div
            style={{
              background: 'var(--status-info-dim)',
              border: '1px solid rgba(56, 189, 248, 0.25)',
              borderRadius: 'var(--radius-sm)',
              padding: '6px 10px',
              display: 'flex',
              justifyContent: 'center',
              alignItems: 'center',
              gap: '6px',
              whiteSpace: 'nowrap',
            }}
          >
            <Zap className="w-3.5 h-3.5 shrink-0" style={{ color: 'var(--status-info)' }} />
            <span style={{ fontSize: '13px', fontWeight: 700, color: 'var(--status-info)', whiteSpace: 'nowrap' }}>
              {isPending ? '--' : `${totalCalculatedTgp} W`}
            </span>
            <span style={{ fontSize: '10.5px', color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>
              {isPending ? '(идёт определение GPU при холодном старте...)' : `(база ${currentBaseWatts}W + DB ${currentDbWatts}W)`}
            </span>
          </div>
        )}
      </div>

      {/* 模具原厂建议功耗参考 (动态读取真机 EC 预设) */}
      {gpuPowerTuningMode !== 'none' && (
        <GpuPresetReferenceCard presets={presets} isDeclarative={isModeLocked} />
      )}
    </div>
  );
};
