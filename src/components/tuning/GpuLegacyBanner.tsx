import React from 'react';
import { ShieldAlert } from 'lucide-react';

export interface GpuLegacyBannerProps {
  gpuName?: string;
}

export const GpuLegacyBanner: React.FC<GpuLegacyBannerProps> = ({ gpuName }) => {
  return (
    <div
      style={{
        background: 'var(--status-warn-dim)',
        border: '1px solid rgba(245, 158, 11, 0.35)',
        borderRadius: 'var(--radius-md)',
        padding: '10px 12px',
        display: 'flex',
        gap: '8px',
        alignItems: 'flex-start',
      }}
    >
      <ShieldAlert className="w-4 h-4 shrink-0 mt-0.5" style={{ color: 'var(--status-warn)' }} />
      <div style={{ fontSize: '11px', color: 'var(--text-secondary)', lineHeight: '1.45' }}>
        <div style={{ fontWeight: 700, color: 'var(--status-warn)', marginBottom: '2px' }}>
          Оборудование не поддерживает динамический перенос мощности EC и супер-производительность
        </div>
        Текущая графика ({gpuName || 'Серия GTX 10/20'}) относится к архитектуре Pascal / Turing: нет чипа динамического заимствования мощности TongFang EC Dynamic Boost и переключателя супер-производительности, потребление полностью определяется прошивкой VBIOS.
      </div>
    </div>
  );
};
