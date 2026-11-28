import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Palette,
  Sliders,
  FolderOpen,
  Download,
  RotateCcw,
  X,
  Sparkles,
  Film,
  Eye,
  BookOpen,
  Check,
  ChevronDown,
  ChevronUp,
  Globe,
  Camera,
  Tv,
} from 'lucide-react';
import { DisplayColorPreset, DisplayTuningState } from '../../types/hardware';
import { trText, useI18n } from '../../i18n/I18nContext';

interface DisplayTuningModalProps {
  onClose: () => void;
  onUpdated?: () => void;
}

export const DisplayTuningModal: React.FC<DisplayTuningModalProps> = ({ onClose, onUpdated }) => {
  useI18n();
  const [state, setState] = useState<DisplayTuningState | null>(null);
  const [activePreset, setActivePreset] = useState<DisplayColorPreset | null>(null);
  const [selectedPresetId, setSelectedPresetId] = useState<string>('preset_vibrant');
  const [showRgbChannels, setShowRgbChannels] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const liveTweakDebounceRef = useRef<any>(null);

  // Load state on mount
  const loadState = async () => {
    try {
      const res = await invoke<DisplayTuningState>('get_display_tuning_state');
      setState(res);
      setActivePreset(res.active_preset);
      setSelectedPresetId(res.active_preset_id || res.active_preset.id);
    } catch (e) {
      console.error('Failed to get display tuning state:', e);
    }
  };

  useEffect(() => {
    loadState();
  }, []);

  // Keyboard shortcut: ESC to close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const latestPresetRef = useRef<DisplayColorPreset | null>(null);

  // Baseline is the factory default if it exists (for builtin presets), or the current preset definition
  const factoryPreset = state?.factory_presets?.find((p) => p.id === selectedPresetId);
  const baselinePreset = factoryPreset || state?.presets.find((p) => p.id === selectedPresetId);
  const isModified = Boolean(
    baselinePreset &&
      activePreset &&
      (activePreset.color_temp_k !== baselinePreset.color_temp_k ||
        Math.abs(activePreset.saturation - baselinePreset.saturation) > 0.001 ||
        Math.abs(activePreset.contrast - baselinePreset.contrast) > 0.001 ||
        Math.abs(activePreset.gamma - baselinePreset.gamma) > 0.001 ||
        Math.abs(activePreset.gain_r - baselinePreset.gain_r) > 0.001 ||
        Math.abs(activePreset.gain_g - baselinePreset.gain_g) > 0.001 ||
        Math.abs(activePreset.gain_b - baselinePreset.gain_b) > 0.001)
  );

  useEffect(() => {
    latestPresetRef.current = activePreset;
  }, [activePreset]);

  // Flush any pending save on unmount so changes are never lost if user closes quickly
  useEffect(() => {
    return () => {
      if (liveTweakDebounceRef.current) {
        clearTimeout(liveTweakDebounceRef.current);
        if (latestPresetRef.current) {
          invoke('apply_display_color_preset', { preset: latestPresetRef.current }).catch(() => {});
        }
      }
    };
  }, []);

  // Debounced live application to GPU hardware
  const triggerLiveApply = (preset: DisplayColorPreset) => {
    if (liveTweakDebounceRef.current) {
      clearTimeout(liveTweakDebounceRef.current);
    }
    liveTweakDebounceRef.current = setTimeout(() => {
      invoke('apply_display_color_preset', { preset })
        .then(() => {
          if (onUpdated) onUpdated();
        })
        .catch((err) => console.warn('Live apply error:', err));
    }, 16);
  };

  const handleSelectPreset = async (preset: DisplayColorPreset) => {
    setSelectedPresetId(preset.id);
    setActivePreset({ ...preset });
    try {
      await invoke('apply_display_color_preset', { preset });
      await loadState();
      if (onUpdated) onUpdated();
      showToast(`已切换至方案：${preset.name}`);
    } catch (e) {
      console.error('Failed to select preset:', e);
    }
  };

  const handleSliderChange = (key: keyof DisplayColorPreset, value: number) => {
    if (!activePreset) return;
    const updated: DisplayColorPreset = {
      ...activePreset,
      [key]: value,
    };
    setActivePreset(updated);
    setState((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        presets: prev.presets.map((p) => (p.id === updated.id ? updated : p)),
        active_preset: updated,
      };
    });
    triggerLiveApply(updated);
  };

  const handleResetToDefault = async () => {
    try {
      const res = await invoke<DisplayTuningState>('reset_color_preset_to_default', {
        presetId: selectedPresetId,
      });
      setState(res);
      setActivePreset(res.active_preset);
      setSelectedPresetId(res.active_preset_id || selectedPresetId);
      if (onUpdated) onUpdated();
      showToast(
        `已撤销更改，恢复【${res.active_preset.name}】官方默认参数`
      );
    } catch (e) {
      console.error('Failed to reset preset:', e);
    }
  };

  const handleImportClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileSelected = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async (evt) => {
      const content = evt.target?.result as string;
      if (!content) return;
      try {
        const imported = await invoke<DisplayColorPreset>('import_color_preset_content', {
          content,
          filename: file.name,
        });
        await loadState();
        setActivePreset(imported);
        if (onUpdated) onUpdated();
        showToast(`成功加载配色文件：${imported.name}`);
      } catch (err: any) {
        alert(`加载 .json 配色文件失败: ${err}`);
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  };

  const handleExportJson = () => {
    if (!activePreset) return;
    const json = JSON.stringify(activePreset, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${activePreset.name || 'color_preset'}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    showToast(trText('已导出配色文件至下载'));
  };

  const showToast = (msg: string) => {
    setFeedback(msg);
    setTimeout(() => setFeedback(null), 2200);
  };

  const getPresetIcon = (id: string) => {
    if (id === 'preset_vibrant') return <Sparkles size={14} className="text-amber-400" />;
    if (id === 'preset_internet') return <Globe size={14} className="text-blue-400" />;
    if (id === 'preset_video') return <Tv size={14} className="text-indigo-400" />;
    if (id === 'preset_lowblue' || id === 'preset_eyecare') return <Eye size={14} className="text-emerald-400" />;
    if (id === 'preset_cinema') return <Film size={14} className="text-purple-400" />;
    if (id === 'preset_photo') return <Camera size={14} className="text-pink-400" />;
    if (id === 'preset_reading') return <BookOpen size={14} className="text-orange-400" />;
    if (id === 'preset_standard') return <Palette size={14} className="text-slate-300" />;
    return <Sliders size={14} className="text-cyan-400" />;
  };

  if (!state || !activePreset) {
    return null;
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        background: 'rgba(11, 15, 25, 0.96)',
        backdropFilter: 'blur(16px)',
        display: 'flex',
        flexDirection: 'column',
        animation: 'fadeIn 0.2s ease-out',
        userSelect: 'none',
        color: '#f8fafc',
      }}
    >
      {/* Hidden file picker for loading any .json */}
      <input
        type="file"
        ref={fileInputRef}
        accept=".json"
        style={{ display: 'none' }}
        onChange={handleFileSelected}
      />

      {/* Top Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '16px 24px',
          borderBottom: '1px solid rgba(255, 255, 255, 0.08)',
          background: 'rgba(15, 23, 42, 0.7)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div
            style={{
              width: '38px',
              height: '38px',
              borderRadius: '10px',
              background: 'linear-gradient(135deg, rgba(236, 72, 153, 0.25) 0%, rgba(168, 85, 247, 0.25) 100%)',
              border: '1px solid rgba(236, 72, 153, 0.4)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <Palette className="w-5 h-5 text-pink-400" />
          </div>
          <div>
            <div style={{ fontSize: '15px', fontWeight: 800, letterSpacing: '0.02em', display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span>{trText('屏幕色彩增强引擎')}</span>
              <span
                style={{
                  fontSize: '10px',
                  fontWeight: 700,
                  padding: '2px 6px',
                  borderRadius: '4px',
                  background: state.enabled ? 'rgba(16, 185, 129, 0.2)' : 'rgba(148, 163, 184, 0.2)',
                  color: state.enabled ? '#34d399' : '#94a3b8',
                  border: `1px solid ${state.enabled ? 'rgba(16, 185, 129, 0.4)' : 'rgba(148, 163, 184, 0.3)'}`,
                }}
              >
                {state.enabled ? 'GPU 硬件生效中' : trText('调色已暂停')}
              </span>
            </div>
            <div style={{ fontSize: '11px', color: '#94a3b8' }}>
              GPU 硬件级 LUT 查找表与全屏 DWM 渲染管线实时色彩重构
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          {feedback && (
            <div
              style={{
                fontSize: '11px',
                color: '#34d399',
                background: 'rgba(52, 211, 153, 0.15)',
                padding: '4px 10px',
                borderRadius: '6px',
                border: '1px solid rgba(52, 211, 153, 0.3)',
                animation: 'fadeIn 0.2s ease-out',
              }}
            >
              {feedback}
            </div>
          )}
          <button
            onClick={onClose}
            style={{
              background: 'rgba(255, 255, 255, 0.06)',
              border: '1px solid rgba(255, 255, 255, 0.1)',
              borderRadius: '8px',
              padding: '6px',
              color: '#94a3b8',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              transition: 'all 0.2s',
            }}
            title="关闭浮层 (ESC)"
          >
            <X size={18} />
          </button>
        </div>
      </div>

      {/* Main Content Body */}
      <div
        style={{
          flex: 1,
          padding: '20px 24px',
          overflowY: 'auto',
          display: 'flex',
          flexDirection: 'column',
          gap: '16px',
        }}
      >
        {/* Section 1: Presets Selection Bar */}
        <div
          style={{
            background: 'rgba(255, 255, 255, 0.03)',
            borderRadius: '12px',
            border: '1px solid rgba(255, 255, 255, 0.08)',
            padding: '14px 16px',
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: '12px',
            }}
          >
            <div style={{ fontSize: '12px', fontWeight: 700, color: '#cbd5e1' }}>
              色彩预设方案
            </div>
            <div style={{ display: 'flex', gap: '8px' }}>
              {/* Load .json Button */}
              <button
                onClick={handleImportClick}
                className="toggle-btn"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '5px 12px',
                  fontSize: '11px',
                  borderRadius: '6px',
                  background: 'rgba(59, 130, 246, 0.15)',
                  border: '1px solid rgba(59, 130, 246, 0.4)',
                  color: '#60a5fa',
                  cursor: 'pointer',
                  fontWeight: 600,
                }}
                title="导入任意本地 .json 配色方案文件 (自动兼容官方出厂配置)"
              >
                <FolderOpen size={13} />
                <span>{trText('加载 .json 文件')}</span>
              </button>

              {/* Export Button */}
              <button
                onClick={handleExportJson}
                className="toggle-btn"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '5px 12px',
                  fontSize: '11px',
                  borderRadius: '6px',
                  background: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid rgba(255, 255, 255, 0.12)',
                  color: '#e2e8f0',
                  cursor: 'pointer',
                }}
                title="将当前调好的色彩参数导出为 .json 文件"
              >
                <Download size={13} />
                <span>{trText('导出当前 .json')}</span>
              </button>

              {/* Undo Changes Button */}
              <button
                onClick={handleResetToDefault}
                disabled={!isModified}
                className="toggle-btn"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '5px 10px',
                  fontSize: '11px',
                  borderRadius: '6px',
                  background: isModified ? 'rgba(239, 68, 68, 0.12)' : 'rgba(255, 255, 255, 0.04)',
                  border: isModified ? '1px solid rgba(239, 68, 68, 0.35)' : '1px solid rgba(255, 255, 255, 0.08)',
                  color: isModified ? '#f87171' : 'var(--text-dim)',
                  cursor: isModified ? 'pointer' : 'not-allowed',
                  opacity: isModified ? 1 : 0.45,
                  fontWeight: 600,
                  transition: 'all 0.2s ease',
                }}
                title={isModified ? "撤销对当前方案的滑块微调，恢复初始参数" : "当前方案未做任何修改"}
              >
                <RotateCcw size={12} />
                <span>{trText('撤销更改')}</span>
              </button>
            </div>
          </div>

          {/* Presets Grid */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(6, 1fr)',
              gap: '8px',
            }}
          >
            {state.presets.map((p) => {
              const isSelected = selectedPresetId === p.id;
              return (
                <div
                  key={p.id}
                  onClick={() => handleSelectPreset(p)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 10px',
                    borderRadius: '8px',
                    background: isSelected
                      ? 'linear-gradient(135deg, rgba(236, 72, 153, 0.22) 0%, rgba(147, 51, 234, 0.22) 100%)'
                      : 'rgba(255, 255, 255, 0.03)',
                    border: isSelected
                      ? '1px solid rgba(236, 72, 153, 0.65)'
                      : '1px solid rgba(255, 255, 255, 0.08)',
                    boxShadow: isSelected
                      ? '0 0 14px rgba(236, 72, 153, 0.25)'
                      : 'none',
                    cursor: 'pointer',
                    transition: 'all 0.2s ease',
                    minWidth: 0,
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0, overflow: 'hidden' }}>
                    <div style={{ flexShrink: 0 }}>{getPresetIcon(p.id)}</div>
                    <span
                      style={{
                        fontSize: '11px',
                        fontWeight: isSelected ? 700 : 500,
                        color: isSelected ? '#fff' : '#cbd5e1',
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                      }}
                      title={p.name}
                    >
                      {p.name}
                    </span>
                  </div>
                  {isSelected && (
                    isModified ? (
                      <span
                        style={{
                          fontSize: '9px',
                          fontWeight: 700,
                          color: '#fbbf24',
                          background: 'rgba(245, 158, 11, 0.16)',
                          padding: '1px 5px',
                          borderRadius: '4px',
                          border: '1px solid rgba(245, 158, 11, 0.35)',
                          flexShrink: 0,
                          marginLeft: '4px',
                          whiteSpace: 'nowrap',
                        }}
                        title="当前方案参数已微调，点击右上角【撤销更改】可一键恢复官方初始参数"
                      >
                        微调
                      </span>
                    ) : (
                      <Check size={12} className="text-pink-400" style={{ flexShrink: 0, marginLeft: '4px' }} />
                    )
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Section 2: Real-time Hardware Sliders */}
        <div
          style={{
            background: 'rgba(255, 255, 255, 0.03)',
            borderRadius: '12px',
            border: '1px solid rgba(255, 255, 255, 0.08)',
            padding: '16px 18px',
            display: 'flex',
            flexDirection: 'column',
            gap: '14px',
          }}
        >
          <div style={{ fontSize: '12px', fontWeight: 700, color: '#cbd5e1', marginBottom: '2px' }}>
            实时显卡硬件微调 (所见即所得，毫秒级生效)
          </div>

          {/* Slider 1: Color Temp */}
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '6px' }}>
              <span style={{ color: '#94a3b8' }}>{trText('色温 (Color Temperature)')}</span>
              <span style={{ fontWeight: 700, color: '#f59e0b' }}>{activePreset.color_temp_k}K</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('4000K 暖黄')}</span>
              <input
                type="range"
                min={4000}
                max={9000}
                step={50}
                value={activePreset.color_temp_k}
                onChange={(e) => handleSliderChange('color_temp_k', Number(e.target.value))}
                style={{ flex: 1, accentColor: '#f59e0b', cursor: 'pointer', height: '6px' }}
              />
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('9000K 冷蓝')}</span>
            </div>
          </div>

          {/* Slider 2: Saturation / Vibrance */}
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '6px' }}>
              <span style={{ color: '#94a3b8' }}>{trText('色彩鲜艳度 / 饱和度 (Vibrance)')}</span>
              <span style={{ fontWeight: 700, color: '#ec4899' }}>
                {Math.round(activePreset.saturation * 100)}%
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('0% 黑白')}</span>
              <input
                type="range"
                min={0}
                max={2.0}
                step={0.05}
                value={activePreset.saturation}
                onChange={(e) => handleSliderChange('saturation', Number(e.target.value))}
                style={{ flex: 1, accentColor: '#ec4899', cursor: 'pointer', height: '6px' }}
              />
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('200% 浓艳')}</span>
            </div>
          </div>

          {/* Slider 3: Contrast */}
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '6px' }}>
              <span style={{ color: '#94a3b8' }}>{trText('对比度 (Contrast)')}</span>
              <span style={{ fontWeight: 700, color: '#38bdf8' }}>
                {Math.round(activePreset.contrast * 100)}%
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('50% 柔和')}</span>
              <input
                type="range"
                min={0.5}
                max={1.5}
                step={0.05}
                value={activePreset.contrast}
                onChange={(e) => handleSliderChange('contrast', Number(e.target.value))}
                style={{ flex: 1, accentColor: '#38bdf8', cursor: 'pointer', height: '6px' }}
              />
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('150% 鲜明')}</span>
            </div>
          </div>

          {/* Slider 4: Gamma */}
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '6px' }}>
              <span style={{ color: '#94a3b8' }}>{trText('伽马曲线 (Gamma)')}</span>
              <span style={{ fontWeight: 700, color: '#a855f7' }}>
                {activePreset.gamma.toFixed(2)}
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('0.7 暗部深沉')}</span>
              <input
                type="range"
                min={0.7}
                max={1.3}
                step={0.05}
                value={activePreset.gamma}
                onChange={(e) => handleSliderChange('gamma', Number(e.target.value))}
                style={{ flex: 1, accentColor: '#a855f7', cursor: 'pointer', height: '6px' }}
              />
              <span style={{ fontSize: '10px', color: '#64748b' }}>{trText('1.3 暗部通透')}</span>
            </div>
          </div>

          {/* Advanced RGB Channels Gain Foldout */}
          <div style={{ borderTop: '1px solid rgba(255, 255, 255, 0.06)', paddingTop: '10px' }}>
            <button
              onClick={() => setShowRgbChannels(!showRgbChannels)}
              style={{
                background: 'transparent',
                border: 'none',
                color: '#94a3b8',
                fontSize: '11px',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                padding: '4px 0',
              }}
            >
              <span>{showRgbChannels ? '收起' : trText('展开')} RGB 独立原色增益微调</span>
              {showRgbChannels ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            </button>

            {showRgbChannels && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', marginTop: '10px' }}>
                {/* R Gain */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                  <span style={{ width: '50px', fontSize: '11px', color: '#f87171' }}>红 (R): {Math.round(activePreset.gain_r * 100)}%</span>
                  <input
                    type="range"
                    min={0.5}
                    max={1.5}
                    step={0.02}
                    value={activePreset.gain_r}
                    onChange={(e) => handleSliderChange('gain_r', Number(e.target.value))}
                    style={{ flex: 1, accentColor: '#ef4444', height: '5px' }}
                  />
                </div>
                {/* G Gain */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                  <span style={{ width: '50px', fontSize: '11px', color: '#4ade80' }}>绿 (G): {Math.round(activePreset.gain_g * 100)}%</span>
                  <input
                    type="range"
                    min={0.5}
                    max={1.5}
                    step={0.02}
                    value={activePreset.gain_g}
                    onChange={(e) => handleSliderChange('gain_g', Number(e.target.value))}
                    style={{ flex: 1, accentColor: '#22c55e', height: '5px' }}
                  />
                </div>
                {/* B Gain */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                  <span style={{ width: '50px', fontSize: '11px', color: '#60a5fa' }}>蓝 (B): {Math.round(activePreset.gain_b * 100)}%</span>
                  <input
                    type="range"
                    min={0.5}
                    max={1.5}
                    step={0.02}
                    value={activePreset.gain_b}
                    onChange={(e) => handleSliderChange('gain_b', Number(e.target.value))}
                    style={{ flex: 1, accentColor: '#3b82f6', height: '5px' }}
                  />
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
