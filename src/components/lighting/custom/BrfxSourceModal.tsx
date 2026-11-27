import React, { useState } from 'react';
import { Code2, X, Copy, Check, ExternalLink } from 'lucide-react';
import { BrfxScript } from '../lightingConstants';

interface BrfxSourceModalProps {
  script: BrfxScript;
  onClose: () => void;
  onOpenInNotepad?: () => void;
}

export const BrfxSourceModal: React.FC<BrfxSourceModalProps> = ({
  script,
  onClose,
  onOpenInNotepad,
}) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(script.code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    } catch {
      const textarea = document.createElement('textarea');
      textarea.value = script.code;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(6px)',
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '24px',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        style={{
          width: '740px',
          maxWidth: '92vw',
          maxHeight: '86vh',
          backgroundColor: 'var(--surface-overlay)',
          border: '1px solid var(--border-medium)',
          borderRadius: 'var(--radius-lg)',
          display: 'flex',
          flexDirection: 'column',
          boxShadow: '0 20px 40px rgba(0, 0, 0, 0.6)',
          overflow: 'hidden',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '14px 18px',
            borderBottom: '1px solid var(--border-subtle)',
            background: 'var(--surface-badge)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Code2 size={16} style={{ color: 'var(--color-brand)' }} />
            <span style={{ fontSize: '13px', fontWeight: 700, color: 'var(--text-primary)' }}>
              {script.name} - исходный файл скрипта
            </span>
            <span
              style={{
                fontSize: '10px',
                padding: '2px 6px',
                borderRadius: 'var(--radius-xs)',
                background: 'var(--border-glow)',
                color: 'var(--color-brand)',
                fontFamily: 'var(--font-mono)',
              }}
            >
              .brfx
            </span>
          </div>
          <button
            type="button"
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '4px',
              borderRadius: 'var(--radius-xs)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Tip Banner */}
        <div
          style={{
            padding: '8px 18px',
            background: 'var(--border-glow)',
            borderBottom: '1px solid var(--border-subtle)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            fontSize: '11px',
            color: 'var(--text-secondary)',
          }}
        >
          <span>
            Подсказка: кнопками ниже можно скопировать исходный текст и отдать его любому ИИ-ассистенту для доработки или генерации нового эффекта, либо отредактировать его в «Блокноте».
          </span>
          {onOpenInNotepad && (
            <button
              type="button"
              onClick={onOpenInNotepad}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                fontSize: '11px',
                color: 'var(--color-brand)',
                background: 'var(--surface-pill)',
                border: '1px solid var(--border-medium)',
                borderRadius: 'var(--radius-xs)',
                padding: '2px 8px',
                cursor: 'pointer',
                whiteSpace: 'nowrap',
              }}
            >
              <ExternalLink size={11} />
              Открыть в «Блокноте»
            </button>
          )}
        </div>

        {/* Code Content */}
        <div
          style={{
            flex: 1,
            overflowY: 'auto',
            padding: '16px 18px',
            background: 'var(--surface-sunken)',
          }}
        >
          <pre
            style={{
              margin: 0,
              fontFamily: 'var(--font-mono)',
              fontSize: '12px',
              lineHeight: 1.6,
              color: 'var(--text-primary)',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
              userSelect: 'text',
            }}
          >
            {script.code}
          </pre>
        </div>

        {/* Footer */}
        <div
          style={{
            padding: '12px 18px',
            borderTop: '1px solid var(--border-subtle)',
            background: 'var(--surface-badge)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span style={{ fontSize: '11px', color: copied ? 'var(--status-ok)' : 'var(--text-dim)' }}>
            {copied ? 'Скопировано в буфер обмена! Можно сразу вставить ИИ-ассистенту' : `${script.code.split('\n').length} строк кода`}
          </span>

          <div style={{ display: 'flex', gap: '8px' }}>
            <button
              type="button"
              onClick={handleCopy}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '6px',
                padding: '6px 14px',
                fontSize: '12px',
                fontWeight: 600,
                borderRadius: 'var(--radius-sm)',
                cursor: 'pointer',
                border: copied ? '1px solid var(--status-ok)' : '1px solid var(--color-brand)',
                background: copied ? 'var(--status-ok-dim)' : 'var(--border-glow)',
                color: copied ? 'var(--status-ok)' : 'var(--text-primary)',
                transition: 'all var(--transition-fast)',
              }}
            >
              {copied ? <Check size={13} /> : <Copy size={13} />}
              {copied ? 'Скопировано' : 'Скопировать код для ИИ'}
            </button>
            <button
              type="button"
              onClick={onClose}
              style={{
                padding: '6px 14px',
                fontSize: '12px',
                borderRadius: 'var(--radius-sm)',
                cursor: 'pointer',
                border: '1px solid var(--border-medium)',
                background: 'var(--surface-pill)',
                color: 'var(--text-primary)',
              }}
            >
              Выключить
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
