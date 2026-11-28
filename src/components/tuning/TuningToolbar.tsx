import React, { useRef, useState, useEffect } from 'react';
import {
  Sliders,
  RotateCcw,
  Check,
  Plus,
  Edit2,
  Download,
  Upload,
  Folder,
  Trash2,
  ChevronDown,
  X,
  Flame,
  Zap,
  Leaf,
  Rocket,
  Star,
} from 'lucide-react';
import { CustomProfilePreset } from '../../types/hardware';
import { trText, useI18n } from '../../i18n/I18nContext';

const renderProfileIcon = (key: string) => {
  switch (key) {
    case 'flame': return <Flame className="w-3.5 h-3.5 text-rose-500 shrink-0" />;
    case 'zap': return <Zap className="w-3.5 h-3.5 text-sky-400 shrink-0" />;
    case 'leaf': return <Leaf className="w-3.5 h-3.5 text-emerald-400 shrink-0" />;
    case 'rocket': return <Rocket className="w-3.5 h-3.5 text-purple-400 shrink-0" />;
    default: return <Star className="w-3.5 h-3.5 text-amber-400 shrink-0" />;
  }
};

export interface TuningToolbarProps {
  presetsList: CustomProfilePreset[];
  activeProfileId: string;
  defaultPresets: CustomProfilePreset[];
  resetSuccess: boolean;
  onSelectProfile: (id: string) => void;
  onCreateProfile: (name: string) => void;
  onResetCurrentProfile: () => void;
  onRenameProfile: (newName: string) => void;
  onDeleteProfile: () => void;
  onExportProfile: () => void;
  onImportProfile: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onOpenProfilesFolder: () => void;
  onClose?: () => void;
}

