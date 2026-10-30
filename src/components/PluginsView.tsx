import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Puzzle,
  FolderOpen,
  RotateCcw,
  Boxes,
  Layers,
} from 'lucide-react';
import { HardwareState } from '../types/hardware';
import { RevBadge, RevSwitch } from './ui';
import { BUILTIN_MODULE_SLOTS, BuiltinModuleSlot } from '../plugins/plugin_manifest';

export interface PluginServiceConfig {
  executable: string;
  args?: string[];
  elevated?: boolean;
  auto_start?: boolean;
  auto_stop?: boolean;
  port?: number;
  health_url?: string;
}

export interface CustomPluginItem {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  plugin_type: string;
  order?: number;
  enabled: boolean;
  dir_name: string;
  target_slot?: string;
  slot_height?: string;
  service?: PluginServiceConfig;
  shortcuts?: any[];
  watchers?: any[];
  permissions?: string[];
  min_host_version?: string;
  is_compatible?: boolean;
  compatibility_notice?: string;
}

export interface PluginsViewProps {
  status: HardwareState;
  moduleToggles?: Record<string, boolean>;
  onToggleModule?: (id: string, enabled: boolean) => void;
  waterCoolerEnabled?: boolean;
  onToggleWaterCooler?: (enabled: boolean) => void;
  activeSkin?: string | null;
  onApplySkin?: (skinId: string | null) => void;
  activeBackground?: string | null;
  onApplyBackground?: (backgroundId: string | null) => void;
  onCustomPluginsUpdated?: () => void;
}

