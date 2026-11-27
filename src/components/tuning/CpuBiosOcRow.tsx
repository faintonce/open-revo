import React from 'react';
import { BiosMenuStatus } from '../../types/hardware';

export interface CpuBiosOcRowProps {
  biosStatus: BiosMenuStatus | null;
  biosOcEnabled: boolean;
  biosToggling: boolean;
  onToggleBiosOc: () => void;
}

export const CpuBiosOcRow: React.FC<CpuBiosOcRowProps> = ({
  biosStatus,
  biosOcEnabled,
  biosToggling,
  onToggleBiosOc,
}) => {
  const isAmd = Boolean(biosStatus?.is_amd);
  const isUnsupported = Boolean(biosStatus?.hardware_read && !biosStatus.supported);
  const isDisabled = biosToggling || isAmd || isUnsupported;

  const descText = isAmd
    ? 'Платформы AMD используют динамическую архитектуру мощности SMU — меню разгона в формате Intel в BIOS отсутствует'
    : isUnsupported
      ? 'Плата текущей модели или мобильный CPU с полным TDP не открывают меню разгона BIOS'
      : biosStatus?.cpu_oc_supported && biosStatus?.mem_oc_supported
        ? `Одновременно разблокировать меню разгона CPU и памяти (CPU: ${biosStatus.cpu_oc_enabled ? 'Вкл.' : 'Не включено'}, память: ${biosStatus.mem_oc_enabled ? 'Вкл.' : 'Не включено'})`
        : 'Синхронно записывает в UEFI NVRAM платы флаги разблокировки меню разгона CPU и памяти';

  const isPartial = Boolean(biosStatus?.cpu_oc_enabled || biosStatus?.mem_oc_enabled);

  const buttonText = biosToggling
    ? 'Запись...'
    : isAmd || isUnsupported
      ? 'Не поддерживается'
      : biosOcEnabled
        ? 'Включено'
        : isPartial
          ? 'Частично включено'
          : 'Выключить';

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
          Расширенное меню разгона BIOS
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
