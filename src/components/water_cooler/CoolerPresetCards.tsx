import React from 'react';
import { Sparkles, ShieldCheck, Zap } from 'lucide-react';
import { trText, useI18n } from '../../i18n/I18nContext';

interface CoolerPresetCardsProps {
  strategy?: string;
  liveMaxTemp: number;
  liveInterpolated: { duty: number; volt: number };
  onUpdateStrategy: (strategy: string) => void;
}

export const CoolerPresetCards: React.FC<CoolerPresetCardsProps> = ({
  strategy,
  liveMaxTemp,
  liveInterpolated,
  onUpdateStrategy,
}) => {
  useI18n();
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', flexShrink: 0 }}>
      <div style={{ fontSize: '10.5px', fontWeight: 700, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span>{trText('散热预设策略与运行模式')}</span>
        <span style={{ fontSize: '10px', color: 'var(--text-dim)', fontWeight: 500, display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
          {strategy === 'custom' ? (
            <>
              <Sparkles size={11} style={{ color: 'var(--status-warn)' }} /> 当前处于手动自定义调节状态（预设全灭）
            </>
          ) : (
            trText('选择【智慧自适应】自动随动；手调水泵或风扇后自动转为自定义')
          )}
        </span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1.35fr 1fr 1fr 1fr', gap: '7px' }}>
        {/* 智慧自适应 */}
        <div
          onClick={() => onUpdateStrategy('smart')}
          className={`cooler-strat-card cooler-strat-smart ${strategy === 'smart' ? 'active' : ''}`}
          style={{
            background: strategy === 'smart' ? 'var(--status-info-dim)' : 'var(--surface-badge)',
            border: strategy === 'smart' ? '1px solid var(--status-info)' : '1px solid var(--border-subtle)',
            boxShadow: strategy === 'smart' ? '0 0 16px rgba(56, 189, 248, 0.25)' : 'none',
            borderRadius: 'var(--radius-sm)',
            padding: '6px 9px',
            cursor: 'pointer',
            display: 'flex',
            flexDirection: 'column',
            gap: '2px',
            transition: 'all var(--transition-fast)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span className="cooler-strat-title" style={{ fontSize: '11px', fontWeight: 800, color: 'var(--status-info)', display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
              <ShieldCheck size={13} /> 智慧自适应
            </span>
            <span
              className="cooler-strat-badge"
              style={{
                fontSize: '8px',
                fontWeight: 800,
                padding: '1px 4px',
                borderRadius: 'var(--radius-xs)',
                background: 'var(--status-info-dim)',
                color: 'var(--status-info)',
              }}
            >
              温控随动
            </span>
          </div>
          <div style={{ fontSize: '9.5px', color: 'var(--text-muted)', lineHeight: 1.3 }}>
            依据核心温度动态调控，<strong style={{ color: 'var(--status-ok)' }}>{trText('&lt;55°C 风扇停转')}</strong>
          </div>
          <div className="cooler-strat-sub" style={{ fontSize: '9px', fontWeight: 700, color: 'var(--status-info)', display: 'inline-flex', alignItems: 'center', gap: '3px' }}>
            <Zap size={10} /> 随动: {liveMaxTemp}°C → 风扇 {liveInterpolated.duty}% | 水泵 {
              liveInterpolated.volt >= 11 || liveInterpolated.volt === 3
                ? trText('极限狂暴')
                : liveInterpolated.volt === 0
                ? trText('停机')
                : trText('标准巡航')
            }
          </div>
        </div>

        {/* 静音策略 */}
        <div
          onClick={() => onUpdateStrategy('quiet')}
          className={`cooler-strat-card cooler-strat-quiet ${strategy === 'quiet' ? 'active' : ''}`}
          style={{
            background: strategy === 'quiet' ? 'var(--status-info-dim)' : 'var(--surface-badge)',
            border: strategy === 'quiet' ? '1px solid var(--status-info)' : '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-sm)',
            padding: '6px 9px',
            cursor: 'pointer',
            display: 'flex',
            flexDirection: 'column',
            gap: '2px',
            transition: 'all var(--transition-fast)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span className="cooler-strat-title" style={{ fontSize: '11px', fontWeight: 800, color: 'var(--status-info)' }}>{trText('静音策略')}</span>
            <span className="cooler-strat-badge" style={{ fontSize: '8px', fontWeight: 800, padding: '1px 4px', borderRadius: 'var(--radius-xs)', background: 'var(--surface-pill)', color: 'var(--text-muted)' }}>{trText('固定')}</span>
          </div>
          <div style={{ fontSize: '9.5px', color: 'var(--text-muted)', lineHeight: 1.3 }}>{trText('标准巡航水泵 + 40% 风扇')}</div>
          <div style={{ fontSize: '9px', color: 'var(--text-dim)', marginTop: '1px' }}>{trText('适合安静夜间使用')}</div>
        </div>

        {/* 均衡策略 */}
        <div
          onClick={() => onUpdateStrategy('balance')}
          className={`cooler-strat-card cooler-strat-balance ${strategy === 'balance' ? 'active' : ''}`}
          style={{
            background: strategy === 'balance' ? 'var(--border-glow)' : 'var(--surface-badge)',
            border: strategy === 'balance' ? '1px solid var(--color-brand)' : '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-sm)',
            padding: '6px 9px',
            cursor: 'pointer',
            display: 'flex',
            flexDirection: 'column',
            gap: '2px',
            transition: 'all var(--transition-fast)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span className="cooler-strat-title" style={{ fontSize: '11px', fontWeight: 800, color: 'var(--color-brand)' }}>{trText('均衡策略')}</span>
            <span className="cooler-strat-badge" style={{ fontSize: '8px', fontWeight: 800, padding: '1px 4px', borderRadius: 'var(--radius-xs)', background: 'var(--surface-pill)', color: 'var(--text-muted)' }}>{trText('固定')}</span>
          </div>
          <div style={{ fontSize: '9.5px', color: 'var(--text-muted)', lineHeight: 1.3 }}>{trText('标准巡航水泵 + 65% 风扇')}</div>
          <div style={{ fontSize: '9px', color: 'var(--text-dim)', marginTop: '1px' }}>{trText('兼顾散热与风噪')}</div>
        </div>

        {/* 极速狂暴 */}
        <div
          onClick={() => onUpdateStrategy('performance')}
          className={`cooler-strat-card cooler-strat-perf ${strategy === 'performance' ? 'active' : ''}`}
          style={{
            background: strategy === 'performance' ? 'var(--status-error-dim)' : 'var(--surface-badge)',
            border: strategy === 'performance' ? '1px solid var(--status-error)' : '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-sm)',
            padding: '6px 9px',
            cursor: 'pointer',
            display: 'flex',
            flexDirection: 'column',
            gap: '2px',
            transition: 'all var(--transition-fast)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span className="cooler-strat-title" style={{ fontSize: '11px', fontWeight: 800, color: 'var(--status-error)' }}>{trText('极速狂暴')}</span>
            <span className="cooler-strat-badge" style={{ fontSize: '8px', fontWeight: 800, padding: '1px 4px', borderRadius: 'var(--radius-xs)', background: 'var(--status-error-dim)', color: 'var(--status-error)' }}>{trText('满血')}</span>
          </div>
          <div style={{ fontSize: '9.5px', color: 'var(--text-muted)', lineHeight: 1.3 }}>{trText('极限狂暴水泵 + 100% 风扇')}</div>
          <div style={{ fontSize: '9px', color: 'var(--text-dim)', marginTop: '1px' }}>{trText('极限压榨散热性能')}</div>
        </div>
      </div>
    </div>
  );
};
