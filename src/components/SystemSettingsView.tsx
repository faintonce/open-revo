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
  const [updateChannel, setUpdateChannel] = useState<'stable' | 'beta'>('stable');
  const [miniAutoHide, setMiniAutoHide] = useState(true);
  const [useClassicWebview, setUseClassicWebview] = useState(false);
  const [showRestartHint, setShowRestartHint] = useState(false);
  const [unlockBatteryLimit, setUnlockBatteryLimit] = useState(!!status.unlock_battery_power_limit);
  const [showUnlockBatteryConfirm, setShowUnlockBatteryConfirm] = useState(false);
  const [togglingUnlockBattery, setTogglingUnlockBattery] = useState(false);

  // 平衡模式 Dynamic Boost 开关（默认开，满足 14900HX 等用户平衡模式跑 3A 显卡满血需求）
  const [balancedDbEnabled, setBalancedDbEnabled] = useState(true);

  // GPU OC 默认值（设置页可改；开启 GPU OC 时生效）
  const [gpuOcDefaults, setGpuOcDefaults] = useState<{ core: number; mem: number }>({ core: 105, mem: 0 });
  const [gpuOcSaveHint, setGpuOcSaveHint] = useState('');

  useEffect(() => {
    invoke<any>('get_app_config')
      .then((cfg) => {
        if (!cfg) return;
        if (typeof cfg.balanced_gpu_dynamic_boost === 'boolean') {
          setBalancedDbEnabled(cfg.balanced_gpu_dynamic_boost);
        }
        const core = Number(cfg.gpu_oc_core_mhz);
        const mem = Number(cfg.gpu_oc_mem_mhz);
        setGpuOcDefaults({
          core: Number.isFinite(core) && core >= 0 ? core : 105,
          mem: Number.isFinite(mem) && mem >= 0 ? mem : 0,
        });
      })
      .catch(() => { });
  }, []);

  const saveGpuOcDefaults = async (core: number, mem: number) => {
    const safeCore = Math.max(0, Math.min(250, Math.round(core)));
    const safeMem = Math.max(0, Math.min(1000, Math.round(mem)));
    setGpuOcDefaults({ core: safeCore, mem: safeMem });
    try {
      const cfg: any = await invoke('get_app_config');
      if (!cfg) return;
      cfg.gpu_oc_core_mhz = safeCore;
      cfg.gpu_oc_mem_mhz = safeMem;
      await invoke('save_app_config', { cfg });
      setGpuOcSaveHint('Сохранено');
      setTimeout(() => setGpuOcSaveHint(''), 2000);
    } catch (e) {
      console.warn('[Settings] save gpu oc defaults failed:', e);
      setGpuOcSaveHint('Ошибка сохранения');
      setTimeout(() => setGpuOcSaveHint(''), 3000);
    }
  };

  const handleToggleBalancedDb = async (enabled: boolean) => {
    setBalancedDbEnabled(enabled);
    try {
      const cfg: any = await invoke('get_app_config');
      if (!cfg) return;
      cfg.balanced_gpu_dynamic_boost = enabled;
      await invoke('save_app_config', { cfg });
    } catch (e) {
      console.warn('[Settings] save balanced_gpu_dynamic_boost failed:', e);
    }
  };

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
        alert('Не удалось выключить снятие ограничения от батареи: ' + err);
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
      alert('Не удалось включить снятие ограничения от батареи: ' + err);
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

    invoke<boolean>('get_classic_webview')
      .then((val) => {
        if (typeof val === 'boolean') {
          setUseClassicWebview(val);
        }
      })
      .catch(() => { });

    invoke<string>('get_update_channel')
      .then((ch) => {
        if (ch === 'beta' || ch === 'stable') {
          setUpdateChannel(ch);
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

  const handleToggleClassicWebview = async (enabled: boolean) => {
    setUseClassicWebview(enabled);
    setShowRestartHint(true);
    try {
      await invoke('set_classic_webview', { enabled });
    } catch (e) {
      console.error('Failed to set classic webview:', e);
    }
  };

  const handleRestartApp = async () => {
    try {
      await invoke('restart_app');
    } catch (e) {
      console.error('Failed to restart app:', e);
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
      alert('Не удалось открыть каталог пользовательских данных: ' + err);
    }
  };

  const handleClearUserData = async () => {
    setClearingData(true);
    try {
      await invoke('clear_user_data_dir');
      setShowClearConfirm(false);
      alert('Пользовательские данные OpenRevo успешно удалены!\nРекомендуется перезапустить приложение для повторной инициализации заводских настроек по умолчанию.');
      fetchUserDataDirInfo();
    } catch (err: any) {
      alert('Не удалось очистить данные: ' + err);
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
      alert('Каталог пользовательских данных успешно перенесён в: ' + migrateTargetPath.trim());
    } catch (err: any) {
      alert('Не удалось перенести каталог данных: ' + err);
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

  const handleUpdateChannel = async (channel: 'stable' | 'beta') => {
    setUpdateChannel(channel);
    try {
      await invoke('set_update_channel', { channel });
    } catch (e) {
      console.error('Failed to set update channel:', e);
    }
  };

  const handleCheckForUpdates = async (silent: boolean = false) => {
    setCheckingUpdate(true);
    try {
      const res = await invoke<UpdateCheckResult>('check_for_updates_cmd', { channel: updateChannel, preferredSource: null });
      setUpdateResult(res);
      if (res.has_update) {
        setShowUpdateModal(true);
      } else if (!silent) {
        alert(`Установлена последняя версия (${res.display_version || res.current_version})!\nОбновлений нет.`);
      }
    } catch (err: any) {
      if (!silent) {
        const errStr = String(err);
        if (errStr.includes('404') && updateChannel === 'beta') {
          alert('Предрелизных обновлений нет (preview.json в Release не найден).');
        } else if (errStr.includes('404')) {
          alert('Файл-манифест версий не найден (404 Not Found).');
        } else {
          alert('Не удалось проверить обновления: ' + err);
        }
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
      alert('Не удалось скачать или обновить: ' + err);
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
      alert('Не удалось переключить схему электропитания: ' + e);
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
      setRestorePlanMsg(res || 'Заводской план электропитания и PPM-диспетчеризация успешно восстановлены!');
      setTimeout(() => setRestorePlanMsg(null), 5000);
    } catch (e: any) {
      console.error('Failed to restore official power plan:', e);
      setRestorePlanMsg('Исправление не удалось: ' + String(e));
      setTimeout(() => setRestorePlanMsg(null), 5000);
    } finally {
      setRestoringPowerPlan(false);
    }
  };

  return (
    <div className="sys-page">
      <div className="glass-card">
        <h3 className="sys-section-title">Переключатели системных функций</h3>
        <div className="sys-rows">

          {/* OSD 屏幕悬浮提示 */}
          <div className="quick-row" style={{ alignItems: 'center' }}>
            <div>
              <div className="sys-row-label">
                <span>Всплывающие подсказки OSD на экране</span>
                <RevBadge variant="info">HUD</RevBadge>
              </div>
              <div className="sys-row-desc">
                Всплывающая подсказка на экране при переключении горячими клавишами; 3 стиля, размер и позиция на выбор
              </div>
            </div>
            <div className="sys-row-controls">
              {/* 风格 */}
              <div className="sys-row-controls">
                <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>Стиль</span>
                <select
                  value={osdConfig.theme}
                  onChange={(e) => handleUpdateOsdConfig({ ...osdConfig, theme: e.target.value as any })}
                  className="rev-select"
                >
                  <option value="cinema">Киносубтитры</option>
                  <option value="fade">Перо вправо</option>
                  <option value="pill">Матовая капсула</option>
                </select>
              </div>

              {/* 大小 */}
              <div className="sys-row-controls">
                <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>Размер</span>
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
                <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>Позиция</span>
                <select
                  value={osdConfig.position}
                  onChange={(e) => handleUpdateOsdConfig({ ...osdConfig, position: e.target.value as any })}
                  className="rev-select"
                >
                  <option value="top-left">Сверху слева</option>
                  <option value="bottom-center">Снизу по центру</option>
                  <option value="center">Центр экрана</option>
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
                title="Мгновенно показать на экране демонстрацию текущей настройки OSD"
                style={{ borderColor: 'rgba(99, 102, 241, 0.3)', color: '#a5b4fc' }}
              >
                Предпросмотр
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
              <div className="sys-row-label">Автозапуск при включении (сворачивается в трей)</div>
              <div className="sys-row-desc">Автозагрузка драйвера и последней конфигурации при включении</div>
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
                <span>Снять ограничение от батареи</span>
                <RevBadge variant={unlockBatteryLimit ? 'warning' : 'default'}>
                  {unlockBatteryLimit ? 'Ограничения сняты (на свой риск)' : 'Заводские защитные лимиты'}
                </RevBadge>
                <RevBadge variant="accent">Power Gating</RevBadge>
              </div>
              <div className="sys-row-desc">
                Разрешает свободно переключать Режим Турбо и Экспертную настройку при работе от батареи или Type-C; при отключении адаптера принудительного сброса на Баланс не происходит. Подходит для экстремально низкого потребления в пользовательской настройке или автономной производительности.
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
                <span>Вид панели при пробуждении</span>
                <RevBadge variant="info">Window</RevBadge>
              </div>
              <div className="sys-row-desc">
                Вид интерфейса при пробуждении из трея или по двойному щелчку в фоне; можно запоминать последнее состояние или зафиксировать предпочитаемый режим
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
                  title="Запоминает вид окна на момент закрытия (была полная панель — откроется полная панель, была мини-панель — откроется мини-панель)"
                >
                  Запоминать последний вид
                </button>
                <button
                  type="button"
                  className={`toggle-btn ${wakeWindowMode === 'mini' ? 'active-indigo' : ''}`}
                  onClick={() => handleUpdateWakeMode('mini')}
                  style={{ padding: '4px 10px', fontSize: '11px', whiteSpace: 'nowrap' }}
                  title="При любом пробуждении по умолчанию открывается лёгкая мини-панель в правом нижнем углу"
                >
                  Всегда мини-панель
                </button>
                <button
                  type="button"
                  className={`toggle-btn ${wakeWindowMode === 'full' ? 'active-indigo' : ''}`}
                  onClick={() => handleUpdateWakeMode('full')}
                  style={{ padding: '4px 10px', fontSize: '11px', whiteSpace: 'nowrap' }}
                  title="При любом пробуждении сразу разворачивается полная панель управления 960×740 — по центру или в запомненной позиции"
                >
                  Всегда полная панель
                </button>
              </div>
            </div>
          </div>

          {/* 迷你抽屉点击外部自动收起 */}
          <div className="quick-row" style={{ alignItems: 'center' }}>
            <div>
              <div className="sys-row-label">
                <span>Мини-панель автоматически сворачивается при щелчке вне её</span>
                <RevBadge variant="info">Mini Drawer</RevBadge>
              </div>
              <div className="sys-row-desc">
                В режиме мини-панели щелчок по рабочему столу или окну другого приложения автоматически сворачивает панель; полная панель не затрагивается
              </div>
            </div>
            <div className="sys-row-controls">
              <RevSwitch
                checked={miniAutoHide}
                onChange={handleToggleMiniAutoHide}
              />
            </div>
          </div>

          {/* 使用经典 WebView2 前端 */}
          <div className="quick-row" style={{ alignItems: 'center' }}>
            <div>
              <div className="sys-row-label">
                <span>Классический фронтенд WebView2</span>
                <RevBadge variant={useClassicWebview ? 'accent' : 'info'}>
                  {useClassicWebview ? 'WebView2 (классика)' : 'Slint нативно (по умолчанию)'}
                </RevBadge>
              </div>
              <div className="sys-row-desc">
                Переключает движок рендеринга главного окна и подсказок OSD. По умолчанию — нативный GUI Slint с минимальным потреблением ресурсов; при включении возвращается классический веб-фронтенд WebView2 (вступает в силу после перезапуска)
              </div>
            </div>
            <div className="sys-row-controls">
              {showRestartHint && (
                <button
                  type="button"
                  onClick={handleRestartApp}
                  className="toggle-btn active-cyan"
                  title="Перезапустить OpenRevo для загрузки нового движка интерфейса"
                >
                  Перезапустить для применения
                </button>
              )}
              <RevSwitch
                checked={useClassicWebview}
                onChange={handleToggleClassicWebview}
              />
            </div>
          </div>

          {/* 数据管理 */}
          <div className="quick-row">
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <div className="sys-row-label">Управление хранением данных</div>
                <RevBadge variant={dataDirInfo?.mode === 'portable' ? 'success' : dataDirInfo?.mode === 'custom' ? 'warning' : 'accent'}>
                  {dataDirInfo?.mode === 'portable'
                    ? 'Портативный каталог (./data)'
                    : dataDirInfo?.mode === 'custom'
                      ? 'Пользовательский путь (data_dir.ptr)'
                      : 'Стандартный AppData (%APPDATA%)'}
                </RevBadge>
              </div>
              <div className="sys-row-desc" title={dataDirInfo?.path || ''}>
                {dataDirInfo?.path
                  ? `Текущий каталог: ${dataDirInfo.path}`
                  : 'Управление постоянными файлами: пользовательские пресеты, эффекты клавиатуры, скачанные плагины и темы'}
              </div>
            </div>
            <div className="sys-row-controls">
              <button
                onClick={handleOpenUserDataDir}
                className="toggle-btn"
                title="Открыть текущий каталог пользовательских данных прямо в проводнике"
              >
                <FolderOpen size={12} />
                <span>Открыть каталог</span>
              </button>
              <button
                onClick={() => {
                  setMigrateTargetPath(dataDirInfo?.path || '');
                  setShowMigrateModal(true);
                }}
                className="toggle-btn"
                title="Перенести пресеты без потерь и создать указатель перенаправления"
              >
                <HardDrive size={12} />
                <span>Перенести хранилище</span>
              </button>
              <button
                onClick={() => setShowClearConfirm(true)}
                className="toggle-btn danger"
                title="Полностью очистить все пользовательские данные и пресеты OpenRevo"
              >
                <Trash2 size={12} />
                <span>Очистить данные</span>
              </button>
            </div>
          </div>

          {/* 软件版本与自动更新 */}
          <div className="quick-row">
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <div className="sys-row-label">Версия ПО и обновления дистрибутива</div>
                <RevBadge variant="info">
                  {updateResult?.display_version || 'v0.9.8'}
                </RevBadge>
              </div>
              <div className="sys-row-desc">
                {updateChannel === 'beta'
                  ? '[Канал предпросмотра] экспериментальные новинки и ранние сборки в приоритете; умная двойная загрузка с двух источников'
                  : '[Стабильный канал] только основательно протестированные стабильные версии; умная двойная загрузка с двух источников'}
              </div>
            </div>
            <div className="sys-row-controls">
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  background: 'var(--surface-sunken)',
                  padding: '2px',
                  borderRadius: 'var(--radius-md, 6px)',
                  border: '1px solid var(--border-subtle)',
                  gap: '2px',
                }}
              >
                <button
                  type="button"
                  className={`toggle-btn ${updateChannel === 'stable' ? 'active-indigo' : ''}`}
                  onClick={() => handleUpdateChannel('stable')}
                  style={{ padding: '4px 10px', fontSize: '11px', whiteSpace: 'nowrap' }}
                  title="Проверять только официальные релизы (latest.json)"
                >
                  Стабильная (Stable)
                </button>
                <button
                  type="button"
                  className={`toggle-btn ${updateChannel === 'beta' ? 'active-indigo' : ''}`}
                  onClick={() => handleUpdateChannel('beta')}
                  style={{ padding: '4px 10px', fontSize: '11px', whiteSpace: 'nowrap' }}
                  title="Проверять ранние и предрелизные сборки в приоритете (preview.json)"
                >
                  Предпросмотр (Beta)
                </button>
              </div>
              <button
                onClick={() => handleCheckForUpdates(false)}
                disabled={checkingUpdate}
                className={`toggle-btn ${checkingUpdate ? 'active-indigo' : ''}`}
                title="Немедленно параллельно запрашивает GitHub и Gitee о наличии новой версии"
              >
                <RefreshCw size={12} className={checkingUpdate ? 'spin-icon' : ''} />
                <span>{checkingUpdate ? 'Проверка...' : 'Проверить обновления'}</span>
              </button>
            </div>
          </div>

          {/* Windows 基础电源方案 */}
          <div className="quick-row">
            <div>
              <div className="sys-row-label">
                <span>Базовая схема электропитания Windows</span>
                <RevBadge variant="success">powercfg</RevBadge>
                {restorePlanMsg && (
                  <RevBadge variant={restorePlanMsg.includes('не удалось') ? 'error' : 'success'}>
                    {restorePlanMsg}
                  </RevBadge>
                )}
              </div>
              <div className="sys-row-desc">
                Низкоуровневая схема электропитания системы; пользовательские схемы андервольта/снижения частот полностью сохраняются и переключаются в любой момент
              </div>
            </div>
            <div className="sys-row-controls">
              <button
                onClick={() => setShowRestorePlanConfirm(true)}
                disabled={restoringPowerPlan}
                className="toggle-btn danger"
                style={{ padding: '4px 10px', fontSize: '11px', minWidth: 'auto' }}
                title="Только для игровых ноутбуков TongFang/Baolongda: одним нажатием удаляет следы аномальной блокировки частот и остаточного нагрева в простое, приводит к логике официальной консоли, восстанавливает заводской Баланс и штатную диспетчеризацию питания"
              >
                {restoringPowerPlan ? 'Идёт исправление...' : 'Исправить / сбросить план'}
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
                    {s.name} {s.is_active ? '(сейчас активно)' : ''}
                  </option>
                ))}
              </select>
              <button
                onClick={fetchPowerSchemes}
                disabled={restoringPowerPlan}
                className="toggle-btn"
                style={{ padding: '4px 7px', minWidth: 'auto' }}
                title="Обновить список схем электропитания системы"
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
                    <div className="sys-row-label">Уровень журналов работы и отладки аномалий</div>
                    <RevBadge variant={logStatus.level === 2 ? 'accent' : 'success'}>
                      {logStatus.level === 2 ? 'Все журналы (подробная трассировка)' : 'Журнал ошибок (только сбои и аварийные падения)'}
                    </RevBadge>
                    {logStatus.level === 2 && (
                      <RevBadge
                        variant={logStatus.filter ? 'warning' : 'default'}
                        title={
                          logStatus.filter
                            ? `Выводятся только совпадения [${logStatus.filter}] по журналам модулей (высокочастотный шум подавлен)`
                            : 'Фильтр не задан: выводятся журналы всех модулей'
                        }
                      >
                        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
                          {logStatus.filter
                            ? <><Crosshair size={10} /> Фильтр: {logStatus.filter}</>
                            : <><Globe size={10} /> Все модули</>}
                        </span>
                      </RevBadge>
                    )}
                  </div>
                  <div className="sys-row-desc">
                    Сейчас запущено с отладочными параметрами; после диагностики нажмите «Закрыть и скрыть», чтобы сэкономить место на диске
                  </div>
                </div>
                <div className="sys-row-controls">
                  <button
                    onClick={() => handleSetLogLevel(logStatus.level === 2 ? 1 : 2)}
                    className={`toggle-btn ${logStatus.level === 2 ? 'active-indigo' : ''}`}
                    style={{ minWidth: '78px' }}
                  >
                    {logStatus.level === 2 ? 'Переключить на журнал ошибок' : 'Переключить на все журналы'}
                  </button>
                  <button
                    onClick={handleOpenLog}
                    className="toggle-btn"
                    style={{ minWidth: '78px' }}
                    title={logStatus.path || 'Открыть файл журнала'}
                  >
                    Открыть журнал
                  </button>
                  <button
                    onClick={() => handleSetLogLevel(0)}
                    className="toggle-btn"
                    style={{ minWidth: '65px', color: 'var(--text-dim)' }}
                    title="Закрыть журнал и снова скрыть от обычных пользователей"
                  >
                    Закрыть и скрыть
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
                      Точная фильтрация модулей при диагностике (вторичные параметры)
                    </div>
                    <div className="sys-row-desc">
                      После выбора модуля тысячи высокочастотных наносекундных обращений чтения EC будут скрыты; поддерживается командная строка{' '}
                      <code style={{ color: 'var(--color-brand)' }}>--debug=имя модуля</code>
                    </div>
                  </div>
                  <div className="sys-chip-row">
                    {[
                      { label: 'Все', value: null, Icon: Globe },
                      { label: 'Внешняя СЖО', value: 'cooler', Icon: Droplets },
                      { label: 'Пользовательская трассировка', value: 'custom', Icon: Gauge },
                      { label: 'RGB и поток подсветки', value: 'light', Icon: Lightbulb },
                      { label: 'Питание и вентиляторы', value: 'power', Icon: Zap },
                      { label: 'Низкоуровневый EC', value: 'ec', Icon: Cpu },
                      { label: 'Устройства и периферия', value: 'device', Icon: Cable },
                      { label: 'Окно и система', value: 'ui', Icon: Sliders },
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
                  <span>Предпросмотр форм-фактора и возможностей модели в реальном времени (Dev Mode)</span>
                </div>
                <RevBadge color="amber">специально для --debug</RevBadge>
              </div>
              <div className="sys-card-desc">
                Позволяет разработчикам, не изменяя низкоуровневое взаимодействие, имитировать на стороне интерфейса обрезку слотов разных платформ (например скрытие dGPU/СЖО в ультрабуках).
              </div>

              <div className="sys-rows">
                <div className="quick-row">
                  <div className="sys-row-label">
                    <span>Текущая эмулируемая конфигурация</span>
                    <span className="sys-row-desc">
                      {previewMode === 'real'
                        ? `Реальное оборудование: ${capabilities.name || 'Адаптивно'} (${capabilities.form_factor || 'gaming'})`
                        : previewMode === 'thin_light'
                          ? 'Эмуляция: Xingyao 14 ультрабук (iGPU + без СЖО + одноцветная подсветка)'
                          : 'Эмуляция: стандартный универсальный игровой ноутбук (прямой вывод dGPU + СЖО + все функции)'}
                    </span>
                  </div>
                  <div className="sys-row-controls">
                    <button
                      className={`toggle-btn ${previewMode === 'real' ? 'active-emerald' : ''}`}
                      onClick={() => onPreviewFormFactor('real')}
                    >
                      Реальное устройство
                    </button>
                    <button
                      className={`toggle-btn ${previewMode === 'thin_light' ? 'active-cyan' : ''}`}
                      onClick={() => onPreviewFormFactor('thin_light')}
                    >
                      Ультрабук Xingyao 14
                    </button>
                    <button
                      className={`toggle-btn ${previewMode === 'gaming' ? 'active-indigo' : ''}`}
                      onClick={() => onPreviewFormFactor('gaming')}
                    >
                      Форм-фактор игрового ноутбука
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
                <div className="sys-modal-title">Удалить все пользовательские данные?</div>
                <div className="sys-modal-subtitle">Эта операция полностью очистит локальный каталог пользовательских данных</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                ⚠️ Внимание: после выполнения всё перечисленное ниже будет безвозвратно удалено:
              </div>
              <ul className="sys-modal-list">
                <li>Файлы пресетов режимов и мощности, созданные или изменённые пользователем</li>
                <li>Пользовательские эффекты клавиатуры, анимационные скрипты и пресеты</li>
                <li>Скачанные и установленные сторонние плагины и пользовательские темы</li>
                <li>Пользовательские имена и сопоставления в базе платформ (models.json)</li>
              </ul>
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={clearingData}
                onClick={() => setShowClearConfirm(false)}
                className="toggle-btn"
              >
                Отмена
              </button>
              <button
                disabled={clearingData}
                onClick={handleClearUserData}
                className="toggle-btn btn-danger-solid"
              >
                {clearingData ? 'Очистка...' : 'Подтвердить очистку'}
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
                <div className="sys-modal-title">Предупреждение о рисках снятия ограничения работы от батареи</div>
                <div className="sys-modal-subtitle">Правила безопасности при разряде оборудования и работе с батареей</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                ⚠️ Официальная прошивка по умолчанию запрещает высокое потребление от батареи. Учтите потенциальные риски:
              </div>
              <ul className="sys-modal-list">
                <li><strong>Отключение питания защитой батареи от сверхтока</strong>: мгновенная токоотдача батареи игрового ноутбука значительно ниже штатного адаптера; тяжёлая нагрузка от батареи может мгновенно вызвать срабатывание защиты BMS от сверхтока — внезапное отключение, чёрный экран или аппаратная перезагрузка.</li>
                <li><strong>Ускоренное старение батареи</strong>: постоянный сильный разряд большим током вызывает сильный нагрев внутри литий-ионной батареи и заметно ускоряет старение и деградацию её ёмкости.</li>
                <li><strong>При отключении адаптера принудительного понижения нет</strong>: после отключения адаптера система сохранит текущий режим Турбо или Экспертный и больше не будет автоматически возвращаться в Баланс.</li>
                <li><strong>Рекомендуемые сценарии</strong>: если вы хотите в Экспертной настройке на батарее «задать потребление CPU/GPU и температурные лимиты ниже офисного режима ради максимальной автономности» — пользуйтесь спокойно; но не запускайте на батарее тяжёлые 3D-игры и рендеринг.</li>
              </ul>
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={togglingUnlockBattery}
                onClick={() => setShowUnlockBatteryConfirm(false)}
                className="toggle-btn"
              >
                Отмена
              </button>
              <button
                disabled={togglingUnlockBattery}
                onClick={handleConfirmUnlockBattery}
                className="toggle-btn btn-danger-solid"
              >
                {togglingUnlockBattery ? 'Включение...' : 'Я осознаю риски и подтверждаю снятие'}
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
                <div className="sys-modal-title">Исправить и сбросить к заводскому плану электропитания</div>
                <div className="sys-modal-subtitle">Выравнивает с логикой официальной консоли, устраняя высокий нагрев в простое и аномальную блокировку частот</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                💡 Операция выполнит следующие безопасные исправления (полностью по заводским стандартам):
              </div>
              <ul className="sys-modal-list">
                <li><strong>Устраняет аномально высокий нагрев в простое</strong>: сбрасывает агрессивные оверлеи слайдеров производительности (Performance Overlay), возвращает парковку ядер CPU (Core Parking) и глубокий сон Package C-State, устраняя накопление 20+ Вт тепла в простое.</li>
                <li><strong>Полностью убирает остатки блокировки частот</strong>: хирургически удаляет ключи статических лимитов частоты (75b0ae3f) из всех схем электропитания, снимая ловушку блокировки на 0,8 ГГц или 25 Вт.</li>
                <li><strong>Повторно внедрить официальный OEM provisioning-пакет</strong>: адаптивно внедряет заводской PPM-пакет планировщика для текущей платформы (официальный Provisioning-комплект AMD для Baolongda / официальный PPM-пакет TongFang).</li>
                <li><strong>Активировать официальный сбалансированный план</strong>: возврат к стандартному сбалансированному плану Windows (Balanced). Личные пользовательские схемы электропитания полностью сохраняются и не удаляются.</li>
              </ul>
            </div>

            <div className="sys-modal-actions">
              <button
                disabled={restoringPowerPlan}
                onClick={() => setShowRestorePlanConfirm(false)}
                className="toggle-btn"
              >
                Отмена
              </button>
              <button
                disabled={restoringPowerPlan}
                onClick={handleRestoreOfficialPowerPlan}
                className="toggle-btn active-indigo"
              >
                {restoringPowerPlan ? 'Идёт исправление...' : 'Подтвердить выполнение исправления'}
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
                <div className="sys-modal-title">Перенести каталог хранения пользовательских данных</div>
                <div className="sys-modal-subtitle">Перенести пресеты конфигурации на новый путь с авто-перенаправлением</div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div className="sys-modal-warning-label">
                💡 Примечание: программа автоматически перенесёт все текущие пользовательские данные в новый каталог без потерь и создаст на старом месте файл-указатель перенаправления данных (data_dir.ptr).
              </div>
              <div style={{ marginTop: '10px', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <label style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                  Абсолютный путь к целевому каталогу (например D:\OpenRevoData):
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
                Отмена
              </button>
              <button
                disabled={migratingData || !migrateTargetPath.trim()}
                onClick={handleMigrateData}
                className="toggle-btn active-indigo"
              >
                {migratingData ? 'Перенос...' : 'Подтвердить перенос'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 显卡调度与超频偏好 */}
      <div className="glass-card" style={{ marginTop: '18px' }}>
        <h3 className="sys-section-title">Диспетчеризация и разгон GPU</h3>
        <div className="sys-rows">
          <div className="quick-row">
            <div>
              <div className="sys-row-label">Динамическое повышение мощности GPU в режиме Баланса (Dynamic Boost)</div>
              <div className="sys-row-desc">
                Позволяет dGPU раскрывать полную заводскую динамическую мощность в режиме Баланса (рекомендуется для 3A-игр на Балансе); при выключении лимитируется базовый TGP, а питание и охлаждение приоритетно отдаются CPU.
              </div>
            </div>
            <div className="sys-row-controls">
              <RevSwitch
                checked={balancedDbEnabled}
                onChange={handleToggleBalancedDb}
              />
            </div>
          </div>
          <div className="quick-row">
            <div>
              <div className="sys-row-label">Смещение частоты ядра для быстрого GPU-разгона по умолчанию</div>
              <div className="sys-row-desc">Частота ядра, применяемая переключателем GPU OC в шапке режима Турбо; вступит в силу при следующем включении</div>
            </div>
            <div className="sys-row-controls">
              <input
                type="number"
                min={0}
                max={250}
                step={5}
                value={gpuOcDefaults.core}
                onChange={(e) => setGpuOcDefaults((p) => ({ ...p, core: Number(e.target.value) }))}
                onBlur={() => saveGpuOcDefaults(gpuOcDefaults.core, gpuOcDefaults.mem)}
                style={{
                  width: '96px',
                  textAlign: 'right',
                  padding: '4px 8px',
                  background: 'var(--surface-sunken)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-medium)',
                  borderRadius: 'var(--radius-sm)',
                }}
              />
              <span style={{ marginLeft: '6px', color: 'var(--text-dim)', fontSize: '12px' }}>MHz</span>
            </div>
          </div>
          <div className="quick-row">
            <div>
              <div className="sys-row-label">Смещение частоты памяти для быстрого GPU-разгона по умолчанию</div>
              <div className="sys-row-desc">Частота памяти, применяемая переключателем GPU OC в шапке режима Турбо; вступит в силу при следующем включении</div>
            </div>
            <div className="sys-row-controls">
              <input
                type="number"
                min={0}
                max={1000}
                step={5}
                value={gpuOcDefaults.mem}
                onChange={(e) => setGpuOcDefaults((p) => ({ ...p, mem: Number(e.target.value) }))}
                onBlur={() => saveGpuOcDefaults(gpuOcDefaults.core, gpuOcDefaults.mem)}
                style={{
                  width: '96px',
                  textAlign: 'right',
                  padding: '4px 8px',
                  background: 'var(--surface-sunken)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-medium)',
                  borderRadius: 'var(--radius-sm)',
                }}
              />
              <span style={{ marginLeft: '6px', color: 'var(--text-dim)', fontSize: '12px' }}>MHz</span>
            </div>
          </div>
          {gpuOcSaveHint && (
            <div className="quick-row" style={{ color: 'var(--status-ok)', fontSize: '12px' }}>{gpuOcSaveHint}</div>
          )}
        </div>
      </div>

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
                  Найдена новая версия {updateResult.display_version || updateResult.latest_version}
                </div>
                <div className="sys-modal-subtitle">
                  Лучший источник: {updateResult.chosen_source === 'gitee' ? 'Ускорение через китайское зеркало Gitee' : updateResult.chosen_source === 'local_mock' ? 'Локальное тестовое эмулирование' : 'Официальный источник GitHub'} · Тип сборки: {updateResult.asset_type === 'installer' ? 'Обновление поверх установочного пакета' : 'Горячее обновление портативной версии без переустановки'}
                </div>
              </div>
            </div>

            <div className="sys-modal-body">
              <div style={{ marginBottom: '8px', fontSize: '11px', color: 'var(--text-muted)' }}>
                Журнал обновлений и описание изменений:
              </div>
              <div className="updater-changelog-box">
                {updateResult.notes || 'Подробный журнал обновлений пока отсутствует.'}
              </div>

              <div style={{ marginTop: '10px', fontSize: '11px', color: 'var(--text-dim)', display: 'flex', justifyContent: 'space-between' }}>
                <span>Файл дистрибутива: {updateResult.filename}</span>
                <span>Размер: {(updateResult.size / (1024 * 1024)).toFixed(2)} MB</span>
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
                Позже
              </button>
              <button
                disabled={downloadingUpdate}
                onClick={handleDownloadAndInstall}
                className="toggle-btn active-indigo"
              >
                {downloadingUpdate ? 'Скачивание обновления...' : 'Обновить сейчас'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
