import React, { useEffect, useId, useRef } from 'react';
import { createPortal } from 'react-dom';
import { AlertTriangle } from 'lucide-react';

export interface PowerLimitPreview {
  label: string;
  current: number;
  target: number;
}

interface PowerChangeConfirmModalProps {
  label: string;
  current: number;
  target: number;
  unit: string;
  changes?: PowerLimitPreview[];
  reasons?: string[];
  onCancel: () => void;
  onConfirm: () => void;
}

export const PowerChangeConfirmModal: React.FC<PowerChangeConfirmModalProps> = ({
  label, current, target, unit, changes, reasons, onCancel, onConfirm,
}) => {
  const titleId = useId();
  const bodyId = useId();
  const cancelRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const previousFocus = document.activeElement;
    cancelRef.current?.focus();
    return () => {
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus();
    };
  }, []);

  // Portal outside the tuning panel so its stacking context cannot hide the confirmation.
  return createPortal(
    <div className="sys-modal-backdrop power-confirm-backdrop" onClick={(e) => {
      e.stopPropagation();
      if (e.target === e.currentTarget) onCancel();
    }}>
      <div
        className="sys-modal-card danger power-confirm-card"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={bodyId}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === 'Escape') {
            e.preventDefault();
            onCancel();
          } else if (e.key === 'Tab') {
            e.preventDefault();
            if (document.activeElement === cancelRef.current) confirmRef.current?.focus();
            else cancelRef.current?.focus();
          }
        }}
      >
        <div className="sys-modal-header">
          <div className="sys-modal-icon-badge danger"><AlertTriangle size={22} /></div>
          <div id={titleId} className="sys-modal-title">确认修改 CPU 功率？</div>
          <div className="sys-modal-subtitle">{label.replace(/[:：]\s*$/, '')}</div>
        </div>
        <div id={bodyId} className="sys-modal-body">
          <div className="power-confirm-values">
            <div><span>当前设置</span><strong>{current}{unit}</strong></div>
            <span aria-hidden="true">→</span>
            <div><span>即将设置</span><strong>{target}{unit}</strong></div>
          </div>
          {changes && changes.length > 1 && (
            <ul className="sys-modal-list power-confirm-linked">
              {changes.map((change) => (
                <li key={change.label}>{change.label}：{change.current}{unit} → {change.target}{unit}</li>
              ))}
            </ul>
          )}
          {reasons?.length ? (
            <ul className="sys-modal-list power-confirm-reasons">
              {reasons.map((reason) => <li key={reason}>{reason}</li>)}
            </ul>
          ) : <p className="sys-modal-warning-label">功率过低可能导致系统卡顿，过高可能增加发热或触发硬件保护。</p>}
          <p>其他 CPU 功率限制可能按现有规则联动调整。请核对数值，确认输入已完成后再应用。</p>
        </div>
        <div className="sys-modal-actions">
          <button ref={cancelRef} type="button" className="toggle-btn" onClick={onCancel}>取消，保持原设置</button>
          <button ref={confirmRef} type="button" className="toggle-btn btn-danger-solid" onClick={onConfirm}>确认应用</button>
        </div>
      </div>
    </div>,
    document.body,
  );
};
