export interface LightingState {
  kb_enabled: boolean;
  kb_effect: number;
  kb_brightness: number;
  kb_speed: number;
  kb_direction: number;
  kb_color: string;

  // Streamer & Custom Engine
  kb_engine?: 'hardware' | 'streamer' | 'custom';
  kb_fps?: number;
  streamer_effect?: number;
  custom_scale?: number;
  custom_reactive?: boolean;
  custom_script_id?: string;

  // Four-Zone Keyboard Colors
  kb_zones?: [string, string, string, string];

  lb_enabled: boolean;
  lb_effect: number;
  lb_brightness: number;
  lb_speed: number;
  lb_color: string;

  // Rear Vent & Hinge Ambient Light (尾部转轴/出风口氛围灯)
  hinge_enabled: boolean;
  hinge_effect: number;
  hinge_brightness: number;
  hinge_speed: number;
  hinge_color: string;

  logo_enabled: boolean;
  logo_effect: number;
  logo_brightness: number;
  logo_color: string;

  sleep_minutes: number;
  ceremony_enabled?: boolean;
}

export interface KeyboardHardwareInfo {
  hardware_type: string; // 'PerKeyRgb' | 'FourZoneRgb' | 'SingleZoneRgb' | 'SingleColor' | 'None'
  has_ite8291: boolean;
  has_ite7001: boolean;
  has_ite7000?: boolean;
  has_front_lightbar?: boolean;
  is_ec_lightbar?: boolean;
  has_logo?: boolean;
  has_usb_logo?: boolean;
  is_ec_logo?: boolean;
  has_hinge?: boolean;
  is_ec_hinge?: boolean;
  is_four_zone?: boolean;
  description: string;
  brightness_level: number;
  is_on: boolean;
}

// 4-Zone RGB Keyboard Modes (Factory Hardware MCU Effects for ITE 048D:CE00)
export const FOURZONE_KEYBOARD_MODES = [
  { id: 1, name: 'Постоянная: моно / 4 зоны', desc: 'Статичная одноцветная подсветка; настройка 4 зон независимо' },
  { id: 2, name: 'Монохромное дыхание', desc: 'Плавное градиентное гало, ритмичное дыхание' },
  { id: 3, name: 'Полноцветные волны', desc: 'Полноцветные переливающиеся волны по 4 зонам' },
  { id: 5, name: 'Цикл радуги', desc: 'Плавная смена семицветного круга оттенков синхронно по 4 зонам' },
  { id: 12, name: 'Мерцание со скачками', desc: 'Динамичное мерцание со скачками по 4 зонам' },
  { id: 13, name: 'Смешанный перелив', desc: 'Чередующиеся многоцветные переливы по 4 зонам' },
];

// Factory Hardware MCU Built-in Effects (Direct ITE 8291 execution, 0% CPU)
export const KEYBOARD_MODES = [
  { id: 1, name: 'Постоянный монохром', desc: 'Классическая статичная подсветка чистым цветом' },
  { id: 2, name: 'Монохромное дыхание', desc: 'Дыхание плавного градиентного гало' },
  { id: 3, name: 'Волны перелива', desc: 'Полноцветные переливающиеся волны' },
  { id: 4, name: 'Слабое свечение клавиш', desc: 'Вспышка в месте нажатия клавиши' },
  { id: 5, name: 'Радужный круг оттенков', desc: 'Плавный цикл семицветного кольца' },
  { id: 6, name: 'Расходящаяся рябь', desc: 'Расходится от нажатой клавиши наружу' },
  { id: 10, name: 'Случайные капли', desc: 'Эффект случайно падающих капель-матрицы' },
  { id: 9, name: 'Полноцветный бегущий огонь', desc: 'Полноцветный циклический бегущий огонь' },
  { id: 17, name: 'Яркий фейерверк', desc: 'Яркий фейерверк расцветает под пальцами' },
  { id: 14, name: 'Тихое сияние', desc: 'Тихое спокойное течение северного сияния' },
  { id: 21, name: 'Киберспортивная подсветка', desc: 'Подсветка WASD и ключевых игровых клавиш' },
];

