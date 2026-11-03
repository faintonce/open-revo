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
  gpu_telemetry_backend?: string | null;
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
  cpu_power_backend?: string;
  cpu_power_max_watts?: number | null;
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
    name: '状态总览',
    tabClassName: 'tab-overview',
    order: 10,
    icon: Gauge,
    description: '整机实时传感器监控、模式切换仪表盘与电池健康看板',
    isVisible: () => true,
  },
  {
    id: 'gpu',
    name: '显卡模式',
    tabClassName: 'tab-gpu',
    order: 20,
    icon: Cpu,
    description: '独显直连/混合输出 MUX 切换、动态超频、屏幕亮度与 ICC 校色',
    isVisible: () => true,
  },
  {
    id: 'tuning',
    name: '电源管理',
    tabClassName: 'tab-tuning',
    order: 30,
    icon: BatteryCharging,
    description: 'CPU PL1/PL2/PL4 功耗墙精调、风扇双通道曲线与温度保护墙',
    isVisible: () => true,
  },
  {
    id: 'lighting',
    name: 'RGB 灯效工坊',
    tabClassName: 'tab-lighting',
    order: 40,
    icon: Palette,
    description: '四区全彩背光键盘、尾翼光带、Logo 灯与自定义流光特效',
    isVisible: (ctx) => {
      const type = ctx.capabilities?.rgb_keyboard?.type;
      return type !== 'none' && type !== 'monochrome_stepped';
    },
  },
  {
    id: 'water',
    name: '水冷魔盒 (BLE)',
    tabClassName: 'tab-water',
    order: 50,
    icon: Droplets,
    description: '第二代外置水冷低功耗蓝牙控制、水温遥测与智能温控曲线',
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
          未连
        </span>
      );
    },
  },
  {
    id: 'system',
    name: '系统',
    tabClassName: 'tab-system',
    order: 60,
    icon: Layers,
    description: '硬件开关矩阵、全局 OSD 浮层排版、自启动及系统参数设置',
    isEssential: true,
    isVisible: () => true,
  },
  {
    id: 'plugins',
    name: '插件',
    tabClassName: 'tab-plugins',
    order: 65,
    icon: Puzzle,
    description: '内置核心功能模块启闭、第三方扩展与自定义插件管理',
    isEssential: true,
    isVisible: () => true,
  },
  {
    id: 'sponsor',
    name: '支持作者',
    tabClassName: 'tab-sponsor',
    order: 70,
    icon: Heart,
    iconClass: 'sponsor-tab-heart',
    description: '开源作者打赏通道、鸣谢名单与硬件逆向资产交接记录',
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
