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
          <span>Здоровье батареи:</span>
          <span style={{ color: 'var(--theme-bat)', fontWeight: 700 }}>
            {status.battery_limit === 60
              ? 'Режим рабочей станции'
              : status.battery_limit === 80
              ? 'Режим Баланса'
              : 'Режим долговечности'}
          </span>
        </div>
        <div className="m-header-right" style={{ color: 'var(--theme-bat)', display: 'flex', alignItems: 'center', gap: '4px' }}>
          <span style={{ width: '4px', height: '4px', borderRadius: '50%', background: status.is_dc_online ? '#34d399' : '#fbbf24' }} />
          {status.is_dc_online ? 'AC подключён' : 'От батареи'}
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
          title="Режим рабочей станции (лимит 60%)"
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(60))}
        >
          Рабочая станция
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
          title="Режим Баланса (лимит 80%)"
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(80))}
        >
          Баланс
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
          title="Режим долговечности (заряд до 100%)"
          onClick={() => checkOemGuard(() => onToggleBatteryLimit(100))}
        >
          Долговечность
        </button>
      </div>
    </div>
  );
};