export const TuningToolbar: React.FC<TuningToolbarProps> = ({
  presetsList,
  activeProfileId,
  defaultPresets,
  resetSuccess,
  onSelectProfile,
  onCreateProfile,
  onResetCurrentProfile,
  onRenameProfile,
  onDeleteProfile,
  onExportProfile,
  onImportProfile,
  onOpenProfilesFolder,
  onClose,
}) => {
  useI18n();
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [modalMode, setModalMode] = useState<'new' | 'rename' | null>(null);
  const [modalInputName, setModalInputName] = useState('');

  const dropdownRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const activePreset = presetsList.find((p) => p.id === activeProfileId) || presetsList[0];

  // Click outside to close dropdown
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    };
    if (dropdownOpen) {
      window.addEventListener('mousedown', handleClickOutside);
    }
    return () => {
      window.removeEventListener('mousedown', handleClickOutside);
    };
  }, [dropdownOpen]);

  return (
    <>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          borderBottom: '1px solid var(--border-subtle)',
          paddingBottom: '10px',
          flexShrink: 0,
        }}
        data-tauri-drag-region
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <div
            style={{
              width: '28px',
              height: '28px',
              borderRadius: 'var(--radius-md)',
              background: 'var(--border-glow)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              border: '1px solid var(--border-medium)',
            }}
          >
            <Sliders className="w-4 h-4" style={{ color: 'var(--color-brand)' }} />
          </div>
          <div
            style={{
              fontWeight: 800,
              color: 'var(--text-primary)',
              fontSize: '14px',
              letterSpacing: '-0.01em',
              whiteSpace: 'nowrap',
            }}
          >
            极客自定义调校
          </div>
        </div>

        {/* Profile Controls in Top Right */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', position: 'relative' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>{trText('方案:')}</span>

          {/* Custom Dark Glass Dropdown */}
          <div ref={dropdownRef} style={{ position: 'relative' }}>
            <button
              type="button"
              onClick={() => setDropdownOpen(!dropdownOpen)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                background: 'var(--surface-pill)',
                border: '1px solid var(--border-medium)',
                color: 'var(--text-primary)',
                padding: '5px 10px',
                borderRadius: 'var(--radius-sm)',
                fontSize: '12px',
                fontWeight: 700,
                cursor: 'pointer',
                minWidth: '110px',
                justifyContent: 'space-between',
                transition: 'all var(--transition-fast)',
                flexShrink: 0,
              }}
              title="选择预设方案"
            >
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>
                {renderProfileIcon(activePreset?.icon_key || 'star')}
                <span style={{ overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis', fontSize: '12px' }}>
                  {activePreset?.name || trText('未知方案')}
                </span>
              </span>
              <ChevronDown className="w-3.5 h-3.5 shrink-0 opacity-70 ml-1" />
            </button>

            {dropdownOpen && (
              <div
                style={{
                  position: 'absolute',
                  top: 'calc(100% + 6px)',
                  left: 0,
                  width: '160px',
                  background: 'var(--surface-overlay)',
                  border: '1px solid var(--border-medium)',
                  borderRadius: 'var(--radius-md)',
                  boxShadow: '0 16px 36px rgba(0, 0, 0, 0.85)',
                  padding: '4px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '2px',
                  zIndex: 9999,
                  backdropFilter: 'blur(16px)',
                }}
              >
                {presetsList.map((p) => {
                  const isActive = p.id === activeProfileId;
                  const def = defaultPresets.find((dp) => dp.id === p.id);
                  const isUnmodified = p.is_builtin && def && JSON.stringify(def.profile) === JSON.stringify(p.profile);
                  return (
                    <div
                      key={p.id}
                      onClick={() => {
                        setDropdownOpen(false);
                        onSelectProfile(p.id);
                      }}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        padding: '7px 8px',
                        borderRadius: 'var(--radius-xs)',
                        fontSize: '12px',
                        color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
                        background: isActive ? 'rgba(99, 102, 241, 0.25)' : 'transparent',
                        fontWeight: isActive ? 700 : 400,
                        cursor: 'pointer',
                        transition: 'all var(--transition-fast)',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis', flex: 1 }}>
                        {renderProfileIcon(p.icon_key)}
                        <span style={{ overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>{p.name}</span>
                      </div>
                      {isUnmodified ? (
                        <span style={{ fontSize: '10px', color: 'var(--text-dim)', marginLeft: '6px', flexShrink: 0 }}>{trText('原厂')}</span>
                      ) : (
                        <span style={{ fontSize: '10px', color: 'var(--status-warn)', marginLeft: '6px', flexShrink: 0 }}>{trText('自定义')}</span>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          <button
            type="button"
            className="toolbar-icon-btn"
            title={activePreset?.is_builtin ? '将当前参数另存为新方案' : trText('将当前参数另存为新方案副本')}
            onClick={() => {
              setModalInputName(`${activePreset?.name || '方案'}_副本`.slice(0, 12));
              setModalMode('new');
            }}
          >
            <Plus className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            className="toolbar-icon-btn"
            style={{
              color: resetSuccess ? 'var(--status-ok)' : 'var(--text-muted)',
              borderColor: resetSuccess ? 'var(--status-ok)' : 'var(--border-medium)',
            }}
            title="还原为默认参数（同时重置 EC 功耗握手，解除 CPU 锁 55W / GPU 锁功耗异常）"
            onClick={onResetCurrentProfile}
          >
            {resetSuccess ? <Check className="w-3.5 h-3.5" style={{ color: 'var(--status-ok)' }} /> : <RotateCcw className="w-3.5 h-3.5" />}
          </button>

          <button
            type="button"
            className="toolbar-icon-btn"
            title={activePreset?.is_builtin ? '原厂预设不可重命名' : trText('重命名当前方案')}
            disabled={activePreset?.is_builtin}
            onClick={() => {
              if (activePreset && !activePreset.is_builtin) {
                setModalInputName(activePreset.name.slice(0, 12));
                setModalMode('rename');
              }
            }}
          >
            <Edit2 className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            className="toolbar-icon-btn"
            title="导出方案为独立 JSON 文件"
            onClick={onExportProfile}
          >
            <Upload className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            className="toolbar-icon-btn"
            title="导入外部独立 JSON 方案"
            onClick={() => fileInputRef.current?.click()}
          >
            <Download className="w-3.5 h-3.5" />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".json"
            style={{ display: 'none' }}
            onChange={onImportProfile}
          />

          <button
            type="button"
            className="toolbar-icon-btn"
            title="在资源管理器中打开 profiles 文件夹"
            onClick={onOpenProfilesFolder}
          >
            <Folder className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            className="toolbar-icon-btn"
            style={{
              color: activePreset?.is_builtin ? 'var(--text-dim)' : 'var(--status-error)',
            }}
            title={activePreset?.is_builtin ? '原厂预设不可删除' : trText('删除当前方案文件')}
            disabled={activePreset?.is_builtin}
            onClick={onDeleteProfile}
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>

          <div style={{ width: '1px', height: '18px', background: 'var(--border-subtle)', margin: '0 2px' }} />

          {onClose && (
            <button
              type="button"
              className="toolbar-icon-btn"
              style={{ marginLeft: '2px' }}
              onClick={onClose}
              title="关闭调校窗口"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>

      {/* Profile Create / Rename Modal Dialog */}
      {modalMode && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(6px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 10001,
          }}
          onClick={() => setModalMode(null)}
        >
          <div
            style={{
              width: '320px',
              background: 'var(--surface-overlay)',
              border: '1px solid var(--border-glow)',
              borderRadius: 'var(--radius-lg)',
              padding: '16px',
              display: 'flex',
              flexDirection: 'column',
              gap: '12px',
              boxShadow: '0 16px 40px rgba(0, 0, 0, 0.85)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ fontWeight: 700, fontSize: '13px', color: 'var(--text-primary)' }}>
              {modalMode === 'new' ? '新建方案副本' : trText('重命名方案')}
            </div>
            <input
              type="text"
              value={modalInputName}
              maxLength={12}
              autoFocus
              placeholder="输入方案名称 (最多12字)..."
              onChange={(e) => setModalInputName(e.target.value.slice(0, 12))}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  const val = modalInputName.trim().slice(0, 12);
                  if (val) {
                    if (modalMode === 'new') onCreateProfile(val);
                    else onRenameProfile(val);
                    setModalMode(null);
                  }
                } else if (e.key === 'Escape') {
                  setModalMode(null);
                }
              }}
              style={{
                background: 'var(--surface-pill)',
                border: '1px solid var(--border-medium)',
                color: 'var(--text-primary)',
                padding: '7px 10px',
                borderRadius: 'var(--radius-sm)',
                fontSize: '12px',
                outline: 'none',
              }}
            />
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px', marginTop: '4px' }}>
              <button
                type="button"
                onClick={() => setModalMode(null)}
                style={{
                  padding: '5px 12px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '12px',
                  border: '1px solid var(--border-medium)',
                  background: 'transparent',
                  color: 'var(--text-muted)',
                  cursor: 'pointer',
                }}
              >
                取消
              </button>
              <button
                type="button"
                onClick={() => {
                  const val = modalInputName.trim().slice(0, 12);
                  if (val) {
                    if (modalMode === 'new') onCreateProfile(val);
                    else onRenameProfile(val);
                    setModalMode(null);
                  }
                }}
                style={{
                  padding: '5px 14px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '12px',
                  fontWeight: 600,
                  border: 'none',
                  background: 'var(--color-brand)',
                  color: 'var(--text-primary)',
                  cursor: 'pointer',
                }}
              >
                确定
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
