import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  HardwareState,
  CustomProfileConfig,
  FanCurvePoint,
  CustomProfilePreset,
  BiosMenuStatus,
  AppConfig,
  GpuTuningCapabilities,
} from '../types/hardware';
import { ModelCapabilities } from '../plugins/plugin_manifest';
import { FanCurveEditor } from './tuning/FanCurveEditor';
import { TuningToolbar } from './tuning/TuningToolbar';
import { CpuTuningSection } from './tuning/CpuTuningSection';
import { GpuTuningSection } from './tuning/GpuTuningSection';
import { BldFanStrategyPanel } from './tuning/BldFanStrategyPanel';

export const DEFAULT_8_POINT_CURVE: FanCurvePoint[] = [
  { temp: 40, duty: 22 },
  { temp: 50, duty: 30 },
  { temp: 60, duty: 45 },
  { temp: 68, duty: 55 },
  { temp: 75, duty: 65 },
  { temp: 82, duty: 75 },
  { temp: 88, duty: 85 },
  { temp: 95, duty: 100 },
];

export function normalizeFanCurve8(curve?: FanCurvePoint[], minDuty: number = 0): FanCurvePoint[] {
  if (!curve || curve.length !== 8) {
    return DEFAULT_8_POINT_CURVE.map((p) => ({ ...p, duty: Math.max(minDuty, p.duty) }));
  }
  return curve.map((p) => ({ ...p, duty: Math.max(minDuty, p.duty) }));
}

interface CustomTuningPanelProps {
  status: HardwareState;
  capabilities?: ModelCapabilities;
  onClose?: () => void;
  onApplied?: () => void;
  onToggleFanBoost?: () => void;
}

const DEFAULT_PROFILE: CustomProfileConfig = {
  cpu_temp_target_enabled: true,
  cpu_temp_target: 95,
  cpu_pl1_watts: 75,
  cpu_pl2_watts: 75,
  cpu_pl4_watts: 125,
  bios_oc_enabled: false,
  gpu_super_perf: true,
  gpu_ctgp_enabled: false,
  gpu_ctgp_watts: 0,
  gpu_db_enabled: true,
  gpu_db_watts: 0,
  gpu_core_offset_mhz: 0,
  gpu_mem_offset_mhz: 0,
  fan_ramp_rate: true,
  fan_ramp_speed_ms: 300,
  fan_isolated_output: true,
  fan_curve_enabled: false,
  cpu_fan_curve: [...DEFAULT_8_POINT_CURVE],
  gpu_fan_curve: [...DEFAULT_8_POINT_CURVE],
};


