import React from 'react';
import {
  Zap,
  Maximize2,
  ArrowRight,
  ArrowLeft,
  ArrowUp,
  ArrowDown,
  Palette,
  Undo2,
  Activity,
  Sliders,
  Type,
} from 'lucide-react';
import { PRESET_COLORS } from '../lightingConstants';

export interface BrfxTuningValues {
  speed: number;       // 0.2 ~ 3.0
  scale: number;       // 0.5 ~ 2.5
  direction: number;   // 0: R, 1: L, 2: U, 3: D
  reactive: boolean;
  color: string;
  customText?: string;
}

interface BrfxTuningPanelProps {
  values: BrfxTuningValues;
  baselineValues: BrfxTuningValues;
  onChange: (patch: Partial<BrfxTuningValues>) => void;
  onReset: () => void;
}

export const BrfxTuningPanel: React.FC<BrfxTuningPanelProps> = ({
  values,
  baselineValues,
  onChange,
  onReset,
}) => {
  const isModified =
    Math.abs(values.speed - baselineValues.speed) > 0.05 ||
    Math.abs(values.scale - baselineValues.scale) > 0.05 ||
    values.direction !== baselineValues.direction ||
    values.reactive !== baselineValues.reactive ||
    values.color.toLowerCase() !== baselineValues.color.toLowerCase() ||
    (values.customText !== undefined && baselineValues.customText !== undefined && values.customText !== baselineValues.customText);

  return (
    <div
      className="lighting-sub-panel"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
      }}
    >
      {/* Header with Title & Reset Button */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Sliders size={13} style={{ color: 'var(--color-brand)' }} />
          <span style={{ fontSize: '12px', fontWeight: 700, color: 'var(--text-primary)' }}>
            上位通用参数微调 (实时映射物理键盘)
          </span>
        </div>

        <button
          type="button"
          onClick={onReset}
          disabled={!isModified}
          title="恢复至当前脚本载入时的初始默认参数"
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '3px 8px',
            fontSize: '11px',
            borderRadius: 'var(--radius-xs)',
            cursor: isModified ? 'pointer' : 'default',
            border: isModified ? '1px solid var(--border-medium)' : '1px solid transparent',
            background: isModified ? 'var(--surface-pill)' : 'transparent',
            color: isModified ? 'var(--text-primary)' : 'var(--text-dim)',
            opacity: isModified ? 1 : 0.4,
            transition: 'all var(--transition-fast)',
          }}
        >
          <Undo2 size={11} />
          <span>恢复默认</span>
        </button>
      </div>

      {/* Sliders Grid: Speed & Scale */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
          gap: '14px',
        }}
      >
        {/* Speed Slider */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', marginBottom: '4px' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-muted)' }}>
              <Zap size={12} style={{ color: 'var(--status-info)' }} />
              动效流速
            </span>
            <span style={{ color: 'var(--status-info)', fontWeight: 700, fontFamily: 'var(--font-mono)' }}>
              {values.speed.toFixed(1)}x
            </span>
          </div>
          <input
            type="range"
            min="0.2"
            max="3.0"
            step="0.1"
            value={values.speed}
            onChange={(e) => onChange({ speed: parseFloat(e.target.value) })}
            style={{ width: '100%', accentColor: 'var(--status-info)', cursor: 'pointer' }}
          />
        </div>

        {/* Scale Slider */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', marginBottom: '4px' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-muted)' }}>
              <Maximize2 size={12} style={{ color: 'var(--color-brand)' }} />
              空间尺度 / 扩散
            </span>
            <span style={{ color: 'var(--color-brand)', fontWeight: 700, fontFamily: 'var(--font-mono)' }}>
              {Math.round(values.scale * 100)}%
            </span>
          </div>
          <input
            type="range"
            min="0.5"
            max="2.5"
            step="0.1"
            value={values.scale}
            onChange={(e) => onChange({ scale: parseFloat(e.target.value) })}
            style={{ width: '100%', accentColor: 'var(--color-brand)', cursor: 'pointer' }}
          />
        </div>
      </div>

      {/* Direction & Keypress Reaction Row */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingTop: '6px',
          borderTop: '1px solid var(--border-subtle)',
          flexWrap: 'wrap',
          gap: '10px',
        }}
      >
        {/* Direction Controls */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-muted)' }}>
            流动方向:
          </span>
          <div style={{ display: 'flex', gap: '4px' }}>
            {[
              { id: 0, icon: ArrowRight, label: '从左至右' },
              { id: 1, icon: ArrowLeft, label: '从右至左' },
              { id: 2, icon: ArrowUp, label: '从下至上' },
              { id: 3, icon: ArrowDown, label: '从上至下' },
            ].map((d) => {
              const Icon = d.icon;
              const active = values.direction === d.id;
              return (
                <button
                  type="button"
                  key={d.id}
                  onClick={() => onChange({ direction: d.id })}
                  title={d.label}
                  style={{
                    width: '28px',
                    height: '28px',
                    borderRadius: 'var(--radius-xs)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    cursor: 'pointer',
                    background: active ? 'var(--color-brand)' : 'var(--surface-badge)',
                    border: active ? '1px solid var(--border-glow)' : '1px solid var(--border-subtle)',
                    color: active ? 'var(--text-primary)' : 'var(--text-dim)',
                    transition: 'all var(--transition-fast)',
                  }}
                >
                  <Icon size={14} />
                </button>
              );
            })}
          </div>
        </div>

        {/* Reactive Toggle */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <button
            type="button"
            onClick={() => onChange({ reactive: !values.reactive })}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '4px 10px',
              fontSize: '11px',
              fontWeight: 600,
              borderRadius: 'var(--radius-xs)',
              cursor: 'pointer',
              border: values.reactive
                ? '1px solid var(--status-ok)'
                : '1px solid var(--border-subtle)',
              background: values.reactive
                ? 'var(--status-ok-dim)'
                : 'var(--surface-badge)',
              color: values.reactive ? 'var(--status-ok)' : 'var(--text-dim)',
              transition: 'all var(--transition-fast)',
            }}
          >
            <Activity size={13} />
            <span>击键互动: {values.reactive ? '开启' : '关闭'}</span>
          </button>
        </div>
      </div>

      {/* Color Palette Section */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingTop: '6px',
          borderTop: '1px solid var(--border-subtle)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Palette size={13} style={{ color: 'var(--color-brand)' }} />
          <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-muted)' }}>
            定制主色调:
          </span>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            {PRESET_COLORS.map((c) => {
              const active = values.color.toLowerCase() === c.hex.toLowerCase();
              return (
                <div
                  key={c.hex}
                  onClick={() => onChange({ color: c.hex })}
                  title={c.name}
                  className={`color-dot ${active ? 'active' : ''}`}
                  style={{
                    backgroundColor: c.hex,
                    boxShadow: active ? `0 0 6px ${c.hex}` : 'none',
                  }}
                />
              );
            })}
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <input
            type="color"
            value={values.color}
            onChange={(e) => onChange({ color: e.target.value })}
            style={{ width: '24px', height: '24px', border: 'none', background: 'transparent', cursor: 'pointer' }}
          />
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', color: 'var(--text-dim)' }}>
            {values.color.toUpperCase()}
          </span>
        </div>
      </div>

      {/* Optional Custom Marquee Text Input */}
      {values.customText !== undefined && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
            paddingTop: '6px',
            borderTop: '1px dashed var(--border-subtle)',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px', color: 'var(--text-muted)' }}>
              <Type size={12} style={{ color: 'var(--color-brand)' }} />
              <span>滚动弹幕文本 (实时映射物理键盘)</span>
            </span>
            <span style={{ fontSize: '10px', color: 'var(--text-dim)' }}>支持大写英文、数字与符号</span>
          </div>
          <input
            type="text"
            value={values.customText}
            onChange={(e) => onChange({ customText: e.target.value })}
            placeholder="输入弹幕文字，例如: I LOVE OPEN-REVO"
            style={{
              width: '100%',
              background: 'rgba(0, 0, 0, 0.35)',
              border: '1px solid var(--border-medium)',
              borderRadius: 'var(--radius-xs)',
              padding: '6px 10px',
              fontSize: '12px',
              color: 'var(--text-primary)',
              outline: 'none',
              fontFamily: 'var(--font-mono)',
              letterSpacing: '1px',
            }}
          />
        </div>
      )}
    </div>
  );
};
