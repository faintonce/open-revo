import React, { useState, useEffect, useRef, useCallback } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { HardwareState, PowerMode, POWER_MODES } from '../types/hardware';
import { OverviewView } from './OverviewView';
import { LightingStudio } from './LightingStudio';
import { GpuModeView } from './GpuModeView';
import { PowerTuningView } from './PowerTuningView';
import { SponsorView } from './SponsorView';
import { CustomTuningPanel } from './CustomTuningPanel';
import { WaterCoolerView, CoolerTelemetry } from './WaterCoolerView';
import { SystemSettingsView, OsdConfig } from './SystemSettingsView';
import { PluginsView, CustomPluginItem } from './PluginsView';
import { PluginTabHost } from './PluginTabHost';
import { ModuleErrorBoundary } from './ModuleErrorBoundary';
import { BUILTIN_MODULE_SLOTS, BuiltinTabId, isSlotActive, ModelCapabilities } from '../plugins/plugin_manifest';
import logoImg from '../assets/logo.png';
import {
  Palette,
  Minimize2,
  X,
  Minus,
  Shield,
  Loader2,
  Zap,
  Square,
  Copy,
  Layers,
} from 'lucide-react';

export type TabId = BuiltinTabId | string;

interface FullDashboardProps {
  status: HardwareState;
  theme?: 'mono' | 'cyber';
  activeSkin?: string | null;
  activeBackground?: string | null;
  onApplySkin?: (skinId: string | null) => void;
  onApplyBackground?: (backgroundId: string | null) => void;
  onToggleTheme?: () => void;
  onSetPowerMode: (mode: PowerMode) => void;
  onToggleFanBoost: (targetRpm?: number) => void;
  onToggleBatteryLimit: () => void;
  onSwitchRefreshRate: (hz: number, invokeBackend?: boolean) => void;
  onToggleWinKey: () => void;
  onToggleAutostart: () => void;
  onToggleOemService?: (takeOver: boolean) => Promise<void> | void;
  onCollapseDashboard: () => void;
  onMinimize: () => void;
  onClose: () => void;
  initialTab?: TabId;
  initialOpenCustomTuning?: boolean;
  capabilities?: ModelCapabilities;
  previewMode?: 'real' | 'thin_light' | 'gaming';
  onPreviewFormFactor?: (mode: 'real' | 'thin_light' | 'gaming') => void;
}

import { invoke } from '@tauri-apps/api/core';

