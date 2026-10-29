import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  FolderOpen,
  Trash2,
  AlertTriangle,
  RotateCcw,
  Crosshair,
  Globe,
  Gauge,
  Lightbulb,
  Zap,
  Cpu,
  Cable,
  Sliders,
  Droplets,
  Download,
  RefreshCw,
  HardDrive,
} from 'lucide-react';
import { HardwareState } from '../types/hardware';
import { RevBadge, RevSwitch } from './ui';
import { ModelCapabilities } from '../plugins/plugin_manifest';

export interface OsdConfig {
  enabled: boolean;
  theme: 'cinema' | 'fade' | 'pill';
  scale: string;
  position: 'top-left' | 'bottom-center' | 'center';
}

export interface SystemSettingsViewProps {
  status: HardwareState;
  osdConfig: OsdConfig;
  onUpdateOsdConfig: (cfg: OsdConfig) => void;
  onToggleWinKey?: () => void;
  onToggleAutostart: () => void;
  onToggleWaterCooler?: (enabled: boolean) => void;
  moduleToggles?: Record<string, boolean>;
  onToggleModule?: (id: string, enabled: boolean) => void;
  capabilities?: ModelCapabilities;
  previewMode?: 'real' | 'thin_light' | 'gaming';
  onPreviewFormFactor?: (mode: 'real' | 'thin_light' | 'gaming') => void;
}