// BetterRGB 60FPS Streamer High-Refresh Effects (100+ Prefix Namespace to isolate from Hardware MCU 1..21)
export const STREAMER_OFFSET = 100;
export const STREAMER_MODES = [
  { id: 101, name: 'Волна (Wave)', desc: 'Полноцветные переливающиеся волны; плавная проекция в четыре направления' },
  { id: 100, name: 'Дыхание (Breath)', desc: 'Мягкое дыхание слабого свечения по высокоточной синусоиде' },
  { id: 102, name: 'Рябь (Ripple)', desc: 'Фирменные концентрические импульсы, плавно расходящиеся наружу' },
  { id: 103, name: 'Текучий песок (Quicksand)', desc: 'Четырёхкратное сглаживание со сверхдискретизацией; двойная синусоидальная интерференция — сверхплавно' },
  { id: 104, name: 'Ток (Current)', desc: 'Восьмикратное сглаживание пульсирующих импульсов; перелив с экспоненциальным затуханием' },
  { id: 105, name: 'Цветовое колесо (Wheel)', desc: 'Радужная «вертушка», вращающаяся из центра в полярных координатах' },
  { id: 106, name: 'Радуга (Rainbow)', desc: 'Синхронное плавное вращение круга оттенков по всем клавишам' },
  { id: 107, name: 'Матрица (Matrix)', desc: 'Цифровой дождь «Матрицы»: динамические «головы» потоков и послесвечение' },
  { id: 108, name: 'Молния (Lightning)', desc: 'Гроза: молнии ветвятся, ударяют вниз и вспыхивают' },
  { id: 109, name: 'Пламя (Flame)', desc: 'Пламя клубится снизу, поднимаясь конвекцией и затухая' },
  { id: 110, name: 'Дождь (Rain)', desc: 'Лёгкий дождь падает случайно, вода слегка рябит' },
  { id: 111, name: 'Звёздная пыль (Sparkle)', desc: 'Вся клавиатура мерцает звёздами, слабое свечение случайно угасает' },
  { id: 112, name: 'Двунаправленный ток (Touch Current)', desc: 'Встречные высокочастотные электрические импульсы' },
  { id: 113, name: 'Постоянный одноцветный (Static)', desc: 'Поклавишная чистая фиксация цвета с точностью до пикселя' },
];

export const LIGHTBAR_MODES = [
  { id: 1, name: 'Постоянный монохром', desc: 'Классическая статичная подсветка чистым цветом' },
  { id: 2, name: 'Энергичное дыхание', desc: 'Одноцветное мягкое гало, ритмичное дыхание' },
  { id: 20, name: 'Радужное дыхание', desc: 'Градиентное дыхание семицветного круга оттенков во всём спектре' },
  { id: 3, name: 'Полноцветные волны', desc: 'Переливающиеся волны с плавным течением' },
  { id: 4, name: 'Динамические столкновения', desc: 'Перелив, встречно сталкивающийся от обоих концов корпуса к центру' },
  { id: 6, name: 'Быстрые метеоры', desc: 'Однонаправленный перелив проносится по передней кромке' },
  { id: 9, name: 'Бегущий огонь', desc: 'Полноцветный бегущий огонь по кругу с преследованием' },
  { id: 10, name: 'Яркое мерцание', desc: 'Случайное мерцание звёзд на передней матрице' },
];

export const LOGO_MODES = [
  { id: 1, name: 'Постоянный монохром', desc: 'Классическая постоянная подсветка механического логотипа MECHREVO' },
  { id: 2, name: 'Монохромное дыхание', desc: 'Медленное дыхание слабого свечения заданного цвета' },
  { id: 20, name: 'Радужное дыхание', desc: 'Мягкое дыхание семицветного круга оттенков во всём спектре' },
  { id: 3, name: 'Полноцветные волны', desc: 'Полноцветные плавные переливающиеся волны' },
  { id: 4, name: 'Динамические столкновения', desc: 'Динамичные встречные импульсы энергии при столкновении' },
  { id: 6, name: 'Летящие метеоры', desc: 'Перелив, проносящийся по логотипу' },
  { id: 9, name: 'Полноцветный бегущий огонь', desc: 'Бегущий огонь по контуру текста с преследованием' },
];

// Single-point direct EC PWM badges (e.g. YAOSHI 16 / L16Pro / CANGLONG shield badge)
export const EC_LOGO_MODES = [
  { id: 1, name: 'Постоянный монохром', desc: 'Щит-логотип: постоянное свечение статическим цветом' },
  { id: 2, name: 'Энергичное дыхание', desc: 'Щит-логотип: ритмичное дыхание с появлением и затуханием слабого свечения' },
  { id: 20, name: 'Радужное дыхание', desc: 'Щит-логотип: дыхание с полноцветным циклическим градиентом' },
];

export const PRESET_COLORS = [
  { name: 'Механический бирюзовый', hex: '#00FFFF' },
  { name: 'Киберспортивный фиолет', hex: '#6366F1' },
  { name: 'Зелёное сияние', hex: '#10B981' },
  { name: 'Пламенный красный', hex: '#EF4444' },
  { name: 'Cyber-золото', hex: '#F59E0B' },
  { name: 'Фантомный розовый', hex: '#EC4899' },
  { name: 'Ледяной синий', hex: '#3B82F6' },
  { name: 'Чистый белый', hex: '#FFFFFF' },
];

export const SLEEP_OPTIONS = [
  { label: 'Никогда не гаснет', value: 0 },
  { label: '1 минута', value: 1 },
  { label: '5 минут', value: 5 },
  { label: '10 минут', value: 10 },
  { label: '15 минут', value: 15 },
  { label: '30 минут', value: 30 },
];

export interface BrfxScript {
  id: string;
  name: string;
  author: string;
  description: string;
  version?: string;
  speed: number;       // 0.2 ~ 3.0
  scale: number;       // 0.5 ~ 2.5
  direction: number;   // 0: R, 1: L, 2: U, 3: D
  reactive: boolean;   // Keypress reaction
  color: string;       // "#RRGGBB"
  code: string;
  filePath?: string;
  builtinEffectId?: number; // Corresponding native engine effect ID (e.g. 200, 201, 202)
  customText?: string;
}

