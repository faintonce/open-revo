import React from 'react';
import { BiosMenuStatus } from '../../types/hardware';

export interface CpuBiosOcRowProps {
  biosStatus: BiosMenuStatus | null;
  biosOcEnabled: boolean;
  biosToggling: boolean;
  disabled?: boolean;
  onToggleBiosOc: () => void;
}

export const CpuBiosOcRow: React.FC<CpuBiosOcRowProps> = ({
  biosStatus,
  biosOcEnabled,
  biosToggling,
  disabled = false,
  onToggleBiosOc,
}) => {
  const isAmd = Boolean(biosStatus?.is_amd);
  const isUnsupported = disabled || Boolean(biosStatus?.hardware_read && !biosStatus.supported);
  const isDisabled = biosToggling || isAmd || isUnsupported;

  const descText = isAmd
    ? 'AMD 平台采用 SMU 动态功耗架构，无 Intel 格式 BIOS 超频菜单'
    : isUnsupported
      ? '当前机型主板或移动标压CPU未开放BIOS超频菜单'
      : biosStatus?.cpu_oc_supported && biosStatus?.mem_oc_supported
        ? `同步解锁 CPU 与内存超频菜单 (CPU: ${biosStatus.cpu_oc_enabled ? '已开' : '未开'}, 内存: ${biosStatus.mem_oc_enabled ? '已开' : '未开'})`
        : '向主板 UEFI NVRAM 同步写入 CPU 与内存超频菜单解锁标志位';

  const isPartial = Boolean(biosStatus?.cpu_oc_enabled || biosStatus?.mem_oc_enabled);

  const buttonText = biosToggling
    ? '写入中...'
    : isAmd || isUnsupported
      ? '不支持'
      : biosOcEnabled
        ? '已开启'
        : isPartial
          ? '部分开启'
          : '关闭';

  const getButtonBg = () => {
    if (isAmd || isUnsupported) return 'var(--surface-pill)';
    if (biosOcEnabled) return 'var(--status-info-dim)';
    if (isPartial) return 'var(--status-warn-dim)';
    return 'var(--surface-badge)';
  };

  const getButtonColor = () => {
    if (isAmd || isUnsupported) return 'var(--text-dim)';
    if (biosOcEnabled) return 'var(--status-info)';
    if (isPartial) return 'var(--status-warn)';
    return 'var(--text-muted)';
  };

  return (
    <div
      className="tuning-sub-card"
      style={{
        flexDirection: 'row',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginTop: '4px',
        gap: '8px',
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
        <span style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '11px', whiteSpace: 'nowrap' }}>
          高级 BIOS 超频菜单
        </span>
        <div style={{ fontSize: '10px', color: 'var(--text-dim)' }}>
          {descText}
        </div>
      </div>
      <button
        type="button"
        onClick={onToggleBiosOc}
        disabled={isDisabled}
        style={{
          padding: '3px 10px',
          borderRadius: 'var(--radius-xs)',
          fontSize: '10px',
          fontWeight: 600,
          border: 'none',
          cursor: isDisabled ? 'not-allowed' : 'pointer',
          background: getButtonBg(),
          color: getButtonColor(),
          whiteSpace: 'nowrap',
          opacity: isDisabled ? 0.5 : 1,
          transition: 'all var(--transition-fast)',
        }}
      >
        {buttonText}
      </button>
    </div>
  );
};
