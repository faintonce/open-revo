import React from 'react';
import {
  Cpu,
  AlertTriangle,
  CheckCircle2,
  RotateCcw,
  ShieldCheck,
  ShieldAlert,
  Lock,
  Unlock,
  Zap,
  Layers,
  PowerOff,
  Clock,
  X,
  ArrowRight,
} from 'lucide-react';
import { GpuCapabilities } from '../GpuModeView';
import { formatGpuShort, formatCpuShort } from './gpuUtils';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface GpuModeInfo {
  current_mode: string;
  has_igpu_present: boolean;
  has_dgpu?: boolean;
  gpu_name: string;
  platform?: string;
  cpu_name?: string;
  hardware_read?: boolean;
  error?: string | null;
  capabilities?: GpuCapabilities;
}

export interface PendingSwitch {
  targetMode: 'dgpu' | 'hybrid' | 'igpu';
  targetTitle: string;
  targetHex: string;
  currentTitle: string;
  currentHex: string;
}

export interface NoticeState {
  type: 'info' | 'success' | 'warning' | 'error';
  text: string;
}

// ─── Sub-components ───────────────────────────────────────────────────────────

const NoticeBanner: React.FC<{ notice: NoticeState; onClose: () => void }> = ({ notice, onClose }) => {
  const colorMap = {
    error:   { bg: 'rgba(239,68,68,0.12)',   border: 'rgba(239,68,68,0.35)',   text: '#f87171' },
    warning: { bg: 'rgba(245,158,11,0.12)',  border: 'rgba(245,158,11,0.35)', text: '#fbbf24' },
    success: { bg: 'rgba(16,185,129,0.12)',  border: 'rgba(16,185,129,0.35)', text: '#34d399' },
    info:    { bg: 'rgba(59,130,246,0.12)',  border: 'rgba(59,130,246,0.35)', text: '#60a5fa' },
  };
  const c = colorMap[notice.type];
  return (
    <div
      style={{
        padding: '10px 14px',
        background: c.bg,
        border: `1px solid ${c.border}`,
        borderRadius: '8px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '8px',
        fontSize: '11px',
        color: c.text,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        {notice.type === 'success'
          ? <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
          : <AlertTriangle className="w-4 h-4 flex-shrink-0" />}
        <div style={{ lineHeight: '1.4' }}>{notice.text}</div>
      </div>
      <button
        onClick={onClose}
        style={{ background: 'transparent', border: 'none', color: 'inherit', cursor: 'pointer', padding: '2px', display: 'flex', alignItems: 'center' }}
      >
        <X className="w-3.5 h-3.5 opacity-70 hover:opacity-100" />
      </button>
    </div>
  );
};

// ─── Main export ──────────────────────────────────────────────────────────────

export interface GpuModeCardsProps {
  gpuInfo: GpuModeInfo;
  loading: boolean;
  rebooting: boolean;
  notice: NoticeState | null;
  pendingSwitch: PendingSwitch | null;
  onCardClick: (mode: 'dgpu' | 'hybrid' | 'igpu') => void;
  onConfirmSwitch: (restartImmediately: boolean) => void;
  onCancelSwitch: () => void;
  onDismissNotice: () => void;
  onUnlockGeekIgpu?: (unlock: boolean) => void;
}

/**
 * GpuModeCards — 显卡模式三卡选择器 + 切换确认弹层。
 * 插件化路径：此组件未来可作为独立"显卡管理插件"的核心视图插槽。
 */
export const GpuModeCards: React.FC<GpuModeCardsProps> = ({
  gpuInfo,
  loading,
  rebooting,
  notice,
  pendingSwitch,
  onCardClick,
  onConfirmSwitch,
  onCancelSwitch,
  onDismissNotice,
  onUnlockGeekIgpu,
}) => {
  const hasPhysicalDgpu = gpuInfo.has_dgpu === true;
  const supportsMuxSwitch = hasPhysicalDgpu && (gpuInfo.capabilities?.supports_dgpu === true || gpuInfo.capabilities?.supports_igpu_only === true);
  const hasDgpu = supportsMuxSwitch;
  const isSafetyLocked = gpuInfo.capabilities?.is_safety_locked === true;
  const isUserUnlocked = gpuInfo.capabilities?.user_unlocked === true;
  const isTriModeSupported = isSafetyLocked
    ? isUserUnlocked
    : (gpuInfo.capabilities ? gpuInfo.capabilities.supports_igpu_only : true);
  const isIntel = gpuInfo.platform !== 'amd';

  const [showGeekUnlockModal, setShowGeekUnlockModal] = React.useState(false);
  const [geekConfirmedRisk, setGeekConfirmedRisk] = React.useState(false);

  const handleInternalCardClick = (mode: 'dgpu' | 'hybrid' | 'igpu') => {
    if (!hasPhysicalDgpu) {
      // 纯核显轻薄本架构，无板载独显硬件通道
      return;
    }
    if (mode === 'igpu' && isSafetyLocked && !isUserUnlocked) {
      setShowGeekUnlockModal(true);
      return;
    }
    onCardClick(mode);
  };

  // ─── Card renderer ────────────────────────────────────────────────────────

  const renderCard = (
    mode: 'dgpu' | 'hybrid' | 'igpu',
    opts: {
      accentColor: string;
      accentRgb: string;
      iconColor: string;
      iconEl: React.ReactNode;
      chipLabel: string;
      desc: string;
      titleText: string;
      isActive: boolean;
      isDisabled: boolean;
      disabledBadge?: React.ReactNode;
      activeBadgeColor: string;
      activeBadgeText: string;
    }
  ) => {
    const { accentRgb, iconColor, iconEl, chipLabel, desc, titleText, isActive, isDisabled, disabledBadge, activeBadgeColor, activeBadgeText } = opts;
    return (
      <div
        onClick={() => handleInternalCardClick(mode)}
        className={`glass-card gpu-mode-card ${isActive ? `active active-${mode}` : ''}`}
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '12px 14px',
          minHeight: '175px',
          cursor: loading || isDisabled ? 'not-allowed' : 'pointer',
          opacity: isDisabled ? 0.38 : 1,
          filter: isDisabled ? 'grayscale(0.6)' : 'none',
          transition: 'all 0.25s ease',
          position: 'relative',
          background: isActive
            ? `linear-gradient(180deg, rgba(${accentRgb}, 0.18) 0%, rgba(30,41,59,0.6) 100%)`
            : 'rgba(255,255,255,0.03)',
          borderColor: isActive ? `rgba(${accentRgb}, 0.7)` : 'var(--glass-border)',
          boxShadow: isActive ? `0 0 22px rgba(${accentRgb}, 0.3)` : 'none',
        }}
      >
        {/* Active badge */}
        {isActive && (
          <div
            className="gpu-mode-active-badge"
            style={{
              position: 'absolute', top: '10px', right: '10px',
              display: 'flex', alignItems: 'center', gap: '4px',
              fontSize: '9px', fontWeight: 700,
              color: activeBadgeColor,
              background: `${activeBadgeColor}26`,
              padding: '2px 7px', borderRadius: '4px',
              border: `1px solid ${activeBadgeColor}4d`,
            }}
          >
            <CheckCircle2 className="w-3 h-3" />
            <span>{activeBadgeText}</span>
          </div>
        )}

        {/* Disabled / locked badge */}
        {!isActive && disabledBadge}

        {/* No-dgpu badge (for dgpu/hybrid cards when no dedicated gpu) */}
        {!hasDgpu && mode !== 'igpu' && (
          <div
            style={{
              position: 'absolute', top: '10px', right: '10px',
              fontSize: '9px', fontWeight: 700,
              color: 'var(--text-muted)',
              background: 'rgba(148,163,184,0.12)',
              padding: '2px 7px', borderRadius: '4px',
              border: '1px solid rgba(148,163,184,0.25)',
            }}
          >
            <span>Нет оборудования dGPU</span>
          </div>
        )}

        {/* IC chip graphic */}
        <div
          style={{
            width: '52px', height: '52px',
            border: `2px dashed ${iconColor}`,
            borderRadius: '8px',
            display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
            background: 'rgba(15,23,42,0.7)',
            marginTop: '4px',
          }}
        >
          {iconEl}
          <span style={{ fontSize: '10px', fontWeight: 900, color: iconColor, letterSpacing: '0.08em' }}>
            {chipLabel}
          </span>
        </div>

        {/* Description */}
        <div style={{ textAlign: 'center', margin: '8px 0 6px' }}>
          <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: 0, lineHeight: '1.35' }}>
            {desc}
          </p>
        </div>

        {/* Title */}
        <span
          className="gpu-mode-title"
          style={{
            fontSize: '14px', fontWeight: 800, letterSpacing: '0.05em',
            color: isActive ? iconColor : (isDisabled ? 'var(--text-muted)' : 'var(--text-secondary)'),
          }}
        >
          {titleText}
        </span>
      </div>
    );
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', userSelect: 'none', position: 'relative' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <h2 style={{ fontSize: '18px', fontWeight: 800, letterSpacing: '0.03em', margin: 0, color: 'var(--text-primary)' }}>
              {hasDgpu
                ? (isTriModeSupported ? 'Режим графики (два GPU, три режима)' : 'Режим графики (два GPU, два режима)')
                : (hasPhysicalDgpu ? 'Архитектура графики (гибрид с dGPU)' : 'Архитектура графики (только iGPU, ультратонкая)')}
            </h2>
          </div>
          <p style={{ fontSize: '10.5px', color: 'var(--text-muted)', margin: '2px 0 0 0' }}>
            {hasDgpu
              ? (isTriModeSupported
                  ? 'Аппаратное переключение топологии MUX на уровне платы: процесс BIOS POST перенастраивает шину PCIe и физическое подключение экрана.'
                  : 'Платформа модели имеет двухрежимную архитектуру с двумя GPU (поддержка прямого вывода dGPU и гибридного вывода); включена аппаратная защитная блокировка от чёрного экрана и «окирпичивания».')
              : (hasPhysicalDgpu
                  ? 'Есть физическая dGPU и iGPU, гибридная топология NVIDIA Optimus: экран питается напрямую от iGPU, dGPU отвечает за производительный 3D.'
                  : 'Текущая модель — энергоэффективное решение только на iGPU: изображение выводится напрямую встроенной графикой CPU, изначально минимальное потребление и отличная автономность, переключение MUX не требуется.')}
          </p>
        </div>

        {/* Chip badge */}
        <div
          className="gpu-chip-badge"
          style={{
            display: 'flex', alignItems: 'center', gap: '6px',
            background: hasPhysicalDgpu ? 'rgba(99,102,241,0.1)' : 'rgba(16,185,129,0.1)',
            border: hasPhysicalDgpu ? '1px solid rgba(99,102,241,0.3)' : '1px solid rgba(16,185,129,0.3)',
            padding: '4px 8px', borderRadius: '6px',
          }}
        >
          <Cpu className={`w-3.5 h-3.5 ${hasPhysicalDgpu ? 'text-indigo-400' : 'text-emerald-400'} gpu-chip-icon`} />
          <span
            className="gpu-chip-text"
            style={{ fontSize: '10.5px', fontWeight: 700, color: hasPhysicalDgpu ? '#c7d2fe' : '#a7f3d0', letterSpacing: '0.02em' }}
          >
            {formatGpuShort(gpuInfo.gpu_name, hasPhysicalDgpu)} · {formatCpuShort(gpuInfo.cpu_name)}
          </span>
        </div>
      </div>

      {/* Architecture banner */}
      <div
        className="glass-card"
        style={{
          position: 'relative', overflow: 'hidden', padding: '8px 16px',
          background: 'linear-gradient(135deg, rgba(30,41,59,0.7), rgba(15,23,42,0.8))',
          display: 'flex', justifyContent: 'space-between', alignItems: 'center', minHeight: '52px',
        }}
      >
        <div style={{ zIndex: 2, maxWidth: '75%' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '2px' }}>
            <h3 style={{ fontSize: '13px', fontWeight: 800, color: 'var(--text-primary)', margin: 0, letterSpacing: '0.03em' }}>
              {hasDgpu
                ? (isTriModeSupported ? 'Изначальная аппаратная архитектура переключения MUX на плате' : 'Аппаратная защитная архитектура платы для двух GPU и двух режимов')
                : (hasPhysicalDgpu ? 'Динамическая гибридная архитектура NVIDIA Optimus' : 'Энергоэффективная ультратонкая архитектура только на iGPU (iGPU Only)')}
            </h3>
            <span style={{ fontSize: '10px', color: isIntel ? '#60a5fa' : '#fb923c', fontWeight: 700 }}>
              [{isIntel ? 'Intel' : 'AMD'} платформа]
            </span>
          </div>
          <div style={{ fontSize: '10.5px', color: 'var(--text-muted)', lineHeight: '1.4' }}>
            {hasDgpu ? (
              <div>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>Аппаратная реконфигурация:</span>
                Во время самопроверки перенастраиваются линии PCIe и физический переключатель экрана — полностью исключаются сбои драйвера, падение FPS и подвисания.
              </div>
            ) : hasPhysicalDgpu ? (
              <div>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>Динамическая гибридная координация:</span>
                При низкой нагрузке — тихо и экономно; при высокой нагрузке аппаратное ускорение физической dGPU подключается автоматически. На плате нет физического чипа MUX — ручная перезагрузка для переключения не нужна.
              </div>
            ) : (
              <div>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>Изначальное прямое управление iGPU:</span>
                Графика и видеосигнал полностью обслуживаются энергоэффективным iGPU, встроенным в CPU; физическое переключение MUX не требуется и не поддерживается.
              </div>
            )}
          </div>
        </div>

        <div
          className="gpu-mux-graphic"
          style={{
            height: '38px', padding: '0 12px',
            background: hasPhysicalDgpu
              ? 'linear-gradient(135deg, rgba(59,130,246,0.15), rgba(99,102,241,0.25))'
              : 'linear-gradient(135deg, rgba(16,185,129,0.15), rgba(59,130,246,0.2))',
            borderRadius: '8px',
            border: hasPhysicalDgpu ? '1px solid rgba(99,102,241,0.35)' : '1px solid rgba(16,185,129,0.35)',
            display: 'flex', alignItems: 'center', gap: '8px',
            boxShadow: hasPhysicalDgpu ? '0 0 14px rgba(59,130,246,0.12)' : '0 0 14px rgba(16,185,129,0.12)',
            flexShrink: 0,
          }}
        >
          <div
            style={{
              width: '26px', height: '26px',
              border: hasPhysicalDgpu ? '1.5px solid rgba(147,197,253,0.8)' : '1.5px solid rgba(52,211,153,0.8)',
              borderRadius: '5px', display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'rgba(15,23,42,0.8)',
            }}
          >
            {hasPhysicalDgpu
              ? <ShieldCheck className="w-3.5 h-3.5 text-blue-400 gpu-mux-icon" />
              : <Cpu className="w-3.5 h-3.5 text-emerald-400 gpu-mux-icon" />}
          </div>
          <span
            className="gpu-mux-text"
            style={{ fontSize: '9px', fontWeight: 700, color: hasPhysicalDgpu ? '#93c5fd' : '#6ee7b7', letterSpacing: '0.08em', whiteSpace: 'nowrap' }}
          >
            {hasDgpu ? 'HARDWARE MUX LINK' : (hasPhysicalDgpu ? 'OPTIMUS HYBRID LINK' : 'NATIVE iGPU SoC')}
          </span>
        </div>
      </div>

      {/* Notice banner */}
      {notice && <NoticeBanner notice={notice} onClose={onDismissNotice} />}

      {/* 3 Mode Cards (Unified for MUX laptops & Pure iGPU Ultrabooks) */}
      {hasDgpu || !hasPhysicalDgpu ? (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '14px' }}>
        {/* dGPU (轻薄本反向锁定) */}
        {renderCard('dgpu', {
          accentColor: !hasPhysicalDgpu ? '#f59e0b' : '#3b82f6',
          accentRgb: !hasPhysicalDgpu ? '245,158,11' : '59,130,246',
          iconColor: !hasPhysicalDgpu ? '#f59e0b' : '#60a5fa',
          iconEl: !hasPhysicalDgpu
            ? <Lock className="w-4 h-4 text-amber-400 mb-0.5" />
            : <Zap className="w-4 h-4 text-blue-400 mb-0.5" />,
          chipLabel: !hasPhysicalDgpu ? 'LOCK' : 'dGPU',
          desc: !hasPhysicalDgpu
            ? 'Текущая модель — ультрабук без дискретной графики: канал прямого вывода dGPU недоступен.'
            : 'dGPU физически подключена напрямую к встроенному экрану: минуя iGPU — максимальный FPS и сверхнизкая задержка для игр.',
          titleText: !hasPhysicalDgpu ? 'Прямой вывод dGPU (нет оборудования)' : 'Прямой вывод dGPU',
          isActive: hasPhysicalDgpu && gpuInfo.current_mode === 'dgpu',
          isDisabled: !hasPhysicalDgpu || !hasDgpu,
          disabledBadge: !hasPhysicalDgpu ? (
            <div
              className="gpu-mode-disabled-badge"
              style={{
                position: 'absolute', top: '10px', right: '10px',
                display: 'flex', alignItems: 'center', gap: '4px',
                fontSize: '9px', fontWeight: 700,
                color: '#f59e0b', background: 'rgba(245,158,11,0.15)',
                padding: '2px 7px', borderRadius: '4px',
                border: '1px solid rgba(245,158,11,0.3)',
              }}
              title="Текущая модель — ультрабук, без физической дискретной графики"
            >
              <Lock className="w-3 h-3 text-amber-400" />
              <span>Блокировка ультрабука</span>
            </div>
          ) : undefined,
          activeBadgeColor: '#38bdf8',
          activeBadgeText: 'Применяется на уровне железа',
        })}

        {/* Hybrid (轻薄本反向锁定) */}
        {renderCard('hybrid', {
          accentColor: !hasPhysicalDgpu ? '#f59e0b' : '#a855f7',
          accentRgb: !hasPhysicalDgpu ? '245,158,11' : '168,85,247',
          iconColor: !hasPhysicalDgpu ? '#f59e0b' : '#c084fc',
          iconEl: !hasPhysicalDgpu
            ? <Lock className="w-4 h-4 text-amber-400 mb-0.5" />
            : <Layers className="w-4 h-4 text-purple-400 mb-0.5" />,
          chipLabel: !hasPhysicalDgpu ? 'LOCK' : 'HYBRID',
          desc: !hasPhysicalDgpu
            ? 'Текущая модель — архитектура с прямым выводом только на iGPU, внешняя dGPU не участвует: гибридный канал недоступен.'
            : 'При низкой нагрузке вывод идёт через iGPU — экономно и тихо; при тяжёлой 3D-графике и играх dGPU ускорение подключается автоматически.',
          titleText: !hasPhysicalDgpu ? 'Гибридный вывод (нет оборудования)' : 'Гибридный вывод',
          isActive: hasPhysicalDgpu && gpuInfo.current_mode === 'hybrid',
          isDisabled: !hasPhysicalDgpu || !hasDgpu,
          disabledBadge: !hasPhysicalDgpu ? (
            <div
              className="gpu-mode-disabled-badge"
              style={{
                position: 'absolute', top: '10px', right: '10px',
                display: 'flex', alignItems: 'center', gap: '4px',
                fontSize: '9px', fontWeight: 700,
                color: '#f59e0b', background: 'rgba(245,158,11,0.15)',
                padding: '2px 7px', borderRadius: '4px',
                border: '1px solid rgba(245,158,11,0.3)',
              }}
              title="Текущая модель — ультрабук, без физической дискретной графики"
            >
              <Lock className="w-3 h-3 text-amber-400" />
              <span>Блокировка ультрабука</span>
            </div>
          ) : undefined,
          activeBadgeColor: '#c084fc',
          activeBadgeText: 'Применяется на уровне железа',
        })}

        {/* iGPU (轻薄本原生生效 / 双模机型安全锁定) */}
        {renderCard('igpu', {
          accentColor: hasPhysicalDgpu && !isTriModeSupported ? '#f59e0b' : '#10b981',
          accentRgb: hasPhysicalDgpu && !isTriModeSupported ? '245,158,11' : '16,185,129',
          iconColor: hasPhysicalDgpu && !isTriModeSupported ? '#f59e0b' : '#34d399',
          iconEl: hasPhysicalDgpu && !isTriModeSupported
            ? <ShieldAlert className="w-4 h-4 text-amber-400 mb-0.5" />
            : <PowerOff className="w-4 h-4 text-emerald-400 mb-0.5" />,
          chipLabel: hasPhysicalDgpu && !isTriModeSupported ? 'LOCK' : 'iGPU',
          desc: !hasPhysicalDgpu
            ? 'Изначально энергоэффективная архитектура на iGPU: графика и вывод на экран выполняются встроенным ядром напрямую, без затрат на dGPU и MUX — очень долгая автономность.'
            : (!isTriModeSupported
                ? 'Оборудование платформы — двухрежимная архитектура без физического переключателя «только iGPU». Включена защитная блокировка, полностью предотвращающая чёрный экран и зависание при старте.'
                : 'Полностью отсекает питание PCIe dGPU и её потребление в простое, максимально снижая энергопотребление и нагрев системы и резко увеличивая автономность.'),
          titleText: !hasPhysicalDgpu ? 'Прямой вывод iGPU' : (!isTriModeSupported ? 'Режим iGPU (блокировка двух режимов)' : 'Режим iGPU'),
          isActive: !hasPhysicalDgpu || (isTriModeSupported && gpuInfo.current_mode === 'igpu'),
          isDisabled: hasPhysicalDgpu && !isTriModeSupported,
          disabledBadge: hasPhysicalDgpu && !isTriModeSupported ? (
            <div
              className="gpu-mode-disabled-badge"
              style={{
                position: 'absolute', top: '10px', right: '10px',
                display: 'flex', alignItems: 'center', gap: '4px',
                fontSize: '9px', fontWeight: 700,
                color: '#f59e0b', background: 'rgba(245,158,11,0.15)',
                padding: '2px 7px', borderRadius: '4px',
                border: '1px solid rgba(245,158,11,0.3)',
                cursor: 'pointer',
              }}
              onClick={(e) => {
                e.stopPropagation();
                setShowGeekUnlockModal(true);
              }}
              title={gpuInfo.capabilities?.reason_igpu_unsupported || 'Аппаратная защита двух режимов двух GPU (по клику — разблокировка на свой риск)'}
            >
              <Lock className="w-3 h-3 text-amber-400" />
              <span>Блокировка защиты двух режимов</span>
            </div>
          ) : (hasPhysicalDgpu && isSafetyLocked && isUserUnlocked ? (
            <div
              style={{
                position: 'absolute', top: '10px', right: '10px',
                display: 'flex', alignItems: 'center', gap: '4px',
                fontSize: '9px', fontWeight: 700,
                color: '#38bdf8', background: 'rgba(56,189,248,0.15)',
                padding: '2px 7px', borderRadius: '4px',
                border: '1px solid rgba(56,189,248,0.35)',
                cursor: 'pointer',
              }}
              onClick={(e) => {
                e.stopPropagation();
                onUnlockGeekIgpu?.(false);
              }}
              title="Экспертное разблокирование на свой риск включено (по клику можно вернуть защитную блокировку двух режимов)"
            >
              <Unlock className="w-3 h-3 text-sky-400" />
              <span>Эксперт разблокирован</span>
            </div>
          ) : undefined),
          activeBadgeColor: '#34d399',
          activeBadgeText: !hasPhysicalDgpu ? 'Встроенный iGPU активен' : 'Применяется на уровне железа',
        })}
      </div>
    ) : (
      /* 仅适用于搭载物理独显但主板完全无物理 MUX 切换芯片的老式 Optimus 机型 */
      <div
        className="sys-sub-panel"
        style={{
          padding: '16px 20px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '16px',
          borderRadius: 'var(--radius-lg)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
          <div
            style={{
              width: '42px',
              height: '42px',
              borderRadius: 'var(--radius-md)',
              background: 'rgba(168, 85, 247, 0.15)',
              border: '1px solid rgba(168, 85, 247, 0.35)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexShrink: 0,
            }}
          >
            <Layers className="w-5 h-5 text-purple-400" />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span style={{ fontSize: '14px', fontWeight: 700, color: 'var(--text-primary)' }}>
                Гибридный вывод (NVIDIA Optimus)
              </span>
              <span
                style={{
                  fontSize: '10px',
                  fontWeight: 700,
                  color: '#c084fc',
                  background: 'rgba(168, 85, 247, 0.15)',
                  border: '1px solid rgba(168, 85, 247, 0.3)',
                  borderRadius: '4px',
                  padding: '1px 6px',
                }}
              >
                Применяется на уровне железа
              </span>
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-dim)', lineHeight: 1.5 }}>
              Текущая модель использует гибридную архитектуру (экран подключён к iGPU, dGPU отвечает за производительный 3D). На плате нет физического переключателя MUX — переключением автоматически управляют Windows и видеодрайвер, ручное переключение не требуется, что полностью исключает чёрные экраны и зависания при переключении. Управление яркостью, улучшение цвета и ICC-калибровка ниже работают в обычном режиме.
            </div>
          </div>
        </div>
        <div
          style={{
            fontSize: '11px',
            color: 'var(--text-muted)',
            textAlign: 'right',
            whiteSpace: 'nowrap',
            flexShrink: 0,
          }}
        >
          Нет физического переключателя MUX (автоматическая координация)
        </div>
      </div>
    )}

      {/* Mode switch confirmation modal */}
      {pendingSwitch && (
        <div
          style={{
            position: 'fixed', inset: 0, zIndex: 100,
            background: 'rgba(0,0,0,0.72)', backdropFilter: 'blur(8px)',
            display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '20px',
          }}
        >
          <div
            className="glass-card"
            style={{
              width: '100%', maxWidth: '460px',
              background: '#0f172a',
              border: '1px solid var(--border-medium)',
              borderRadius: '16px', padding: '24px',
              boxShadow: '0 25px 50px -12px rgba(0,0,0,0.6)',
              display: 'flex', flexDirection: 'column', gap: '18px',
              animation: 'fadeIn 0.2s ease-out',
            }}
          >
            {/* Modal header */}
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <div
                  style={{
                    width: '36px', height: '36px', borderRadius: '10px',
                    background: 'var(--status-warn-dim)',
                    border: '1px solid rgba(245,158,11,0.3)',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                  }}
                >
                  <RotateCcw className="w-5 h-5 text-amber-400" />
                </div>
                <div>
                  <h3 style={{ fontSize: '16px', fontWeight: 800, color: 'var(--text-primary)', margin: 0 }}>
                    Подсказка при переключении режима графики
                  </h3>
                  <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 0' }}>
                    Изменение конфигурации аппаратной шины UEFI завода MECHREVO
                  </p>
                </div>
              </div>
              {!loading && !rebooting && (
                <button
                  onClick={onCancelSwitch}
                  style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', padding: '4px', display: 'flex', alignItems: 'center' }}
                >
                  <X className="w-5 h-5 hover:text-white" />
                </button>
              )}
            </div>

            {/* Mode transition visual */}
            <div
              style={{
                background: 'rgba(30,41,59,0.5)',
                border: '1px solid var(--border-subtle)',
                borderRadius: '10px', padding: '12px 16px',
                display: 'flex', alignItems: 'center', justifyContent: 'space-between',
              }}
            >
              <div style={{ display: 'flex', flexDirection: 'column' }}>
                <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Текущий аппаратный режим</span>
                <span style={{ fontSize: '14px', fontWeight: 700, color: 'var(--text-secondary)' }}>{pendingSwitch.currentTitle}</span>
                <span style={{ fontSize: '9px', color: 'var(--text-dim)', fontFamily: 'monospace' }}>[{pendingSwitch.currentHex}]</span>
              </div>
              <ArrowRight size={18} className="text-blue-400 gpu-modal-arrow" />
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end' }}>
                <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Целевой режим переключения</span>
                <span className="gpu-modal-target-title" style={{ fontSize: '14px', fontWeight: 800, color: 'var(--status-info)' }}>{pendingSwitch.targetTitle}</span>
                <span className="gpu-modal-target-hex" style={{ fontSize: '9px', color: 'var(--status-info)', fontFamily: 'monospace' }}>[{pendingSwitch.targetHex}]</span>
              </div>
            </div>

            {/* Reinforced Hardware Switching Note */}
            <div
              style={{
                fontSize: '12px',
                color: 'var(--text-secondary)',
                lineHeight: '1.55',
                background: 'var(--status-warn-dim)',
                border: '1px solid rgba(245, 158, 11, 0.25)',
                borderRadius: 'var(--radius-md)',
                padding: '10px 14px',
              }}
            >
              <div>
                Переключить на [<strong style={{ color: 'var(--text-primary)' }}>{pendingSwitch.targetTitle}</strong>» затрагивает перепрограммирование чипа MUX в физических цепях встроенного дисплея и вступает в силу через перезагрузку системы, на этапе POST BIOS материнской платы.
              </div>
              <div style={{ marginTop: '6px', color: 'var(--status-info)', fontSize: '11px', fontWeight: 600 }}>
                🛡️ Усиленная защита OpenRevo от чёрного экрана: перед перезагрузкой автоматически пробуждает шину питания iGPU, обновляет дерево PnP-устройств и выполняет стабильную запись SPI NVRAM, обходя ловушку быстрого запуска.
              </div>
              <div style={{ marginTop: '4px', color: 'var(--text-muted)', fontSize: '10.5px' }}>
                💡 Совет на крайний случай: если на отдельных моделях после перезагрузки появляется чёрный экран — удерживайте кнопку питания 15 секунд; EC/BIOS полностью сбросится и изображение вернётся к настройкам по умолчанию.
              </div>
            </div>

            {/* Actions */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '4px' }}>
              <button
                disabled={loading || rebooting}
                onClick={() => onConfirmSwitch(true)}
                className="gpu-modal-reboot-btn"
                style={{
                  width: '100%',
                  padding: '12px',
                  borderRadius: 'var(--radius-md)',
                  background: 'linear-gradient(135deg, #3b82f6 0%, #2563eb 100%)',
                  border: '1px solid rgba(147, 197, 253, 0.4)',
                  color: '#fff',
                  fontSize: '13px',
                  fontWeight: 700,
                  cursor: loading ? 'not-allowed' : 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '8px',
                  boxShadow: '0 4px 14px rgba(37, 99, 235, 0.4)',
                  transition: 'all 0.2s ease',
                }}
              >
                <RotateCcw className={`w-4 h-4 ${rebooting ? 'animate-spin' : ''}`} />
                <span>{rebooting ? 'Выполняется плавный безопасный перезапуск...' : 'Немедленный плавный перезапуск (рекомендуется)'}</span>
              </button>

              <button
                disabled={loading || rebooting}
                onClick={() => onConfirmSwitch(false)}
                style={{
                  width: '100%',
                  padding: '10px',
                  borderRadius: 'var(--radius-md)',
                  background: 'var(--surface-pill)',
                  border: '1px solid var(--border-subtle)',
                  color: 'var(--text-secondary)',
                  fontSize: '12px',
                  fontWeight: 600,
                  cursor: loading ? 'not-allowed' : 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '6px',
                  transition: 'all 0.2s ease',
                }}
              >
                <Clock className="w-4 h-4 text-slate-400" />
                <span>Перезагрузить позже (вступит в силу при следующем включении)</span>
              </button>

                  {!rebooting && (
                    <button
                      disabled={loading}
                      onClick={onCancelSwitch}
                      style={{
                        width: '100%', padding: '8px', borderRadius: '8px',
                        background: 'transparent', border: 'none',
                        color: 'var(--text-muted)', fontSize: '12px',
                        cursor: 'pointer', transition: 'color 0.2s ease',
                      }}
                    >
                      Отмена
                    </button>
                  )}
            </div>
          </div>
        </div>
      )}

      {/* Geek unlock disclaimer modal */}
      {showGeekUnlockModal && (
        <div
          className="sys-modal-backdrop"
          onClick={() => {
            setShowGeekUnlockModal(false);
            setGeekConfirmedRisk(false);
          }}
        >
          <div
            className="sys-modal-card danger"
            onClick={(e) => e.stopPropagation()}
            style={{ maxWidth: '520px' }}
          >
            <div className="sys-modal-header">
              <div className="sys-modal-icon-badge danger">
                <AlertTriangle size={20} />
              </div>
              <div>
                <div className="sys-modal-title">Аппаратная защита двух режимов двух GPU · экспертная разблокировка на свой риск</div>
                <div className="sys-modal-subtitle">Предотвращает постоянный чёрный экран и «окирпичивание» при старте, если на плате нет отдельного байпасного переключателя «только iGPU»</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                ⚠️ Предупреждение о серьёзном аппаратном риске (Hardware Brick Risk):
              </div>
              <ul className="sys-modal-list">
                <li>
                  <strong>Электрические особенности платформы</strong>: шасси текущего устройства (например Yilong 16 Pro и другие разработки TRX1/HP45) — официальная плата<strong>Есть только цепи двух GPU и двух режимов</strong>, встроенный экран обычно не имеет распаянного отдельного байпасного переключателя прямого вывода iGPU.
                </li>
                <li>
                  <strong>Последствие — постоянный чёрный экран</strong>: если принудительно записать этот режим на плате, чей BIOS не поддерживает «только iGPU», после перезагрузки встроенный экран<strong>Постоянный чёрный экран, без подсветки и сигналов самотестирования</strong>, восстановить можно только сбросом CMOS с разбором и обесточиванием или программатором!
                </li>
                <li>
                  <strong>Принципы решения на свой риск</strong>: OpenRevo принципиально не использует базы предположений и никогда не рискует «окирпичить» устройство за пользователя. Если вы уже<strong>Перезагрузитесь в расширенные настройки BIOS и убедитесь лично</strong>существует и поддерживает режим «iGPU Only» — отметьте дисклеймер ниже для принудительной разблокировки.
                </li>
              </ul>

              <label
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: '10px',
                  marginTop: '14px',
                  cursor: 'pointer',
                  fontSize: '12px',
                  color: 'var(--text-primary)',
                  lineHeight: '1.45',
                  padding: '10px',
                  background: 'var(--status-warn-dim)',
                  borderRadius: '8px',
                  border: '1px solid rgba(245,158,11,0.25)',
                }}
              >
                <input
                  type="checkbox"
                  checked={geekConfirmedRisk}
                  onChange={(e) => setGeekConfirmedRisk(e.target.checked)}
                  style={{ marginTop: '2px', cursor: 'pointer' }}
                />
                <span>
                  Я лично убедился в BIOS платы в наличии опции «только iGPU», осознаю риск чёрного экрана и «окирпичивания» и добровольно принимаю последствия для оборудования.
                </span>
              </label>
            </div>

            <div className="sys-modal-actions">
              <button
                onClick={() => {
                  setShowGeekUnlockModal(false);
                  setGeekConfirmedRisk(false);
                }}
                className="toggle-btn"
              >
                Отмена (рекомендуется сохранить защитную блокировку)
              </button>
              <button
                disabled={!geekConfirmedRisk}
                onClick={() => {
                  setShowGeekUnlockModal(false);
                  setGeekConfirmedRisk(false);
                  onUnlockGeekIgpu?.(true);
                }}
                className={`toggle-btn ${geekConfirmedRisk ? 'btn-danger-solid' : ''}`}
                style={{
                  opacity: geekConfirmedRisk ? 1 : 0.45,
                  cursor: geekConfirmedRisk ? 'pointer' : 'not-allowed',
                }}
              >
                Принудительно снять защитную блокировку (на свой риск)
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
