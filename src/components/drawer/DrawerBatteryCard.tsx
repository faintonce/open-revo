import React from 'react';
import { HardwareState } from '../../types/hardware';
import { Battery } from 'lucide-react';

interface DrawerBatteryCardProps {
  status: HardwareState;
  onToggleBatteryLimit: (limit?: number) => void;
  checkOemGuard: (action: () => void) => void;
  onExpandDashboard?: (tab?: 'overview' | 'gpu' | 'tuning' | 'lighting' | 'water' | 'system' | 'sponsor' | 'custom_tuning') => void;
}

export const DrawerBatteryCard: React.FC<DrawerBatteryCardProps> = ({
  status,
  onToggleBatteryLimit,
  checkOemGuard,
  onExpandDashboard: _onExpandDashboard,
}) => {
  return (
    <div className="m-card">
      <div className="m-card-header">
        <div className="m-header-left">
          <Battery size={13} style={{ color: 'var(--theme-bat)' }} />
          <span>电池健康:</span>
          <span style={{ color: 'var(--theme-bat)', fontWeight: 700 }}>
            {status.battery_limit === 60
              ? '工作站模式'
              : status.battery_limit === 80
              ? '平衡模式'
              : '长效模式'}
          </span>
        </div>
        <div className="m-header-right" style={{ color: 'var(--theme-bat)', display: 'flex', alignItems: 'center', gap: '4px' }}>
          <span style={{ width: '4px', height: '4px', borderRadius: '50%', background: status.is_dc_online ? '#34d399' : '#fbbf24' }} />
          {(() => {
            if (!status.is_dc_online) return '电池供电';
            const adapterKnown = (status.adapter_type && status.adapter_type !== '未知') || status.adapter_watts != null;
            if (!adapterKnown) return 'AC 已连接';
            const ratedW = status.adapter_watts
              ? ` ${status.adapter_watts}/240W`
              : status.adapter_raw != null
                ? ` (EC 0x${status.adapter_raw.toString(16).toUpperCase().padStart(2, '0')})`
                : '';
            const rate = status.battery_rate_watts ?? 0;
            if (rate > 0) return `适配器${ratedW} · 充电中 ${rate}W`;
            const limit = status.battery_limit ?? 100;
            const pct = status.battery_percent ?? 100;
            if (limit < 100 && pct >= limit) return `适配器${ratedW} · 已限充`;
            if (pct >= 100) return `适配器${ratedW} · 已充满`;
            return `适配器${ratedW}`;
          })()}
        </div>
      </div>

      <div className="m-grid-3">
        <button
          className="m-btn-bat"
          style={
            status.battery_limit === 60
              ? {
                background: 'var(--theme-bat-dim)',
                borderColor: 'var(--theme-bat-border)',
                color: 'var(--theme-bat)',
                boxShadow: '0 0 10px var(--theme-bat-glow)',
                fontWeight: 700,
              }
              : {}
          }
          title="工作站模式 (限制 60%)"
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(60))}
        >
          工作站
        </button>

        <button
          className="m-btn-bat"
          style={
            status.battery_limit === 80
              ? {
                background: 'var(--theme-bat-dim)',
                borderColor: 'var(--theme-bat-border)',
                color: 'var(--theme-bat)',
                boxShadow: '0 0 10px var(--theme-bat-glow)',
                fontWeight: 700,
              }
              : {}
          }
          title="平衡模式 (限制 80%)"
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(80))}
        >
          平衡
        </button>

        <button
          className="m-btn-bat"
          style={
            status.battery_limit === 100
              ? {
                background: 'var(--theme-bat-dim)',
                borderColor: 'var(--theme-bat-border)',
                color: 'var(--theme-bat)',
                boxShadow: '0 0 10px var(--theme-bat-glow)',
                fontWeight: 700,
              }
              : {}
          }
          title="长效模式 (100% 充满)"
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(100))}
        >
          长效
        </button>
      </div>
    </div>
  );
};
