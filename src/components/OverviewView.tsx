import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { HardwareState, PowerMode, POWER_MODES } from '../types/hardware';
import { ModelCapabilities } from '../plugins/plugin_manifest';
import { ModeSelector } from './ModeSelector';
import { OsdConfig } from './SystemSettingsView';
import { HardwareStatCards } from './overview/HardwareStatCards';
import { QuickActionCards } from './overview/QuickActionCards';
import { HardwareQuickSwitches, QuickSwitchesState } from './overview/HardwareQuickSwitches';

export type { QuickSwitchesState };

export interface OverviewViewProps {
  status: HardwareState;
  osdConfig: OsdConfig;
  capabilities?: ModelCapabilities;
  onUpdateOsdConfig: (cfg: OsdConfig) => void;
  onSetPowerMode: (mode: PowerMode) => void;
  onToggleFanBoost: (targetRpm?: number) => void;
  onToggleBatteryLimit: () => void;
  onSwitchRefreshRate: (hz: number, invokeBackend?: boolean) => void;
  onOpenCustomTuning: () => void;
  checkOemGuard: (action: () => void) => void;
  profileVersion?: number;
}

export const OverviewView: React.FC<OverviewViewProps> = ({
  status,
  osdConfig,
  capabilities,
  onUpdateOsdConfig: handleUpdateOsdConfig,
  onSetPowerMode,
  onToggleFanBoost,
  onToggleBatteryLimit,
  onSwitchRefreshRate,
  onOpenCustomTuning,
  checkOemGuard,
  profileVersion = 0,
}) => {
  const [activeCustomProfile, setActiveCustomProfile] = useState<{
    cpu_pl1_watts: number;
    gpu_ctgp_enabled: boolean;
    gpu_ctgp_watts: number;
    gpu_db_enabled: boolean;
    gpu_db_watts: number;
  } | null>(null);

  const fetchActiveCustomProfile = useCallback(async () => {
    try {
      const [list, activeId] = await Promise.all([
        invoke<any[]>('get_profiles_list'),
        invoke<string>('get_active_profile_id'),
      ]);
      const matched = list.find((p) => p.id === activeId) || list[0];
      if (matched?.profile) {
        setActiveCustomProfile(matched.profile);
      }
    } catch (_) {}
  }, []);

  useEffect(() => {
    fetchActiveCustomProfile();
  }, [profileVersion, fetchActiveCustomProfile]);

  const currentModeInfo = POWER_MODES.find((m) => m.id === status.power_mode) || POWER_MODES[1];

  const getDisplayPowerTarget = () => {
    const liveTgp = status.gpu_tgp_watts && status.gpu_tgp_watts > 0 ? status.gpu_tgp_watts : 0;
    const livePl1 = status.live_pl1_w && status.live_pl1_w > 0 ? status.live_pl1_w : 0;
    const withLiveTgp = (label: string, pl1: number, tgp: number) => ({
      label,
      // 优先用后端读到的「当前档位实时生效 PL1」（EC 真值）
      pl1: livePl1 > 0 ? livePl1 : pl1,
      // 优先用后端读到的「实际生效 TGP」（基础 TGP + DB，DB 关闭时只算基础）
      tgp: liveTgp > 0 ? liveTgp : tgp,
    });
    const presets = status.power_presets;
    const isCustom = status.power_mode === 4;

    if (isCustom) {
      const pl1 = activeCustomProfile?.cpu_pl1_watts ?? (presets?.balance_pl1 ? presets.balance_pl1 : currentModeInfo.pl1);
      let tgp = currentModeInfo.tgp;
      if (activeCustomProfile) {
        if (activeCustomProfile.gpu_ctgp_enabled && activeCustomProfile.gpu_ctgp_watts > 0) {
          tgp = activeCustomProfile.gpu_ctgp_watts;
        } else {
          const base = presets?.base_tgp && presets.base_tgp > 0 ? presets.base_tgp : currentModeInfo.tgp;
          tgp = base + (activeCustomProfile.gpu_db_enabled ? (activeCustomProfile.gpu_db_watts || 0) : 0);
        }
      } else if (presets?.base_tgp && presets.base_tgp > 0) {
        tgp = presets.base_tgp + (presets.max_db || 0);
      }

      return withLiveTgp('当前设定', pl1, tgp);
    }

    let pl1 = currentModeInfo.pl1;
    let tgp = currentModeInfo.tgp;

    if (capabilities?.power_mode_watts) {
      const modeKey = status.power_mode === 1 ? 'office' : status.power_mode === 3 ? 'turbo' : 'balance';
      const pl1Val = capabilities.power_mode_watts[modeKey];
      if (pl1Val !== undefined) {
        return withLiveTgp('', pl1Val, 0);
      }
    }

    const isPureIgpu = capabilities?.discrete_gpu === false || capabilities?.form_factor === 'thin_light';

    if (isPureIgpu) {
      const thinLightPl1 = status.power_mode === 1 ? 35 : status.power_mode === 3 ? 54 : 45;
      return withLiveTgp('', thinLightPl1, 0);
    }

    if (presets && presets.hardware_read) {
      switch (status.power_mode) {
        case 1: // Office
          pl1 = presets.office_pl1 > 0 ? presets.office_pl1 : currentModeInfo.pl1;
          tgp = presets.base_tgp > 0 ? presets.base_tgp : currentModeInfo.tgp;
          break;
        case 2: // Balance
          pl1 = presets.balance_pl1 > 0 ? presets.balance_pl1 : currentModeInfo.pl1;
          tgp = presets.base_tgp > 0 ? presets.base_tgp + (presets.max_db || 0) : currentModeInfo.tgp;
          break;
        case 3: // Beast
          pl1 = presets.turbo_pl1 > 0 ? presets.turbo_pl1 : currentModeInfo.pl1;
          tgp = presets.base_tgp > 0 ? presets.base_tgp + (presets.max_db || 0) : currentModeInfo.tgp;
          break;
        default:
          break;
      }
    }

    return withLiveTgp(
      presets?.hardware_read ? '' : '参考值 (读取预设值失败)',
      pl1,
      tgp,
    );
  };

  const powerTarget = getDisplayPowerTarget();

  return (
    <div className="overview-slot-grid" data-slot="overview-grid">
      {/* 1. 顶部 4 块硬件监控对称卡片 (CPU / GPU / 风扇散热 / 显存时钟) */}
      <div className="overview-slot-telemetry" data-slot="telemetry">
        <HardwareStatCards status={status} capabilities={capabilities} />
      </div>

      {/* 2. 性能释放模式 (Power Profiles) */}
      <div className="glass-card overview-slot-power" data-slot="power" style={{ padding: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: 700, fontFamily: 'var(--font-mech)' }}>
              性能释放模式 (Power Profiles)
            </h3>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
              当前配置：{currentModeInfo.name} ({currentModeInfo.nameEn}) - {currentModeInfo.desc}
            </p>
          </div>
          <div
            style={{
              padding: '4px 12px',
              borderRadius: '20px',
              background: `${currentModeInfo.color}20`,
              border: `1px solid ${currentModeInfo.color}60`,
              color: currentModeInfo.color,
              fontSize: '11px',
              fontWeight: 700,
            }}
          >
            {powerTarget.label ? `${powerTarget.label} ` : ''}PL1: {powerTarget.pl1}W{capabilities?.discrete_gpu !== false ? ` | TGP: ${powerTarget.tgp > 0 ? `${powerTarget.tgp}W` : '--'}` : ''}
          </div>
        </div>
        <ModeSelector
          currentMode={status.power_mode}
          capabilities={capabilities}
          isDcOnline={status.is_dc_online}
          unlockBatteryLimit={status.unlock_battery_power_limit}
          onSelectMode={(mode) => checkOemGuard(() => onSetPowerMode(mode))}
          onOpenCustomTuning={() => checkOemGuard(onOpenCustomTuning)}
        />
      </div>

      {/* 3. 中部高频功能微调 (一键强冷 / 拔电自动低刷 / 屏幕多屏刷新率) */}
      <div className="overview-slot-quickactions" data-slot="quickactions">
        <QuickActionCards
          status={status}
          capabilities={capabilities}
          onToggleFanBoost={onToggleFanBoost}
          onToggleBatteryLimit={onToggleBatteryLimit}
          onSwitchRefreshRate={onSwitchRefreshRate}
          checkOemGuard={checkOemGuard}
        />
      </div>

      {/* 4. 底部 9 联原厂电竞级热键与外设直控开关卡片网格 */}
      <div className="overview-slot-switches" data-slot="switches">
        <HardwareQuickSwitches
          status={status}
          osdConfig={osdConfig}
          onUpdateOsdConfig={handleUpdateOsdConfig}
        />
      </div>
    </div>
  );
};
