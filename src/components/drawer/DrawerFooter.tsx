import React, { useState } from 'react';
import { HardwareState } from '../../types/hardware';
import { trText, useI18n } from '../../i18n/I18nContext';
import {
  Shield,
  Zap,
  Repeat,
  Maximize2,
  Loader2,
} from 'lucide-react';

interface DrawerFooterProps {
  status: HardwareState;
  onToggleAutostart: () => void;
  onToggleOemService: (takeOver: boolean) => Promise<void>;
  onExpandDashboard: (tab?: 'overview' | 'gpu' | 'tuning' | 'lighting' | 'water' | 'system' | 'sponsor' | 'custom_tuning') => void;
  onClose?: () => void;
  showOemModal: boolean;
  setShowOemModal: (show: boolean) => void;
}

export const DrawerFooter: React.FC<DrawerFooterProps> = ({
  status,
  onToggleAutostart,
  onToggleOemService,
  onExpandDashboard,
  showOemModal,
  setShowOemModal,
}) => {
  useI18n();
  const [isSwitchingService, setIsSwitchingService] = useState(false);

  const handleServiceToggle = async (takeOver: boolean) => {
    setIsSwitchingService(true);
    try {
      await onToggleOemService(takeOver);
    } finally {
      setIsSwitchingService(false);
    }
  };

  return (
    <>
      {/* 底部操作区：开机自启、官方接管切换、展开大面板与退出 */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '5px 2px 2px 2px',
          borderTop: '1px solid rgba(255, 255, 255, 0.06)',
          gap: '8px',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {/* 开机自启 */}
          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              fontSize: '10.5px',
              color: 'var(--text-dim)',
              cursor: 'pointer',
              userSelect: 'none',
            }}
            onClick={onToggleAutostart}
            title={status.autostart ? '已启用开机自启动 (最小化至托盘)' : trText('未启用开机自启')}
          >
            <input
              type="checkbox"
              checked={!!status.autostart}
              readOnly
              style={{ accentColor: '#38bdf8', cursor: 'pointer', margin: 0 }}
            />
            <span style={{ color: status.autostart ? '#f1f5f9' : 'var(--text-dim)', whiteSpace: 'nowrap' }}>
              开机自启
            </span>
          </label>

          {/* 接管/恢复官方控制台按钮 (若机器未安装官方控制台则隐藏) */}
          {status.oem_installed && (
            <button
              onClick={() => handleServiceToggle(status.oem_running ?? false)}
              disabled={isSwitchingService}
              title={
                status.oem_running
                  ? trText('检测到机械革命官方电竞服务正在运行。点击后将一键停止并禁用官方服务，防止与 OpenRevo 争抢 EC 硬件寄存器。')
                  : trText('机械革命官方电竞服务当前已被屏蔽/接管。点击后可重新恢复启动官方原厂后台服务。')
              }
              style={{
                padding: '2px 7px',
                borderRadius: '4px',
                fontSize: '9.5px',
                fontWeight: 600,
                cursor: isSwitchingService ? 'wait' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                whiteSpace: 'nowrap',
                transition: 'all 0.15s ease',
                background: status.oem_running ? 'rgba(245, 158, 11, 0.15)' : 'rgba(56, 189, 248, 0.1)',
                color: status.oem_running ? '#fbbf24' : '#38bdf8',
                border: status.oem_running ? '1px solid rgba(245, 158, 11, 0.35)' : '1px solid rgba(56, 189, 248, 0.25)',
              }}
            >
              {isSwitchingService ? (
                <Loader2 size={10} className="animate-spin" />
              ) : status.oem_running ? (
                <Shield size={10} />
              ) : (
                <Repeat size={10} />
              )}
              <span>{status.oem_running ? '接管官方控制台' : trText('恢复官方控制台')}</span>
            </button>
          )}
        </div>

        <div style={{ display: 'flex', alignItems: 'center' }}>
          <button className="m-foot-btn expand" onClick={() => onExpandDashboard('overview')}>
            <Maximize2 size={12} />
            <span>{trText('展开大面板')}</span>
          </button>
        </div>
      </div>

      {/* 官方控制中心冲突安全避让向导模态框 */}
      {showOemModal && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            backgroundColor: 'rgba(0, 0, 0, 0.72)',
            backdropFilter: 'blur(8px)',
            zIndex: 9999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '16px',
            animation: 'fadeIn 0.15s ease',
          }}
          onClick={() => setShowOemModal(false)}
        >
          <div
            style={{
              background: 'linear-gradient(135deg, rgba(30, 41, 59, 0.98), rgba(15, 23, 42, 0.99))',
              border: '1px solid rgba(245, 158, 11, 0.4)',
              boxShadow: '0 20px 40px rgba(0, 0, 0, 0.7), 0 0 25px rgba(245, 158, 11, 0.2)',
              borderRadius: '12px',
              padding: '18px',
              width: '100%',
              maxWidth: '320px',
              textAlign: 'center',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '10px' }}>
              <div
                style={{
                  width: '40px',
                  height: '40px',
                  borderRadius: '50%',
                  background: 'rgba(245, 158, 11, 0.15)',
                  border: '1px solid rgba(245, 158, 11, 0.3)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: '#fbbf24',
                }}
              >
                <Shield size={22} />
              </div>
            </div>

            <div style={{ fontSize: '14px', fontWeight: 700, color: '#f8fafc', marginBottom: '6px' }}>
              官方控制中心正在运行
            </div>

            <div style={{ fontSize: '11px', color: '#94a3b8', lineHeight: 1.5, marginBottom: '16px', textAlign: 'left' }}>
              检测到机械革命官方电竞后台正在运行。为防止与官方争抢主板 EC 寄存器并导致<strong>{trText('屏幕持续闪烁')}</strong>{trText('，当前处于')}<strong>{trText('安全只读避让模式')}</strong>。
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <button
                onClick={async () => {
                  try {
                    await handleServiceToggle(true);
                  } finally {
                    setShowOemModal(false);
                  }
                }}
                disabled={isSwitchingService}
                style={{
                  background: 'linear-gradient(135deg, #f59e0b, #d97706)',
                  color: '#ffffff',
                  fontWeight: 700,
                  fontSize: '11.5px',
                  padding: '8px 12px',
                  borderRadius: '6px',
                  border: 'none',
                  boxShadow: '0 4px 12px rgba(245, 158, 11, 0.35)',
                  cursor: isSwitchingService ? 'wait' : 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '6px',
                }}
              >
                {isSwitchingService ? <Loader2 size={13} className="animate-spin" /> : <Zap size={13} />}
                <span>{isSwitchingService ? '正在接管服务并释放硬件...' : trText('立即一键接管官方服务')}</span>
              </button>

              <button
                onClick={() => setShowOemModal(false)}
                style={{
                  background: 'rgba(255, 255, 255, 0.06)',
                  color: '#94a3b8',
                  fontSize: '10.5px',
                  padding: '6px 10px',
                  borderRadius: '6px',
                  border: '1px solid rgba(255, 255, 255, 0.1)',
                  cursor: 'pointer',
                }}
              >
                暂不接管，保持只读监控
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
