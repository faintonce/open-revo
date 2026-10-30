export interface DisplayMonitor {
  device_name: string;
  friendly_name: string;
  is_primary: boolean;
  is_internal: boolean;
  width: number;
  height: number;
  current_hz: number;
  supported_rates: number[];
}

export interface HardwareState {
  power_mode: number;         // 1: Office, 2: Balance, 3: Beast, 4: Custom
  fan_boost: boolean;        // true if max fan
  battery_limit: number;      // 80 or 100
  cpu_temp: number;          // °C
  cpu_usage: number;         // %
  cpu_freq: number;          // MHz
  gpu_temp: number;          // °C
  cpu_fan_rpm: number;       // RPM
  gpu_fan_rpm: number;       // RPM
  gpu_usage: number;         // %
  gpu_core_freq: number;     // MHz
  gpu_mem_freq: number;      // MHz
  gpu_vram_mb: number;       // MB
  gpu_vram_used_mb: number;  // MB
  refresh_rate: number;      // Hz (e.g. 60, 240)
  available_refresh_rates?: number[]; // Supported display frequencies (e.g. [30, 48, 60, 75, 100, 120] or [60, 144, 240])
  monitors?: DisplayMonitor[]; // All active display monitors
  auto_min_refresh_on_battery?: boolean; // Automatically switch to minimum refresh rate on battery
  unlock_battery_power_limit?: boolean;  // Unlock Turbo and Custom mode restrictions on battery
  win_key_locked: boolean;   // bool
  is_mock?: boolean;         // true if running in safe mock mode
  oem_running?: boolean;     // true if GCUService is running
  oem_installed?: boolean;   // true if official control center is installed
  autostart?: boolean;       // true if autostart enabled
  is_intel_13_14_gen?: boolean; // true if running on 13th or 14th Gen Intel CPU
  water_cooler_enabled?: boolean; // Water cooler module enabled status (user preference or detected)
  support_flags?: HardwareSupportFlags; // Physical motherboard hardware capability flags
  cpu_name?: string;         // e.g. "Intel(R) Core(TM) i9-14900HX" or "AMD Ryzen 7 7840H"
  gpu_name?: string;         // e.g. "NVIDIA GeForce RTX 4090 Laptop GPU"
  model_name?: string;       // e.g. "GM6IX9B" or "MECHREVO"
  board_name?: string;       // e.g. "XINGYAO Series-P916F-STX" or "GM6IX9B"
  power_presets?: FactoryPowerPresets; // Real hardware factory power limits from motherboard EC
  is_dc_online?: boolean;    // true if connected to round-socket DC adapter (required for Turbo & Custom)
  cpu_power_w?: number;      // CPU package real-time power (W)
  gpu_power_w?: number;      // GPU real-time power draw (W)
  gpu_tgp_watts?: number;    // 实际生效的 GPU 总功耗上限（基础 TGP + DB；DB 关闭时只算基础）
  live_pl1_w?: number;      // 当前档位实时生效的 CPU PL1（W，EC 真值）
  live_pl2_w?: number;      // 当前档位实时生效的 CPU PL2（W，EC 真值）
  gpu_tuning_caps?: GpuTuningCapabilities; // Multi-generation GPU tuning & DB/cTGP capabilities
}

export interface GpuTuningCapabilities {
  gpu_generation: number;      // 0, 10, 16, 20, 30, 40, 50, 60
  gpu_name: string;            // GPU Device Name
  supports_gpu_tuning: boolean;// False for 10/16/20 series, true for 30..60 series
  supports_db: boolean;        // Dynamic Boost support
  supports_ctgp: boolean;      // cTGP down-regulation support (Non-full-power cards)
  is_full_power: boolean;      // True if Max TGP - Floor <= 25W (e.g. 4090 175W)
  ctgp_floor_watts: number;    // Lowest hardware cTGP floor (e.g. 50W, 65W, 115W, 150W)
  base_tgp_max_watts: number;  // Max base TGP before DB (e.g. 90W, 115W, 150W)
  max_db_watts: number;        // Maximum Dynamic Boost watts (e.g. 25W)
  max_total_tgp: number;       // Physical maximum total TGP (e.g. 115W, 140W, 175W)
  is_pending?: boolean;        // True when GPU is waking up from D3Cold and power limits are pending
}

export interface FactoryPowerPresets {
  office_pl1: number;
  office_pl2: number;
  office_pl4?: number;
  balance_pl1: number;
  balance_pl2: number;
  balance_pl4?: number;
  turbo_pl1: number;
  turbo_pl2: number;
  turbo_pl4?: number;
  base_tgp: number;
  max_db: number;
  hardware_read: boolean;
}

