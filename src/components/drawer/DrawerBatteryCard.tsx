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
            const type = status.adapter_type || '';
            const isTypeC = type.includes('Type-C');
            const isBarrel = type === '圆口';
            const watts = status.adapter_watts;
            const rated = watts
              ? (isBarrel && watts === 230 ? ' 230/240W' : ` ${watts}W`)
              : '';
            const label = isTypeC ? `Type-C适配器${rated}` : `适配器${rated}`;
            const rate = status.battery_rate_watts ?? 0;
            if (rate > 0) return `${label} · 充电 ${rate}W`;
            return label;
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

      {status.battery_cycle_count != null && status.battery_cycle_count > 0 && (
        <div style={{ marginTop: '4px', fontSize: '10.5px', color: 'var(--text-dim)', textAlign: 'right' }}>
          循环 {status.battery_cycle_count} 次
        </div>
      )}
    </div>
  );
};