export const FullDashboard: React.FC<FullDashboardProps> = ({
  status,
  theme = 'mono',
  activeSkin,
  activeBackground,
  onApplySkin,
  onApplyBackground,
  onToggleTheme,
  onSetPowerMode,
  onToggleFanBoost,
  onToggleBatteryLimit,
  onSwitchRefreshRate,
  onToggleWinKey,
  onToggleAutostart,
  onToggleOemService,
  onCollapseDashboard,
  onMinimize,
  onClose,
  initialTab,
  initialOpenCustomTuning = false,
  capabilities: propCapabilities,
  previewMode: propPreviewMode,
  onPreviewFormFactor: propOnPreviewFormFactor,
}) => {
  const [activeTab, setActiveTab] = useState<TabId>(initialTab || 'overview');
  const [isCustomTuningOpen, setIsCustomTuningOpen] = useState(initialOpenCustomTuning);
  const [showOemModal, setShowOemModal] = useState(false);
  const [isSwitchingService, setIsSwitchingService] = useState(false);
  const [profileVersion, setProfileVersion] = useState(0);
  const [copiedVersion, setCopiedVersion] = useState(false);

  const handleCopyVersion = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(`v${__APP_VERSION__}`);
    setCopiedVersion(true);
    setTimeout(() => setCopiedVersion(false), 1500);
  };

  const checkOemGuard = (action: () => void) => {
    if (status.oem_running) {
      setShowOemModal(true);
      return;
    }
    action();
  };

  const handleTakeoverOem = async () => {
    setIsSwitchingService(true);
    try {
      if (onToggleOemService) {
        await onToggleOemService(true);
      } else {
        await invoke('toggle_oem_service', { takeOver: true });
      }
      setShowOemModal(false);
    } finally {
      setIsSwitchingService(false);
    }
  };

  useEffect(() => {
    if (initialTab) {
      setActiveTab(initialTab);
    }
  }, [initialTab]);

  useEffect(() => {
    if (initialOpenCustomTuning) {
      setIsCustomTuningOpen(true);
    }
  }, [initialOpenCustomTuning]);

  // Persistent Module Toggles (Safe-Mode Isolation)
  const [moduleToggles, setModuleToggles] = useState<Record<string, boolean>>({});

  useEffect(() => {
    invoke<Record<string, boolean>>('get_module_toggles')
      .then((res) => {
        if (res) setModuleToggles(res);
      })
      .catch((err) => console.warn('Failed to load module toggles:', err));
  }, []);

  const [localWaterCoolerEnabled, setLocalWaterCoolerEnabled] = useState<boolean | null>(null);
  const isWaterCoolerEnabled = localWaterCoolerEnabled !== null
    ? localWaterCoolerEnabled
    : (moduleToggles.water !== undefined
        ? moduleToggles.water
        : (status.water_cooler_enabled ?? status.support_flags?.has_water_cooler ?? false));

  useEffect(() => {
    if (typeof moduleToggles.water === 'boolean') {
      setLocalWaterCoolerEnabled(moduleToggles.water);
    } else if (typeof status.water_cooler_enabled === 'boolean') {
      setLocalWaterCoolerEnabled(status.water_cooler_enabled);
    }
  }, [moduleToggles.water, status.water_cooler_enabled]);

  // Safeguard: if water cooler module is disabled, fallback to overview
  useEffect(() => {
    if (!isWaterCoolerEnabled && activeTab === 'water') {
      setActiveTab('overview');
    }
  }, [isWaterCoolerEnabled, activeTab]);

  // Model Capabilities (Declarative Hardware Driver Contract)
  const realCapabilitiesRef = useRef<ModelCapabilities | undefined>(undefined);
  const [internalCapabilities, setInternalCapabilities] = useState<ModelCapabilities | undefined>(undefined);
  const [internalPreviewMode, setInternalPreviewMode] = useState<'real' | 'thin_light' | 'gaming'>('real');

  const capabilities = propCapabilities !== undefined ? propCapabilities : internalCapabilities;
  const previewMode = propPreviewMode !== undefined ? propPreviewMode : internalPreviewMode;

  useEffect(() => {
    if (propCapabilities === undefined) {
      invoke<ModelCapabilities>('get_model_capabilities')
        .then((caps) => {
          if (caps) {
            realCapabilitiesRef.current = caps;
            setInternalCapabilities(caps);
          }
        })
        .catch((err) => console.warn('Failed to load model capabilities:', err));
    }
  }, [propCapabilities]);

  const handlePreviewFormFactor = useCallback((mode: 'real' | 'thin_light' | 'gaming') => {
    if (propOnPreviewFormFactor) {
      propOnPreviewFormFactor(mode);
      return;
    }
    setInternalPreviewMode(mode);
    if (mode === 'real') {
      if (realCapabilitiesRef.current) {
        setInternalCapabilities({ ...realCapabilitiesRef.current });
      }
    } else if (mode === 'thin_light') {
      setInternalCapabilities({
        ...(realCapabilitiesRef.current || {}),
        id: 'preview_xingyao_14',
        name: 'MECHREVO Xingyao 14 (UI Preview)',
        form_factor: 'thin_light',
        ui_preset: 'ultrabook_compact',
        discrete_gpu: false,
        water_cooling: false,
        fan_custom_curve: false,
        battery_charge_limit: true,
        power_modes: ['office', 'balance', 'turbo'],
        rgb_keyboard: { supported: true, type: 'monochrome_stepped', steps: [0, 1, 2, 3, 4] },
        is_debug_mode: true,
      });
    } else if (mode === 'gaming') {
      setInternalCapabilities({
        ...(realCapabilitiesRef.current || {}),
        id: 'preview_gaming',
        name: 'MECHREVO Gaming Laptop (UI Preview)',
        form_factor: 'gaming',
        ui_preset: 'gaming_full',
        discrete_gpu: true,
        water_cooling: true,
        fan_custom_curve: true,
        battery_charge_limit: true,
        power_modes: ['office', 'balance', 'turbo', 'custom'],
        rgb_keyboard: { supported: true, type: 'per_key_rgb' },
        is_debug_mode: true,
      });
    }
  }, [propOnPreviewFormFactor]);

  // On non-dGPU machines, GPU tab remains accessible for display brightness & color tuning controls

  const handleToggleModule = async (id: string, enabled: boolean) => {
    setModuleToggles((prev) => ({ ...prev, [id]: enabled }));
    try {
      await invoke('set_module_enabled', { id, enabled });
      if (id === 'water') {
        setLocalWaterCoolerEnabled(enabled);
      }
    } catch (e) {
      console.error('Failed to update module toggle:', e);
      setModuleToggles((prev) => ({ ...prev, [id]: !enabled }));
    }
  };

  // Safe Mode fallback: if currently viewing a tab that got disabled, safely revert to overview
  useEffect(() => {
    if (moduleToggles[activeTab] === false) {
      setActiveTab('overview');
    }
  }, [moduleToggles, activeTab]);

  // Third-party Tab Plugins State
  const [tabPlugins, setTabPlugins] = useState<CustomPluginItem[]>([]);

  const loadTabPlugins = useCallback(async () => {
    try {
      const plugins = await invoke<CustomPluginItem[]>('get_custom_plugins');
      if (plugins) {
        setTabPlugins(plugins.filter((p) => p.plugin_type === 'tab' && p.enabled && p.is_compatible !== false));
      }
      const toggles = await invoke<Record<string, boolean>>('get_module_toggles');
      if (toggles) {
        setModuleToggles(toggles);
      }
    } catch (e) {
      console.warn('Failed to load tab plugins or module toggles:', e);
    }
  }, []);

  useEffect(() => {
    loadTabPlugins();
  }, [loadTabPlugins]);

  // Safe fallback if custom tab got disabled
  useEffect(() => {
    const isBuiltin = BUILTIN_MODULE_SLOTS.some((s) => s.id === activeTab);
    const isCustomActive = tabPlugins.some((p) => p.id === activeTab);
    if (!isBuiltin && !isCustomActive && activeTab !== 'overview') {
      setActiveTab('overview');
    }
  }, [tabPlugins, activeTab]);

  const [isCoolerConnected, setIsCoolerConnected] = useState(false);

  useEffect(() => {
    invoke<CoolerTelemetry>('get_water_cooler_status')
      .then((res) => {
        if (res) setIsCoolerConnected(res.connected);
      })
      .catch(() => {});
  }, []);

  const [osdConfig, setOsdConfig] = useState<OsdConfig>({
    enabled: true,
    theme: 'cinema',
    scale: '100%',
    position: 'bottom-center',
  });

  useEffect(() => {
    invoke<any>('get_osd_config')
      .then((res) => {
        if (res) {
          setOsdConfig({
            enabled: res.enabled ?? true,
            theme: res.theme || 'cinema',
            scale: res.scale || '100%',
            position: res.position || 'bottom-center',
          });
        }
      })
      .catch(() => {});
  }, []);

  const handleUpdateOsdConfig = async (newCfg: OsdConfig) => {
    setOsdConfig(newCfg);
    try {
      await invoke('save_osd_config', {
        config: newCfg,
        enabled: newCfg.enabled,
        theme: newCfg.theme,
        scale: newCfg.scale,
        position: newCfg.position,
      });
    } catch (err) {
      console.error('Failed to save OSD config:', err);
    }
  };

  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    try {
      const win = getCurrentWindow();
      win.isMaximized().then(setIsMaximized).catch(() => {});
      let unlistenResize: (() => void) | undefined;
      win.listen('tauri://resize', async () => {
        try {
          const max = await win.isMaximized();
          setIsMaximized(max);
        } catch (_) {}
      }).then((fn) => { unlistenResize = fn; });

      return () => {
        if (unlistenResize) unlistenResize();
      };
    } catch (_) {}
  }, []);

  // Sub-nav tab horizontal scroll state & edge fade indicators
  const navBarRef = useRef<HTMLDivElement>(null);
  const [canScrollNavLeft, setCanScrollNavLeft] = useState(false);
  const [canScrollNavRight, setCanScrollNavRight] = useState(false);

  const checkNavScroll = () => {
    const el = navBarRef.current;
    if (!el) return;
    setCanScrollNavLeft(el.scrollLeft > 2);
    setCanScrollNavRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 2);
  };

  useEffect(() => {
    checkNavScroll();
    const el = navBarRef.current;
    if (!el) return;
    el.addEventListener('scroll', checkNavScroll, { passive: true });
    window.addEventListener('resize', checkNavScroll);
    return () => {
      el.removeEventListener('scroll', checkNavScroll);
      window.removeEventListener('resize', checkNavScroll);
    };
  }, [status, isWaterCoolerEnabled, isCoolerConnected, moduleToggles, tabPlugins]);

  useEffect(() => {
    const el = navBarRef.current;
    if (!el) return;
    const activeBtn = el.querySelector('.nav-tab.active') as HTMLElement;
    if (activeBtn) {
      activeBtn.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' });
    }
    checkNavScroll();
  }, [activeTab]);

  const handleToggleMaximize = async () => {
    try {
      const res = await invoke<boolean>('toggle_maximize_window');
      setIsMaximized(res);
    } catch (e) {
      console.warn('Failed to toggle maximize:', e);
      try {
        const win = getCurrentWindow();
        const max = await win.isMaximized();
        if (max) {
          await win.unmaximize();
          await invoke('resize_window', { expanded: true });
          setIsMaximized(false);
        } else {
          await win.maximize();
          setIsMaximized(true);
        }
      } catch (_) {}
    }
  };

  const currentModeInfo = POWER_MODES.find((m) => m.id === status.power_mode) || POWER_MODES[1];

  return (

    <div
      className="acrylic-container openrevo-shell"
      data-slot="shell"
      data-theme={theme}
      data-form-factor={capabilities?.form_factor || 'gaming'}
    >
      {/* Top Titlebar: Left Logo + Center Draggable Flex + Right Controls */}
      <div
        className="mech-titlebar shell-titlebar"
        data-slot="titlebar"
        data-tauri-drag-region
        onDoubleClick={handleToggleMaximize}
        style={{ flexShrink: 0, userSelect: 'none', cursor: 'default' }}
      >
        <div
          style={{ display: 'flex', alignItems: 'center', gap: '10px' }}
          data-tauri-drag-region
        >
          <div className="logo-badge" data-tauri-drag-region>
            <img
              src={logoImg}
              alt="OpenRevo Logo"
              data-tauri-drag-region
              style={{
                width: '22px',
                height: '22px',
                borderRadius: '4px',
                objectFit: 'contain',
                filter: `drop-shadow(0 0 8px ${currentModeInfo.color})`,
              }}
            />
            <span style={{ fontSize: '15px', fontWeight: 800, letterSpacing: '0.08em' }} data-tauri-drag-region>OPENREVO</span>
            <span
              className="badge-tag"
              onClick={handleCopyVersion}
              title={`点击复制版本号: v${__APP_VERSION__}`}
            >
              {copiedVersion ? '已复制' : `PRO v${__APP_VERSION__}`}
            </span>
            {status.is_mock && (
              <span
                style={{
                  fontSize: '9px',
                  background: 'rgba(245, 158, 11, 0.2)',
                  border: '1px solid rgba(245, 158, 11, 0.5)',
                  color: '#fbbf24',
                  padding: '1px 6px',
                  borderRadius: '4px',
                  fontWeight: 600,
                }}
              >
                SAFE MOCK
              </span>
            )}
          </div>
        </div>

        {/* Central Draggable Area */}
        <div
          style={{ flex: 1, height: '100%', minWidth: '40px' }}
          data-tauri-drag-region
        />

        {/* Window controls - Strictly isolated from drag events */}
        <div
          style={{ display: 'flex', alignItems: 'center', gap: '6px', zIndex: 20 }}
          data-tauri-drag-region="false"
          onMouseDown={(e) => e.stopPropagation()}
          onDoubleClick={(e) => e.stopPropagation()}
        >
          {status.oem_installed && status.oem_running && (
            <button
              onClick={() => handleTakeoverOem()}
              disabled={isSwitchingService}
              title="检测到机械革命官方电竞服务正在运行。点击后将一键停止并禁用官方服务，避免与 OpenRevo 争抢主板 EC 寄存器与屏幕闪烁。"
              style={{
                padding: '3px 8px',
                borderRadius: '5px',
                fontSize: '10px',
                fontWeight: 600,
                cursor: isSwitchingService ? 'wait' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                whiteSpace: 'nowrap',
                transition: 'all 0.15s ease',
                background: 'rgba(245, 158, 11, 0.15)',
                color: '#fbbf24',
                border: '1px solid rgba(245, 158, 11, 0.35)',
              }}
            >
              {isSwitchingService ? <Loader2 size={11} className="animate-spin" /> : <Shield size={11} />}
              <span>接管官方控制台</span>
            </button>
          )}

          {/* 主题切换开关 (单色工业风 / 赛博多彩风格) */}
          {onToggleTheme && (
            <button
              onClick={onToggleTheme}
              title={theme === 'mono' ? '切换到：多彩风格' : '切换到：极致单色工业风'}
              style={{
                background: theme === 'cyber' ? 'rgba(56, 189, 248, 0.15)' : 'transparent',
                border: theme === 'cyber' ? '1px solid rgba(56, 189, 248, 0.4)' : '1px solid rgba(255, 255, 255, 0.1)',
                color: theme === 'cyber' ? '#38bdf8' : 'var(--text-muted)',
                cursor: 'pointer',
                padding: '4px 7px',
                borderRadius: '6px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: 'all 0.15s ease',
              }}
            >
              <Palette className="w-3.5 h-3.5" />
            </button>
          )}

          {/* 迷你抽屉模式切换 */}
          <button
            onClick={onCollapseDashboard}
            title="折叠为迷你抽屉模式"
            style={{
              background: 'transparent',
              border: '1px solid rgba(255,255,255,0.1)',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '4px 8px',
              borderRadius: '6px',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              fontSize: '11px',
            }}
          >
            <Minimize2 className="w-3 h-3" />
            <span>迷你模式</span>
          </button>

          {/* 窗口 / 全屏切换 */}
          <button
            onClick={handleToggleMaximize}
            title={isMaximized ? '还原为 960×740 窗口模式' : '最大化全屏显示'}
            style={{
              background: 'transparent',
              border: '1px solid rgba(255,255,255,0.1)',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '4px 8px',
              borderRadius: '6px',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              fontSize: '11px',
            }}
          >
            {isMaximized ? <Copy className="w-3 h-3" /> : <Square className="w-3 h-3" />}
            <span>{isMaximized ? '窗口' : '全屏'}</span>
          </button>

          <button
            onClick={onMinimize}
            title="最小化"
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '4px',
              borderRadius: '4px',
            }}
          >
            <Minus className="w-4 h-4" />
          </button>
          <button
            onClick={onClose}
            title="关闭窗口"
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '4px',
              borderRadius: '4px',
            }}
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Sub-Nav Pill Tabs Header: Single-line, Mouse Wheel Horizontal Scroll, Elegant (Plugin Slot Driven) */}
      <div className="sub-nav-wrapper shell-navbar" data-slot="navbar">
        {canScrollNavLeft && <div className="sub-nav-fade sub-nav-fade-left" />}
        <div
          ref={navBarRef}
          className="sub-nav-bar"
          onWheel={(e) => {
            if (e.deltaY !== 0) {
              e.currentTarget.scrollLeft += e.deltaY;
            }
          }}
        >
          {BUILTIN_MODULE_SLOTS.filter((slot) => isSlotActive(slot, { status, isWaterCoolerEnabled, isCoolerConnected, moduleToggles, capabilities }))
            .sort((a, b) => a.order - b.order)
            .map((slot) => {
              const IconComponent = slot.icon;
              const isActive = activeTab === slot.id;
              return (
                <button
                  key={slot.id}
                  onClick={() => setActiveTab(slot.id)}
                  className={`nav-tab ${slot.tabClassName} ${isActive ? 'active' : ''}`}
                >
                  <IconComponent size={14} className={slot.iconClass || 'w-3.5 h-3.5'} />
                  <span>{slot.name}</span>
                  {slot.renderBadge && slot.renderBadge({ status, isWaterCoolerEnabled, isCoolerConnected })}
                </button>
              );
            })}
          {tabPlugins.map((plugin) => {
            const isActive = activeTab === plugin.id;
            return (
              <button
                key={plugin.id}
                onClick={() => setActiveTab(plugin.id)}
                className={`nav-tab tab-plugin-custom ${isActive ? 'active' : ''}`}
                title={plugin.description}
              >
                <Layers size={14} className="w-3.5 h-3.5" />
                <span>{plugin.name}</span>
              </button>
            );
          })}
        </div>
        {canScrollNavRight && <div className="sub-nav-fade sub-nav-fade-right" />}
      </div>

      {/* Main Body Viewport Slot */}
      <div className="shell-viewport" data-slot="viewport">
        {activeTab === 'overview' && (
          <ModuleErrorBoundary moduleName="状态概览">
            <OverviewView
              status={status}
              capabilities={capabilities}
              osdConfig={osdConfig}
              onUpdateOsdConfig={handleUpdateOsdConfig}
              onSetPowerMode={onSetPowerMode}
              onToggleFanBoost={onToggleFanBoost}
              onToggleBatteryLimit={onToggleBatteryLimit}
              onSwitchRefreshRate={onSwitchRefreshRate}
              onOpenCustomTuning={() => setIsCustomTuningOpen(true)}
              checkOemGuard={checkOemGuard}
              profileVersion={profileVersion}
            />
          </ModuleErrorBoundary>
        )}

        {activeTab === 'gpu' && (
          <ModuleErrorBoundary moduleName="显卡模式" onNavigateToOverview={() => setActiveTab('overview')}>
            <GpuModeView />
          </ModuleErrorBoundary>
        )}

        {activeTab === 'tuning' && (
          <ModuleErrorBoundary moduleName="电源管理" onNavigateToOverview={() => setActiveTab('overview')}>
            <PowerTuningView />
          </ModuleErrorBoundary>
        )}

        {activeTab === 'lighting' && (
          <ModuleErrorBoundary moduleName="RGB 灯效工坊" onNavigateToOverview={() => setActiveTab('overview')}>
            <LightingStudio supportFlags={status.support_flags} />
          </ModuleErrorBoundary>
        )}

        {activeTab === 'water' && isWaterCoolerEnabled && (
          <ModuleErrorBoundary moduleName="水冷魔盒 (BLE)" onNavigateToOverview={() => setActiveTab('overview')}>
            <WaterCoolerView
              status={status}
              onConnectionChange={setIsCoolerConnected}
            />
          </ModuleErrorBoundary>
        )}

        {activeTab === 'system' && (
          <ModuleErrorBoundary moduleName="系统设置" onNavigateToOverview={() => setActiveTab('overview')}>
            <SystemSettingsView
              status={status}
              osdConfig={osdConfig}
              onUpdateOsdConfig={handleUpdateOsdConfig}
              onToggleWinKey={onToggleWinKey}
              onToggleAutostart={onToggleAutostart}
              onToggleWaterCooler={(enabled) => setLocalWaterCoolerEnabled(enabled)}
              moduleToggles={moduleToggles}
              onToggleModule={handleToggleModule}
              capabilities={capabilities}
              previewMode={previewMode}
              onPreviewFormFactor={handlePreviewFormFactor}
            />
          </ModuleErrorBoundary>
        )}

        {activeTab === 'plugins' && (
          <ModuleErrorBoundary moduleName="插件管理" onNavigateToOverview={() => setActiveTab('overview')}>
            <PluginsView
              status={status}
              moduleToggles={moduleToggles}
              onToggleModule={handleToggleModule}
              waterCoolerEnabled={isWaterCoolerEnabled}
              onToggleWaterCooler={(enabled) => {
                setLocalWaterCoolerEnabled(enabled);
                setModuleToggles((prev) => ({ ...prev, water: enabled }));
              }}
              activeSkin={activeSkin}
              onApplySkin={onApplySkin}
              activeBackground={activeBackground}
              onApplyBackground={onApplyBackground}
              onCustomPluginsUpdated={loadTabPlugins}
            />
          </ModuleErrorBoundary>
        )}

        {activeTab === 'sponsor' && (
          <ModuleErrorBoundary moduleName="支持作者" onNavigateToOverview={() => setActiveTab('overview')}>
            <SponsorView />
          </ModuleErrorBoundary>
        )}

        {/* 第三方 Tab 插件渲染沙盒 */}
        {tabPlugins.some((p) => p.id === activeTab) && (
          <ModuleErrorBoundary
            moduleName={tabPlugins.find((p) => p.id === activeTab)?.name || '扩展模块'}
            onNavigateToOverview={() => setActiveTab('overview')}
          >
            <PluginTabHost
              plugin={tabPlugins.find((p) => p.id === activeTab)!}
              status={status}
            />
          </ModuleErrorBoundary>
        )}
      </div>

      {/* Global Custom Tuning Modal Overlay */}
      {isCustomTuningOpen && (
        <CustomTuningPanel
          status={status}
          capabilities={capabilities}
          onToggleFanBoost={onToggleFanBoost}
          onClose={() => {
            setIsCustomTuningOpen(false);
            setProfileVersion((v) => v + 1);
          }}
          onApplied={() => {
            setProfileVersion((v) => v + 1);
          }}
        />
      )}

      {/* OEM Running Modal Alert */}
      {showOemModal && (
        <div className="sys-modal-backdrop" onClick={() => setShowOemModal(false)}>
          <div className="sys-modal-card" onClick={(e) => e.stopPropagation()}>
            <div className="sys-modal-header">
              <div className="sys-modal-icon-badge">
                <Shield size={24} />
              </div>
              <div className="sys-modal-title">官方控制中心正在运行</div>
            </div>
            <div className="sys-modal-body">
              检测到机械革命官方电竞后台正在运行。为防止与官方争抢主板 EC 寄存器并导致<strong>屏幕持续闪烁</strong>，当前处于<strong>安全只读避让模式</strong>。
            </div>
            <div className="sys-modal-actions">
              <button
                onClick={handleTakeoverOem}
                disabled={isSwitchingService}
                className="toggle-btn btn-warning-solid"
              >
                {isSwitchingService ? <Loader2 size={15} className="animate-spin" /> : <Zap size={15} />}
                <span>{isSwitchingService ? '正在接管服务并释放硬件...' : '立即一键接管官方服务'}</span>
              </button>
              <button
                onClick={() => setShowOemModal(false)}
                className="toggle-btn"
              >
                暂不接管，保持只读监控
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
