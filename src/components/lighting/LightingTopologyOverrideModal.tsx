import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Cpu, Check, RotateCcw, Loader2 } from 'lucide-react';

export interface LightingHardwareOverride {
  enabled: boolean;
  keyboard: string;   // "auto" | "per_key" | "four_zone" | "single_zone" | "single_white" | "none"
  front_bar: string;  // "auto" | "present" | "none"
  ambient: string;    // "auto" | "vent" | "shoulder" | "none"
  logo: string;       // "auto" | "letter" | "shield" | "none"
}

interface LightingRuntimeStatus {
  keyboard_engine: string;
  keyboard_online: boolean;
  keyboard_active: boolean;
  keyboard_fps: number;
  lightbar_online: boolean;
  lightbar_active: boolean;
  hinge_online: boolean;
  hinge_active: boolean;
  logo_online: boolean;
  logo_active: boolean;
  is_sleeping: boolean;
  hardware_override: LightingHardwareOverride;
  detected_baseline: LightingHardwareOverride;
}

interface LightingTopologyOverrideModalProps {
  isOpen: boolean;
  onClose: () => void;
  onApplied?: () => void;
}

export const LightingTopologyOverrideModal: React.FC<LightingTopologyOverrideModalProps> = ({
  isOpen,
  onClose,
  onApplied,
}) => {
  const [config, setConfig] = useState<LightingHardwareOverride>({
    enabled: false,
    keyboard: 'auto',
    front_bar: 'auto',
    ambient: 'auto',
    logo: 'auto',
  });

  const [baseline, setBaseline] = useState<LightingHardwareOverride>({
    enabled: false,
    keyboard: 'auto',
    front_bar: 'auto',
    ambient: 'auto',
    logo: 'auto',
  });

  const [isApplying, setIsApplying] = useState(false);
  const [applyStep, setApplyStep] = useState('');
  const [errorMessage, setErrorMessage] = useState('');

  useEffect(() => {
    if (isOpen) {
      setErrorMessage('');
      invoke<LightingRuntimeStatus>('get_lighting_runtime_status')
        .then((status) => {
          if (status.hardware_override) {
            setConfig(status.hardware_override);
          }
          if (status.detected_baseline) {
            setBaseline(status.detected_baseline);
          }
        })
        .catch((err) => {
          console.error('[LightingTopology] Failed to fetch runtime status:', err);
        });
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSelect = (category: keyof LightingHardwareOverride, value: string) => {
    setErrorMessage('');
    setConfig((prev) => ({
      ...prev,
      enabled: true,
      [category]: value,
    }));
  };

  const handleResetToAuto = () => {
    setErrorMessage('');
    setConfig({
      enabled: false,
      keyboard: 'auto',
      front_bar: 'auto',
      ambient: 'auto',
      logo: 'auto',
    });
  };

  const handleApply = async () => {
    setIsApplying(true);
    setErrorMessage('');
    setApplyStep('正在执行驱动安全退火与硬件复位...');

    try {
      // 保证用户纠偏时如果全是 auto 则将 enabled 置为 false
      const isAllAuto =
        config.keyboard === 'auto' &&
        config.front_bar === 'auto' &&
        config.ambient === 'auto' &&
        config.logo === 'auto';

      const finalCfg: LightingHardwareOverride = {
        ...config,
        enabled: !isAllAuto,
      };

      await invoke('apply_lighting_hardware_override', {
        overrideCfg: finalCfg,
        override_cfg: finalCfg,
      });

      setConfig(finalCfg);
      setApplyStep('硬件通道绑定成功！');
      setTimeout(() => {
        setIsApplying(false);
        onClose();
        if (onApplied) onApplied();
      }, 500);
    } catch (err) {
      console.error('[LightingTopology] Failed to apply override:', err);
      setIsApplying(false);
      setApplyStep('');
      setErrorMessage(String(err));
    }
  };

  // 计算差异项
  const effectiveKb = config.keyboard === 'auto' ? baseline.keyboard : config.keyboard;
  const effectiveFront = config.front_bar === 'auto' ? baseline.front_bar : config.front_bar;
  const effectiveAmb = config.ambient === 'auto' ? baseline.ambient : config.ambient;
  const effectiveLogo = config.logo === 'auto' ? baseline.logo : config.logo;

  let diffCount = 0;
  if (config.keyboard !== 'auto' && config.keyboard !== baseline.keyboard) diffCount++;
  if (config.front_bar !== 'auto' && config.front_bar !== baseline.front_bar) diffCount++;
  if (config.ambient !== 'auto' && config.ambient !== baseline.ambient) diffCount++;
  if (config.logo !== 'auto' && config.logo !== baseline.logo) diffCount++;

  return (
    <div className="sys-modal-backdrop" onClick={onClose}>
      <div
        className="sys-modal-card"
        style={{ maxWidth: '840px', width: '94%', overflow: 'visible' }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="sys-modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <div
              className="sys-modal-icon-badge"
              style={{ background: 'var(--status-info-dim)', color: 'var(--status-info)' }}
            >
              <Cpu size={18} />
            </div>
            <div>
              <div className="sys-modal-title" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <span>灯效硬件设备拓扑纠偏</span>
                <span
                  style={{
                    fontSize: '10.5px',
                    fontWeight: 600,
                    color: 'var(--status-info)',
                    background: 'var(--status-info-dim)',
                    padding: '1px 7px',
                    borderRadius: 'var(--radius-sm)',
                    border: '1px solid var(--status-info)',
                  }}
                >
                  超薄自适应
                </span>
              </div>
              <div className="sys-modal-subtitle">
                若自动检测与物理硬件不符，请在此手动指定物理设备。鼠标悬停在选项或 <strong>ⓘ</strong> 上可查看硬件协议与适用机型。
              </div>
            </div>
          </div>
        </div>

        {/* Body: 紧凑行 4 维度拓扑 */}
        <div className="sys-modal-body" style={{ overflow: 'visible', padding: '14px 20px' }}>
          {errorMessage && (
            <div
              style={{
                marginBottom: '12px',
                padding: '8px 12px',
                borderRadius: 'var(--radius-sm)',
                background: 'var(--status-error-dim)',
                color: 'var(--status-error)',
                fontSize: '12px',
                border: '1px solid var(--status-error)',
              }}
            >
              应用失败: {errorMessage}
            </div>
          )}
          <div className="topology-container">

            {/* 1. 键盘背光 */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">⌨️ 1. 键盘背光硬件</span>
                <span className="topology-row-hint">
                  自动探测: <strong>{baseline.keyboard || '检测中'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                {/* 全彩逐键 */}
                <div
                  className={`topology-pill ${effectiveKb === 'per_key' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'per_key')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">全彩逐键 RGB</span>
                  {baseline.keyboard === 'per_key' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>全彩逐键 RGB</span>
                      <span className="topology-tip-chip">048D:600B / CE00</span>
                    </div>
                    <p className="topology-tip-desc">每个按键独立设色，支持 BetterRGB 60FPS 极客流光、水波涟漪与自定义着色器。</p>
                    <div className="topology-tip-example">典型: 耀世16 Pro/Ultra, 苍龙16, 蛟龙 GM7ZG7S(CE00) 等高配</div>
                  </div>
                </div>

                {/* 4区 RGB */}
                <div
                  className={`topology-pill ${effectiveKb === 'four_zone' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'four_zone')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">4区 RGB</span>
                  {baseline.keyboard === 'four_zone' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>4区 RGB 硬件分段</span>
                      <span className="topology-tip-chip">048D:CE00</span>
                    </div>
                    <p className="topology-tip-desc">键盘横向分为 4 个物理区域独立设色，支持硬件级波浪跑马与呼吸。</p>
                    <div className="topology-tip-example">典型: 蛟龙16, X10 Pro, 极光早期版</div>
                  </div>
                </div>

                {/* 单区 RGB */}
                <div
                  className={`topology-pill ${effectiveKb === 'single_zone' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'single_zone')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">单区 RGB</span>
                  {baseline.keyboard === 'single_zone' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>单区 RGB (整盘同色)</span>
                      <span className="topology-tip-chip">EC RAM PWM</span>
                    </div>
                    <p className="topology-tip-desc">由主板 EC 直驱，全键盘作为一个整体变色，支持单色常亮、呼吸与整体彩虹渐变。</p>
                    <div className="topology-tip-example">典型: 极光X, 耀世15单区版, 旷世G16</div>
                  </div>
                </div>

                {/* 单色白光 */}
                <div
                  className={`topology-pill ${effectiveKb === 'single_white' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'single_white')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">单色白光</span>
                  {baseline.keyboard === 'single_white' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>单色白光背光</span>
                      <span className="topology-tip-chip">EC 1932</span>
                    </div>
                    <p className="topology-tip-desc">纯白光 LED，支持关 / 弱光 (50%) / 强光 (100%) 三档无频闪梯级调光。</p>
                    <div className="topology-tip-example">典型: 无界14X, 无界15X 等轻薄本</div>
                  </div>
                </div>

                {/* 禁用 */}
                <div
                  className={`topology-pill ${effectiveKb === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">无背光/禁用</span>
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>无键盘背光</span>
                      <span className="topology-tip-chip">Disabled</span>
                    </div>
                    <p className="topology-tip-desc">模具无发光背光，或在软件中彻底隐藏键盘控制卡片。</p>
                  </div>
                </div>
              </div>
            </div>

            {/* 2. 前置海岸灯条 */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">🌊 2. 前置海岸灯条</span>
                <span className="topology-row-hint">
                  自动探测: <strong>{baseline.front_bar === 'present' ? '具备灯条' : '未检测到'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                <div
                  className={`topology-pill ${effectiveFront === 'present' ? 'selected' : ''}`}
                  onClick={() => handleSelect('front_bar', 'present')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">前置海岸灯条 (具备)</span>
                  {baseline.front_bar === 'present' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>前唇海岸流光灯条</span>
                      <span className="topology-tip-chip">048D:7001 / EC</span>
                    </div>
                    <p className="topology-tip-desc">位于触摸板正下方前边缘的细长多段流光灯条，支持对撞、跑马与全彩波浪。</p>
                    <div className="topology-tip-example">典型: 耀世16, 蛟龙16 等前唇灯机型</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveFront === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('front_bar', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">无海岸灯条 (屏蔽)</span>
                  {baseline.front_bar === 'none' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>无前海岸灯</span>
                      <span className="topology-tip-chip">None</span>
                    </div>
                    <p className="topology-tip-desc">模具前唇物理无发光条，选择此项可彻底移除幽灵海岸灯卡片并停止发包。</p>
                  </div>
                </div>
              </div>
            </div>

            {/* 3. 特色氛围灯 */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">💨 3. 特色氛围灯</span>
                <span className="topology-row-hint">
                  自动探测: <strong>{baseline.ambient || '未检测到'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                <div
                  className={`topology-pill ${effectiveAmb === 'vent' ? 'selected' : ''}`}
                  onClick={() => handleSelect('ambient', 'vent')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">尾气灯 (后出风口氛围灯)</span>
                  {baseline.ambient === 'vent' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>尾部排气格栅氛围灯</span>
                      <span className="topology-tip-chip">048D:7000 / Hinge</span>
                    </div>
                    <p className="topology-tip-desc">后出风口左右对称的跑车式排气灯带，水冷模具经典硬核视觉标识。</p>
                    <div className="topology-tip-example">典型: 旷世16 Super 水冷版等旗舰</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveAmb === 'shoulder' ? 'selected' : ''}`}
                  onClick={() => handleSelect('ambient', 'shoulder')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">八字肩灯 (飞翼同步灯)</span>
                  {baseline.ambient === 'shoulder' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>八字双飞翼肩灯</span>
                      <span className="topology-tip-chip">IDZ Sync</span>
                    </div>
                    <p className="topology-tip-desc">C面键盘左右上方呈“八”字斜向展开的跑马飞翼双灯条，与键盘同频律动。</p>
                    <div className="topology-tip-example">典型: 深海幽灵 IDZ 模具系列</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveAmb === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('ambient', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">无氛围灯 (默认)</span>
                  {baseline.ambient === 'none' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>无额外氛围灯</span>
                      <span className="topology-tip-chip">None</span>
                    </div>
                    <p className="topology-tip-desc">普通机型无后排出风口灯或肩灯，控制台中不挂载氛围灯控制卡片。</p>
                  </div>
                </div>
              </div>
            </div>

            {/* 4. A面 LOGO */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">✨ 4. A面 LOGO 灯</span>
                <span className="topology-row-hint">
                  自动探测: <strong>{baseline.logo || '未检测到'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                <div
                  className={`topology-pill ${effectiveLogo === 'letter' ? 'selected' : ''}`}
                  onClick={() => handleSelect('logo', 'letter')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">字母 LOGO (MECHREVO)</span>
                  {baseline.logo === 'letter' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>英文字母透光 LOGO</span>
                      <span className="topology-tip-chip">USB NewLogo (35)</span>
                    </div>
                    <p className="topology-tip-desc">整串 MECHREVO 英文全拼透光发光，由独立 USB 芯片驱动，支持彩色波浪与跑马流光。</p>
                    <div className="topology-tip-example">典型: GM6IX0B, 耀世早期特殊版</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveLogo === 'shield' ? 'selected' : ''}`}
                  onClick={() => handleSelect('logo', 'shield')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">盾形 LOGO (上下螺母标)</span>
                  {baseline.logo === 'shield' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>盾形螺母发光标</span>
                      <span className="topology-tip-chip">Motherboard EC 1963</span>
                    </div>
                    <p className="topology-tip-desc">上下两瓣组成的盾形/螺母发光标，由主板 EC 直驱，支持单色常亮、呼吸与三色混光。</p>
                    <div className="topology-tip-example">典型: 苍龙, 耀世全系列, 极光X, 蛟龙</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveLogo === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('logo', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">无 A面 LOGO 灯</span>
                  {baseline.logo === 'none' && <span className="topology-badge-auto">自动</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>无 A面 LOGO 背光</span>
                      <span className="topology-tip-chip">None</span>
                    </div>
                    <p className="topology-tip-desc">A面仅为金属贴牌、高光印字或不发光，移除 A 面灯效卡片避免误操作。</p>
                  </div>
                </div>
              </div>
            </div>

          </div>
        </div>

        {/* Footer Actions */}
        <div className="sys-modal-actions" style={{ justifyContent: 'space-between', padding: '12px 20px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <button
              type="button"
              className="toggle-btn"
              onClick={handleResetToAuto}
              style={{ fontSize: '11px', display: 'flex', alignItems: 'center', gap: '5px' }}
            >
              <RotateCcw size={12} />
              <span>恢复全自动探测</span>
            </button>
            <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
              {diffCount === 0 ? (
                '与自动探测一致'
              ) : (
                <span style={{ color: 'var(--status-warning)', fontWeight: 600 }}>
                  {diffCount} 项已手动纠偏
                </span>
              )}
            </span>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <button
              type="button"
              className="toggle-btn"
              onClick={onClose}
              disabled={isApplying}
            >
              取消
            </button>
            <button
              type="button"
              className="toggle-btn active-indigo"
              onClick={handleApply}
              disabled={isApplying}
              style={{ fontWeight: 700 }}
            >
              {isApplying ? (
                <>
                  <Loader2 size={13} className="animate-spin" />
                  <span>{applyStep || '正在应用...'}</span>
                </>
              ) : (
                <>
                  <Check size={13} />
                  <span>保存并重新初始化硬件</span>
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
