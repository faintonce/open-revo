import React, { useState } from 'react';
import { HardwareState } from '../../types/hardware';
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
            title={status.autostart ? 'Автозапуск включён (сворачивается в трей)' : 'Автозапуск при включении не включён'}
          >
            <input
              type="checkbox"
              checked={!!status.autostart}
              readOnly
              style={{ accentColor: '#38bdf8', cursor: 'pointer', margin: 0 }}
            />
            <span style={{ color: status.autostart ? '#f1f5f9' : 'var(--text-dim)', whiteSpace: 'nowrap' }}>
              Автозапуск при включении
            </span>
          </label>

          {/* 接管/恢复官方控制台按钮 (若机器未安装官方控制台则隐藏) */}
          {status.oem_installed && (
            <button
              onClick={() => handleServiceToggle(status.oem_running ?? false)}
              disabled={isSwitchingService}
              title={
                status.oem_running
                  ? 'Обнаружена работающая официальная игровая служба MECHREVO. По клику она будет остановлена и отключена одним нажатием, чтобы OpenRevo не конфликтовал за аппаратные EC-регистры.'
                  : 'Официальная игровая служба MECHREVO сейчас заблокирована/перехвачена. По клику можно снова запустить штатную фоновую службу производителя.'
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
              <span>{status.oem_running ? 'Перехватить официальную консоль' : 'Вернуть официальную консоль'}</span>
            </button>
          )}
        </div>

        <div style={{ display: 'flex', alignItems: 'center' }}>
          <button className="m-foot-btn expand" onClick={() => onExpandDashboard('overview')}>
            <Maximize2 size={12} />
            <span>Развернуть большую панель</span>
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
              Официальный центр управления запущен
            </div>

            <div style={{ fontSize: '11px', color: '#94a3b8', lineHeight: 1.5, marginBottom: '16px', textAlign: 'left' }}>
              Обнаружен работающий официальный игровой фон MECHREVO. Чтобы не конфликтовать за EC-регистры платы и не вызвать<strong>Постоянное мерцание экрана</strong>, сейчас находится в<strong>Безопасный режим «только чтение» с уклонением</strong>.
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
                <span>{isSwitchingService ? 'Перехват службы и освобождение оборудования...' : 'Немедленно перехватить официальную службу одним нажатием'}</span>
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
                Пока не перехватывать: оставить мониторинг «только чтение»
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
