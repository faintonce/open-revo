import React, { useRef, useState, useEffect } from 'react';
import { Compass, Upload, Trash2, Laptop, Tv, ChevronDown, Check } from 'lucide-react';

import { ColorCalibrationState } from '../GpuModeView';

export interface DisplayColorCalibrationProps {
  calibrationState: ColorCalibrationState | null;
  isCalibrating: boolean;
  onSelect?: (profileId: string) => void;
  onApply: (profileId: string) => void;
  onImport: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onDelete: () => void;
  onSelectMonitor?: (device_name: string) => void;
}

/**
 * DisplayColorCalibration — ICC/WCS 色彩校正横条。
 * 包含：多屏幕独立切换、配置档位选择与应用、导入 .icm/.icc、删除自定义档位。
 * 与 Windows 颜色管理实时双向同步，按应用即写入生效。
 */
export const DisplayColorCalibration: React.FC<DisplayColorCalibrationProps> = ({
  calibrationState,
  isCalibrating,
  onSelect,
  onApply,
  onImport,
  onDelete,
  onSelectMonitor,
}) => {
  const [showMonMenu, setShowMonMenu] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const activeId = calibrationState?.active_profile_id || 'default_native';
  const isCalibrated = activeId !== 'default_native';

  const [selectedProfileId, setSelectedProfileId] = useState<string>(activeId);

  // Synchronize dropdown when external Windows profile changes or monitor changes
  useEffect(() => {
    if (calibrationState?.active_profile_id) {
      setSelectedProfileId(calibrationState.active_profile_id);
    }
  }, [calibrationState?.active_profile_id, calibrationState?.current_device_name]);

  const isDirty = selectedProfileId !== activeId;

  useEffect(() => {
    const handleClickOutside = () => setShowMonMenu(false);
    if (showMonMenu) {
      window.addEventListener('click', handleClickOutside);
      return () => window.removeEventListener('click', handleClickOutside);
    }
  }, [showMonMenu]);

  const isCustomProfileActive =
    !!calibrationState &&
    !calibrationState.profiles.find((p) => p.id === calibrationState.active_profile_id)?.is_builtin;

  const monitors = calibrationState?.monitors || [];
  const currentMon =
    monitors.find((m) => m.device_name === calibrationState?.current_device_name) ||
    monitors.find((m) => m.is_internal) ||
    monitors[0];

  return (
    <div
      className="glass-card"
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '7px 14px',
        background: 'var(--surface-card)',
        borderRadius: 'var(--radius-lg)',
        border: '1px solid var(--border-subtle)',
        boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
        gap: '12px',
        flexWrap: 'nowrap',
        position: 'relative',
        zIndex: showMonMenu ? 250 : 1,
      }}
    >
      {/* Left: icon + label + status */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', minWidth: 0, flex: '1 1 auto' }}>
        <div
          style={{
            width: '32px',
            height: '32px',
            borderRadius: 'var(--radius-md)',
            background: isCalibrated ? 'var(--status-info-dim)' : 'var(--surface-pill)',
            border: `1px solid ${isCalibrated ? 'rgba(56, 189, 248, 0.4)' : 'var(--border-subtle)'}`,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: 0,
            transition: 'all var(--transition-normal)',
          }}
        >
          <Compass className={`w-4 h-4 ${isCalibrated ? 'text-sky-400' : 'text-slate-400'}`} />
        </div>
        <div style={{ minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span style={{ fontSize: '12.5px', fontWeight: 700, color: 'var(--text-primary)' }}>
              Коррекция цвета экрана (Color Management)
            </span>
            {calibrationState?.hdr_active && (
              <span
                style={{
                  fontSize: '9.5px',
                  padding: '1px 5px',
                  borderRadius: 'var(--radius-xs)',
                  background: 'var(--status-warn-dim)',
                  color: 'var(--status-warn)',
                  border: '1px solid rgba(245, 158, 11, 0.3)',
                }}
                title="Обнаружен включённый Windows HDR: управление цветом SDR может быть перехвачено системой"
              >
                HDR активен
              </span>
            )}
          </div>
          <div
            style={{
              fontSize: '10.5px',
              color: 'var(--text-dim)',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
            }}
            title={calibrationState?.active_profile_name}
          >
            {isCalibrated
              ? `Текущая схема:${calibrationState!.active_profile_name}${currentMon?.friendly_name ? ` · ${currentMon.friendly_name}` : ''}`
              : `Текущее состояние: системная схема по умолчанию (не откалибрована)${currentMon?.friendly_name ? ` · ${currentMon.friendly_name}` : ''}`}
          </div>
        </div>
      </div>

      {/* Right: controls (Monitor switcher -> Profile selector -> Import -> Delete -> Toggle) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
        {/* Multi-monitor switcher directly beside profile selector */}
        {monitors.length > 1 && (
          <div style={{ position: 'relative' }} onClick={(e) => e.stopPropagation()}>
            <button
              onClick={() => setShowMonMenu(!showMonMenu)}
              title="Нажмите, чтобы выбрать управляемый монитор"
              className="monitor-select-btn"
            >
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
                {currentMon?.is_internal ? <Laptop size={11} /> : <Tv size={11} />}
                {currentMon?.friendly_name?.split(' ')[0] || 'Экран'}
              </span>
              <ChevronDown size={10} />
            </button>

            {showMonMenu && (
              <div className="monitor-select-dropdown">
                {monitors.map((m) => (
                  <div
                    key={m.device_name}
                    onClick={() => {
                      onSelectMonitor?.(m.device_name);
                      setShowMonMenu(false);
                    }}
                    className={`monitor-select-item ${m.device_name === calibrationState?.current_device_name ? 'active' : ''}`}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                      <span style={{ display: 'flex', alignItems: 'center' }}>
                        {m.is_internal ? <Laptop size={12} /> : <Tv size={12} />}
                      </span>
                      <span style={{ fontWeight: 600 }}>{m.friendly_name}</span>
                    </div>
                    <span
                      style={{
                        fontSize: '9.5px',
                        fontFamily: 'monospace',
                        background: 'var(--surface-badge)',
                        padding: '1px 5px',
                        borderRadius: 'var(--radius-xs)',
                      }}
                    >
                      {m.width}x{m.height}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Profile selector */}
        <select
          value={selectedProfileId}
          onChange={(e) => {
            const val = e.target.value;
            setSelectedProfileId(val);
            onSelect?.(val);
          }}
          disabled={isCalibrating}
          className="rev-select"
          style={{
            maxWidth: '190px',
            opacity: isCalibrating ? 0.6 : 1,
            cursor: isCalibrating ? 'wait' : 'pointer',
          }}
        >
          {calibrationState?.profiles.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>

        {/* Apply Button */}
        <button
          type="button"
          onClick={() => onApply(selectedProfileId)}
          disabled={isCalibrating || !isDirty}
          className={`toggle-btn ${isDirty ? 'active-cyan' : ''}`}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '4px 9px',
            fontWeight: 600,
            fontSize: '11px',
            opacity: isCalibrating ? 0.6 : isDirty ? 1 : 0.45,
            cursor: isDirty && !isCalibrating ? 'pointer' : 'default',
            borderColor: isDirty ? 'rgba(6, 182, 212, 0.45)' : 'var(--border-subtle)',
            background: isDirty ? 'rgba(6, 182, 212, 0.16)' : 'var(--surface-overlay)',
            color: isDirty ? 'var(--status-info)' : 'var(--text-dim)',
            transition: 'all var(--transition-fast)',
          }}
          title={isDirty ? 'Нажмите, чтобы немедленно применить эту схему калибровки к аппаратной LUT экрана и управлению цветом Windows' : 'Текущая схема активна'}
        >
          <Check size={12} />
          <span>Применить</span>
        </button>

        {/* Hidden file input */}
        <input
          type="file"
          ref={fileInputRef}
          onChange={onImport}
          accept=".icm,.icc"
          style={{ display: 'none' }}
        />

        {/* Import */}
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          disabled={isCalibrating}
          className="toggle-btn"
          style={{
            color: 'var(--status-info)',
            background: 'var(--status-info-dim)',
            borderColor: 'rgba(56, 189, 248, 0.35)',
            opacity: isCalibrating ? 0.6 : 1,
          }}
          title="Импорт внешнего файла калибровки .icm / .icc"
        >
          <Upload size={12} />
          <span>Импорт</span>
        </button>

        {/* Delete (only for custom profiles) */}
        <button
          type="button"
          onClick={onDelete}
          disabled={isCalibrating || !isCustomProfileActive}
          className={`toggle-btn ${isCustomProfileActive && !isCalibrating ? 'danger' : ''}`}
          style={{
            opacity: isCustomProfileActive && !isCalibrating ? 1 : 0.35,
            cursor: isCustomProfileActive && !isCalibrating ? 'pointer' : 'not-allowed',
            padding: '4px 7px',
          }}
          title={isCustomProfileActive ? 'Удалить этот пользовательский файл калибровки' : 'Заводские и встроенные в систему схемы нельзя удалять'}
        >
          <Trash2 size={12} />
        </button>
      </div>
    </div>
  );
};
