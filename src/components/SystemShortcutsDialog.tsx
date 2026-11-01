import React, { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { Keyboard, RefreshCw, X } from 'lucide-react';
import { RevSwitch } from './ui';

interface SystemShortcutSetting {
  id: string;
  name: string;
  combo: string;
  enabled: boolean;
}

interface SystemShortcutsDialogProps {
  onClose: () => void;
}

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const SystemShortcutsDialog: React.FC<SystemShortcutsDialogProps> = ({ onClose }) => {
  const [shortcuts, setShortcuts] = useState<SystemShortcutSetting[]>([]);
  const [loading, setLoading] = useState(true);
  const [savingId, setSavingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [savedMessage, setSavedMessage] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  const mountedRef = useRef(false);
  const savingRef = useRef(false);

  const loadShortcuts = async () => {
    setLoading(true);
    setError(null);
    setSavedMessage('');
    try {
      const result = await invoke<SystemShortcutSetting[]>('get_system_shortcut_settings');
      if (mountedRef.current) setShortcuts(result);
    } catch (err) {
      if (mountedRef.current) setError('读取快捷键设置失败：' + describeError(err));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    const previousFocus = document.activeElement;
    dialogRef.current?.focus();
    void loadShortcuts();
    return () => {
      mountedRef.current = false;
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus();
    };
  }, []);

  const closeDialog = () => {
    if (!savingRef.current) onClose();
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      closeDialog();
      return;
    }
    if (event.key !== 'Tab') return;

    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [tabindex="0"]',
    );
    if (!focusable?.length) {
      event.preventDefault();
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogRef.current)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === dialogRef.current)) {
      event.preventDefault();
      first.focus();
    }
  };

  const handleToggle = async (shortcut: SystemShortcutSetting, enabled: boolean) => {
    if (savingRef.current) return;
    savingRef.current = true;
    setSavingId(shortcut.id);
    setError(null);
    setSavedMessage('');
    try {
      const result = await invoke<SystemShortcutSetting[]>('set_system_shortcut_enabled', {
        id: shortcut.id,
        enabled,
      });
      if (mountedRef.current) {
        setShortcuts(result);
        setSavedMessage('设置已保存，立即生效，重启后保持。');
      }
    } catch (err) {
      if (mountedRef.current) setError('保存失败，开关未更改：' + describeError(err));
    } finally {
      savingRef.current = false;
      if (mountedRef.current) setSavingId(null);
    }
  };

  return createPortal(
    <div className="sys-modal-backdrop" onClick={closeDialog}>
      <div
        ref={dialogRef}
        className="sys-modal-card shortcuts-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="system-shortcuts-title"
        aria-describedby="system-shortcuts-description"
        aria-busy={loading || savingId !== null}
        tabIndex={-1}
        onClick={(event) => event.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="sys-modal-header shortcuts-dialog-header">
          <div className="shortcuts-dialog-heading">
            <Keyboard size={20} aria-hidden="true" />
            <div>
              <h2 id="system-shortcuts-title" className="sys-modal-title">全局快捷键</h2>
              <div className="sys-modal-subtitle">分别控制三个内置快捷键</div>
            </div>
          </div>
          <button
            type="button"
            className="toggle-btn shortcuts-close-button"
            onClick={closeDialog}
            disabled={savingId !== null}
            aria-label="关闭快捷键设置"
          >
            <X size={16} aria-hidden="true" />
          </button>
        </div>

        <div className="sys-modal-body shortcuts-dialog-body">
          <p id="system-shortcuts-description" className="shortcuts-dialog-note">
            开启后，OpenRevo 在后台也会响应。关闭可避免与其他应用的快捷键冲突。
          </p>

          {loading ? (
            <div className="shortcuts-loading" role="status">正在读取快捷键设置…</div>
          ) : (
            <div className="sys-rows">
              {shortcuts.map((shortcut) => (
                <div key={shortcut.id} className="quick-row shortcuts-setting-row">
                  <div className="shortcuts-setting-label">
                    <div className="sys-row-label">{shortcut.name}</div>
                    <kbd className="shortcuts-key-combo">{shortcut.combo}</kbd>
                  </div>
                  <div className="sys-row-controls">
                    <span className="shortcuts-setting-status">
                      {savingId === shortcut.id ? '保存中…' : shortcut.enabled ? '已开启' : '已关闭'}
                    </span>
                    <RevSwitch
                      checked={shortcut.enabled}
                      disabled={savingId !== null}
                      onChange={(enabled) => void handleToggle(shortcut, enabled)}
                      ariaLabel={`${shortcut.name}（${shortcut.combo}）`}
                      activeColor="var(--accent-pwr)"
                      activeGlow="0 0 10px var(--accent-pwr-glow)"
                    />
                  </div>
                </div>
              ))}
            </div>
          )}

          {error && (
            <div className="shortcuts-error" role="alert">
              <p>{error}</p>
              {!shortcuts.length && (
                <button type="button" className="toggle-btn" onClick={() => void loadShortcuts()} disabled={loading}>
                  <RefreshCw size={12} aria-hidden="true" />
                  重新读取
                </button>
              )}
            </div>
          )}

          <p className="shortcuts-save-status" role="status" aria-live="polite">{savedMessage}</p>
          <p className="shortcuts-dialog-note">
            修改开关只控制快捷键，不会执行强冷、模式或刷新率切换。开启的组合键仍会传给前台应用。
          </p>
        </div>

        <div className="sys-modal-actions shortcuts-dialog-actions">
          <button type="button" className="toggle-btn active-indigo" onClick={closeDialog} disabled={savingId !== null}>
            完成
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
};
