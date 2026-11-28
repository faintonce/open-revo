import React from 'react';
import { Palette, Sliders } from 'lucide-react';
import { DisplayTuningState } from '../../types/hardware';
import { trText, useI18n } from '../../i18n/I18nContext';

export interface DisplayColorTuningProps {
  tuningState: DisplayTuningState | null;
  onToggle: () => void;
  onOpenModal: () => void;
}

/**
 * DisplayColorTuning — 屏幕色彩增强控制行（含开关 + 调色控制按钮）。
 * 插件化路径：未来作为独立显示管理插件的一个子插槽。
 */
export const DisplayColorTuning: React.FC<DisplayColorTuningProps> = ({
  tuningState,
  onToggle,
  onOpenModal,
}) => {
  useI18n();
  const enabled = !!tuningState?.enabled;

  return (
    <div
      className="glass-card"
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '14px',
        padding: '10px 16px',
        marginTop: '4px',
      }}
    >
      {/* Left: icon + label */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexShrink: 0 }}>
        <div
          style={{
            width: '32px',
            height: '32px',
            borderRadius: '7px',
            background: enabled ? 'rgba(236, 72, 153, 0.15)' : 'rgba(255, 255, 255, 0.05)',
            border: `1px solid ${enabled ? 'rgba(236, 72, 153, 0.4)' : 'rgba(255, 255, 255, 0.1)'}`,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.25s ease',
          }}
        >
          <Palette className={`w-4 h-4 ${enabled ? 'text-pink-400' : 'text-slate-400'}`} />
        </div>
        <div>
          <div style={{ fontSize: '12.5px', fontWeight: 700, color: 'var(--text-primary)' }}>
            屏幕色彩增强 (Display Tuning)
          </div>
          <div style={{ fontSize: '10.5px', color: 'var(--text-dim)' }}>
            {enabled
              ? `当前方案：${tuningState!.active_preset.name}`
              : trText('当前状态：已关闭 (原厂直出)')}
          </div>
        </div>
      </div>

      {/* Right: controls */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexShrink: 0 }}>
        {enabled && (
          <button
            onClick={onOpenModal}
            className="toggle-btn"
            style={{
              background: 'linear-gradient(135deg, rgba(236, 72, 153, 0.25) 0%, rgba(168, 85, 247, 0.25) 100%)',
              borderColor: 'rgba(236, 72, 153, 0.5)',
              color: '#f472b6',
              boxShadow: '0 0 10px rgba(236, 72, 153, 0.25)',
            }}
            title="打开全屏沉浸式调色控制面板"
          >
            <Sliders size={13} />
            <span>{trText('调色控制')}</span>
          </button>
        )}

        {/* Pill toggle */}
        <button
          type="button"
          onClick={onToggle}
          style={{
            position: 'relative',
            width: '42px',
            height: '22px',
            borderRadius: '11px',
            background: enabled ? '#ec4899' : 'rgba(255, 255, 255, 0.18)',
            border: 'none',
            padding: '2px',
            cursor: 'pointer',
            outline: 'none',
            transition: 'background-color 0.2s ease, box-shadow 0.2s ease',
            boxShadow: enabled ? '0 0 10px rgba(236, 72, 153, 0.5)' : 'none',
            flexShrink: 0,
          }}
          title={enabled ? '点击关闭色彩增强' : trText('点击开启色彩增强')}
        >
          <div
            style={{
              width: '18px',
              height: '18px',
              borderRadius: '50%',
              background: '#fff',
              boxShadow: '0 1px 3px rgba(0, 0, 0, 0.35)',
              transform: enabled ? 'translateX(20px)' : 'translateX(0px)',
              transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
            }}
          />
        </button>
      </div>
    </div>
  );
};
