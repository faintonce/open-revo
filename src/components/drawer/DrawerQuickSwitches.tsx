import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { HardwareState } from '../../types/hardware';
import {
  Fan,
  Camera,
  CameraOff,
  Mic,
  MicOff,
  Lock,
  Unlock,
  Coffee,
  Moon,
  Laptop,
} from 'lucide-react';

interface QuickSwitchesState {
  wifi: boolean;
  bluetooth: boolean;
  touchpad: boolean;
  camera: boolean;
  mic: boolean;
  keep_awake: boolean;
}

import { ModelCapabilities } from '../../plugins/plugin_manifest';
import { trText, useI18n } from '../../i18n/I18nContext';

interface DrawerQuickSwitchesProps {
  status: HardwareState;
  capabilities?: ModelCapabilities;
  checkOemGuard: (action: () => void) => void;
  onToggleFanBoost: () => void;
  onToggleWinKey: () => void;
}

export const DrawerQuickSwitches: React.FC<DrawerQuickSwitchesProps> = ({
  status,
  capabilities: _capabilities,
  checkOemGuard,
  onToggleFanBoost,
  onToggleWinKey,
}) => {
  useI18n();
  const [switches, setSwitches] = useState<{
    camera: boolean;
    mic: boolean;
    keep_awake: boolean;
  }>({
    camera: true,
    mic: true,
    keep_awake: false,
  });

  const syncSwitches = () => {
    invoke<QuickSwitchesState>('get_device_switches')
      .then((res) => {
        if (res) {
          setSwitches({
            camera: res.camera,
            mic: res.mic,
            keep_awake: res.keep_awake,
          });
        }
      })
      .catch((err) => console.warn('Failed to get quick switches:', err));
  };

  const toggleSwitch = async (name: 'camera' | 'mic' | 'keep_awake') => {
    const current = switches[name];
    const target = !current;
    setSwitches((prev) => ({ ...prev, [name]: target }));
    try {
      await invoke('set_device_switch', { switchName: name, enable: target });
    } catch (err) {
      console.warn(`Toggle ${name} error:`, err);
      setSwitches((prev) => ({ ...prev, [name]: current }));
    }
  };

  useEffect(() => {
    syncSwitches();
    window.addEventListener('focus', syncSwitches);

    let unlistenCam: (() => void) | undefined;
    let unlistenMic: (() => void) | undefined;
    let unlistenKeepAwake: (() => void) | undefined;

    listen<boolean>('camera-state-changed', (e) => {
      setSwitches((p) => ({ ...p, camera: e.payload }));
    }).then((fn) => { unlistenCam = fn; });

    listen<boolean>('mic-state-changed', (e) => {
      setSwitches((p) => ({ ...p, mic: e.payload }));
    }).then((fn) => { unlistenMic = fn; });

    listen<boolean>('keep-awake-state-changed', (e) => {
      setSwitches((p) => ({ ...p, keep_awake: e.payload }));
    }).then((fn) => { unlistenKeepAwake = fn; });

    return () => {
      window.removeEventListener('focus', syncSwitches);
      if (unlistenCam) unlistenCam();
      if (unlistenMic) unlistenMic();
      if (unlistenKeepAwake) unlistenKeepAwake();
    };
  }, []);

  return (
    <>
      {/* =========================================================
          第 7 排：快捷开关矩阵 (5个方按钮: 强冷, 摄像头, 麦克风, Win键锁, 防休眠)
          ========================================================= */}
      <div className="m-quick-grid">
        {/* 1. 一键强冷 */}
        {status.support_flags?.has_fan_boost !== false && (
          <button
            className={`m-quick-btn ${status.fan_boost ? 'active' : ''}`}
            onClick={() => checkOemGuard(onToggleFanBoost)}
            title={status.fan_boost ? '双风扇狂暴强冷运转中 (点击关闭)' : trText('一键开启双风扇极速强冷')}
          >
            <Fan size={16} className={status.fan_boost ? 'animate-spin' : ''} />
            <span className="m-quick-btn-label">{trText('一键强冷')}</span>
          </button>
        )}

        {/* 2. 摄像头开关 */}
        <button
          className={`m-quick-btn ${switches.camera ? 'active' : ''}`}
          onClick={() => toggleSwitch('camera')}
          title={switches.camera ? '摄像头已供电开启 (点击硬件断电禁用)' : trText('摄像头已硬件断电禁用 (点击开启)')}
        >
          {switches.camera ? <Camera size={16} /> : <CameraOff size={16} />}
          <span className="m-quick-btn-label">{switches.camera ? '摄像头' : trText('相机已关')}</span>
        </button>

        {/* 3. 麦克风开关 */}
        <button
          className={`m-quick-btn ${switches.mic ? 'active' : ''}`}
          onClick={() => toggleSwitch('mic')}
          title={switches.mic ? '麦克风已就绪拾音 (点击一键静音)' : trText('麦克风已一键静音 (点击解除)')}
        >
          {switches.mic ? <Mic size={16} /> : <MicOff size={16} />}
          <span className="m-quick-btn-label">{switches.mic ? '麦克风' : trText('麦克静音')}</span>
        </button>

        {/* 4. Win 键锁 */}
        <button
          className={`m-quick-btn ${status.win_key_locked ? 'active' : ''}`}
          onClick={onToggleWinKey}
          title={status.win_key_locked ? 'Windows 徽标键已锁定防误触 (点击解锁)' : trText('Windows 徽标键未锁定 (点击锁定)')}
        >
          {status.win_key_locked ? <Lock size={16} /> : <Unlock size={16} />}
          <span className="m-quick-btn-label">{status.win_key_locked ? 'Win已锁' : trText('Win键锁')}</span>
        </button>

        {/* 5. 防休眠 */}
        <button
          className={`m-quick-btn ${switches.keep_awake ? 'active' : ''}`}
          onClick={() => toggleSwitch('keep_awake')}
          title={switches.keep_awake ? '防休眠模式已开启 (保持系统常亮不锁屏)' : trText('防休眠模式已关闭 (遵循系统电源计划)')}
        >
          <Coffee size={16} />
          <span className="m-quick-btn-label">{switches.keep_awake ? '常亮保持' : trText('防休眠')}</span>
        </button>

        {/* 6. 关闭屏幕 (3秒防误触) */}
        <button
          className="m-quick-btn"
          onClick={async () => {
            try {
              await invoke('turn_off_screen');
            } catch (e) {
              console.error('Failed to turn off screen:', e);
            }
          }}
          title="立即关闭屏幕显示器（3秒防外设误触唤醒，配合防休眠整夜挂机）"
        >
          <Moon size={16} />
          <span className="m-quick-btn-label">{trText('关闭屏幕')}</span>
        </button>
      </div>

      {/* 底部设备型号与硬件就绪状态栏 */}
      <div
        style={{
          marginTop: '3px',
          marginBottom: '0px',
          padding: '4px 9px',
          borderRadius: '6px',
          background: 'rgba(255, 255, 255, 0.025)',
          border: '1px solid rgba(255, 255, 255, 0.06)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          fontSize: '10px',
          color: 'var(--text-dim)',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', overflow: 'hidden' }}>
          <Laptop size={13} style={{ color: 'var(--text-muted)', flexShrink: 0 }} />
          <span
            style={{
              color: '#cbd5e1',
              fontWeight: 600,
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              maxWidth: '220px',
            }}
            title={status.model_name || 'MECHREVO Gaming Laptop'}
          >
            {status.model_name || 'MECHREVO Gaming Laptop'}
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '5px', flexShrink: 0, paddingLeft: '8px' }}>
          <div style={{ width: '5px', height: '5px', borderRadius: '50%', background: '#34d399', boxShadow: '0 0 5px #34d399' }} />
          <span style={{ fontSize: '9.5px', color: '#6ee7b7', fontWeight: 700, letterSpacing: '0.04em' }}>READY</span>
        </div>
      </div>
    </>
  );
};
