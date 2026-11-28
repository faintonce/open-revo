import React from 'react';
import { HardwareState } from '../../types/hardware';
import { Battery } from 'lucide-react';
import { trText, useI18n } from '../../i18n/I18nContext';

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
  useI18n();
  return (
    <div className="m-card">
      <div className="m-card-header">
        <div className="m-header-left">
          <Battery size={13} style={{ color: 'var(--theme-bat)' }} />
          <span>{trText('电池健康:')}</span>
          <span style={{ color: 'var(--theme-bat)', fontWeight: 700 }}>
            {status.battery_limit === 60
              ? trText('工作站模式')
              : status.battery_limit === 80
              ? trText('平衡模式')
              : trText('长效模式')}
          </span>
        </div>
        <div className="m-header-right" style={{ color: 'var(--theme-bat)', display: 'flex', alignItems: 'center', gap: '4px' }}>
          <span style={{ width: '4px', height: '4px', borderRadius: '50%', background: status.is_dc_online ? '#34d399' : '#fbbf24' }} />
          {status.is_dc_online ? trText('AC 已连接') : trText('电池供电')}
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
          title={trText(trText("工作站模式 (限制 60%)"))}
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(60))}
        > {trText('工作站')} </button>

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
          title={trText(trText("平衡模式 (限制 80%)"))}
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(80))}
        > {trText('平衡')} </button>

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
          title={trText(trText("长效模式 (100% 充满)"))}
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(100))}
        > {trText('长效')} </button>
      </div>
    </div>
  );
};
