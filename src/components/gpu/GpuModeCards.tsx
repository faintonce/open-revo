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
            <span>无独显硬件</span>
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
                ? (isTriModeSupported ? '显卡模式 (双显三模)' : '显卡模式 (双显双模)')
                : (hasPhysicalDgpu ? '显卡架构 (独显混合输出架构)' : '显卡架构 (纯核显轻薄架构)')}
            </h2>
          </div>
          <p style={{ fontSize: '10.5px', color: 'var(--text-muted)', margin: '2px 0 0 0' }}>
            {hasDgpu
              ? (isTriModeSupported
                  ? '主板硬件级 MUX 拓扑切换，由 BIOS POST 过程重配 PCIe 总线与屏幕物理连接。'
                  : '当前机型模具为双显双模架构 (支持独显直连与混合输出)，已启用硬件安全保护锁定以防止黑屏变砖。')
              : (hasPhysicalDgpu
                  ? '搭载物理独立显卡与核显，采用 NVIDIA Optimus 混合拓扑。屏幕内屏由核显直接输出，独显负责高性能 3D 加速。'
                  : '当前机型为高能效纯核显设计，由 CPU 内建核显直出，原生享有超低功耗与极佳续航，无需切换 MUX。')}
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
                ? (isTriModeSupported ? '主板原生硬件 MUX 切换架构' : '主板双显双模安全保护架构')
                : (hasPhysicalDgpu ? 'NVIDIA Optimus 动态混合协同架构' : '高能效纯核显轻薄架构 (iGPU Only)')}
            </h3>
            <span style={{ fontSize: '10px', color: isIntel ? '#60a5fa' : '#fb923c', fontWeight: 700 }}>
              [{isIntel ? 'Intel' : 'AMD'} 平台架构]
            </span>
          </div>
          <div style={{ fontSize: '10.5px', color: 'var(--text-muted)', lineHeight: '1.4' }}>
            {hasDgpu ? (
              <div>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>物理硬件重构：</span>
                自检过程重配 PCIe 通道与屏幕物理开关，彻底杜绝驱动崩溃、掉帧与卡顿。
              </div>
            ) : hasPhysicalDgpu ? (
              <div>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>动态混合协同：</span>
                轻载静音省电，高负载自动调用物理独显硬件加速。主板无物理 MUX 切换芯片，无需手动重启切换。
              </div>
            ) : (
              <div>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>原生核显直驱：</span>
                图形与显示信号由 CPU 内建的高能效核显统一接管，无需且不支持 MUX 物理切换。
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
            ? '当前机型为轻薄本架构，未搭载独立显卡硬件，独显直连通道不可用。'
            : '独立显卡物理直连内置屏幕。避开核显中转，发挥极致电竞帧率与超低延迟。',
          titleText: !hasPhysicalDgpu ? '独显直连 (无硬件)' : '独显直连',
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
              title="当前机型为轻薄本架构，无物理独立显卡"
            >
              <Lock className="w-3 h-3 text-amber-400" />
              <span>轻薄本锁定</span>
            </div>
          ) : undefined,
          activeBadgeColor: '#38bdf8',
          activeBadgeText: '硬件生效中',
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
            ? '当前机型为纯核显直驱架构，无外部独立显卡介入加速，混合通道不可用。'
            : '轻载时核显输出以省电静音，高负载 3D 图形与游戏时自动介入独显加速。',
          titleText: !hasPhysicalDgpu ? '混合输出 (无硬件)' : '混合输出',
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
              title="当前机型为轻薄本架构，无物理独立显卡"
            >
              <Lock className="w-3 h-3 text-amber-400" />
              <span>轻薄本锁定</span>
            </div>
          ) : undefined,
          activeBadgeColor: '#c084fc',
          activeBadgeText: '硬件生效中',
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
            ? '原生高能效核显架构。图形与屏幕显示由内建核心显卡直出，无独显与 MUX 开销，享有极高续航。'
            : (!isTriModeSupported
                ? '当前模具硬件为双模架构，无物理核显直连开关。已锁定保护以彻底防止开机黑屏死锁。'
                : '彻底切断独立显卡 PCIe 供电与待机功耗，极致降低整机能耗与发热，极大提升续航。'),
          titleText: !hasPhysicalDgpu ? '原生核显直出' : (!isTriModeSupported ? '核显模式 (双模锁定)' : '核显模式'),
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
              title={gpuInfo.capabilities?.reason_igpu_unsupported || '双显双模硬件保护 (点击可自主免责解锁)'}
            >
              <Lock className="w-3 h-3 text-amber-400" />
              <span>双模保护锁定</span>
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
              title="极客已自主免责解锁（点击可重新恢复双模安全保护锁定）"
            >
              <Unlock className="w-3 h-3 text-sky-400" />
              <span>极客已解锁</span>
            </div>
          ) : undefined),
          activeBadgeColor: '#34d399',
          activeBadgeText: !hasPhysicalDgpu ? '原生核显生效中' : '硬件生效中',
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
                混合输出模式 (NVIDIA Optimus)
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
                硬件生效中
              </span>
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-dim)', lineHeight: 1.5 }}>
              当前机型为独显混合输出架构（由核显直连屏幕输出，独立显卡负责高性能 3D 加速）。主板硬件未设计 MUX 物理切换开关，固定由 Windows 与显卡驱动自动协同调度，无需手动切换，杜绝任何切换黑屏与死机风险。下方亮度控制、色彩增强与 ICC 校色均可正常调节。
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
          无物理 MUX 开关 (自动协同)
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
                    切换显卡模式提示
                  </h3>
                  <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 0' }}>
                    机械革命原厂 UEFI 硬件总线配置变更
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
                <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>当前硬件模式</span>
                <span style={{ fontSize: '14px', fontWeight: 700, color: 'var(--text-secondary)' }}>{pendingSwitch.currentTitle}</span>
                <span style={{ fontSize: '9px', color: 'var(--text-dim)', fontFamily: 'monospace' }}>[{pendingSwitch.currentHex}]</span>
              </div>
              <ArrowRight size={18} className="text-blue-400 gpu-modal-arrow" />
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end' }}>
                <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>目标切换模式</span>
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
                切换至【<strong style={{ color: 'var(--text-primary)' }}>{pendingSwitch.targetTitle}</strong>】涉及显示内屏物理走线 MUX 芯片重配，需要通过系统重启由主板 BIOS POST 阶段生效。
              </div>
              <div style={{ marginTop: '6px', color: 'var(--status-info)', fontSize: '11px', fontWeight: 600 }}>
                🛡️ OpenRevo 强化防黑屏保护：重启前自动唤醒核显供电总线、刷新 PnP 设备树并执行 SPI NVRAM 稳定写入，跳过快速启动陷阱。
              </div>
              <div style={{ marginTop: '4px', color: 'var(--text-muted)', fontSize: '10.5px' }}>
                💡 极客急救锦囊：若个别机型在极端情况下重启遭遇黑屏，长按电源键 15 秒即可彻底复位 EC/BIOS 恢复默认显示。
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
                <span>{rebooting ? '正在平稳安全重启...' : '立即平稳重启 (推荐)'}</span>
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
                <span>稍后重启 (下次开机生效)</span>
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
                      取消
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
                <div className="sys-modal-title">双显双模硬件保护 · 极客免责解锁</div>
                <div className="sys-modal-subtitle">防止主板无纯核显独立旁路开关导致开机永久黑屏变砖</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                ⚠️ 严重硬件安全风险警告 (Hardware Brick Risk):
              </div>
              <ul className="sys-modal-list">
                <li>
                  <strong>模具电路特征</strong>：当前机器底盘（如翼龙 16 Pro 等 TRX1/HP45 代工设计）官方主板<strong>仅具备双显双模电路</strong>，内置屏幕通常未焊装旁路独立核显直连开关。
                </li>
                <li>
                  <strong>永久黑屏后果</strong>：若在 BIOS 不支持纯核显的主板上强行写入此模式，电脑重启后内屏将<strong>永久黑屏、无背光且无自检信号</strong>，只能通过拆机断电复位 CMOS 甚至烧录器救砖！
                </li>
                <li>
                  <strong>免责决策原则</strong>：OpenRevo 坚决不使用模糊猜测数据库，绝不替用户冒变砖风险。若您已<strong>重启进入 BIOS 高级设置亲眼确认</strong>存在并支持『iGPU Only』模式，可勾选下方免责声明强制解锁。
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
                  我已在主板 BIOS 中亲眼确认具备纯核显选项，已知晓黑屏变砖风险并自愿承担硬件后果。
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
                取消 (推荐保持安全锁定)
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
                强制解除保护锁定 (自负风险)
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