export interface AppConfig {
  power_mode?: number;
  gpu_offset_step?: number;
  gpu_core_offset_max?: number;
  gpu_mem_offset_max?: number;
  cpu_temp_target_min?: number;
  cpu_temp_target_max?: number;
  cpu_tjmax?: number;
  cpu_pl4_margin?: number;
  cpu_pl1_max?: number;
  cpu_pl2_max?: number;
  cpu_pl4_max?: number;
  cpu_pl1_offset?: number;
  cpu_pl2_offset?: number;
  cpu_pl4_offset?: number;
  cpu_temp_offset?: number;
  pl1_offset?: number;
  pl2_offset?: number;
  pl4_offset?: number;
  temp_offset?: number;
  [key: string]: any;
}

export interface HardwareSupportFlags {
  has_fan_boost: boolean;
  has_lightbar: boolean;
  has_win_lock: boolean;
  has_fn_lock?: boolean;
  has_battery_limit: boolean;
  is_rgb_keyboard: boolean;
  has_water_cooler?: boolean;
}

export type PowerMode = 1 | 2 | 3 | 4;

export interface ModeInfo {
  id: PowerMode;
  name: string;
  nameEn: string;
  desc: string;
  color: string;
  accentClass: string;
  pl1: number; // Official OEM CPU PL1
  pl2: number; // Official OEM CPU PL2
  tgp: number; // Official OEM GPU TGP
}

export const POWER_MODES: ModeInfo[] = [
  {
    id: 1,
    name: '办公模式',
    nameEn: 'OFFICE / SILENT',
    desc: '轻载静音长续航，温控保守兼顾静音体验',
    color: '#10b981',
    accentClass: 'mode-office',
    pl1: 45,
    pl2: 45,
    tgp: 150,
  },
  {
    id: 2,
    name: '均衡模式',
    nameEn: 'BALANCE / GAMING',
    desc: '日常娱乐与工作，平稳释放兼顾流畅散热',
    color: '#3b82f6',
    accentClass: 'mode-balance',
    pl1: 75,
    pl2: 75,
    tgp: 150,
  },
  {
    id: 3,
    name: '狂暴模式',
    nameEn: 'BEAST / EXTREME',
    desc: '原厂高能释放，强效动力尽享峰值性能',
    color: '#f43f5e',
    accentClass: 'mode-beast',
    pl1: 205,
    pl2: 205,
    tgp: 175,
  },
  {
    id: 4,
    name: '自定义模式',
    nameEn: 'CUSTOM / PROFILE',
    desc: '专属极客调优与风扇曲线，深度释放性能潜能',
    color: '#a855f7',
    accentClass: 'mode-custom',
    pl1: 75,
    pl2: 89,
    tgp: 175,
  },
];

export interface FanCurvePoint {
  temp: number;
  duty: number;
}

export interface CustomProfileConfig {
  cpu_temp_target_enabled: boolean;
  cpu_temp_target: number;
  cpu_pl1_watts: number;
  cpu_pl2_watts: number;
  cpu_pl4_watts: number;
  bios_oc_enabled: boolean;
  gpu_super_perf: boolean;
  gpu_ctgp_enabled: boolean;
  gpu_ctgp_watts: number;
  gpu_db_enabled: boolean;
  gpu_db_watts: number;
  gpu_core_offset_mhz: number;
  gpu_mem_offset_mhz: number;
  fan_ramp_rate: boolean;
  fan_ramp_speed_ms?: number;
  fan_isolated_output: boolean;
  fan_curve_enabled: boolean;
  cpu_fan_curve: FanCurvePoint[];
  gpu_fan_curve: FanCurvePoint[];
  fan_curve_linked?: boolean;
}

export interface CustomProfilePreset {
  id: string;
  name: string;
  file_name: string;
  icon_key: string;
  is_builtin: boolean;
  profile: CustomProfileConfig;
}

export interface BiosMenuStatus {
  supported: boolean;
  enabled: boolean;
  ap_version?: number;
  is_50_series?: boolean;
  is_amd?: boolean;
  hardware_read: boolean;
  error?: string | null;
  cpu_oc_supported?: boolean;
  cpu_oc_enabled?: boolean;
  mem_oc_supported?: boolean;
  mem_oc_enabled?: boolean;
}

export interface DisplayColorPreset {
  id: string;
  name: string;
  author?: string;
  brightness_pct?: number;
  color_temp_k: number;
  gain_r: number;
  gain_g: number;
  gain_b: number;
  saturation: number;
  contrast: number;
  gamma: number;
  is_builtin: boolean;
}

export interface DisplayTuningState {
  enabled: boolean;
  active_preset_id: string;
  active_preset: DisplayColorPreset;
  presets: DisplayColorPreset[];
  factory_presets?: DisplayColorPreset[];
}

export interface CpuFreqInfo {
  enabled: boolean;
  current_limit_mhz: number;
  min_mhz: number;
  max_mhz: number;
  recommended_mhz: number;
  is_intel_13_14: boolean;
  cpu_brand: string;
  cpu_tjmax?: number;
}


