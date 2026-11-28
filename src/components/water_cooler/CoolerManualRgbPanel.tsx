import React from 'react';
import { Zap, Fan, Palette } from 'lucide-react';
import { CoolerTelemetry, COOLER_COLOR_PRESETS } from './coolerTypes';
import { RevSwitch } from '../ui';
import { trText, useI18n } from '../../i18n/I18nContext';

interface CoolerManualRgbPanelProps {
  coolerData: CoolerTelemetry;
  onUpdateCooler: (pumpV?: number, fanD?: number) => void;
  onUpdateCoolerLed: (on?: boolean, r?: number, g?: number, b?: number, mode?: number) => void;
}

export const CoolerManualRgbPanel: React.FC<CoolerManualRgbPanelProps> = ({
  coolerData,
  onUpdateCooler,
  onUpdateCoolerLed,
}) => {
  useI18n();
  return (
    <>
      {/* 1. 手动目标精调栏 (手调后预设高亮全灭，进入自定义) */}
      <div
        className="cooler-manual-bar"
        style={{
          background: 'var(--surface-sunken)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-sm)',
          padding: '6px 12px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
          gap: '16px',
        }}
      >
        {/* 左侧：水泵电压手动挡 */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <span style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-muted)', whiteSpace: 'nowrap', display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
            <Zap size={11} className="text-cyan-400 cooler-pump-icon" /> 水泵档位：
          </span>
          <div style={{ display: 'flex', gap: '5px' }}>
            {[
              { tier: 0, label: trText('停泵'), desc: trText('完全关闭 (0%)') },
              { tier: 2, label: trText('标准'), desc: trText('标准巡航 (平稳水流 45%)') },
              { tier: 3, label: trText('狂暴'), desc: trText('极限狂暴 (100% 满血极速)') },
            ].map(({ tier, label, desc }) => {
              const currentTier = coolerData.pump_tier !== undefined
                ? coolerData.pump_tier
                : (coolerData.pump_voltage >= 11 ? 3 : (coolerData.pump_voltage === 0 ? 0 : 2));
              const isActive = currentTier === tier;
              return (
                <button
                  key={tier}
                  onClick={() => onUpdateCooler(tier, undefined)}
                  title={desc}
                  className={`cooler-pump-btn ${isActive ? 'active' : ''}`}
                  style={{
                    background: isActive ? 'var(--color-fan)' : 'var(--surface-pill)',
                    color: isActive ? '#000' : 'var(--text-secondary)',
                    border: `1px solid ${isActive ? 'var(--status-info)' : 'var(--border-subtle)'}`,
                    boxShadow: isActive ? '0 0 10px var(--status-info-dim)' : 'none',
                    borderRadius: 'var(--radius-xs)',
                    padding: '3px 9px',
                    fontSize: '10px',
                    fontWeight: 700,
                    cursor: 'pointer',
                    transition: 'all var(--transition-fast)',
                  }}
                >
                  {label}
                </button>
              );
            })}
          </div>
          <span className="cooler-pump-label" style={{ fontSize: '10px', color: 'var(--color-fan)', minWidth: '50px' }}>
            {coolerData.pump_tier === 3 || coolerData.pump_voltage >= 11
              ? trText('极速狂暴 (100%)')
              : (coolerData.pump_tier === 0 || coolerData.pump_voltage === 0
                ? trText('已停机 (0%)')
                : trText('标准巡航 (45%)'))}
          </span>
        </div>

        {/* 右侧：冷排风扇手动滑块 */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <span style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-muted)', whiteSpace: 'nowrap', display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
            <Fan size={11} className="text-purple-400 cooler-fan-icon" /> 冷排风扇目标转速 (Duty)：
          </span>
          <input
            type="range"
            min="0"
            max="100"
            step="5"
            value={coolerData.fan_duty}
            onChange={(e) => onUpdateCooler(undefined, Number(e.target.value))}
            className="cooler-fan-slider"
            style={{ width: '150px', accentColor: 'var(--color-brand)', cursor: 'pointer' }}
          />
          <span className="cooler-fan-pct" style={{ fontSize: '11px', fontWeight: 800, color: 'var(--color-brand)', minWidth: '40px', textAlign: 'right' }}>
            {coolerData.fan_duty}%
          </span>
        </div>
      </div>

      {/* 2. 水箱与冷排 RGB 灯效控制栏 */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <div
          className="cooler-rgb-card"
          style={{
            background: 'var(--surface-sunken)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-sm)',
            padding: '5px 10px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            flexShrink: 0,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Palette className="w-3.5 h-3.5 text-pink-400 cooler-rgb-palette-icon" />
            <span
              onClick={() => onUpdateCoolerLed(!coolerData.led_on)}
              style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-primary)', cursor: 'pointer', userSelect: 'none' }}
              title="点击开关水冷 RGB 灯效"
            >
              水箱与冷排 RGB 灯效：
            </span>
            <RevSwitch
              checked={coolerData.led_on}
              onChange={(checked) => onUpdateCoolerLed(checked)}
              size="sm"
            />
          </div>

          <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
            {[
              { id: 2, label: trText('炫彩流动 (默认)') },
              { id: 3, label: trText('七彩呼吸') },
              { id: 4, label: trText('旋转光斑') },
              { id: 1, label: trText('单色呼吸 (定色)') },
            ].map((m) => (
              <button
                key={m.id}
                onClick={() => {
                  const isAllZero = coolerData.led_r === 0 && coolerData.led_g === 0 && coolerData.led_b === 0;
                  const r = isAllZero ? 0 : coolerData.led_r;
                  const g = isAllZero ? 128 : coolerData.led_g;
                  const b = isAllZero ? 0 : coolerData.led_b;
                  onUpdateCoolerLed(true, r, g, b, m.id);
                }}
                className={`cooler-rgb-mode-btn ${coolerData.led_mode === m.id ? 'active' : ''}`}
                style={{
                  background: coolerData.led_mode === m.id ? 'rgba(236, 72, 153, 0.22)' : 'var(--surface-pill)',
                  border: coolerData.led_mode === m.id ? '1px solid #ec4899' : '1px solid var(--border-subtle)',
                  padding: '3px 8px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '9.5px',
                  color: coolerData.led_mode === m.id ? '#f472b6' : 'var(--text-muted)',
                  fontWeight: coolerData.led_mode === m.id ? 700 : 500,
                  cursor: 'pointer',
                  transition: 'all var(--transition-fast)',
                }}
              >
                {m.label}
              </button>
            ))}
          </div>
        </div>

        {/* Mode 1 单色呼吸定色调色盘 */}
        {coolerData.led_on && coolerData.led_mode === 1 && (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', background: 'var(--surface-sunken)', padding: '5px 10px', borderRadius: 'var(--radius-sm)' }}>
            <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
              <span style={{ fontSize: '9.5px', color: 'var(--text-dim)' }}>{trText('原厂色：')}</span>
              {COOLER_COLOR_PRESETS.map((c) => (
                <div
                  key={c.name}
                  onClick={() => onUpdateCoolerLed(true, c.r, c.g, c.b, 1)}
                  title={c.name}
                  style={{
                    width: '16px',
                    height: '16px',
                    borderRadius: 'var(--radius-xs)',
                    backgroundColor: c.hex,
                    cursor: 'pointer',
                    border: (coolerData.led_r === c.r && coolerData.led_g === c.g && coolerData.led_b === c.b)
                      ? '2px solid #fff'
                      : '1px solid rgba(0,0,0,0.3)',
                  }}
                />
              ))}
            </div>
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
              <span style={{ fontSize: '9.5px', color: 'var(--status-error)' }}>R</span>
              <input
                type="range" min="0" max="255" value={coolerData.led_r}
                onChange={(e) => onUpdateCoolerLed(true, Number(e.target.value), undefined, undefined, 1)}
                style={{ width: '45px', accentColor: 'var(--status-error)' }}
              />
              <span style={{ fontSize: '9.5px', color: 'var(--status-ok)' }}>G</span>
              <input
                type="range" min="0" max="255" value={coolerData.led_g}
                onChange={(e) => onUpdateCoolerLed(true, undefined, Number(e.target.value), undefined, 1)}
                style={{ width: '45px', accentColor: 'var(--status-ok)' }}
              />
              <span style={{ fontSize: '9.5px', color: 'var(--status-info)' }}>B</span>
              <input
                type="range" min="0" max="255" value={coolerData.led_b}
                onChange={(e) => onUpdateCoolerLed(true, undefined, undefined, Number(e.target.value), 1)}
                style={{ width: '45px', accentColor: 'var(--status-info)' }}
              />
            </div>
          </div>
        )}
      </div>
    </>
  );
};
