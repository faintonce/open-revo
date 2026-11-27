export interface CoolerCurvePoint {
  temp: number;
  fan_duty: number;
  pump_voltage: number;
}

export interface WaterCoolerDevice {
  mac_address: string;
  mac_u64: number;
  name: string;
  rssi?: number;
  is_paired: boolean;
  is_current: boolean;
}

export interface CoolerTelemetry {
  connected: boolean;
  is_flow_ok: boolean;
  meter_state: number; // 0=No Flow, 1=Normal, 2=Priming/Trickle
  pump_voltage: number;
  pump_tier?: number;  // 0=Stop, 1=Silent, 2=Standard, 3=Turbo
  fan_duty: number;
  pump_rpm: number;
  running: boolean;
  safety_tripped: boolean;
  is_priming: boolean;
  led_on: boolean;
  led_r: number;
  led_g: number;
  led_b: number;
  led_mode: number;
  mac_address?: string;
  strategy?: string;
  curve_points?: CoolerCurvePoint[];
  pump_start_temp?: number;
  pump_turbo_temp?: number;
}

export const DEFAULT_COOLER_CURVE: CoolerCurvePoint[] = [
  { temp: 45, fan_duty: 0, pump_voltage: 0 },
  { temp: 55, fan_duty: 35, pump_voltage: 2 },
  { temp: 65, fan_duty: 50, pump_voltage: 2 },
  { temp: 80, fan_duty: 60, pump_voltage: 2 },
  { temp: 90, fan_duty: 75, pump_voltage: 2 },
  { temp: 95, fan_duty: 100, pump_voltage: 3 },
];

export const COOLER_COLOR_PRESETS = [
  { name: 'Классический изумрудный', r: 0, g: 128, b: 0, hex: '#008000' },
  { name: 'Ледниковый cyber-бирюзовый', r: 0, g: 229, b: 255, hex: '#00e5ff' },
  { name: 'Быстрое пламя — красный', r: 255, g: 23, b: 68, hex: '#ff1744' },
  { name: 'Фантастический фиолет ночи', r: 213, g: 0, b: 249, hex: '#d500f9' },
  { name: 'Золотистый тёплый оранж', r: 255, g: 145, b: 0, hex: '#ff9100' },
  { name: 'Чисто-белая линия горизонта', r: 255, g: 255, b: 255, hex: '#ffffff' },
];