export const CustomTuningPanel: React.FC<CustomTuningPanelProps> = ({
  status,
  capabilities,
  onClose,
  onApplied,
  onToggleFanBoost,
}) => {
  const [modelCaps, setModelCaps] = useState<ModelCapabilities | undefined>(capabilities);
  useEffect(() => {
    const isBldPreview = typeof window !== 'undefined' && window.location.search.includes('preview=bld');
    if (isBldPreview) {
      setModelCaps({
        id: 'mechrevo_jiaolong_16_bld',
        name: 'MECHREVO Jiaolong 16 Pro (AMD + RTX / Baolongda BM5235)',
        form_factor: 'gaming',
        discrete_gpu: true,
        water_cooling: false,
        fan_custom_curve: true,
        fan_independent_channels: true,
        fan_ramp_rate: true,
        fan_control_mode: 'direct_rpm_clamp',
        fan_boost_mode: 'direct_rpm_clamp',
        max_fan_rpm: 5800,
        cpu_vendor: 'amd',
        cpu_tuning_type: 'amd_smu',
        cpu_supports_fppt: false,
        gpu_power_tuning_mode: 'mode_locked',
        battery_charge_limit: false,
        power_modes: ['office', 'balance', 'turbo', 'custom'],
      });
      return;
    }

    if (!modelCaps) {
      invoke<ModelCapabilities>('get_model_capabilities')
        .then((res) => {
          if (res) setModelCaps(res);
        })
        .catch(console.warn);
    }
  }, [modelCaps]);

  const [profile, setProfile] = useState<CustomProfileConfig>(DEFAULT_PROFILE);
  const [presetsList, setPresetsList] = useState<CustomProfilePreset[]>([]);
  const [defaultPresets, setDefaultPresets] = useState<CustomProfilePreset[]>([]);
  const [activeProfileId, setActiveProfileId] = useState<string>('preset_silent_beast');

  const [fanCurveTab, setFanCurveTab] = useState<'cpu' | 'gpu'>('cpu');
  const [biosStatus, setBiosStatus] = useState<BiosMenuStatus | null>(null);
  const [biosToggling, setBiosToggling] = useState(false);
  const [resetSuccess, setResetSuccess] = useState(false);
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  const appConfigRef = useRef<AppConfig | null>(null);
  appConfigRef.current = appConfig;
  const [gpuCaps, setGpuCaps] = useState<GpuTuningCapabilities | null>(null);

  const isDirectRpmFan = capabilities?.fan_boost_mode === 'direct_rpm_clamp' || modelCaps?.fan_boost_mode === 'direct_rpm_clamp';
  const minDutyFloor = isDirectRpmFan ? 22 : 0;

  useEffect(() => {
    if (minDutyFloor > 0) {
      setProfile((prev) => {
        const hasCpuBelow = prev.cpu_fan_curve?.some((p) => p.duty < minDutyFloor);
        const hasGpuBelow = prev.gpu_fan_curve?.some((p) => p.duty < minDutyFloor);
        if (hasCpuBelow || hasGpuBelow) {
          return {
            ...prev,
            cpu_fan_curve: normalizeFanCurve8(prev.cpu_fan_curve, minDutyFloor),
            gpu_fan_curve: normalizeFanCurve8(prev.gpu_fan_curve, minDutyFloor),
          };
        }
        return prev;
      });
    }
  }, [minDutyFloor]);

  const presets = React.useMemo(() => {
    const isBldPreview = typeof window !== 'undefined' && window.location.search.includes('preview=bld');
    if (isBldPreview) {
      return {
        office_pl1: 45,
        office_pl2: 45,
        office_pl4: 45,
        balance_pl1: 80,
        balance_pl2: 80,
        balance_pl4: 80,
        turbo_pl1: 115,
        turbo_pl2: 115,
        turbo_pl4: 115,
        base_tgp: 115,
        max_db: 25,
        hardware_read: true,
      };
    }
    return status.power_presets;
  }, [status.power_presets]);
  const minPl = Math.min(15, presets?.office_pl1 || 35);
  const turboPl1 = presets?.turbo_pl1 || 90;
  const turboPl2 = presets?.turbo_pl2 || turboPl1;
  const turboPl4 = presets?.turbo_pl4 || (turboPl1 + 65);

  const pl1Offset = appConfig?.cpu_pl1_offset ?? appConfig?.pl1_offset ?? 0;
  const pl2Offset = appConfig?.cpu_pl2_offset ?? appConfig?.pl2_offset ?? 0;
  const pl4Offset = appConfig?.cpu_pl4_offset ?? appConfig?.pl4_offset ?? 0;
  const tempOffset = appConfig?.cpu_temp_offset ?? appConfig?.temp_offset ?? 0;

  const pl4Margin = appConfig?.cpu_pl4_margin ?? 20;
  const tempTargetMin = appConfig?.cpu_temp_target_min ?? 85;
  const nativeTjMax = appConfig?.cpu_tjmax ?? 100;
  const maxTempWall = Math.max(nativeTjMax, nativeTjMax + tempOffset);
  const configTempTargetMax = appConfig?.cpu_temp_target_max ?? (Math.min(95, nativeTjMax) + tempOffset);
  const [tempTargetMax, setTempTargetMax] = useState<number>(configTempTargetMax);

  const defaultMaxPl1 = turboPl1 + 25 + pl1Offset;
  const minMaxPl1 = turboPl1 + pl1Offset;
  const maxMaxPl1 = turboPl1 + 50 + pl1Offset;

  const defaultMaxPl2 = turboPl2 + 25 + pl2Offset;
  const minMaxPl2 = turboPl2 + pl2Offset;
  const maxMaxPl2 = turboPl2 + 50 + pl2Offset;

  const defaultMaxPl4 = turboPl4 + 50 + pl4Offset;
  const minMaxPl4 = Math.max(defaultMaxPl4 - 30, turboPl2 + pl2Offset + pl4Margin);
  const maxMaxPl4 = defaultMaxPl4 + 30;

  const [maxPl1, setMaxPl1] = useState<number>(() => {
    if (appConfig?.cpu_pl1_max != null) {
      return Math.max(minMaxPl1, Math.min(maxMaxPl1, appConfig.cpu_pl1_max));
    }
    return defaultMaxPl1;
  });
  const [maxPl2, setMaxPl2] = useState<number>(() => {
    if (appConfig?.cpu_pl2_max != null) {
      return Math.max(minMaxPl2, Math.min(maxMaxPl2, appConfig.cpu_pl2_max));
    }
    return defaultMaxPl2;
  });
  const [maxPl4, setMaxPl4] = useState<number>(() => {
    if (appConfig?.cpu_pl4_max != null) {
      return Math.max(minMaxPl4, Math.min(maxMaxPl4, appConfig.cpu_pl4_max));
    }
    return defaultMaxPl4;
  });

  const initialProfileRef = React.useRef<string>('');
  const liveTweakDebounceRef = useRef<any>(null);
  const autoSaveDebounceRef = useRef<any>(null);
  const profileRef = useRef(profile);
  profileRef.current = profile;
  const activeProfileIdRef = useRef(activeProfileId);
  activeProfileIdRef.current = activeProfileId;
  const presetsListRef = useRef(presetsList);
  presetsListRef.current = presetsList;

  const loadProfilesData = async () => {
    try {
      const [list, activeId, defPresets, cfg, caps] = await Promise.all([
        invoke<CustomProfilePreset[]>('get_profiles_list'),
        invoke<string>('get_active_profile_id'),
        invoke<CustomProfilePreset[]>('get_default_builtin_presets'),
        invoke<AppConfig>('get_app_config').catch(() => null),
        invoke<GpuTuningCapabilities>('get_gpu_tuning_capabilities').catch(() => null),
      ]);
      setPresetsList(list);
      setActiveProfileId(activeId);
      setDefaultPresets(defPresets);
      if (cfg) {
        let needsSave = false;
        const populatedCfg: AppConfig = { ...cfg };
        if (populatedCfg.cpu_pl1_max == null) {
          populatedCfg.cpu_pl1_max = defaultMaxPl1;
          needsSave = true;
        }
        if (populatedCfg.cpu_pl2_max == null) {
          populatedCfg.cpu_pl2_max = defaultMaxPl2;
          needsSave = true;
        }
        if (populatedCfg.cpu_pl4_max == null) {
          populatedCfg.cpu_pl4_max = defaultMaxPl4;
          needsSave = true;
        }
        if (populatedCfg.cpu_temp_target_max == null) {
          populatedCfg.cpu_temp_target_max = 95 + tempOffset;
          needsSave = true;
        }
        if (needsSave) {
          invoke('save_app_config', { cfg: populatedCfg }).catch(console.warn);
        }
        setAppConfig(populatedCfg);
        appConfigRef.current = populatedCfg;
      }
      const isBldPreview = typeof window !== 'undefined' && window.location.search.includes('preview=bld');
      if (isBldPreview) {
        setGpuCaps({
          gpu_generation: 40,
          gpu_name: 'NVIDIA GeForce RTX 4060 Laptop GPU',
          supports_gpu_tuning: true,
          supports_db: true,
          supports_ctgp: true,
          is_full_power: false,
          ctgp_floor_watts: 50,
          base_tgp_max_watts: 115,
          max_db_watts: 25,
          max_total_tgp: 140,
        });
      } else if (caps) {
        setGpuCaps(caps);
      }
      const activePreset = list.find((p) => p.id === activeId) || list[0];
      if (activePreset) {
        const targetMin = cfg?.cpu_temp_target_min ?? 85;
        const targetMax = Math.max(cfg?.cpu_temp_target_max ?? 95, activePreset.profile.cpu_temp_target || 95);
        if (targetMax > configTempTargetMax) {
          setTempTargetMax(targetMax);
        }
        const safeProfile: CustomProfileConfig = {
          ...activePreset.profile,
          cpu_temp_target: Math.max(targetMin, Math.min(targetMax, activePreset.profile.cpu_temp_target || 95)),
          cpu_fan_curve: normalizeFanCurve8(activePreset.profile.cpu_fan_curve, minDutyFloor),
          gpu_fan_curve: normalizeFanCurve8(activePreset.profile.gpu_fan_curve, minDutyFloor),
        };
        initialProfileRef.current = JSON.stringify(safeProfile);
        setProfile(safeProfile);
        // Silently restore hardware contract baseline to clear any locked DB or corrupt EC state ONLY if currently in Custom Mode
        if (status.power_mode === 4) {
          invoke('heal_gpu_power_contract', {
            superPerfEnable: safeProfile.gpu_super_perf,
            targetDbWatts: safeProfile.gpu_db_watts,
          }).catch((err) => console.warn('Failed to heal GPU power contract:', err));
        }
      }
    } catch (e) {
      console.error('Failed to load profiles list:', e);
    }
  };

  // Listen for real-time GPU clock offset sync from plugins or external callers
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ core_offset_mhz: number; mem_offset_mhz: number }>('gpu-clock-offsets-changed', (event) => {
      if (event.payload) {
        setProfile((prev) => {
          if (
            prev.gpu_core_offset_mhz === event.payload.core_offset_mhz &&
            prev.gpu_mem_offset_mhz === event.payload.mem_offset_mhz
          ) {
            return prev;
          }
          return {
            ...prev,
            gpu_core_offset_mhz: event.payload.core_offset_mhz,
            gpu_mem_offset_mhz: event.payload.mem_offset_mhz,
          };
        });
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    if (!activeProfileId || presetsList.length === 0) return;
    const currentPreset = presetsList.find((p) => p.id === activeProfileId);
    if (!currentPreset) return;

    const currentJson = JSON.stringify(profile);
    if (initialProfileRef.current && initialProfileRef.current === currentJson) {
      return;
    }

    // Update preset in presetsList immediately so badges and lists stay in sync
    setPresetsList((prevList) =>
      prevList.map((p) => (p.id === activeProfileId ? { ...p, profile } : p))
    );

    if (autoSaveDebounceRef.current) {
      clearTimeout(autoSaveDebounceRef.current);
    }

    autoSaveDebounceRef.current = setTimeout(async () => {
      try {
        const updated: CustomProfilePreset = {
          ...currentPreset,
          profile,
        };
        await invoke('save_profile_preset', { preset: updated });
        initialProfileRef.current = currentJson;
        if (onApplied) onApplied();
      } catch (e) {
        console.error('Failed to auto-save profile:', e);
      }
    }, 350);

    return () => {
      if (autoSaveDebounceRef.current) {
        clearTimeout(autoSaveDebounceRef.current);
      }
    };
  }, [profile, activeProfileId]);

  // Flush pending auto-save on unmount
  useEffect(() => {
    return () => {
      if (autoSaveDebounceRef.current) {
        clearTimeout(autoSaveDebounceRef.current);
        const pId = activeProfileIdRef.current;
        const pList = presetsListRef.current;
        const currentPreset = pList.find((p) => p.id === pId);
        if (currentPreset && JSON.stringify(profileRef.current) !== initialProfileRef.current) {
          invoke('save_profile_preset', {
            preset: { ...currentPreset, profile: profileRef.current },
          }).catch(console.error);
        }
      }
    };
  }, []);

  useEffect(() => {
    loadProfilesData();

    // Probe UEFI NVRAM hardware status directly from Motherboard SPI Flash
    invoke<BiosMenuStatus>('get_bios_advanced_menu_status')
      .then((res) => {
        setBiosStatus(res);
        if (res.hardware_read) {
          setProfile((prev) => ({ ...prev, bios_oc_enabled: res.enabled }));
        }
      })
      .catch((e) => console.error('Failed to probe BIOS NVRAM status:', e));

    let unlistenGpu: (() => void) | undefined;
    listen<GpuTuningCapabilities>('gpu-power-limits-ready', (event) => {
      if (event.payload) {
        setGpuCaps(event.payload);
      }
    }).then((fn) => {
      unlistenGpu = fn;
    });

    return () => {
      if (unlistenGpu) unlistenGpu();
    };
  }, []);

  const triggerLiveTweak = (patch: Partial<{
    pl1: number;
    pl2: number;
    pl4: number;
    tempTarget: number;
    ctgpEnabled: boolean;
    ctgpWatts: number;
    dbEnabled: boolean;
    dbWatts: number;
    superPerf: boolean;
    gpuCoreOffset: number;
    gpuMemOffset: number;
  }>) => {
    if (liveTweakDebounceRef.current) {
      clearTimeout(liveTweakDebounceRef.current);
    }
    liveTweakDebounceRef.current = setTimeout(() => {
      invoke('apply_live_custom_tweak', patch).catch(console.warn);
    }, 150);
  };

  const handleSelectProfile = async (id: string) => {
    try {
      const target = await invoke<CustomProfilePreset>('set_active_profile_id', { id });
      setActiveProfileId(id);
      const targetMin = appConfig?.cpu_temp_target_min ?? 85;
      const targetMax = Math.max(tempTargetMax, target.profile.cpu_temp_target || 95);
      if (targetMax > tempTargetMax) {
        setTempTargetMax(targetMax);
      }
      const safeProfile: CustomProfileConfig = {
        ...target.profile,
        cpu_temp_target: Math.max(targetMin, Math.min(targetMax, target.profile.cpu_temp_target || 95)),
        cpu_fan_curve: normalizeFanCurve8(target.profile.cpu_fan_curve, minDutyFloor),
        gpu_fan_curve: normalizeFanCurve8(target.profile.gpu_fan_curve, minDutyFloor),
      };
      initialProfileRef.current = JSON.stringify(safeProfile);
      setProfile(safeProfile);
      if (onApplied) onApplied();
    } catch (e) {
      console.error('Failed to switch profile:', e);
    }
  };

  const handleCreateProfile = async (name: string) => {
    try {
      const newPreset = await invoke<CustomProfilePreset>('create_profile_preset', {
        name,
        baseId: activeProfileId,
        initialProfile: profile,
      });
      await loadProfilesData();
      await handleSelectProfile(newPreset.id);
    } catch (e) {
      console.error('Failed to create profile:', e);
    }
  };

  const handleResetCurrentProfile = async () => {
    const currentPreset = presetsList.find((p) => p.id === activeProfileId);
    if (!currentPreset) return;
    const confirmMsg = currentPreset.is_builtin
      ? `确定要将原厂方案【${currentPreset.name}】重置为出厂最佳预设吗？\n（将同步重置 CPU 与 GPU 硬件功耗握手，解除可能的过热锁 55W 或 GPU 锁功耗异常）`
      : `确定要放弃修改，恢复方案【${currentPreset.name}】为默认推荐参数吗？\n（将同步重置 CPU 与 GPU 硬件功耗握手，解除可能的过热锁 55W 或 GPU 锁功耗异常）`;
    if (!window.confirm(confirmMsg)) return;

    try {
      const res = await invoke<CustomProfilePreset>('reset_profile_preset', { id: activeProfileId });

      // 1. 同步恢复温度墙上限与 PL1/PL2/PL4 上限为出厂默认值
      const defaultTargetMax = Math.max(95, res.profile.cpu_temp_target || 95);
      setTempTargetMax(defaultTargetMax);
      setMaxPl1(defaultMaxPl1);
      setMaxPl2(defaultMaxPl2);
      setMaxPl4(defaultMaxPl4);

      // 2. 同步重置 appConfig 中的上限配置并持久化保存为明文数值（绝不再写入 null/undefined）
      const cfg = appConfigRef.current || appConfig;
      if (cfg) {
        const nextCfg: AppConfig = {
          ...cfg,
          cpu_temp_target_max: defaultTargetMax,
          cpu_pl1_max: defaultMaxPl1,
          cpu_pl2_max: defaultMaxPl2,
          cpu_pl4_max: defaultMaxPl4,
        };
        setAppConfig(nextCfg);
        invoke('save_app_config', { cfg: nextCfg }).catch(console.warn);
      }

      // 3. 构建 Profile，在出厂默认上限的约束下完好无损地恢复设定值
      const safeProfile: CustomProfileConfig = {
        ...res.profile,
        cpu_temp_target: Math.max(tempTargetMin, Math.min(defaultTargetMax, res.profile.cpu_temp_target || 95)),
        cpu_pl1_watts: Math.max(minPl, Math.min(defaultMaxPl1, res.profile.cpu_pl1_watts)),
        cpu_pl2_watts: Math.max(minPl, Math.min(defaultMaxPl2, res.profile.cpu_pl2_watts)),
        cpu_pl4_watts: Math.max(minPl + pl4Margin, Math.min(defaultMaxPl4, res.profile.cpu_pl4_watts)),
        cpu_fan_curve: normalizeFanCurve8(res.profile.cpu_fan_curve, minDutyFloor),
        gpu_fan_curve: normalizeFanCurve8(res.profile.gpu_fan_curve, minDutyFloor),
      };
      initialProfileRef.current = JSON.stringify(safeProfile);
      setProfile(safeProfile);
      setPresetsList((prev) => prev.map((p) => (p.id === activeProfileId ? { ...res, profile: safeProfile } : p)));
      await invoke('apply_custom_profile', { profile: safeProfile });
      const isBld = isDirectRpmFan;
      if (safeProfile.fan_isolated_output || isBld) {
        triggerLiveFanApply(safeProfile.cpu_fan_curve, safeProfile.gpu_fan_curve);
      }
      setResetSuccess(true);
      if (onApplied) onApplied();
      setTimeout(() => setResetSuccess(false), 1500);
    } catch (e) {
      console.error('Failed to reset profile:', e);
    }
  };

  const handleRenameProfile = async (newName: string) => {
    try {
      await invoke<CustomProfilePreset>('rename_profile_preset', {
        id: activeProfileId,
        newName,
      });
      await loadProfilesData();
    } catch (e) {
      console.error('Failed to rename profile:', e);
    }
  };

  const handleDeleteProfile = async () => {
    const currentPreset = presetsList.find((p) => p.id === activeProfileId);
    if (!currentPreset || currentPreset.is_builtin) return;
    if (!window.confirm(`确定要删除方案【${currentPreset.name}】(${currentPreset.file_name}) 吗？`)) return;
    try {
      await invoke('delete_profile_preset', { id: activeProfileId });
      await loadProfilesData();
      await handleSelectProfile('preset_silent_beast');
    } catch (e) {
      console.error('Failed to delete profile:', e);
    }
  };

  const handleExportProfile = async () => {
    try {
      const savedPath = await invoke<string | null>('export_profile_preset', { id: activeProfileId });
      if (savedPath) {
        alert('调教方案已成功导出至：\n' + savedPath);
      }
    } catch (e: any) {
      console.error('Failed to export profile:', e);
      alert('导出调教方案失败: ' + e);
    }
  };

  const handleImportProfile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files && e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = async (event) => {
      const content = event.target?.result as string;
      if (!content) return;
      try {
        const imported = await invoke<CustomProfilePreset>('import_profile_preset', {
          jsonContent: content,
          fileName: file.name,
        });
        await loadProfilesData();
        await handleSelectProfile(imported.id);
      } catch (err: any) {
        alert('导入失败: ' + err);
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  };

  const handleOpenProfilesFolder = async () => {
    try {
      await invoke('open_profiles_folder');
    } catch (e) {
      console.error('Failed to open profiles folder:', e);
    }
  };



  const handleTempTargetChange = (rawVal: number, maxCap = tempTargetMax) => {
    const clampedVal = Math.max(tempTargetMin, Math.min(maxCap, rawVal));
    setProfile((prev) => ({ ...prev, cpu_temp_target: clampedVal }));
    if (profileRef.current.cpu_temp_target_enabled) {
      triggerLiveTweak({ tempTarget: clampedVal });
    }
  };

  useEffect(() => {
    if (appConfig?.cpu_temp_target_max) {
      const newMax = appConfig.cpu_temp_target_max;
      setTempTargetMax(newMax);
      if (profileRef.current.cpu_temp_target > newMax) {
        handleTempTargetChange(newMax, newMax);
      }
    }
  }, [appConfig?.cpu_temp_target_max]);

  const handleAdjustTempTargetMax = (delta: number) => {
    setTempTargetMax((prev) => {
      const next = Math.max(tempTargetMin, Math.min(maxTempWall, prev + delta));
      if (profileRef.current.cpu_temp_target > next) {
        handleTempTargetChange(next, next);
      }
      const cfg = appConfigRef.current || appConfig;
      if (cfg) {
        const nextCfg = { ...cfg, cpu_temp_target_max: next };
        setAppConfig(nextCfg);
        invoke('save_app_config', { cfg: nextCfg }).catch(console.warn);
      }
      return next;
    });
  };




  useEffect(() => {
    const p1 = appConfig?.cpu_pl1_max != null
      ? Math.max(minMaxPl1, Math.min(maxMaxPl1, appConfig.cpu_pl1_max))
      : defaultMaxPl1;
    const p2 = appConfig?.cpu_pl2_max != null
      ? Math.max(minMaxPl2, Math.min(maxMaxPl2, appConfig.cpu_pl2_max))
      : defaultMaxPl2;
    const p4 = appConfig?.cpu_pl4_max != null
      ? Math.max(minMaxPl4, Math.min(maxMaxPl4, appConfig.cpu_pl4_max))
      : defaultMaxPl4;

    setMaxPl1(p1);
    setMaxPl2(p2);
    setMaxPl4(p4);
  }, [turboPl1, turboPl2, turboPl4, pl1Offset, pl2Offset, pl4Offset, appConfig?.cpu_pl1_max, appConfig?.cpu_pl2_max, appConfig?.cpu_pl4_max]);

  // PL1 change auto pushes PL2 and PL4 up if exceeded
  const handlePl1Change = (rawVal: number) => {
    const val = Math.max(minPl, Math.min(maxPl1, rawVal));
    const nextPl2 = Math.max(val, profile.cpu_pl2_watts);
    const nextPl4 = Math.max(nextPl2 + pl4Margin, profile.cpu_pl4_watts);
    setProfile((prev) => ({
      ...prev,
      cpu_pl1_watts: val,
      cpu_pl2_watts: nextPl2,
      cpu_pl4_watts: nextPl4,
    }));
    triggerLiveTweak({ pl1: val, pl2: nextPl2, pl4: nextPl4 });
  };

  // PL2 change pulls PL1 down if PL2 < PL1, and pushes PL4 up to preserve >= pl4Margin transient buffer
  const handlePl2Change = (rawVal: number) => {
    const val = Math.max(minPl, Math.min(maxPl2, rawVal));
    const nextPl1 = Math.min(val, profile.cpu_pl1_watts);
    const nextPl4 = Math.max(val + pl4Margin, profile.cpu_pl4_watts);
    setProfile((prev) => ({
      ...prev,
      cpu_pl1_watts: nextPl1,
      cpu_pl2_watts: val,
      cpu_pl4_watts: nextPl4,
    }));
    triggerLiveTweak({ pl1: nextPl1, pl2: val, pl4: nextPl4 });
  };

  // PL4 change: ensures PL4 is at least PL2 + pl4Margin (hardware transient safety buffer).
  // If user forces PL4 lower, smoothly pull PL2 (and PL1) down to keep PL2 <= PL4 - pl4Margin.
  const handlePl4Change = (rawVal: number) => {
    const val = Math.max(minPl + pl4Margin, Math.min(maxPl4, rawVal));
    const maxAllowedPl2 = Math.max(minPl, val - pl4Margin);
    const nextPl2 = Math.min(maxAllowedPl2, profile.cpu_pl2_watts);
    const nextPl1 = Math.min(nextPl2, profile.cpu_pl1_watts);
    setProfile((prev) => ({
      ...prev,
      cpu_pl1_watts: nextPl1,
      cpu_pl2_watts: nextPl2,
      cpu_pl4_watts: val,
    }));
    triggerLiveTweak({ pl1: nextPl1, pl2: nextPl2, pl4: val });
  };

  const handleAdjustMaxPl1 = (delta: number) => {
    setMaxPl1((prev) => {
      const next = Math.max(minMaxPl1, Math.min(maxMaxPl1, prev + delta));
      if (profile.cpu_pl1_watts > next) {
        handlePl1Change(next);
      }
      const cfg = appConfigRef.current || appConfig;
      if (cfg) {
        const nextCfg = { ...cfg, cpu_pl1_max: next };
        setAppConfig(nextCfg);
        invoke('save_app_config', { cfg: nextCfg }).catch(console.warn);
      }
      return next;
    });
  };

  const handleAdjustMaxPl2 = (delta: number) => {
    setMaxPl2((prev) => {
      const next = Math.max(minMaxPl2, Math.min(maxMaxPl2, prev + delta));
      if (profile.cpu_pl2_watts > next) {
        handlePl2Change(next);
      }
      const cfg = appConfigRef.current || appConfig;
      if (cfg) {
        const nextCfg = { ...cfg, cpu_pl2_max: next };
        setAppConfig(nextCfg);
        invoke('save_app_config', { cfg: nextCfg }).catch(console.warn);
      }
      return next;
    });
  };

  const handleAdjustMaxPl4 = (delta: number) => {
    setMaxPl4((prev) => {
      const next = Math.max(minMaxPl4, Math.min(maxMaxPl4, prev + delta));
      if (profile.cpu_pl4_watts > next) {
        handlePl4Change(next);
      }
      const cfg = appConfigRef.current || appConfig;
      if (cfg) {
        const nextCfg = { ...cfg, cpu_pl4_max: next };
        setAppConfig(nextCfg);
        invoke('save_app_config', { cfg: nextCfg }).catch(console.warn);
      }
      return next;
    });
  };

  // Toggle BIOS OC switch with cross-generation NVRAM hardware execution
  const handleToggleBiosOc = async () => {
    if (biosStatus?.hardware_read && !biosStatus.supported) {
      alert('当前机器主板固件或处理器未声明支持高级超频菜单 (Support=0x00)，已安全拦截！');
      return;
    }

    const nextVal = !profile.bios_oc_enabled;
    setBiosToggling(true);
    try {
      const res = await invoke<BiosMenuStatus>('toggle_bios_advanced_menu', { enable: nextVal });
      setBiosStatus(res);
      setProfile((prev) => ({ ...prev, bios_oc_enabled: res.enabled }));
    } catch (e: any) {
      console.error('Failed to toggle BIOS OC menu:', e);
      alert(`修改高级 BIOS 超频菜单失败: ${e}\n\n请尝试【以管理员身份运行】OpenRevo，以获取读写主板 SPI Flash NVRAM 的 SeSystemEnvironmentPrivilege 权限！`);
    } finally {
      setBiosToggling(false);
    }
  };

  const liveFanDebounceRef = useRef<any>(null);
  const rampDebounceRef = useRef<any>(null);

  const handleFanRampSpeedChange = (rawVal: number) => {
    const val = Math.max(100, Math.min(12700, Math.round(rawVal / 100) * 100));
    setProfile((p) => ({ ...p, fan_ramp_speed_ms: val }));
    if (status.power_mode === 4 && profile.fan_ramp_rate) {
      if (rampDebounceRef.current) clearTimeout(rampDebounceRef.current);
      rampDebounceRef.current = setTimeout(() => {
        invoke('toggle_fan_ramp_rate', { enable: true, speedMs: val }).catch(console.warn);
      }, 200);
    }
  };

  const triggerLiveFanApply = (cpuCurve: FanCurvePoint[], gpuCurve: FanCurvePoint[]) => {
    const isBld = isDirectRpmFan;
    if (!isBld && status.power_mode !== 4) return;
    if (liveFanDebounceRef.current) {
      clearTimeout(liveFanDebounceRef.current);
    }
    liveFanDebounceRef.current = setTimeout(() => {
      invoke('apply_fan_curve_live', { cpuCurve, gpuCurve }).catch((err) => {
        console.warn('Failed to apply live fan curve:', err);
      });
    }, 250);
  };

  const handleCurveChange = (type: 'cpu' | 'gpu', newCurve: FanCurvePoint[]) => {
    const isBld = isDirectRpmFan;
    if (profile.fan_curve_linked) {
      setProfile((prev) => ({
        ...prev,
        cpu_fan_curve: newCurve,
        gpu_fan_curve: newCurve,
      }));
      if (profile.fan_isolated_output || isBld) {
        triggerLiveFanApply(newCurve, newCurve);
      }
    } else {
      const curveKey = type === 'cpu' ? 'cpu_fan_curve' : 'gpu_fan_curve';
      const nextCpu = type === 'cpu' ? newCurve : profile.cpu_fan_curve;
      const nextGpu = type === 'gpu' ? newCurve : profile.gpu_fan_curve;
      setProfile((prev) => ({ ...prev, [curveKey]: newCurve }));
      if (profile.fan_isolated_output || isBld) {
        triggerLiveFanApply(nextCpu, nextGpu);
      }
    }
  };

  const handleToggleFanCurveLinked = (linked: boolean) => {
    const isBld = isDirectRpmFan;
    setProfile((prev) => {
      const next = { ...prev, fan_curve_linked: linked };
      if (linked) {
        const activeCurve = fanCurveTab === 'cpu' ? prev.cpu_fan_curve : prev.gpu_fan_curve;
        next.cpu_fan_curve = activeCurve;
        next.gpu_fan_curve = activeCurve;
        if (prev.fan_isolated_output || isBld) {
          triggerLiveFanApply(activeCurve, activeCurve);
        }
      }
      return next;
    });
  };

  const handleToggleFanRampRate = async (nextVal: boolean) => {
    const isBld = isDirectRpmFan;
    setProfile((p) => ({ ...p, fan_ramp_rate: nextVal }));
    if (status.power_mode === 4 || isBld) {
      try {
        await invoke('toggle_fan_ramp_rate', {
          enable: nextVal,
          speedMs: profile.fan_ramp_speed_ms || 300,
        });
      } catch (e) {
        console.warn('Failed to toggle fan ramp rate:', e);
      }
    }
  };

  const handleToggleFanIsolated = async (nextVal: boolean) => {
    const isBld = isDirectRpmFan;
    setProfile((p) => ({ ...p, fan_isolated_output: nextVal, fan_curve_enabled: nextVal }));
    if (status.power_mode === 4 || isBld) {
      try {
        await invoke('toggle_fan_isolated_output', {
          enable: nextVal,
          cpuCurve: profile.cpu_fan_curve,
          gpuCurve: profile.gpu_fan_curve,
        });
      } catch (e) {
        console.warn('Failed to toggle fan isolated output:', e);
      }
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        background: 'rgba(11, 15, 25, 0.98)',
        backdropFilter: 'blur(16px)',
        display: 'flex',
        flexDirection: 'column',
        animation: 'fadeIn 0.2s ease-out',
        userSelect: 'none',
      }}
    >
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          gap: '12px',
          padding: '16px 22px',
          fontSize: '12px',
          overflowY: 'auto',
          boxSizing: 'border-box',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header Bar with Profiles Toolbar */}
        <TuningToolbar
          presetsList={presetsList}
          activeProfileId={activeProfileId}
          defaultPresets={defaultPresets}
          resetSuccess={resetSuccess}
          onSelectProfile={handleSelectProfile}
          onCreateProfile={handleCreateProfile}
          onResetCurrentProfile={handleResetCurrentProfile}
          onRenameProfile={handleRenameProfile}
          onDeleteProfile={handleDeleteProfile}
          onExportProfile={handleExportProfile}
          onImportProfile={handleImportProfile}
          onOpenProfilesFolder={handleOpenProfilesFolder}
          onClose={onClose}
        />

        {/* Content Body: Protected Unadapted Notice OR 3-Column Tuning Grid */}
        {modelCaps?.is_adapted === false ? (
          <div
            style={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '40px 24px',
              textAlign: 'center',
            }}
          >
            <div
              style={{
                width: '64px',
                height: '64px',
                borderRadius: '50%',
                background: 'var(--surface-overlay)',
                border: '1px solid var(--border-medium)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: '28px',
                marginBottom: '16px',
              }}
            >
              🛡️
            </div>
            <div
              style={{
                fontSize: '16px',
                fontWeight: 600,
                color: 'var(--text-main)',
                marginBottom: '8px',
              }}
            >
              当前机型尚未适配硬件契约
            </div>
            <div
              style={{
                fontSize: '13px',
                color: 'var(--text-dim)',
                lineHeight: 1.6,
                maxWidth: '460px',
              }}
            >
              为保障硬件与供电安全，OpenRevo 已自动启用硬件只读保护，暂不支持对未适配机型下发功耗控制与风扇曲线调节。
            </div>
          </div>
        ) : (
          /* 3-Column Grid with Comfortable Padding */
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '12px', flex: 1 }}>
            {/* Column 1: CPU Power & Temperature */}
            <CpuTuningSection
            cpuTemp={status.cpu_temp}
            cpuFanRpm={status.cpu_fan_rpm}
            cpuPowerW={status.cpu_power_w}
            gpuTemp={status.gpu_temp}
            gpuFanRpm={status.gpu_fan_rpm}
            gpuPowerW={status.gpu_power_w}
            gpuName={status.gpu_name}
            discreteGpu={modelCaps?.discrete_gpu}
            minPl={minPl}
            maxPl1={maxPl1}
            maxPl2={maxPl2}
            maxPl4={maxPl4}
            minMaxPl1={minMaxPl1}
            maxMaxPl1={maxMaxPl1}
            minMaxPl2={minMaxPl2}
            maxMaxPl2={maxMaxPl2}
            minMaxPl4={minMaxPl4}
            maxMaxPl4={maxMaxPl4}
            onAdjustMaxPl1={handleAdjustMaxPl1}
            onAdjustMaxPl2={handleAdjustMaxPl2}
            onAdjustMaxPl4={handleAdjustMaxPl4}
            tempTargetEnabled={profile.cpu_temp_target_enabled}
            tempTarget={profile.cpu_temp_target}
            tempTargetMin={tempTargetMin}
            tempTargetMax={tempTargetMax}
            maxTempWall={maxTempWall}
            onAdjustTempTargetMax={handleAdjustTempTargetMax}
            pl4Margin={pl4Margin}
            pl1={profile.cpu_pl1_watts}
            pl2={profile.cpu_pl2_watts}
            pl4={profile.cpu_pl4_watts}
            biosOcEnabled={profile.bios_oc_enabled}
            biosStatus={biosStatus}
            biosToggling={biosToggling}
            cpuVendor={modelCaps?.cpu_vendor}
            cpuTuningType={modelCaps?.cpu_tuning_type}
            cpuSupportsFppt={modelCaps?.cpu_supports_fppt}
            cpuName={modelCaps?.cpu_name}
            onToggleTempTarget={(enabled) => {
              setProfile((p) => ({ ...p, cpu_temp_target_enabled: enabled }));
              triggerLiveTweak({ tempTarget: enabled ? Math.min(profile.cpu_temp_target, tempTargetMax) : tempTargetMax });
            }}
            onChangeTempTarget={handleTempTargetChange}
            onChangePl1={handlePl1Change}
            onChangePl2={handlePl2Change}
            onChangePl4={handlePl4Change}
            onToggleBiosOc={handleToggleBiosOc}
          />

          {/* Column 2: GPU Performance & Overclocking */}
          <GpuTuningSection
            presets={presets}
            capabilities={gpuCaps}
            gpuSuperPerf={profile.gpu_super_perf}
            gpuPowerTuningMode={modelCaps?.gpu_power_tuning_mode}
            gpuCtgpEnabled={profile.gpu_ctgp_enabled}
            gpuCtgpWatts={profile.gpu_ctgp_watts}
            gpuDbEnabled={profile.gpu_db_enabled}
            gpuDbWatts={profile.gpu_db_watts}
            gpuCoreOffsetMhz={profile.gpu_core_offset_mhz}
            gpuMemOffsetMhz={profile.gpu_mem_offset_mhz}
            gpuOffsetStep={appConfig?.gpu_offset_step ?? 5}
            gpuCoreOffsetMax={appConfig?.gpu_core_offset_max ?? 250}
            gpuMemOffsetMax={appConfig?.gpu_mem_offset_max ?? 1000}
            onToggleSuperPerf={(enabled) => {
              setProfile((p) => ({ ...p, gpu_super_perf: enabled }));
              const patch: any = {
                superPerf: enabled,
                dbEnabled: enabled ? true : profile.gpu_db_enabled,
                dbWatts: enabled ? (gpuCaps?.max_db_watts || 0) : profile.gpu_db_watts,
                gpuCoreOffset: enabled ? profile.gpu_core_offset_mhz : 0,
                gpuMemOffset: enabled ? profile.gpu_mem_offset_mhz : 0,
              };
              if (gpuCaps?.supports_ctgp) {
                patch.ctgpEnabled = enabled ? false : profile.gpu_ctgp_enabled;
                patch.ctgpWatts = enabled ? (gpuCaps?.base_tgp_max_watts || 0) : (profile.gpu_ctgp_watts || gpuCaps?.base_tgp_max_watts || 0);
              }
              triggerLiveTweak(patch);
            }}
            onChangeCtgpWatts={(val) => {
              setProfile((p) => ({ ...p, gpu_ctgp_watts: val, gpu_ctgp_enabled: true }));
              triggerLiveTweak({
                ctgpEnabled: true,
                ctgpWatts: val,
                dbEnabled: profile.gpu_db_enabled,
                dbWatts: profile.gpu_db_watts,
              });
            }}
            onToggleDb={(enabled) => {
              setProfile((p) => ({ ...p, gpu_db_enabled: enabled }));
              const patch: any = { dbEnabled: enabled, dbWatts: profile.gpu_db_watts };
              if (gpuCaps?.supports_ctgp) {
                patch.ctgpEnabled = profile.gpu_ctgp_enabled;
                patch.ctgpWatts = profile.gpu_ctgp_watts || gpuCaps?.base_tgp_max_watts || 0;
              }
              triggerLiveTweak(patch);
            }}
            onChangeDbWatts={(val) => {
              setProfile((p) => ({ ...p, gpu_db_watts: val, gpu_db_enabled: true }));
              const patch: any = { dbEnabled: true, dbWatts: val };
              if (gpuCaps?.supports_ctgp) {
                patch.ctgpEnabled = profile.gpu_ctgp_enabled;
                patch.ctgpWatts = profile.gpu_ctgp_watts || gpuCaps?.base_tgp_max_watts || 0;
              }
              triggerLiveTweak(patch);
            }}
            onChangeCoreOffset={(val) => {
              setProfile((p) => ({ ...p, gpu_core_offset_mhz: val }));
              if (profile.gpu_super_perf) {
                triggerLiveTweak({ gpuCoreOffset: val, gpuMemOffset: profile.gpu_mem_offset_mhz });
              }
            }}
            onChangeMemOffset={(val) => {
              setProfile((p) => ({ ...p, gpu_mem_offset_mhz: val }));
              if (profile.gpu_super_perf) {
                triggerLiveTweak({ gpuCoreOffset: profile.gpu_core_offset_mhz, gpuMemOffset: val });
              }
            }}
          />

          {/* Column 3: Fan Controls & Curves */}
          {modelCaps?.fan_custom_curve === false ? (
            <BldFanStrategyPanel
              cpuFanRpm={status.cpu_fan_rpm}
              gpuFanRpm={status.gpu_fan_rpm}
              cpuTemp={status.cpu_temp}
              gpuTemp={status.gpu_temp}
              fanBoost={status.fan_boost}
              maxRpm={modelCaps?.max_fan_rpm}
              fanBoostMode={modelCaps?.fan_boost_mode}
              onToggleFanBoost={onToggleFanBoost || (() => invoke('set_fan_boost', { enable: !status.fan_boost }))}
            />
          ) : (
            <FanCurveEditor
              fanCurveTab={fanCurveTab}
              onTabChange={setFanCurveTab}
              cpuCurve={profile.cpu_fan_curve}
              gpuCurve={profile.gpu_fan_curve}
              onCurveChange={handleCurveChange}
              fanIsolated={profile.fan_isolated_output}
              onToggleFanIsolated={handleToggleFanIsolated}
              fanRampRate={profile.fan_ramp_rate}
              onToggleFanRampRate={handleToggleFanRampRate}
              fanRampSpeedMs={profile.fan_ramp_speed_ms || 300}
              onChangeFanRampSpeed={handleFanRampSpeedChange}
              fanCurveLinked={profile.fan_curve_linked}
              onToggleFanCurveLinked={handleToggleFanCurveLinked}
              fanBoost={status.fan_boost}
              onToggleFanBoost={onToggleFanBoost || (() => invoke('set_fan_boost', { enable: !status.fan_boost }))}
              fanBoostMode={modelCaps?.fan_boost_mode}
              maxRpm={modelCaps?.max_fan_rpm}
              fanIndependentChannels={modelCaps?.fan_independent_channels ?? true}
              supportFanRampRate={modelCaps?.fan_ramp_rate ?? (capabilities?.fan_ramp_rate ?? true)}
              minDuty={modelCaps?.fan_boost_mode === 'direct_rpm_clamp' ? 22 : 0}
            />
          )}
        </div>
      )}

        {/* Bottom Actions Bar */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            borderTop: '1px solid rgba(255, 255, 255, 0.08)',
            paddingTop: '12px',
            marginTop: '6px',
            gap: '16px',
            flexShrink: 0,
          }}
        >
          <div
            style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              fontSize: '11px',
              color: 'rgba(255, 255, 255, 0.45)',
              lineHeight: '1.4',
            }}
          >
            <span style={{ fontSize: '13px', lineHeight: 1 }}>⚠️</span>
            <span>
              <strong style={{ color: 'rgba(255, 255, 255, 0.7)' }}>免责声明：</strong>
              OpenRevo 仅提供底层硬件操控能力，参数配置权完全归属用户。超出出厂规格的极限功耗、温控或频率偏移可能导致硬件保护性黑屏、断电或重置，请根据整机散热与供电合理调校，相关风险由操作者自负。
            </span>
          </div>

          <div style={{ display: 'flex', gap: '10px', alignItems: 'center', flexShrink: 0 }}>


            {onClose && (
              <button
                onClick={onClose}
                style={{
                  height: '36px',
                  padding: '0 20px',
                  borderRadius: '8px',
                  fontSize: '12px',
                  fontWeight: 600,
                  whiteSpace: 'nowrap',
                  background: 'rgba(255, 255, 255, 0.08)',
                  color: '#cbd5e1',
                  border: '1px solid rgba(255, 255, 255, 0.15)',
                  cursor: 'pointer',
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  transition: 'all 0.2s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'rgba(255, 255, 255, 0.15)';
                  e.currentTarget.style.color = '#fff';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'rgba(255, 255, 255, 0.08)';
                  e.currentTarget.style.color = '#cbd5e1';
                }}
              >
                关闭
              </button>
            )}
          </div>
        </div>
      </div>




    </div>
  );
};
