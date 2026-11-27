import React from 'react';
import { AlertTriangle } from 'lucide-react';
import { HardwareState } from '../../types/hardware';

export interface GpuPresetReferenceCardProps {
  presets?: HardwareState['power_presets'];
  isDeclarative?: boolean;
}

export const GpuPresetReferenceCard: React.FC<GpuPresetReferenceCardProps> = ({ presets, isDeclarative }) => {
  return (
    <div
      className="tuning-sub-card"
      style={{
        marginTop: 'auto',
        padding: '10px 12px',
        border: '1px solid var(--border-subtle)',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '5px',
          fontSize: '11px',
          fontWeight: 700,
          color: 'var(--status-warn)',
          whiteSpace: 'nowrap',
        }}
      >
        <AlertTriangle size={13} className="shrink-0" />
        <span>Рекомендованные заводом лимиты мощности платформы</span>
        {presets?.hardware_read && (
          <span style={{ fontSize: '10px', color: 'var(--status-ok)', fontWeight: 600, marginLeft: 'auto' }}>
            {isDeclarative ? 'Заводское прямое чтение оборудования' : 'Прямое аппаратное чтение EC'}
          </span>
        )}
      </div>
      <div style={{ fontSize: '10.5px', color: 'var(--text-secondary)', lineHeight: '1.5' }}>
        {presets?.hardware_read ? (
          <>
            <div>
              • CPU заводской пресет: Офис {presets.office_pl1}Вт / Баланс {presets.balance_pl1}Вт / Турбо {presets.turbo_pl1}W
            </div>
            <div>
              • GPU заводской пресет: база {presets.base_tgp}W + DB {presets.max_db}Вт (заводской максимум {presets.base_tgp + presets.max_db}W)
            </div>
          </>
        ) : (
          <>
            <div>
              • CPU рекомендуемо: длительный PL1  {presets?.office_pl1 || 45}W~{presets?.balance_pl1 || 75} Вт / пиковый PL2 {presets?.balance_pl2 || 75}W~{presets?.turbo_pl2 || 115}W
            </div>
            <div>
              • GPU рекомендуемо: база  {presets?.base_tgp || 'Base'}W + DB {presets?.max_db ? `+${presets.max_db}W` : '15W~25W'} динамическое распределение
            </div>
          </>
        )}
      </div>
    </div>
  );
};
