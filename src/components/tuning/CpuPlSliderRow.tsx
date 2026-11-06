import React from 'react';

export interface CpuPlSliderRowProps {
  label: string;
  value: number;
  onChange: (val: number) => void;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  minMax?: number;
  maxMax?: number;
  onAdjustMax?: (delta: number) => void;
  adjustStep?: number;
  colorVar?: string;
  tooltip?: string;
  hint?: string;
  disabled?: boolean;
}

export const CpuPlSliderRow: React.FC<CpuPlSliderRowProps> = ({
  label,
  value,
  onChange,
  min,
  max,
  step = 1,
  unit = 'W',
  minMax,
  maxMax,
  onAdjustMax,
  adjustStep = 5,
  colorVar = 'var(--color-balance)',
  tooltip,
  hint,
  disabled = false,
}) => {
  const minCap = minMax ?? min;
  const maxCap = maxMax ?? 300;

  return (
    <div className="tuning-row" style={{ opacity: disabled ? 0.45 : 1, transition: 'opacity var(--transition-fast)' }}>
      <div className="tuning-row-header">
        <span className="tuning-row-label" title={tooltip}>
          {label}
          {hint && (
            <span style={{ fontSize: '9.5px', color: 'var(--text-dim)', marginLeft: '4px', fontWeight: 400 }}>
              {hint}
            </span>
          )}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          <input
            type="number"
            min={min}
            max={max}
            step={step}
            value={value}
            disabled={disabled}
            onChange={(e) => onChange(parseInt(e.target.value, 10) || min)}
            className="tuning-input-number"
            style={{ color: disabled ? 'var(--text-dim)' : colorVar, cursor: disabled ? 'not-allowed' : 'text' }}
          />
          <span style={{ color: 'var(--text-dim)', fontSize: '11px' }}>{unit}</span>
        </div>
      </div>

      <div style={{ padding: '2px 0' }}>
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          disabled={disabled}
          onChange={(e) => onChange(parseInt(e.target.value, 10))}
          style={{ width: '100%', accentColor: colorVar, height: '7px', cursor: disabled ? 'not-allowed' : 'pointer' }}
        />
      </div>

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginTop: '1px',
          fontSize: '10px',
          color: 'var(--text-dim)',
        }}
      >
        <span>
          {min}
          {unit}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          <span>
            上限: {max}
            {unit}
          </span>
          <button
            type="button"
            onClick={() => onAdjustMax?.(-adjustStep)}
            disabled={disabled || max <= minCap}
            title={`以 ${adjustStep}${unit} 步长调小上限`}
            className="tuning-step-btn"
          >
            -
          </button>
          <button
            type="button"
            onClick={() => onAdjustMax?.(adjustStep)}
            disabled={disabled || max >= maxCap}
            title={`以 ${adjustStep}${unit} 步长调大上限`}
            className="tuning-step-btn"
          >
            +
          </button>
        </div>
      </div>
    </div>
  );
};
