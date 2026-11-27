import React, { useRef, useState } from 'react';
import { RotateCw, Code2, Upload, Download } from 'lucide-react';
import { BrfxScript } from '../lightingConstants';

interface BrfxSelectorBarProps {
  scripts: BrfxScript[];
  activeScriptId: string;
  onSelectScript: (id: string) => void;
  onRefresh: () => Promise<void> | void;
  onViewSource: () => void;
  onImport: (content: string, filename: string) => void;
  onExport: () => void;
}

export const BrfxSelectorBar: React.FC<BrfxSelectorBarProps> = ({
  scripts,
  activeScriptId,
  onSelectScript,
  onRefresh,
  onViewSource,
  onImport,
  onExport,
}) => {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);

  const handleRefreshClick = async () => {
    setIsRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setTimeout(() => setIsRefreshing(false), 500);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const text = event.target?.result as string;
      if (text) {
        onImport(text, file.name);
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  };

  const activeScript = scripts.find((s) => s.id === activeScriptId) || scripts[0];

  return (
    <div
      className="lighting-sub-panel"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
      }}
    >
      {/* Top row: Label, Dropdown and Actions */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexWrap: 'wrap',
          gap: '8px',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flex: '1 1 240px' }}>
          <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>
            Скрипт эффектов:
          </span>
          <select
            value={activeScriptId}
            onChange={(e) => onSelectScript(e.target.value)}
            className="rev-select"
            style={{
              flex: 1,
              maxWidth: '300px',
              height: '32px',
              padding: '0 10px',
              fontSize: '12px',
              fontWeight: 600,
            }}
          >
            {scripts.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </div>

        {/* Action Buttons */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          {/* Refresh button */}
          <button
            type="button"
            onClick={handleRefreshClick}
            disabled={isRefreshing}
            title="Перечитать файл скрипта из локального каталога и немедленно подхватить внешние изменения"
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              height: '30px',
              padding: '0 10px',
              fontSize: '11px',
              fontWeight: 600,
              borderRadius: 'var(--radius-sm)',
              cursor: 'pointer',
              border: '1px solid var(--border-medium)',
              background: 'var(--surface-pill)',
              color: 'var(--text-primary)',
              transition: 'all var(--transition-fast)',
            }}
          >
            <RotateCw size={12} className={isRefreshing ? 'animate-spin' : ''} />
            <span>Обновить</span>
          </button>

          {/* View Source button */}
          <button
            type="button"
            onClick={onViewSource}
            title="Исходный код: открыть в «Блокноте» или скопировать текст и отдать любому ИИ-ассистенту для доработки/генерации нового эффекта"
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              height: '30px',
              padding: '0 10px',
              fontSize: '11px',
              fontWeight: 600,
              borderRadius: 'var(--radius-sm)',
              cursor: 'pointer',
              border: '1px solid var(--border-glow)',
              background: 'var(--border-glow)',
              color: 'var(--color-brand)',
              transition: 'all var(--transition-fast)',
            }}
          >
            <Code2 size={12} />
            <span>Исходный код</span>
          </button>

          {/* Combined Import & Export */}
          <div style={{ display: 'flex', border: '1px solid var(--border-medium)', borderRadius: 'var(--radius-sm)', overflow: 'hidden' }}>
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              title="Импорт внешнего файла скрипта эффектов .brfx"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                height: '30px',
                padding: '0 9px',
                fontSize: '11px',
                fontWeight: 600,
                border: 'none',
                background: 'var(--surface-pill)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                borderRight: '1px solid var(--border-subtle)',
              }}
            >
              <Upload size={12} />
              <span>Импорт</span>
            </button>
            <button
              type="button"
              onClick={onExport}
              title="Экспортировать текущий скрипт с параметрами в файл .brfx"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                height: '30px',
                padding: '0 9px',
                fontSize: '11px',
                fontWeight: 600,
                border: 'none',
                background: 'var(--surface-pill)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
              }}
            >
              <Download size={12} />
              <span>Экспорт</span>
            </button>
          </div>

          <input
            ref={fileInputRef}
            type="file"
            accept=".brfx,.revo-fx,.rhai,.txt"
            style={{ display: 'none' }}
            onChange={handleFileChange}
          />
        </div>
      </div>

      {/* Description line */}
      {activeScript?.description && (
        <div style={{ fontSize: '11px', color: 'var(--text-dim)', paddingLeft: '2px' }}>
          {activeScript.description}
          {activeScript.author && <span style={{ marginLeft: '8px', opacity: 0.8 }}>by {activeScript.author}</span>}
        </div>
      )}
    </div>
  );
};
