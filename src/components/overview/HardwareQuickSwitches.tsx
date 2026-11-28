import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Wifi,
  Bluetooth,
  Camera,
  Mic,
  MicOff,
  Coffee,
  Moon,
  Monitor,
} from 'lucide-react';
import { HardwareState } from '../../types/hardware';
import { OsdConfig } from '../SystemSettingsView';
import { trText, useI18n } from '../../i18n/I18nContext';

export interface QuickSwitchesState {
  wifi: boolean;
  bluetooth: boolean;
  touchpad: boolean;
  camera: boolean;
  mic: boolean;
  osd: boolean;
  win_key: boolean;
  fn_lock: boolean;
  keep_awake: boolean;
}

interface HardwareQuickSwitchesProps {
  status: HardwareState;
  osdConfig: OsdConfig;
  onUpdateOsdConfig: (cfg: OsdConfig) => void;
}

export const HardwareQuickSwitches: React.FC<HardwareQuickSwitchesProps> = ({
  status,
  osdConfig,
  onUpdateOsdConfig: handleUpdateOsdConfig,
}) => {
  useI18n();
  const [switches, setSwitches] = useState<QuickSwitchesState>({
    wifi: true,
    bluetooth: true,
    touchpad: true,
    camera: true,
    mic: true,
    osd: true,
    win_key: status.win_key_locked,
    fn_lock: false,
    keep_awake: false,
  });

  const [togglingSwitches, setTogglingSwitches] = useState<Record<string, boolean>>({});

  useEffect(() => {
    const fetchSwitches = () => {
      invoke<QuickSwitchesState>('get_device_switches')
        .then((res) => {
          if (res) setSwitches(res);
        })
        .catch(() => {});
    };
    fetchSwitches();
    window.addEventListener('focus', fetchSwitches);

    let unlistenTouchpad: (() => void) | undefined;
    let unlistenBt: (() => void) | undefined;
    let unlistenWifi: (() => void) | undefined;
    let unlistenCam: (() => void) | undefined;
    let unlistenMic: (() => void) | undefined;
    let unlistenWinLock: (() => void) | undefined;
    let unlistenFnLock: (() => void) | undefined;
    let unlistenKeepAwake: (() => void) | undefined;

    listen<boolean>('touchpad-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, touchpad: event.payload }));
    }).then((fn) => { unlistenTouchpad = fn; });

    listen<boolean>('bluetooth-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, bluetooth: event.payload }));
    }).then((fn) => { unlistenBt = fn; });

    listen<boolean>('wifi-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, wifi: event.payload }));
    }).then((fn) => { unlistenWifi = fn; });

    listen<boolean>('camera-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, camera: event.payload }));
    }).then((fn) => { unlistenCam = fn; });

    listen<boolean>('mic-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, mic: event.payload }));
    }).then((fn) => { unlistenMic = fn; });

    listen<boolean>('win-lock-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, win_key: event.payload }));
    }).then((fn) => { unlistenWinLock = fn; });

    listen<boolean>('fn-lock-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, fn_lock: event.payload }));
    }).then((fn) => { unlistenFnLock = fn; });

    listen<boolean>('keep-awake-state-changed', (event) => {
      setSwitches((prev) => ({ ...prev, keep_awake: event.payload }));
    }).then((fn) => { unlistenKeepAwake = fn; });

    return () => {
      window.removeEventListener('focus', fetchSwitches);
      if (unlistenTouchpad) unlistenTouchpad();
      if (unlistenBt) unlistenBt();
      if (unlistenWifi) unlistenWifi();
      if (unlistenCam) unlistenCam();
      if (unlistenMic) unlistenMic();
      if (unlistenWinLock) unlistenWinLock();
      if (unlistenFnLock) unlistenFnLock();
      if (unlistenKeepAwake) unlistenKeepAwake();
    };
  }, []);

  useEffect(() => {
    setSwitches((prev) => ({ ...prev, win_key: status.win_key_locked }));
  }, [status.win_key_locked]);

  const handleToggleSwitch = async (name: keyof QuickSwitchesState) => {
    if (togglingSwitches[name]) return; // Guard: prevent concurrent/racing toggle while request is pending
    setTogglingSwitches((prev) => ({ ...prev, [name]: true }));
    const nextVal = !switches[name];
    setSwitches((prev) => ({ ...prev, [name]: nextVal }));

    if (name === 'osd') {
      await handleUpdateOsdConfig({ ...osdConfig, enabled: nextVal });
      setTogglingSwitches((prev) => ({ ...prev, [name]: false }));
      return;
    }

    try {
      const res = await invoke<boolean>('set_device_switch', { switchName: name, enable: nextVal });
      if (typeof res === 'boolean') {
        setSwitches((prev) => ({ ...prev, [name]: res }));
      }
    } catch (e) {
      console.error('Failed to toggle switch:', name, e);
      setSwitches((prev) => ({ ...prev, [name]: !nextVal }));
    } finally {
      setTogglingSwitches((prev) => ({ ...prev, [name]: false }));
    }
  };

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <div style={{ width: '4px', height: '14px', background: '#38bdf8', borderRadius: '2px' }} />
          <span style={{ fontSize: '13px', fontWeight: 700, letterSpacing: '0.04em', color: '#f1f5f9' }}>
            快捷设备硬件开关
          </span>
        </div>
        <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
          原厂电竞级热键与外设直控
        </span>
      </div>

      <div className="oem-switch-grid">
        {/* 1. WIFI 开关 */}
        <div
          onClick={() => handleToggleSwitch('wifi')}
          title="启用 / 禁用无线网络 (Wi-Fi)"
          className={`oem-switch-card ${switches.wifi ? 'active' : ''} ${togglingSwitches.wifi ? 'opacity-60 cursor-wait' : ''}`}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            <Wifi className="w-4 h-4" />
          </div>
          <div className="oem-switch-title">WIFI</div>
        </div>

        {/* 2. 蓝牙开关 */}
        <div
          onClick={() => handleToggleSwitch('bluetooth')}
          title="启用 / 禁用蓝牙 (Bluetooth)"
          className={`oem-switch-card ${switches.bluetooth ? 'active' : ''} ${togglingSwitches.bluetooth ? 'opacity-60 cursor-wait' : ''}`}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            <Bluetooth className="w-4 h-4" />
          </div>
          <div className="oem-switch-title">{trText('蓝牙')}</div>
        </div>

        {/* 3. 触摸板开关 */}
        <div
          onClick={() => handleToggleSwitch('touchpad')}
          title="启用 / 禁用触控板 (Touchpad)"
          className={`oem-switch-card ${switches.touchpad ? 'active' : ''} ${togglingSwitches.touchpad ? 'opacity-60 cursor-wait' : ''}`}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            <svg viewBox="0 0 24 24" className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth="2">
              <rect x="3" y="4" width="18" height="16" rx="2" />
              <line x1="3" y1="14" x2="21" y2="14" />
              <line x1="12" y1="14" x2="12" y2="20" />
            </svg>
          </div>
          <div className="oem-switch-title">{trText('触摸板')}</div>
        </div>

        {/* 4. 网络摄像头开关 */}
        <div
          onClick={() => handleToggleSwitch('camera')}
          title="启用 / 禁用内置网络摄像头 (Webcam)"
          className={`oem-switch-card ${switches.camera ? 'active' : ''} ${togglingSwitches.camera ? 'opacity-60 cursor-wait' : ''}`}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            <Camera className="w-4 h-4" />
          </div>
          <div className="oem-switch-title">{trText('摄像头')}</div>
        </div>

        {/* 5. 麦克风全局开关 (与摄像头相邻) */}
        <div
          onClick={() => handleToggleSwitch('mic')}
          title="启用 / 静音全局麦克风输入 (Microphone)"
          className={`oem-switch-card ${switches.mic ? 'active' : ''} ${togglingSwitches.mic ? 'opacity-60 cursor-wait' : ''}`}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            {switches.mic ? <Mic className="w-4 h-4" /> : <MicOff className="w-4 h-4 text-rose-400" />}
          </div>
          <div className="oem-switch-title">{switches.mic ? '麦克风' : trText('麦克风静音')}</div>
        </div>

        {/* 6. OSD 悬浮显示开关 */}
        <div
          onClick={() => handleToggleSwitch('osd')}
          title="启用 / 禁用屏幕快捷键 OSD 悬浮提示"
          className={`oem-switch-card ${switches.osd ? 'active' : ''} ${togglingSwitches.osd ? 'opacity-60 cursor-wait' : ''}`}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            <Monitor className="w-4 h-4" />
          </div>
          <div className="oem-switch-title">{trText('OSD显示')}</div>
        </div>

        {/* 7. Win 键 (Only show if hardware supports it) */}
        {status.support_flags?.has_win_lock !== false && (
          <div
            onClick={() => handleToggleSwitch('win_key')}
            title="游戏模式：锁定 / 放行 Windows 徽标键"
            className={`oem-switch-card ${switches.win_key ? 'active' : ''} ${togglingSwitches.win_key ? 'opacity-60 cursor-wait' : ''}`}
          >
            <div className="oem-bracket-tl" />
            <div className="oem-bracket-tr" />
            <div className="oem-bracket-bl" />
            <div className="oem-bracket-br" />

            <div className="oem-icon-badge">
              <svg viewBox="0 0 24 24" className="w-4 h-4" fill="currentColor">
                <path d="M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-12.9-1.801" />
              </svg>
            </div>
            <div className="oem-switch-title">{trText('Win键锁')}</div>
          </div>
        )}

        {/* 8. 功能键 (Fn 锁) */}
        {status.support_flags?.has_fn_lock !== false && (
          <div
            onClick={() => handleToggleSwitch('fn_lock')}
            title={switches.fn_lock ? 'Fn 锁已开启：需按 Fn 组合键' : trText('Fn 锁已关闭：F1-F12 键直接触发')}
            className={`oem-switch-card ${switches.fn_lock ? 'active' : ''} ${togglingSwitches.fn_lock ? 'opacity-60 cursor-wait' : ''}`}
          >
            <div className="oem-bracket-tl" />
            <div className="oem-bracket-tr" />
            <div className="oem-bracket-bl" />
            <div className="oem-bracket-br" />

            <div className="oem-icon-badge">
              <span style={{ fontSize: '12px', fontWeight: 900, letterSpacing: '0.04em' }}>FN</span>
            </div>
            <div className="oem-switch-title">{switches.fn_lock ? 'Fn锁定' : 'F1-F12'}</div>
          </div>
        )}

        {/* 9. 防休眠常亮开关 */}
        <div
          onClick={() => handleToggleSwitch('keep_awake')}
          title="防休眠常亮：挂机下载大游戏、渲染或演示时阻止屏幕变暗关闭与系统休眠"
          className={`oem-switch-card ${switches.keep_awake ? 'active' : ''} ${togglingSwitches.keep_awake ? 'opacity-60 cursor-wait' : ''}`}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            <Coffee className="w-4 h-4" />
          </div>
          <div className="oem-switch-title">{trText('防休眠')}</div>
        </div>

        {/* 10. 关闭屏幕 (3秒防外设误触唤醒) */}
        <div
          onClick={async () => {
            try {
              await invoke('turn_off_screen');
            } catch (e) {
              console.error('Failed to turn off screen:', e);
            }
          }}
          title="关闭屏幕：立即关闭显示器背光（3秒防外设误触，配合防休眠整夜挂机下载不刺眼）"
          className="oem-switch-card cursor-pointer"
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          <div className="oem-icon-badge">
            <Moon className="w-4 h-4" />
          </div>
          <div className="oem-switch-title">{trText('关闭屏幕')}</div>
        </div>
      </div>
    </div>
  );
};