export const PluginsView: React.FC<PluginsViewProps> = ({
  status,
  moduleToggles,
  onToggleModule,
  waterCoolerEnabled: propWaterCoolerEnabled,
  onToggleWaterCooler,
  activeSkin,
  onApplySkin,
  activeBackground,
  onApplyBackground,
  onCustomPluginsUpdated,
}) => {
  // Water cooler special toggle handling
  const [waterCoolerEnabled, setWaterCoolerEnabled] = useState<boolean>(() => {
    return propWaterCoolerEnabled ?? status.water_cooler_enabled ?? status.support_flags?.has_water_cooler ?? false;
  });
  const [togglingWaterCooler, setTogglingWaterCooler] = useState(false);

  useEffect(() => {
    if (typeof propWaterCoolerEnabled === 'boolean') {
      setWaterCoolerEnabled(propWaterCoolerEnabled);
    } else if (typeof status.water_cooler_enabled === 'boolean') {
      setWaterCoolerEnabled(status.water_cooler_enabled);
    }
  }, [propWaterCoolerEnabled, status.water_cooler_enabled]);

  const handleToggleWaterCooler = async () => {
    if (togglingWaterCooler) return;
    setTogglingWaterCooler(true);
    const nextVal = !waterCoolerEnabled;
    setWaterCoolerEnabled(nextVal);
    try {
      await invoke('set_water_cooler_enabled', { enabled: nextVal });
      if (onToggleWaterCooler) {
        onToggleWaterCooler(nextVal);
      }
    } catch (e) {
      console.error('Failed to toggle water cooler module:', e);
      setWaterCoolerEnabled(!nextVal);
    } finally {
      setTogglingWaterCooler(false);
    }
  };

  const isModuleOn = (id: string) => {
    if (id === 'water') {
      return waterCoolerEnabled;
    }
    return moduleToggles?.[id] ?? true;
  };

  const handleModuleSwitch = (slot: BuiltinModuleSlot, nextVal: boolean) => {
    if (slot.isEssential) return;
    if (slot.id === 'water') {
      handleToggleWaterCooler();
      return;
    }
    if (onToggleModule) {
      onToggleModule(slot.id, nextVal);
    }
  };

  // Custom Plugins State
  const [customPlugins, setCustomPlugins] = useState<CustomPluginItem[]>([]);
  const [loadingCustom, setLoadingCustom] = useState(false);
  const [currentSkin, setCurrentSkin] = useState<string | null>(activeSkin || null);
  const [currentBg, setCurrentBg] = useState<string | null>(activeBackground || null);

  useEffect(() => {
    setCurrentSkin(activeSkin || null);
  }, [activeSkin]);

  useEffect(() => {
    setCurrentBg(activeBackground || null);
  }, [activeBackground]);

  const fetchCustomPlugins = async (isManual = false) => {
    setLoadingCustom(true);
    try {
      const res = await invoke<CustomPluginItem[]>('get_custom_plugins');
      const list = res || [];
      setCustomPlugins(list);

      // 手动点击刷新时，连带对当前激活的皮肤与背景执行热重载 (Hot-Reload)
      if (isManual) {
        let currentSkin = activeSkin;
        let currentBg = activeBackground;
        try {
          const backendSkin = await invoke<string | null>('get_active_skin');
          if (backendSkin !== undefined) {
            currentSkin = backendSkin;
          }
          const backendBg = await invoke<string | null>('get_active_background');
          if (backendBg !== undefined) {
            currentBg = backendBg;
          }
        } catch (_) {}

        if (onApplySkin) {
          if (currentSkin && list.some((p) => p.id === currentSkin)) {
            onApplySkin(currentSkin);
          } else if (activeSkin) {
            onApplySkin(null);
          }
        }

        if (onApplyBackground) {
          if (currentBg && list.some((p) => p.id === currentBg)) {
            onApplyBackground(currentBg);
          } else if (activeBackground) {
            onApplyBackground(null);
          }
        }
      }

      if (onCustomPluginsUpdated) {
        onCustomPluginsUpdated();
      }
    } catch (err) {
      console.error('Failed to load custom plugins:', err);
    } finally {
      setLoadingCustom(false);
    }
  };

  useEffect(() => {
    fetchCustomPlugins();
  }, []);

  const handleOpenPluginsDir = async () => {
    try {
      await invoke('open_plugins_dir');
    } catch (err) {
      console.error('Failed to open plugins directory:', err);
    }
  };

  const handleToggleCustomPlugin = async (plugin: CustomPluginItem) => {
    // 皮肤主题互斥逻辑：同一个时间只能开启一个皮肤，滑开新皮肤时其他皮肤开关瞬间弹回关闭
    if (plugin.plugin_type === 'skin') {
      const isCurrentlyActive = currentSkin === plugin.id;
      const targetSkin = isCurrentlyActive ? null : plugin.id;

      // 1. 立即同步本地状态：其他皮肤开关即刻弹回关闭，当前项开或关
      setCurrentSkin(targetSkin);
      setCustomPlugins((prev) =>
        prev.map((p) => {
          if (p.plugin_type === 'skin') {
            return { ...p, enabled: p.id === targetSkin };
          }
          return p;
        })
      );

      // 2. 异步通知主窗口与后端装载/卸载样式
      if (onApplySkin) {
        onApplySkin(targetSkin);
      }
      if (onCustomPluginsUpdated) {
        onCustomPluginsUpdated();
      }
      return;
    }

    // 背景壁纸互斥逻辑：同一个时间只能生效一张壁纸，滑开新壁纸时其他壁纸开关瞬间弹回关闭
    if (plugin.plugin_type === 'background') {
      const isCurrentlyActive = currentBg === plugin.id;
      const targetBg = isCurrentlyActive ? null : plugin.id;

      // 1. 立即同步本地状态：其他壁纸开关即刻弹回关闭
      setCurrentBg(targetBg);
      setCustomPlugins((prev) =>
        prev.map((p) => {
          if (p.plugin_type === 'background') {
            return { ...p, enabled: p.id === targetBg };
          }
          return p;
        })
      );

      // 2. 异步通知主窗口与后端装载/卸载壁纸
      if (onApplyBackground) {
        onApplyBackground(targetBg);
      }
      if (onCustomPluginsUpdated) {
        onCustomPluginsUpdated();
      }
      return;
    }

    if (plugin.is_compatible === false) {
      return;
    }

    const nextVal = !plugin.enabled;
    setCustomPlugins((prev) =>
      prev.map((p) => (p.id === plugin.id ? { ...p, enabled: nextVal } : p))
    );
    try {
      await invoke('set_custom_plugin_enabled', { id: plugin.id, enabled: nextVal });
      if (plugin.plugin_type === 'widget') {
        if (nextVal) {
          await invoke('open_custom_widget', { pluginId: plugin.id });
        } else {
          await invoke('close_custom_widget', { pluginId: plugin.id });
        }
      }
      if (onCustomPluginsUpdated) {
        onCustomPluginsUpdated();
      }
    } catch (err) {
      console.error('Failed to toggle custom plugin:', err);
      setCustomPlugins((prev) =>
        prev.map((p) => (p.id === plugin.id ? { ...p, enabled: !nextVal } : p))
      );
    }
  };

  // Builtin modules: Strictly exclude "system", "plugins", and "sponsor"
  const builtinSlots = BUILTIN_MODULE_SLOTS.filter(
    (slot) => slot.id !== 'system' && slot.id !== 'plugins' && slot.id !== 'sponsor'
  );

  return (
    <div className="sys-page">
      {/* 1. 内置插件 */}
      <div className="glass-card">
        <div className="plugin-section-header">
          <div className="plugin-section-title-wrap">
            <div className="plugin-section-title">
              <Puzzle size={16} />
              <span>内置插件</span>
              <RevBadge variant="accent">Safe-Mode 模块隔离</RevBadge>
            </div>
            <div className="plugin-section-desc">
              系统核心功能插槽，可根据使用偏好按需开启或关闭。关闭后模块将从顶栏导航与后台调度中隐藏
            </div>
          </div>
          <RevBadge variant="default">共 {builtinSlots.length} 项</RevBadge>
        </div>

        <div className="plugins-list">
          {builtinSlots.map((slot) => {
            const Icon = slot.icon;
            const active = isModuleOn(slot.id);
            return (
              <div key={slot.id} className="plugin-item-card">
                <div className="plugin-item-left">
                  <div className={`plugin-item-icon-box ${active ? 'active' : ''}`}>
                    <Icon size={16} />
                  </div>
                  <div className="plugin-item-info">
                    <div className="plugin-item-header">
                      <span className="plugin-item-name">{slot.name}</span>
                      <span className="plugin-item-id">[{slot.id}]</span>
                    </div>
                    <span className="plugin-item-desc">{slot.description}</span>
                  </div>
                </div>

                <div className="plugin-item-controls">
                  <RevSwitch
                    checked={active}
                    onChange={(val) => handleModuleSwitch(slot, val)}
                    disabled={slot.id === 'water' && togglingWaterCooler}
                    size="sm"
                    activeColor="var(--status-ok)"
                  />
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* 2. 自定义插件 */}
      <div className="glass-card">
        <div className="plugin-section-header">
          <div className="plugin-section-title-wrap">
            <div className="plugin-section-title">
              <Boxes size={16} />
              <span>自定义插件</span>
              <RevBadge variant="info">社区生态扩展</RevBadge>
            </div>
            <div className="plugin-section-desc">
              支持加载由社区与开发者提供的扩展（桌面透明挂件、调谐脚本、自动化服务等），基于安全沙盒运行
            </div>
          </div>

          <div className="sys-row-controls">
            <button
              onClick={handleOpenPluginsDir}
              className="toggle-btn"
              title="打开用户插件目录 (%APPDATA%\OpenRevo\plugins)"
            >
              <FolderOpen size={13} />
              <span>打开插件目录</span>
            </button>
            <button
              onClick={() => fetchCustomPlugins(true)}
              disabled={loadingCustom}
              className="toggle-btn"
              title="刷新已安装插件列表并热重载当前样式"
            >
              <RotateCcw size={13} className={loadingCustom ? 'animate-spin' : ''} />
              <span>刷新</span>
            </button>
          </div>
        </div>

        {customPlugins.length === 0 ? (
          <div className="plugin-empty-state">
            <div className="plugin-empty-icon">
              <Boxes size={24} />
            </div>
            <div className="plugin-empty-title">暂未检测到第三方自定义插件</div>
            <div className="plugin-empty-desc">
              将包含 manifest.json 的插件文件夹或 .rhai 脚本放入 plugins 目录后点击右上角“刷新”即可加载。详情可查阅《OpenRevo 第三方插件开发手册》。
            </div>
            <button onClick={handleOpenPluginsDir} className="toggle-btn active-indigo">
              <FolderOpen size={13} />
              <span>打开 plugins 目录</span>
            </button>
          </div>
        ) : (
          <div className="plugins-list">
            {customPlugins.map((plugin) => {
              const isPluginActive =
                plugin.plugin_type === 'skin'
                  ? currentSkin === plugin.id
                  : plugin.plugin_type === 'background'
                  ? currentBg === plugin.id
                  : plugin.enabled;
              return (
                <div key={plugin.id} className="plugin-item-card">
                  <div className="plugin-item-left">
                    <div className={`plugin-item-icon-box ${isPluginActive ? 'active' : ''}`}>
                      <Layers size={16} />
                    </div>
                    <div className="plugin-item-info">
                      <div className="plugin-item-header">
                        <span className="plugin-item-name">{plugin.name}</span>
                        <span className="plugin-item-id">[{plugin.id}]</span>
                        <RevBadge variant="default">v{plugin.version}</RevBadge>
                        {plugin.plugin_type === 'skin' ? (
                          <RevBadge variant="accent">主窗皮肤</RevBadge>
                        ) : plugin.plugin_type === 'background' ? (
                          <RevBadge variant="warning">背景壁纸</RevBadge>
                        ) : plugin.plugin_type === 'tab' ? (
                          <RevBadge variant="info">面板插槽</RevBadge>
                        ) : plugin.plugin_type === 'widget' ? (
                          <RevBadge variant="success">桌面挂件</RevBadge>
                        ) : plugin.plugin_type === 'slot' ? (
                          <RevBadge variant="accent">微插槽 {plugin.target_slot ? `[${plugin.target_slot}]` : ''}</RevBadge>
                        ) : (
                          <RevBadge variant="default">{plugin.plugin_type}</RevBadge>
                        )}
                        {plugin.is_compatible === false ? (
                          <RevBadge variant="error" title={plugin.compatibility_notice}>
                            ⚠️ {plugin.compatibility_notice || `需 OpenRevo v${plugin.min_host_version}+`}
                          </RevBadge>
                        ) : plugin.min_host_version ? (
                          <RevBadge variant="default" title={`最低要求 OpenRevo v${plugin.min_host_version}`}>
                            v{plugin.min_host_version}+
                          </RevBadge>
                        ) : null}
                      </div>
                      <span className="plugin-item-desc">{plugin.description}</span>
                      <div className="plugin-item-meta">
                        <span>作者: {plugin.author}</span>
                        <span>目录: {plugin.dir_name}</span>
                      </div>
                    </div>
                  </div>

                  <div className="plugin-item-controls">
                    <RevSwitch
                      checked={isPluginActive}
                      onChange={() => handleToggleCustomPlugin(plugin)}
                      disabled={plugin.is_compatible === false}
                      size="sm"
                      activeColor="var(--status-ok)"
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
