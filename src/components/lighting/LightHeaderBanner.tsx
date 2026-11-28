import React from 'react';
import { Power } from 'lucide-react';
import { RevBadge, RevBadgeVariant } from '../ui/RevBadge';
import { trText, useI18n } from '../../i18n/I18nContext';

export interface LightHeaderBannerProps {
  title: string;
  badgeText: string;
  badgeVariant?: RevBadgeVariant;
  description: string;
  enabled: boolean;
  onToggle: () => void;
  disabled?: boolean;
  activeColorClass?: string;
}

export const LightHeaderBanner: React.FC<LightHeaderBannerProps> = ({
  title,
  badgeText,
  badgeVariant = 'accent',
  description,
  enabled,
  onToggle,
  disabled = false,
  activeColorClass = 'active-indigo',
}) => {
  useI18n();
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <h3 style={{ fontSize: '16px', fontWeight: 700, color: 'var(--text-primary)' }}>{title}</h3>
          <RevBadge variant={badgeVariant}>{badgeText}</RevBadge>
        </div>
        <p style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '2px' }}>{description}</p>
      </div>

      <button
        type="button"
        onClick={onToggle}
        disabled={disabled}
        className={`toggle-btn ${enabled ? activeColorClass : ''}`}
        style={{ padding: '6px 14px', fontSize: '12px' }}
      >
        <Power className="w-3.5 h-3.5" />
        <span>{enabled ? '已开启' : trText('已关闭')}</span>
      </button>
    </div>
  );
};