export const SystemSettingsView: React.FC<SystemSettingsViewProps> = ({
  status,
  osdConfig,
  onUpdateOsdConfig: handleUpdateOsdConfig,
  onToggleWinKey: _onToggleWinKey,
  onToggleAutostart,
  onToggleWaterCooler: _onToggleWaterCooler,
  moduleToggles: _moduleToggles,
  onToggleModule: _onToggleModule,
  capabilities,
  previewMode = 'real',
  onPreviewFormFactor,
}) => {

  const [logStatus, setLogStatus] = useState<{
    visible: boolean;
    enabled: boolean;
    level: number;
    path: string;
    filter?: string | null;
  }>({
    visible: false,
    enabled: false,
    level: 1,
    path: '',
    filter: null,
  });

  const [wakeWindowMode, setWakeWindowMode] = useState<'remember' | 'mini' | 'full'>('remember');
  const [miniAutoHide, setMiniAutoHide] = useState(true);
  const [unlockBatteryLimit, setUnlockBatteryLimit] = useState(!!status.unlock_battery_power_limit);
  const [showUnlockBatteryConfirm, setShowUnlockBatteryConfirm] = useState(false);
  const [togglingUnlockBattery, setTogglingUnlockBattery] = useState(false);

  useEffect(() => {
    if (typeof status.unlock_battery_power_limit === 'boolean') {
      setUnlockBatteryLimit(status.unlock_battery_power_limit);
    }
  }, [status.unlock_battery_power_limit]);

  const handleToggleUnlockBattery = async (nextVal: boolean) => {
    if (nextVal) {
      setShowUnlockBatteryConfirm(true);
    } else {
      setTogglingUnlockBattery(true);
      try {
        await invoke('set_unlock_battery_power_limit', { enable: false });
        setUnlockBatteryLimit(false);
      } catch (err: any) {
        alert('关闭解除离电限制失败: ' + err);
      } finally {
        setTogglingUnlockBattery(false);
      }
    }
  };

  const handleConfirmUnlockBattery = async () => {
    setTogglingUnlockBattery(true);
    try {
      await invoke('set_unlock_battery_power_limit', { enable: true });
      setUnlockBatteryLimit(true);
      setShowUnlockBatteryConfirm(false);
    } catch (err: any) {
      alert('开启解除离电限制失败: ' + err);
    } finally {
      setTogglingUnlockBattery(false);
    }
  };

  useEffect(() => {
    invoke<string>('get_wake_window_mode')
      .then((mode) => {
        if (mode === 'remember' || mode === 'mini' || mode === 'full') {
          setWakeWindowMode(mode);
        }
      })
      .catch(() => { });

    invoke<boolean>('get_mini_auto_hide')
      .then((val) => {
        if (typeof val === 'boolean') {
          setMiniAutoHide(val);
        }
      })
      .catch(() => { });
  }, []);

  const handleUpdateWakeMode = async (mode: 'remember' | 'mini' | 'full') => {
    setWakeWindowMode(mode);
    try {
      await invoke('set_wake_window_mode', { mode });
    } catch (e) {
      console.error('Failed to set wake window mode:', e);
    }
  };

  const handleToggleMiniAutoHide = async (enabled: boolean) => {
    setMiniAutoHide(enabled);
    try {
      await invoke('set_mini_auto_hide', { enabled });
    } catch (e) {
      console.error('Failed to set mini drawer auto hide:', e);
    }
  };

  useEffect(() => {
    invoke<any>('get_log_status')
      .then((res) => {
        if (res) {
          setLogStatus({
            visible: !!res.visible,
            enabled: !!res.enabled,
            level: typeof res.level === 'number' ? res.level : 1,
            path: res.path || '',
            filter: res.filter || null,
          });
        }
      })
      .catch(() => { });
  }, []);

  const handleSetLogLevel = async (lvl: number) => {
    try {
      const res = await invoke<any>('set_log_level', { level: lvl });
      if (res) {
        setLogStatus({
          visible: !!res.visible,
          enabled: !!res.enabled,
          level: typeof res.level === 'number' ? res.level : lvl,
          path: res.path || logStatus.path,
          filter: res.filter || null,
        });
      }
    } catch (_) { }
  };

  const handleSetLogFilter = async (filter: string | null) => {
    try {
      const res = await invoke<any>('set_log_filter', { filter: filter || null });
      if (res) {
        setLogStatus({
          visible: !!res.visible,
          enabled: !!res.enabled,
          level: typeof res.level === 'number' ? res.level : logStatus.level,
          path: res.path || logStatus.path,
          filter: res.filter || null,
        });
      }
    } catch (_) { }
  };

  const handleOpenLog = () => {
    invoke('open_log_file').catch(() => { });
  };

  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const [clearingData, setClearingData] = useState(false);

  // User Data Directory & Migration
  const [dataDirInfo, setDataDirInfo] = useState<{ path: string; mode: string } | null>(null);
  const [showMigrateModal, setShowMigrateModal] = useState(false);
  const [migrateTargetPath, setMigrateTargetPath] = useState('');
  const [migratingData, setMigratingData] = useState(false);

  const fetchUserDataDirInfo = async () => {
    try {
      const info = await invoke<{ path: string; mode: string }>('get_user_data_dir_info');
      setDataDirInfo(info);
    } catch (_) { }
  };

  useEffect(() => {
    fetchUserDataDirInfo();
  }, []);

  const handleOpenUserDataDir = async () => {
    try {
      await invoke('open_user_data_dir');
    } catch (err: any) {
      alert('无法打开用户数据目录: ' + err);
    }
  };

  const handleClearUserData = async () => {
    setClearingData(true);
    try {
      await invoke('clear_user_data_dir');
      setShowClearConfirm(false);
      alert('OpenRevo 用户数据已成功清除！\n建议重启应用以重新初始化出厂默认配置。');
      fetchUserDataDirInfo();
    } catch (err: any) {
      alert('清除数据失败: ' + err);
    } finally {
      setClearingData(false);
    }
  };

  const handleMigrateData = async () => {
    if (!migrateTargetPath.trim()) return;
    setMigratingData(true);
    try {
      await invoke('migrate_user_data_dir', { targetPath: migrateTargetPath.trim() });
      await fetchUserDataDirInfo();
      setShowMigrateModal(false);
      setMigrateTargetPath('');
      alert('用户数据目录已成功迁移至: ' + migrateTargetPath.trim());
    } catch (err: any) {
      alert('迁移数据目录失败: ' + err);
    } finally {
      setMigratingData(false);
    }
  };

  // Auto-Updater State & Handlers
  interface UpdateCheckResult {
    has_update: boolean;
    current_version: string;
    current_commit: string;
    latest_version: string;
    latest_commit: string;
    display_version: string;
    notes: string;
    chosen_source: string;
    asset_type: string;
    filename: string;
    download_url: string;
    size: number;
    sha256: string;
  }

  interface DownloadProgressPayload {
    downloaded: number;
    total: number;
    percentage: number;
    speed_bytes_per_sec: number;
  }

  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const [downloadingUpdate, setDownloadingUpdate] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgressPayload | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<DownloadProgressPayload>('updater-progress', (event) => {
      setDownloadProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleCheckForUpdates = async (silent: boolean = false) => {
    setCheckingUpdate(true);
    try {
      const res = await invoke<UpdateCheckResult>('check_for_updates_cmd', { preferredSource: null });
      setUpdateResult(res);
      if (res.has_update) {
        setShowUpdateModal(true);
      } else if (!silent) {
        alert(`当前已是最新版本 (${res.display_version || res.current_version})！\n暂无可用更新。`);
      }
    } catch (err: any) {
      if (!silent) {
        alert('检查更新失败: ' + err);
      }
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handleDownloadAndInstall = async () => {
    if (!updateResult) return;
    setDownloadingUpdate(true);
    setDownloadProgress({ downloaded: 0, total: updateResult.size, percentage: 0, speed_bytes_per_sec: 0 });
    try {
      await invoke('download_and_install_update_cmd', {
        downloadUrl: updateResult.download_url,
        expectedSha256: updateResult.sha256,
        filename: updateResult.filename,
        isInstaller: updateResult.asset_type === 'installer',
      });
    } catch (err: any) {
      alert('下载或升级安装失败: ' + err);
      setDownloadingUpdate(false);
    }
  };

  const [powerSchemes, setPowerSchemes] = useState<{ guid: string; name: string; is_active: boolean }[]>([]);
  const [switchingScheme, setSwitchingScheme] = useState(false);

  const fetchPowerSchemes = async () => {
    try {
      const list = await invoke<any[]>('get_windows_power_schemes');
      if (list && Array.isArray(list)) {
        setPowerSchemes(list);
      }
    } catch (_) { }
  };

  React.useEffect(() => {
    fetchPowerSchemes();
  }, []);

  const handleSwitchPowerScheme = async (guid: string) => {
    if (!guid) return;
    setSwitchingScheme(true);
    try {
      await invoke('set_active_windows_power_scheme', { guid });
      await fetchPowerSchemes();
    } catch (e: any) {
      alert('切换电源方案失败: ' + e);
    } finally {
      setSwitchingScheme(false);
    }
  };

  const [restoringPowerPlan, setRestoringPowerPlan] = useState(false);
  const [showRestorePlanConfirm, setShowRestorePlanConfirm] = useState(false);
  const [restorePlanMsg, setRestorePlanMsg] = useState<string | null>(null);

  const handleRestoreOfficialPowerPlan = async () => {
    setShowRestorePlanConfirm(false);
    setRestoringPowerPlan(true);
    try {
      const res = await invoke<string>('restore_official_power_plan');
      await fetchPowerSchemes();
      setRestorePlanMsg(res || '已成功恢复官方出厂电源计划与 PPM 调度！');
      setTimeout(() => setRestorePlanMsg(null), 5000);
    } catch (e: any) {
      console.error('Failed to restore official power plan:', e);
      setRestorePlanMsg('修复失败: ' + String(e));
      setTimeout(() => setRestorePlanMsg(null), 5000);
    } finally {
      setRestoringPowerPlan(false);
    }
  };

  return (
    <div className="sys-page">
      <div className="glass-card">
        <h3 className="sys-section-title">系统功能开关</h3>
        <div className="sys-rows">

          {/* OSD 屏幕悬浮提示 */}
          <div className="quick-row" style={{ alignItems: 'center' }}>
            <div>
              <div className="sys-row-label">
                <span>OSD 屏幕悬浮提示</span>
                <RevBadge variant="info">HUD</RevBadge>
              </div>
              <div className="sys-row-desc">
                快捷键切换时在屏幕弹出状态提示；可选3种风格、尺寸与显示位置
              </div>
            </div>
            <div className="sys-row-controls">
              {/* 风格 */}
              <div className="sys-row-controls">
                <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>风格</span>
                <select
                  value={osdConfig.theme}
                  onChange={(e) => handleUpdateOsdConfig({ ...osdConfig, theme: e.target.value as any })}
                  className="rev-select"
                >
                  <option value="cinema">电影字幕</option>
                  <option value="fade">向右羽化</option>
                  <option value="pill">磨砂胶囊</option>
                </select>
              </div>

              {/* 大小 */}
              <div className="sys-row-controls">
                <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>大小</span>
                <select
                  value={osdConfig.scale}
                  onChange={(e) => handleUpdateOsdConfig({ ...osdConfig, scale: e.target.value })}
                  className="rev-select"
                >
                  <option value="80%">80%</option>
                  <option value="100%">100%</option>
                  <option value="120%">120%</option>
                  <option value="140%">140%</option>
                </select>
              </div>

              {/* 位置 */}
              <div className="sys-row-controls">
                <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>位置</span>
                <select
                  value={osdConfig.position}
                  onChange={(e) => handleUpdateOsdConfig({ ...osdConfig, position: e.target.value as any })}
                  className="rev-select"
                >
                  <option value="top-left">左上角</option>
                  <option value="bottom-center">下方中央</option>
                  <option value="center">屏幕正中</option>
                </select>
              </div>

              {/* 效果预览 */}
              <button
                onClick={async () => {
                  try {
                    await invoke('trigger_osd_preview');
                  } catch (err) {
                    console.error('Failed to trigger OSD preview:', err);
                  }
                }}
                className="toggle-btn"
                title="在屏幕上立即弹出当前配置的 OSD 效果演示"
                style={{ borderColor: 'rgba(99, 102, 241, 0.3)', color: '#a5b4fc' }}
              >
                预览
              </button>

              {/* 启用/关闭切换 */}
              <RevSwitch
                checked={osdConfig.enabled}
                onChange={(enabled) => handleUpdateOsdConfig({ ...osdConfig, enabled })}
              />
            </div>
          </div>

          {/* 开机自动启动 */}
          <div className="quick-row">
            <div>
              <div className="sys-row-label">开机自动启动 (最小化至托盘)</div>
              <div className="sys-row-desc">开机自动加载驱动与上次配置</div>
            </div>
            <RevSwitch
              checked={!!status.autostart}
              onChange={onToggleAutostart}
            />
          </div>

          {/* 解除离电限制 (允许电池下启用狂暴与自定义) */}
          <div className="quick-row" style={{ alignItems: 'center' }}>
            <div>
              <div className="sys-row-label">
                <span>解除离电限制</span>
                <RevBadge variant={unlockBatteryLimit ? 'warning' : 'default'}>
                  {unlockBatteryLimit ? '已解除限制 (风险自担)' : '出厂安全限制'}
                </RevBadge>
                <RevBadge variant="accent">Power Gating</RevBadge>
              </div>
              <div className="sys-row-desc">
                允许在拔掉电源或 Type-C 供电时自由切换狂暴模式与极客自定义调校，且拔电时不会强制降级为平衡模式；适合进自定义设置极致低功耗或离电性能释放
              </div>
            </div>
            <div className="sys-row-controls">
              <RevSwitch
                checked={unlockBatteryLimit}
                disabled={togglingUnlockBattery}
                onChange={handleToggleUnlockBattery}
              />
            </div>
          </div>

          {/* 唤醒面板形态 */}
          <div className="quick-row" style={{ alignItems: 'center' }}>
            <div>
              <div className="sys-row-label">
                <span>唤醒面板形态</span>
                <RevBadge variant="info">Window</RevBadge>
              </div>
              <div className="sys-row-desc">
                从系统托盘或后台双击唤醒时展现的界面形态；可记忆最后状态或锁定首选模式
              </div>
            </div>
            <div className="sys-row-controls">
              <div
                style={{
                  display: 'inline-flex',
                  gap: '4px',
                  background: 'var(--surface-overlay)',
                  padding: '3px',
                  borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--border-subtle)',
                }}
              >
                <button
                  type="button"
                  className={`toggle-btn ${wakeWindowMode === 'remember' ? 'active-indigo' : ''}`}
                  onClick={() => handleUpdateWakeMode('remember')}
                  style={{ padding: '4px 10px', fontSize: '11px', whiteSpace: 'nowrap' }}
                  title="记忆最后关闭时的窗口形态（关闭时是完整面板就开完整面板，是迷你抽屉就开迷你抽屉）"
                >
                  记忆上次形态
                </button>
                <button
                  type="button"
                  className={`toggle-btn ${wakeWindowMode === 'mini' ? 'active-indigo' : ''}`}
                  onClick={() => handleUpdateWakeMode('mini')}
                  style={{ padding: '4px 10px', fontSize: '11px', whiteSpace: 'nowrap' }}
                  title="无论何时唤醒，默认进入右下角轻量迷你抽屉"
                >
                  始终迷你抽屉
                </button>
                <button
                  type="button"
                  className={`toggle-btn ${wakeWindowMode === 'full' ? 'active-indigo' : ''}`}
                  onClick={() => handleUpdateWakeMode('full')}
                  style={{ padding: '4px 10px', fontSize: '11px', whiteSpace: 'nowrap' }}
                  title="无论何时唤醒，直接居中/记忆位置展开 960×740 完整控制面板"
                >
                  始终完整面板
                </button>
              </div>
            </div>
          </div>

          {/* 迷你抽屉点击外部自动收起 */}
          <div className="quick-row" style={{ alignItems: 'center' }}>
            <div>
              <div className="sys-row-label">
                <span>迷你抽屉点击外部自动收起</span>
                <RevBadge variant="info">Mini Drawer</RevBadge>
              </div>
              <div className="sys-row-desc">
                在迷你抽屉模式下，点击桌面或其他应用窗口时自动回收抽屉；完整面板不受此影响
              </div>
            </div>
            <div className="sys-row-controls">
              <RevSwitch
                checked={miniAutoHide}
                onChange={handleToggleMiniAutoHide}
              />
            </div>
          </div>

          {/* 数据管理 */}
          <div className="quick-row">
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <div className="sys-row-label">数据存储管理</div>
                <RevBadge variant={dataDirInfo?.mode === 'portable' ? 'success' : dataDirInfo?.mode === 'custom' ? 'warning' : 'accent'}>
                  {dataDirInfo?.mode === 'portable'
                    ? '便携目录 (./data)'
                    : dataDirInfo?.mode === 'custom'
                      ? '自定义路径 (data_dir.ptr)'
                      : '标准 AppData (%APPDATA%)'}
                </RevBadge>
              </div>
              <div className="sys-row-desc" title={dataDirInfo?.path || ''}>
                {dataDirInfo?.path
                  ? `当前目录: ${dataDirInfo.path}`
                  : '管理用户预设、键盘动效、下载的插件与皮肤等持久化文件'}
              </div>
            </div>
            <div className="sys-row-controls">
              <button
                onClick={handleOpenUserDataDir}
                className="toggle-btn"
                title="直接在文件资源管理器中打开当前用户数据目录"
              >
                <FolderOpen size={12} />
                <span>打开目录</span>
              </button>
              <button
                onClick={() => {
                  setMigrateTargetPath(dataDirInfo?.path || '');
                  setShowMigrateModal(true);
                }}
                className="toggle-btn"
                title="无损迁移配置预设并建立重定向指针"
              >
                <HardDrive size={12} />
                <span>迁移存储</span>
              </button>
              <button
                onClick={() => setShowClearConfirm(true)}
                className="toggle-btn danger"
                title="彻底清空 OpenRevo 所有用户数据与预设"
              >
                <Trash2 size={12} />
                <span>清除数据</span>
              </button>
            </div>
          </div>

          {/* 软件版本与自动更新 */}
          <div className="quick-row">
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <div className="sys-row-label">软件版本与分发更新</div>
                <RevBadge variant="info">
                  {updateResult?.display_version || 'v0.9.4'}
                </RevBadge>
              </div>
              <div className="sys-row-desc">
                支持 GitHub 官方与国内 Gitee 镜像智能双源竞速拉取；适配安装版与绿色便携版
              </div>
            </div>
            <div className="sys-row-controls">
              <button
                onClick={() => handleCheckForUpdates(false)}
                disabled={checkingUpdate}
                className={`toggle-btn ${checkingUpdate ? 'active-indigo' : ''}`}
                title="立即并发向 GitHub 与 Gitee 查询是否有新版本发布"
              >
                <RefreshCw size={12} className={checkingUpdate ? 'spin-icon' : ''} />
                <span>{checkingUpdate ? '正在检测...' : '检查更新'}</span>
              </button>
            </div>
          </div>

          {/* Windows 基础电源方案 */}
          <div className="quick-row">
            <div>
              <div className="sys-row-label">
                <span>Windows 基础电源方案</span>
                <RevBadge variant="success">powercfg</RevBadge>
                {restorePlanMsg && (
                  <RevBadge variant={restorePlanMsg.includes('失败') ? 'error' : 'success'}>
                    {restorePlanMsg}
                  </RevBadge>
                )}
              </div>
              <div className="sys-row-desc">
                系统底层电源计划；完全保留用户自建的降压/降频方案，可随时自由切换
              </div>
            </div>
            <div className="sys-row-controls">
              <button
                onClick={() => setShowRestorePlanConfirm(true)}
                disabled={restoringPowerPlan}
                className="toggle-btn danger"
                style={{ padding: '4px 10px', fontSize: '11px', minWidth: 'auto' }}
                title="仅限同方/宝龙达游戏本：一键清除异常锁频/待机高温残留，对齐官方控制台，还原官方平衡方案与原生出厂调度"
              >
                {restoringPowerPlan ? '正在修复...' : '修复/重置计划'}
              </button>
              <select
                value={powerSchemes.find((s) => s.is_active)?.guid || ''}
                onChange={(e) => handleSwitchPowerScheme(e.target.value)}
                disabled={switchingScheme || restoringPowerPlan}
                className="rev-select"
                style={{ maxWidth: '190px' }}
              >
                {powerSchemes.map((s) => (
                  <option key={s.guid} value={s.guid}>
                    {s.name} {s.is_active ? '(当前生效)' : ''}
                  </option>
                ))}
              </select>
              <button
                onClick={fetchPowerSchemes}
                disabled={restoringPowerPlan}
                className="toggle-btn"
                style={{ padding: '4px 7px', minWidth: 'auto' }}
                title="刷新系统电源方案列表"
              >
                <RotateCcw size={12} />
              </button>
            </div>
          </div>

          {/* 调试日志（仅在通过调试参数启动时可见） */}
          {logStatus.visible && (
            <>
              <div className="quick-row">
                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <div className="sys-row-label">运行与异常调试日志等级</div>
                    <RevBadge variant={logStatus.level === 2 ? 'accent' : 'success'}>
                      {logStatus.level === 2 ? '所有日志 (详细追踪)' : '错误日志 (仅故障及异常崩溃)'}
                    </RevBadge>
                    {logStatus.level === 2 && (
                      <RevBadge
                        variant={logStatus.filter ? 'warning' : 'default'}
                        title={
                          logStatus.filter
                            ? `当前仅输出命中 [${logStatus.filter}] 的模块日志 (高频噪音已抑制)`
                            : '未设置过滤，输出所有模块日志'
                        }
                      >
                        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
                          {logStatus.filter
                            ? <><Crosshair size={10} /> 过滤: {logStatus.filter}</>
                            : <><Globe size={10} /> 全部模块</>}
                        </span>
                      </RevBadge>
                    )}
                  </div>
                  <div className="sys-row-desc">
                    当前已通过调试参数启动；排查完毕可点击关闭隐藏以节省磁盘
                  </div>
                </div>
                <div className="sys-row-controls">
                  <button
                    onClick={() => handleSetLogLevel(logStatus.level === 2 ? 1 : 2)}
                    className={`toggle-btn ${logStatus.level === 2 ? 'active-indigo' : ''}`}
                    style={{ minWidth: '78px' }}
                  >
                    {logStatus.level === 2 ? '切为错误日志' : '切为所有日志'}
                  </button>
                  <button
                    onClick={handleOpenLog}
                    className="toggle-btn"
                    style={{ minWidth: '78px' }}
                    title={logStatus.path || '打开日志文件'}
                  >
                    查看日志
                  </button>
                  <button
                    onClick={() => handleSetLogLevel(0)}
                    className="toggle-btn"
                    style={{ minWidth: '65px', color: 'var(--text-dim)' }}
                    title="关闭日志并重新向普通用户隐藏"
                  >
                    关闭隐藏
                  </button>
                </div>
              </div>

              {/* 二级：模块精准过滤（仅 level==2 时展开） */}
              {logStatus.level === 2 && (
                <div
                  className="quick-row"
                  style={{ backgroundColor: 'rgba(0, 0, 0, 0.15)', borderTop: '1px solid var(--border-subtle)' }}
                >
                  <div>
                    <div className="sys-row-label" style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      排障模块精准过滤 (二级参数)
                    </div>
                    <div className="sys-row-desc">
                      选定模块后将屏蔽数千条 EC 纳秒高频读取噪音；支持命令行{' '}
                      <code style={{ color: 'var(--color-brand)' }}>--debug=模块名</code>
                    </div>
                  </div>
                  <div className="sys-chip-row">
                    {[
                      { label: '全部', value: null, Icon: Globe },
                      { label: '外置水冷', value: 'cooler', Icon: Droplets },
                      { label: '自定义追踪', value: 'custom', Icon: Gauge },
                      { label: 'RGB与推流', value: 'light', Icon: Lightbulb },
                      { label: '电源与风扇', value: 'power', Icon: Zap },
                      { label: '底层EC', value: 'ec', Icon: Cpu },
                      { label: '设备外设', value: 'device', Icon: Cable },
                      { label: '窗口与系统', value: 'ui', Icon: Sliders },
                    ].map((item) => {
                      const isActive =
                        (!item.value && !logStatus.filter) ||
                        (item.value && logStatus.filter?.toLowerCase() === item.value);
                      const ItemIcon = item.Icon;
                      return (
                        <button
                          key={item.label}
                          onClick={() => handleSetLogFilter(item.value)}
                          className={`toggle-btn ${isActive ? 'active-indigo' : ''}`}
                          style={{ fontSize: '10px', padding: '2px 8px', minWidth: 'auto', borderRadius: 'var(--radius-xs)' }}
                        >
                          <ItemIcon size={11} />
                          <span>{item.label}</span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
            </>
          )}



          {/* 只有在 --debug 模式下才显示的开发者形态预览卡片 */}
          {capabilities?.is_debug_mode && onPreviewFormFactor && (
            <div className="sys-card sys-card--highlight">
              <div className="sys-card-header">
                <div className="sys-card-title">
                  <Sliders className="sys-card-icon" />
                  <span>机型形态与能力实时预览 (Dev Mode)</span>
                </div>
                <RevBadge color="amber">--debug 专属</RevBadge>
              </div>
              <div className="sys-card-desc">
                允许开发者在不修改底层通信的前提下，纯前端模拟不同模具插槽裁切效果（如轻薄本隐藏独显/水冷）。
              </div>

              <div className="sys-rows">
                <div className="quick-row">
                  <div className="sys-row-label">
                    <span>当前模拟形态</span>
                    <span className="sys-row-desc">
                      {previewMode === 'real'
                        ? `真实硬件: ${capabilities.name || '自适应'} (${capabilities.form_factor || 'gaming'})`
                        : previewMode === 'thin_light'
                          ? '模拟: 星耀 14 轻薄本 (核显 + 无水冷 + 单色背光)'
                          : '模拟: 标准全能游戏本 (独显直连 + 水冷 + 全功能)'}
                    </span>
                  </div>
                  <div className="sys-row-controls">
                    <button
                      className={`toggle-btn ${previewMode === 'real' ? 'active-emerald' : ''}`}
                      onClick={() => onPreviewFormFactor('real')}
                    >
                      真实本机
                    </button>
                    <button
                      className={`toggle-btn ${previewMode === 'thin_light' ? 'active-cyan' : ''}`}
                      onClick={() => onPreviewFormFactor('thin_light')}
                    >
                      星耀 14 轻薄本
                    </button>
                    <button
                      className={`toggle-btn ${previewMode === 'gaming' ? 'active-indigo' : ''}`}
                      onClick={() => onPreviewFormFactor('gaming')}
                    >
                      游戏本形态
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

        </div>
      </div>

      {/* 清除数据二次确认弹窗 */}
      {showClearConfirm && (
        <div
          className="sys-modal-backdrop"
          onClick={() => !clearingData && setShowClearConfirm(false)}
        >
          <div
            className="sys-modal-card danger"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="sys-modal-header">
              <div className="sys-modal-icon-badge danger">
                <AlertTriangle size={20} />
              </div>
              <div>
                <div className="sys-modal-title">清除所有用户数据？</div>
                <div className="sys-modal-subtitle">此操作将彻底清空本地用户数据目录</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                ⚠️ 请注意，执行后以下所有内容将被永久清除：
              </div>
              <ul className="sys-modal-list">
                <li>用户自建或调整的模式与功耗预设文件</li>
                <li>键盘自制灯效、动画脚本与预设</li>
                <li>已下载安装的第三方插件与自定义皮肤</li>
                <li>模具库自定义昵称与映射 (models.json)</li>
              </ul>
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={clearingData}
                onClick={() => setShowClearConfirm(false)}
                className="toggle-btn"
              >
                取消
              </button>
              <button
                disabled={clearingData}
                onClick={handleClearUserData}
                className="toggle-btn btn-danger-solid"
              >
                {clearingData ? '正在清除...' : '确认清除'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 解除离电限制安全风险提示二次确认弹窗 */}
      {showUnlockBatteryConfirm && (
        <div
          className="sys-modal-backdrop"
          onClick={() => !togglingUnlockBattery && setShowUnlockBatteryConfirm(false)}
        >
          <div
            className="sys-modal-card danger"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="sys-modal-header">
              <div className="sys-modal-icon-badge danger">
                <AlertTriangle size={20} />
              </div>
              <div>
                <div className="sys-modal-title">解除离电模式限制风险警示</div>
                <div className="sys-modal-subtitle">硬件放电与电池安全性须知</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                ⚠️ 官方固件默认禁止电池状态下开启高功耗模式，请知悉以下潜在风险：
              </div>
              <ul className="sys-modal-list">
                <li><strong>电池过流保护断电</strong>：游戏本电池瞬时放电功率远低于原装电源适配器，离电下运行重负载可能瞬间触发主板 BMS 电池过流保护，导致突然掉电黑屏或硬件重启。</li>
                <li><strong>电池寿命加速衰减</strong>：持续的大倍率放电会使锂电池内部剧烈发热，显著加速电池容量老化与衰减。</li>
                <li><strong>拔电不再强制降级</strong>：拔掉适配器后，系统将保持当前的狂暴或极客模式，不会再自动回切至平衡模式。</li>
                <li><strong>推荐应用场景</strong>：如果您是为了在离电下进入极客调校“设定比办公模式更低的 CPU/GPU 功耗与温控以追求极致长续航”，可以放心使用；但切勿在离电下运行重度 3D 游戏或高负载渲染。</li>
              </ul>
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={togglingUnlockBattery}
                onClick={() => setShowUnlockBatteryConfirm(false)}
                className="toggle-btn"
              >
                取消
              </button>
              <button
                disabled={togglingUnlockBattery}
                onClick={handleConfirmUnlockBattery}
                className="toggle-btn btn-danger-solid"
              >
                {togglingUnlockBattery ? '正在开启...' : '我已知晓风险，确认解除'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 恢复官方出厂电源计划与 PPM 调度二次确认弹窗 */}
      {showRestorePlanConfirm && (
        <div
          className="sys-modal-backdrop"
          onClick={() => !restoringPowerPlan && setShowRestorePlanConfirm(false)}
        >
          <div
            className="sys-modal-card"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="sys-modal-header">
              <div className="sys-modal-icon-badge warning">
                <RotateCcw size={20} />
              </div>
              <div>
                <div className="sys-modal-title">修复并重置为官方出厂电源计划</div>
                <div className="sys-modal-subtitle">对齐官方控制台逻辑，消除待机高温与异常锁频</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                💡 该操作将执行以下安全修复（完全对齐官方出厂标准）：
              </div>
              <ul className="sys-modal-list">
                <li><strong>消除待机异常高温</strong>：重置激进的性能滑块覆盖层 (Performance Overlay)，恢复 CPU 核心休眠 (Core Parking) 与 Package C-State 深睡，消除待机 20W+ 积热。</li>
                <li><strong>彻底清除锁频残留</strong>：外科手术式剔除所有电源方案中的静态频率限制键值 (75b0ae3f)，解除 0.8GHz 或 25W 锁频陷阱。</li>
                <li><strong>重新注入官方 OEM 预配包</strong>：根据当前机型模具自适应注入出厂 PPM 调度包（宝龙达 AMD 官方 Provisioning 套件 / 同方官方 PPM 策略包）。</li>
                <li><strong>激活官方平衡方案</strong>：切回 Windows 官方标准平衡方案 (Balanced)。若您有自建的个性化电源计划，它们会被完整保留，不会被删除。</li>
              </ul>
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={restoringPowerPlan}
                onClick={() => setShowRestorePlanConfirm(false)}
                className="toggle-btn"
              >
                取消
              </button>
              <button
                disabled={restoringPowerPlan}
                onClick={handleRestoreOfficialPowerPlan}
                className="toggle-btn active-indigo"
              >
                {restoringPowerPlan ? '正在修复...' : '确认执行修复'}
              </button>
            </div>
          </div>
        </div>
      )}
      {/* 迁移用户数据目录弹窗 */}
      {showMigrateModal && (
        <div
          className="sys-modal-backdrop"
          onClick={() => !migratingData && setShowMigrateModal(false)}
        >
          <div
            className="sys-modal-card"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="sys-modal-header">
              <div className="sys-modal-icon-badge info">
                <HardDrive size={20} />
              </div>
              <div>
                <div className="sys-modal-title">迁移用户数据存储目录</div>
                <div className="sys-modal-subtitle">将配置预设迁移至新路径并自动重定向</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                💡 说明：程序将自动把当前所有用户数据无损复制至新目录，并在原位置创建数据重定向指针 (data_dir.ptr)。
              </div>
              <div style={{ marginTop: '10px', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <label style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                  目标目录绝对路径（例如：D:\OpenRevoData）：
                </label>
                <input
                  type="text"
                  value={migrateTargetPath}
                  onChange={(e) => setMigrateTargetPath(e.target.value)}
                  disabled={migratingData}
                  placeholder="D:\OpenRevoData"
                  style={{
                    width: '100%',
                    padding: '8px 10px',
                    fontSize: '12px',
                    borderRadius: 'var(--radius-md)',
                    border: '1px solid var(--border-medium)',
                    background: 'var(--surface-overlay)',
                    color: 'var(--text-primary)',
                    outline: 'none',
                  }}
                />
              </div>
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={migratingData}
                onClick={() => setShowMigrateModal(false)}
                className="toggle-btn"
              >
                取消
              </button>
              <button
                disabled={migratingData || !migrateTargetPath.trim()}
                onClick={handleMigrateData}
                className="toggle-btn active-indigo"
              >
                {migratingData ? '正在迁移...' : '确认迁移'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 自动更新检测与安装弹窗 */}
      {showUpdateModal && updateResult && (
        <div
          className="sys-modal-backdrop"
          onClick={() => !downloadingUpdate && setShowUpdateModal(false)}
        >
          <div
            className="sys-modal-card"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="sys-modal-header">
              <div className="sys-modal-icon-badge info">
                <Download size={20} />
              </div>
              <div>
                <div className="sys-modal-title">
                  发现新版本 {updateResult.display_version || updateResult.latest_version}
                </div>
                <div className="sys-modal-subtitle">
                  测速首选: {updateResult.chosen_source === 'gitee' ? 'Gitee 国内镜像加速' : updateResult.chosen_source === 'local_mock' ? '本地测试模拟' : 'GitHub 官方源'} · 分发形态: {updateResult.asset_type === 'installer' ? '安装包覆盖升级' : '绿色版脱壳热更'}
                </div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div style={{ marginBottom: '8px', fontSize: '11px', color: 'var(--text-muted)' }}>
                更新日志与变更说明：
              </div>
              <div className="updater-changelog-box">
                {updateResult.notes || '暂无详细更新日志。'}
              </div>

              <div style={{ marginTop: '10px', fontSize: '11px', color: 'var(--text-dim)', display: 'flex', justifyContent: 'space-between' }}>
                <span>分发文件: {updateResult.filename}</span>
                <span>大小: {(updateResult.size / (1024 * 1024)).toFixed(2)} MB</span>
              </div>

              {downloadingUpdate && downloadProgress && (
                <div className="updater-progress-container">
                  <div className="updater-progress-bar-bg">
                    <div
                      className="updater-progress-bar-fill"
                      style={{ width: `${Math.min(100, Math.max(0, downloadProgress.percentage))}%` }}
                    />
                  </div>
                  <div className="updater-progress-stats">
                    <span>
                      {(downloadProgress.downloaded / (1024 * 1024)).toFixed(2)} MB / {(downloadProgress.total / (1024 * 1024)).toFixed(2)} MB
                    </span>
                    <span>
                      {(downloadProgress.speed_bytes_per_sec / (1024 * 1024)).toFixed(2)} MB/s ({downloadProgress.percentage.toFixed(1)}%)
                    </span>
                  </div>
                </div>
              )}
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={downloadingUpdate}
                onClick={() => setShowUpdateModal(false)}
                className="toggle-btn"
              >
                稍后再说
              </button>
              <button
                disabled={downloadingUpdate}
                onClick={handleDownloadAndInstall}
                className="toggle-btn active-indigo"
              >
                {downloadingUpdate ? '正在下载升级...' : '立即升级'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
