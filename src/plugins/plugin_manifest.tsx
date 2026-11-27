import React from 'react';
import {
  Gauge,
  Cpu,
  BatteryCharging,
  Palette,
  Droplets,
  Layers,
  Heart,
  Puzzle,
  LucideIcon,
} from 'lucide-react';
import { HardwareState } from '../types/hardware';

export type BuiltinTabId = 'overview' | 'gpu' | 'tuning' | 'lighting' | 'water' | 'system' | 'plugins' | 'sponsor';

export interface ModelCapabilities {
  id?: string;
  name?: string;
  form_factor?: 'thin_light' | 'gaming' | 'mini_pc' | string;
  ui_preset?: string;
  discrete_gpu?: boolean;
  water_cooling?: boolean;
  fan_custom_curve?: boolean;
  fan_independent_channels?: boolean;
  fan_ramp_rate?: boolean;
  fan_control_mode?: 'discrete_table_16' | 'direct_rpm_clamp' | 'fixed_boost_only' | 'none' | string;
  fan_boost_mode?: 'hardware_toggle' | 'direct_rpm_clamp' | string;
  max_fan_rpm?: number;
  cpu_vendor?: 'amd' | 'intel' | string;
  cpu_tuning_type?: 'intel_pl' | 'amd_smu' | 'none' | string;
  cpu_supports_fppt?: boolean;
  gpu_power_tuning_mode?: 'discrete_manual' | 'mode_locked' | 'none' | string;
  cpu_name?: string;
  battery_charge_limit?: boolean;
  battery_limit_mode?: string;
  fn_lock?: boolean;
  power_modes?: string[];
  power_mode_watts?: Record<string, number>;
  rgb_keyboard?: {
    supported?: boolean;
    type?: string;
    steps?: number[];
    sleep_timeout_supported?: boolean;
    hardware_autonomous?: boolean;
    hotkey_osd_only?: boolean;
  };
  is_adapted?: boolean;
  is_debug_mode?: boolean;
}

export interface ModuleSlotContext {
  status: HardwareState;
  isWaterCoolerEnabled: boolean;
  isCoolerConnected: boolean;
  moduleToggles?: Record<string, boolean>;
  capabilities?: ModelCapabilities;
}

export interface BuiltinModuleSlot {
  id: BuiltinTabId;
  name: string;
  tabClassName: string;
  order: number;
  icon: LucideIcon;
  iconClass?: string;
  description: string;
  /**
   * 是否为核心基础模块（如系统设置），基础模块不可停用防呆
   */
  isEssential?: boolean;
  /**
   * 是否在导航栏渲染该模块
   */
  isVisible: (ctx: ModuleSlotContext) => boolean;
  /**
   * 导航栏角标或指示状态（如水冷魔盒未连接/连接指示点）
   */
  renderBadge?: (ctx: ModuleSlotContext) => React.ReactNode;
}

export const BUILTIN_MODULE_SLOTS: BuiltinModuleSlot[] = [
  {
    id: 'overview',
    name: 'Общий обзор состояния',
    tabClassName: 'tab-overview',
    order: 10,
    icon: Gauge,
    description: 'Мониторинг датчиков всего устройства в реальном времени, панель переключения режимов и доска здоровья батареи',
    isVisible: () => true,
  },
  {
    id: 'gpu',
    name: 'Режим графики',
    tabClassName: 'tab-gpu',
    order: 20,
    icon: Cpu,
    description: 'Переключение MUX (прямой вывод dGPU / гибридный), динамический разгон, яркость экрана и ICC-калибровка',
    isVisible: () => true,
  },
  {
    id: 'tuning',
    name: 'Управление питанием',
    tabClassName: 'tab-tuning',
    order: 30,
    icon: BatteryCharging,
    description: 'Точная настройка лимитов мощности CPU PL1/PL2/PL4, двухканальные кривые вентиляторов и температурные защитные пороги',
    isVisible: () => true,
  },
  {
    id: 'lighting',
    name: 'Мастерская RGB',
    tabClassName: 'tab-lighting',
    order: 40,
    icon: Palette,
    description: '4-зонная полноцветная подсветка клавиатуры, хвостовая световая полоса, логотип и пользовательские эффекты перелива',
    isVisible: (ctx) => {
      const type = ctx.capabilities?.rgb_keyboard?.type;
      return type !== 'none' && type !== 'monochrome_stepped';
    },
  },
  {
    id: 'water',
    name: 'Блок СЖО (BLE)',
    tabClassName: 'tab-water',
    order: 50,
    icon: Droplets,
    description: 'Bluetooth LE управление СЖО второго поколения, телеметрия температуры воды и умные температурные кривые',
    isVisible: (ctx) => ctx.isWaterCoolerEnabled,
    renderBadge: (ctx) => {
      if (ctx.isCoolerConnected) {
        return <span className="cooler-tab-dot" />;
      }
      return (
        <span
          style={{
            fontSize: '9px',
            opacity: 0.65,
            padding: '0 4px',
            borderRadius: 'var(--radius-xs)',
            background: 'var(--surface-badge)',
          }}
        >
          Нет связи
        </span>
      );
    },
  },
  {
    id: 'system',
    name: 'Система',
    tabClassName: 'tab-system',
    order: 60,
    icon: Layers,
    description: 'Матрица аппаратных переключателей, компоновка глобального слоя OSD, автозапуск и системные параметры',
    isEssential: true,
    isVisible: () => true,
  },
  {
    id: 'plugins',
    name: 'Плагины',
    tabClassName: 'tab-plugins',
    order: 65,
    icon: Puzzle,
    description: 'Включение/отключение встроенных основных модулей, сторонние расширения и управление пользовательскими плагинами',
    isEssential: true,
    isVisible: () => true,
  },
  {
    id: 'sponsor',
    name: 'Поддержать автора',
    tabClassName: 'tab-sponsor',
    order: 70,
    icon: Heart,
    iconClass: 'sponsor-tab-heart',
    description: 'Канал донатов автору открытого проекта, список благодарностей и история передачи активов реверс-инжиниринга оборудования',
    isEssential: true,
    isVisible: () => true,
  },
];

/**
 * 判断模块是否处于活跃激活状态：
 * 1. 若 config.json 中该模块显式设为 false，直接停用隐藏（排错安全模式）
 * 2. 否则按模块硬件条件与可见性函数判定
 */
export const isSlotActive = (slot: BuiltinModuleSlot, ctx: ModuleSlotContext): boolean => {
  if (ctx.moduleToggles && ctx.moduleToggles[slot.id] === false) {
    return false;
  }
  return slot.isVisible(ctx);
};
