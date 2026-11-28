import React, { useEffect, useId, useRef, useState } from 'react';
import { PowerChangeConfirmModal, PowerLimitPreview } from './PowerChangeConfirmModal';
import { PowerAssessment } from './powerSafety';

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
  editContext?: string;
  previewChange?: (target: number) => PowerLimitPreview[];
  assessChange?: (target: number, confirmed?: boolean) => Promise<PowerAssessment>;
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
  editContext,
  previewChange,
  assessChange,
}) => {
  const minCap = minMax ?? min;
  const maxCap = maxMax ?? 300;
  const inputId = useId();
  const [draft, setDraft] = useState(String(value));
  const draftRef = useRef(draft);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [assessing, setAssessing] = useState(false);
  const assessingRef = useRef(false);
  const assessmentToken = useRef(0);
  const [pendingChange, setPendingChange] = useState<{
    current: number;
    target: number;
    min: number;
    max: number;
    editContext?: string;
    adjustMax?: number;
    assessment?: PowerAssessment;
  } | null>(null);

  const updateDraft = (next: string) => {
    draftRef.current = next;
    setDraft(next);
    setError('');
  };

  useEffect(() => {
    // Discard drafts and confirmations when the profile or allowed range changes.
    draftRef.current = String(value);
    setDraft(String(value));
    setError('');
    setPendingChange(null);
    assessingRef.current = false;
    setAssessing(false);
    assessmentToken.current += 1;
    return () => { assessmentToken.current += 1; };
  }, [value, min, max, disabled, editContext]);

  const assessAndCommit = async (target: number, adjustMax?: number) => {
    if (assessingRef.current || disabled) return;
    assessingRef.current = true;
    setAssessing(true);
    const token = ++assessmentToken.current;
    let assessment: PowerAssessment;
    try {
      assessment = assessChange ? await assessChange(target) : {
        level: 'high', requires_confirmation: true, reasons: [], predicted_temperature_c: null,
      };
    } catch {
      // A failed IPC is different from insufficient sensor data, which the backend
      // handles without interrupting ordinary small changes.
      assessment = {
        level: 'high', requires_confirmation: true,
        reasons: ['暂时无法取得风险评估，请核对功率及联动变化后再应用。'],
        predicted_temperature_c: null,
      };
    }
    if (token !== assessmentToken.current) return;
    assessingRef.current = false;
    setAssessing(false);
    setNotice(assessment.requires_confirmation ? '' : assessment.reasons[0] ?? '');
    if (assessment.requires_confirmation) {
      setPendingChange({ current: value, target, min, max, editContext, adjustMax, assessment });
    } else if (adjustMax !== undefined) {
      onAdjustMax?.(adjustMax);
    } else {
      onChange(target);
    }
  };

  const requestChange = () => {
    if (disabled || assessingRef.current) return;
    const text = draftRef.current.trim();
    const target = Number(text);
    if (!text || !Number.isInteger(target) || target < min || target > max) {
      setError(`请输入 ${min}–${max}${unit} 范围内的整数；当前功率未改变。`);
      return;
    }
    updateDraft(String(value));
    if (target !== value) {
      void assessAndCommit(target);
    }
  };

  const requestAdjustMax = (delta: number) => {
    if (disabled || assessingRef.current || !onAdjustMax) return;
    const targetMax = Math.max(minCap, Math.min(maxCap, max + delta));
    if (targetMax < value) {
      // Lowering the ceiling can also lower the live power limit.
      void assessAndCommit(targetMax, delta);
    } else {
      onAdjustMax(delta);
    }
  };

  const confirmChange = () => {
    const change = pendingChange;
    setPendingChange(null);
    if (!change || disabled || change.current !== value || change.min !== min || change.max !== max || change.editContext !== editContext) return;
    void assessChange?.(change.target, true).catch(() => {});
    if (change.adjustMax !== undefined) {
      onAdjustMax?.(change.adjustMax);
    } else {
      onChange(change.target);
    }
  };

  return (
    <div
      className={`tuning-row tuning-power-row${disabled ? ' is-disabled' : ''}`}
      aria-busy={assessing}
      style={{ '--tuning-power-color': colorVar } as React.CSSProperties}
    >
      <div className="tuning-row-header">
        <label className="tuning-row-label" htmlFor={inputId} title={tooltip}>
          {label}
          {hint && (
            <span className="tuning-power-hint">
              {hint}
            </span>
          )}
        </label>
        <div className="tuning-power-controls">
          <input
            id={inputId}
            type="number"
            min={min}
            max={max}
            step={step}
            value={draft}
            disabled={disabled || assessing}
            aria-invalid={Boolean(error)}
            aria-describedby={error ? `${inputId}-error` : undefined}
            onChange={(e) => updateDraft(e.target.value)}
            onBlur={requestChange}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
                e.preventDefault();
                e.currentTarget.blur();
              } else if (e.key === 'Escape') {
                e.preventDefault();
                e.stopPropagation();
                updateDraft(String(value));
                e.currentTarget.blur();
              }
            }}
            className="tuning-input-number"
          />
          <span className="tuning-power-unit">{unit}</span>
        </div>
      </div>

      {error && <div id={`${inputId}-error`} className="tuning-power-error" role="alert">{error}</div>}
      {!error && notice && <div className="tuning-power-notice" role="status">{notice}</div>}

      <div className="tuning-power-slider">
        <input
          type="range"
          aria-label={`${label}滑块`}
          min={min}
          max={max}
          step={step}
          value={draft !== '' && Number.isFinite(Number(draft)) ? Number(draft) : value}
          disabled={disabled || assessing}
          onChange={(e) => updateDraft(e.target.value)}
          onPointerUp={requestChange}
          onPointerCancel={() => updateDraft(String(value))}
          onBlur={requestChange}
          onKeyUp={(e) => {
            if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown'].includes(e.key)) {
              requestChange();
            }
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              e.preventDefault();
              e.stopPropagation();
              updateDraft(String(value));
            }
          }}
        />
      </div>

      <div className="tuning-power-footer">
        <span>
          {min}
          {unit}
        </span>
        <div className="tuning-power-controls">
          <span>
            上限: {max}
            {unit}
          </span>
          <button
            type="button"
            onClick={() => requestAdjustMax(-adjustStep)}
            disabled={disabled || assessing || !onAdjustMax || max <= minCap}
            title={`以 ${adjustStep}${unit} 步长调小上限`}
            className="tuning-step-btn"
          >
            -
          </button>
          <button
            type="button"
            onClick={() => requestAdjustMax(adjustStep)}
            disabled={disabled || assessing || !onAdjustMax || max >= maxCap}
            title={`以 ${adjustStep}${unit} 步长调大上限`}
            className="tuning-step-btn"
          >
            +
          </button>
        </div>
      </div>
      {pendingChange && (
        <PowerChangeConfirmModal
          label={label}
          current={pendingChange.current}
          target={pendingChange.target}
          unit={unit}
          changes={previewChange?.(pendingChange.target)}
          reasons={pendingChange.assessment?.reasons}
          onCancel={() => setPendingChange(null)}
          onConfirm={confirmChange}
        />
      )}
    </div>
  );
};
