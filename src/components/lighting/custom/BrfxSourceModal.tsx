import React, { useState } from 'react';
import { Code2, X, Copy, Check, ExternalLink } from 'lucide-react';
import { BrfxScript } from '../lightingConstants';
import { trText, useI18n } from '../../../i18n/I18nContext';

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
  useI18n();
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
              {script.name} - 脚本源文件
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
            提示：可直接点击下方按钮复制纯文本喂给各大 AI 助手改写或生成新特效，或用记事本直接修改。
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
              用记事本打开
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
            {copied ? '已复制到剪贴板！可直接粘贴给 AI 助手' : `${script.code.split('\n').length} 行代码`}
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
              {copied ? '已复制' : trText('复制代码喂给 AI')}
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
              关闭
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
