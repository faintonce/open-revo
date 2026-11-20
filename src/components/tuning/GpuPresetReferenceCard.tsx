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
        <span>模具原厂建议功耗参考</span>
        {presets?.hardware_read && (
          <span style={{ fontSize: '10px', color: 'var(--status-ok)', fontWeight: 600, marginLeft: 'auto' }}>
            {isDeclarative ? '原厂硬件直读' : 'EC 硬件直读'}
          </span>
        )}
      </div>
      <div style={{ fontSize: '10.5px', color: 'var(--text-secondary)', lineHeight: '1.5' }}>
        {presets?.hardware_read ? (
          <>
            <div>
              • CPU 原厂预设: 办公 {presets.office_pl1}W / 平衡 {presets.balance_pl1}W / 狂暴 {presets.turbo_pl1}W
            </div>
            <div>
              • GPU 原厂预设: 基础 {presets.base_tgp}W + DB {presets.max_db}W (原厂最大 {presets.base_tgp + presets.max_db}W)
            </div>
          </>
        ) : (
          <>
            <div>
              • CPU 推荐: 持续 PL1 {presets?.office_pl1 || 45}W~{presets?.balance_pl1 || 75}W / 爆发 PL2 {presets?.balance_pl2 || 75}W~{presets?.turbo_pl2 || 115}W
            </div>
            <div>
              • GPU 推荐: 基础 {presets?.base_tgp || 'Base'}W + DB {presets?.max_db ? `+${presets.max_db}W` : '15W~25W'} 动态分配
            </div>
          </>
        )}
      </div>
    </div>
  );
};
