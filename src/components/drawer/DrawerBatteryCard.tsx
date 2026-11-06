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
  const supported = status.support_flags?.has_battery_limit !== false;
  return (
    <div className="m-card">
      <div className="m-card-header">
        <div className="m-header-left">
          <Battery size={13} style={{ color: 'var(--theme-bat)' }} />
          <span>电池健康:</span>
          <span style={{ color: 'var(--theme-bat)', fontWeight: 700 }}>
            {!supported ? '固件默认' : status.battery_limit === 60
              ? '工作站模式'
              : status.battery_limit === 80
              ? '平衡模式'
              : '长效模式'}
          </span>
        </div>
        <div className="m-header-right" style={{ color: 'var(--theme-bat)', display: 'flex', alignItems: 'center', gap: '4px' }}>
          <span style={{ width: '4px', height: '4px', borderRadius: '50%', background: status.is_dc_online ? '#34d399' : '#fbbf24' }} />
          {status.is_dc_online ? 'AC 已连接' : '电池供电'}
        </div>
      </div>

      {supported ? <div className="m-grid-3">
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
      </div> : <div className="sys-row-desc">本机充电上限尚未适配</div>}
    </div>
  );
};
