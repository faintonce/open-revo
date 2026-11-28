import React from 'react';
import { ShieldAlert } from 'lucide-react';
import { trText, useI18n } from '../../i18n/I18nContext';

export interface GpuLegacyBannerProps {
  gpuName?: string;
}

export const GpuLegacyBanner: React.FC<GpuLegacyBannerProps> = ({ gpuName }) => {
  useI18n();
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
          硬件不支持 EC 动态借电与超性能
        </div>
        当前显卡 ({gpuName || trText('GTX 10/20 系列')}) 属于 Pascal / Turing 架构，无同方 EC Dynamic Boost 借电芯片与超性能开关，功耗完全由 VBIOS 固件锁死掌控。
      </div>
    </div>
  );
};
