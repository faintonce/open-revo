import React, { useRef, useState, useEffect } from 'react';
import { Wind, Link2, Link2Off, Plus, Minus, Fan } from 'lucide-react';
import { FanCurvePoint } from '../../types/hardware';
import { RevSwitch } from '../ui/RevSwitch';

export interface FanCurveEditorProps {
  fanCurveTab: 'cpu' | 'gpu';
  onTabChange: (tab: 'cpu' | 'gpu') => void;
  cpuCurve: FanCurvePoint[];
  gpuCurve: FanCurvePoint[];
  onCurveChange: (type: 'cpu' | 'gpu', newCurve: FanCurvePoint[]) => void;
  fanIsolated: boolean;
  onToggleFanIsolated: (enabled: boolean) => void;
  fanRampRate: boolean;
  onToggleFanRampRate: (enabled: boolean) => void;
  fanRampSpeedMs: number;
  onChangeFanRampSpeed: (speedMs: number) => void;
  fanCurveLinked?: boolean;
  onToggleFanCurveLinked?: (linked: boolean) => void;
  fanBoost?: boolean;
  onToggleFanBoost?: () => void;
  fanBoostMode?: string;
  maxRpm?: number;
  fanIndependentChannels?: boolean;
  supportFanRampRate?: boolean;
  minDuty?: number;
}

export const FanCurveEditor: React.FC<FanCurveEditorProps> = ({
  fanCurveTab,
  onTabChange,
  cpuCurve,
  gpuCurve,
  onCurveChange,
  fanIsolated,
  onToggleFanIsolated,
  fanRampRate,
  onToggleFanRampRate,
  fanRampSpeedMs,
  onChangeFanRampSpeed,
  fanCurveLinked = false,
  onToggleFanCurveLinked,
  fanBoost = false,
  onToggleFanBoost,
  fanBoostMode,
  maxRpm = 5800,
  fanIndependentChannels = true,
  supportFanRampRate = true,
  minDuty: propMinDuty,
}) => {
  const minDuty = propMinDuty ?? (fanBoostMode === 'direct_rpm_clamp' ? 22 : 0);
  const [draggingIdx, setDraggingIdx] = useState<number | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const repeatTimerRef = useRef<{ timeout?: any; interval?: any }>({});

  // Fan curve chart dimensions (expanded by ~50% vertically for generous, precise dragging)
  const SVG_W = 320;
  const SVG_H = 285;
  const PAD_L = 32;
  const PAD_R = 14;
  const PAD_T = 20;
  const PAD_B = 26;
  const PLOT_W = SVG_W - PAD_L - PAD_R; // 274
  const PLOT_H = SVG_H - PAD_T - PAD_B; // 239

  const getSvgX = (temp: number) => {
    const clamped = Math.max(30, Math.min(100, temp));
    return PAD_L + ((clamped - 30) / 70) * PLOT_W;
  };

  const getSvgY = (duty: number) => {
    const clamped = Math.max(0, Math.min(100, duty));
    return PAD_T + (1 - clamped / 100) * PLOT_H;
  };

  // Fan curve duty change with strict Monotonic Non-Decreasing constraint
  const handleCurveDutyChange = (type: 'cpu' | 'gpu', index: number, duty: number) => {
    const baseCurve = type === 'cpu' ? cpuCurve : gpuCurve;
    const newCurve = [...baseCurve];
    if (newCurve[index]) {
      const targetDuty = Math.max(minDuty, Math.min(100, Math.round(duty)));
      newCurve[index] = { ...newCurve[index], duty: targetDuty };

      // Monotonic Rule 1: Points after current index cannot have lower duty (push higher)
      for (let j = index + 1; j < newCurve.length; j++) {
        if (newCurve[j].duty < targetDuty) {
          newCurve[j] = { ...newCurve[j], duty: targetDuty };
        }
      }

      // Monotonic Rule 2: Points before current index cannot have higher duty (pull lower), but floor is minDuty
      for (let j = 0; j < index; j++) {
        if (newCurve[j].duty > targetDuty) {
          newCurve[j] = { ...newCurve[j], duty: Math.max(minDuty, targetDuty) };
        }
      }

      onCurveChange(type, newCurve);
    }
  };

  // Fine-tuning 1% step handler with continuous long-press repeat
  const stopRepeat = () => {
    if (repeatTimerRef.current.timeout) clearTimeout(repeatTimerRef.current.timeout);
    if (repeatTimerRef.current.interval) clearInterval(repeatTimerRef.current.interval);
    repeatTimerRef.current = {};
  };

  const isEditingAllowed = fanIndependentChannels ? fanIsolated : true;

  const handleStepDuty = (index: number, delta: number) => {
    if (!isEditingAllowed) return;
    const baseCurve = fanCurveTab === 'cpu' ? cpuCurve : gpuCurve;
    const curVal = baseCurve[index]?.duty ?? minDuty;
    const nextVal = Math.max(minDuty, Math.min(100, curVal + delta));
    handleCurveDutyChange(fanCurveTab, index, nextVal);
    if (!fanIndependentChannels) {
      handleCurveDutyChange(fanCurveTab === 'cpu' ? 'gpu' : 'cpu', index, nextVal);
    }
  };

  const startRepeat = (index: number, delta: number) => {
    if (!isEditingAllowed) return;
    stopRepeat();
    handleStepDuty(index, delta);
    repeatTimerRef.current.timeout = setTimeout(() => {
      repeatTimerRef.current.interval = setInterval(() => {
        handleStepDuty(index, delta);
      }, 50);
    }, 260);
  };

  useEffect(() => {
    return () => stopRepeat();
  }, []);

  const handleSvgPointerMove = (e: React.PointerEvent<SVGSVGElement>) => {
    if (draggingIdx === null || !isEditingAllowed || !svgRef.current) return;
    const rect = svgRef.current.getBoundingClientRect();
    const clientY = e.clientY - rect.top;
    const scaleY = SVG_H / rect.height;
    const svgY = clientY * scaleY;
    const rawDuty = Math.round(100 * (1 - (svgY - PAD_T) / PLOT_H));
    // Precise 1% continuous resolution, strictly bounded in [minDuty, 100]
    const clampedDuty = Math.max(minDuty, Math.min(100, rawDuty));
    handleCurveDutyChange(fanCurveTab, draggingIdx, clampedDuty);
    if (!fanIndependentChannels) {
      handleCurveDutyChange(fanCurveTab === 'cpu' ? 'gpu' : 'cpu', draggingIdx, clampedDuty);
    }
  };

  const pts = fanCurveTab === 'cpu' ? cpuCurve : gpuCurve;
  const mappedPts = pts.map((p) => ({
    x: getSvgX(p.temp),
    y: getSvgY(p.duty),
    temp: p.temp,
    duty: p.duty,
  }));

  const startX = PAD_L;
  const endX = PAD_L + PLOT_W;
  const floorY = PAD_T + PLOT_H;
  const startY = mappedPts.length > 0 ? mappedPts[0].y : getSvgY(minDuty > 0 ? minDuty : 20);
  const endY = mappedPts.length > 0 ? mappedPts[mappedPts.length - 1].y : getSvgY(100);

  const pointsStr = mappedPts.map((p) => `${p.x},${p.y}`).join(' L ');
  const linePath = mappedPts.length > 0
    ? `M ${startX},${startY} L ${pointsStr} L ${endX},${endY}`
    : '';
  const areaPath = mappedPts.length > 0
    ? `M ${startX},${floorY} L ${startX},${startY} L ${pointsStr} L ${endX},${endY} L ${endX},${floorY} Z`
    : '';

  return (
    <div className="tuning-card" style={{ height: '100%' }}>
      {/* Title */}
      <div className="tuning-header">
        <div className="tuning-header-title" style={{ color: 'var(--color-brand)' }}>
          <Wind className="w-4 h-4" />
          <span>风扇策略与曲线</span>
        </div>
      </div>

      {/* Single-row Physical Fan Boost Switch for BLD Direct Clamp only */}
      {fanBoostMode === 'direct_rpm_clamp' && onToggleFanBoost && (
        <div
          className="tuning-sub-card"
          style={{
            flexDirection: 'row',
            justifyContent: 'space-between',
            alignItems: 'center',
            padding: '6px 10px',
            background: fanBoost ? 'var(--status-info-dim)' : undefined,
            border: fanBoost ? '1px solid var(--status-info)' : undefined,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Fan className={`w-3.5 h-3.5 ${fanBoost ? 'animate-spin' : ''}`} style={{ color: fanBoost ? 'var(--status-info)' : 'var(--text-muted)' }} />
            <span style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '11px', whiteSpace: 'nowrap' }}>
              硬件物理强锁 ({maxRpm} RPM)
            </span>
          </div>
          <RevSwitch
            checked={fanBoost}
            onChange={() => onToggleFanBoost()}
            size="sm"
            activeColor="var(--status-info)"
          />
        </div>
      )}

      {/* Fan Ramp Rate (EC hardware slope ramp, Tongfang only) */}
      {supportFanRampRate && (
        <div
          className="tuning-sub-card"
          style={{
            flexDirection: 'row',
            justifyContent: 'space-between',
            alignItems: 'center',
            padding: '7px 10px',
          }}
        >
          <div>
            <div style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '11px', whiteSpace: 'nowrap', flexShrink: 0 }}>平滑过渡</div>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <select
              disabled={!fanRampRate}
              value={fanRampSpeedMs}
              onChange={(e) => onChangeFanRampSpeed(parseInt(e.target.value, 10))}
              className="rev-select"
              style={{
                padding: '2px 8px',
                color: fanRampRate ? 'var(--color-brand)' : 'var(--text-dim)',
                cursor: fanRampRate ? 'pointer' : 'not-allowed',
                minWidth: '68px',
                textAlign: 'center',
              }}
            >
              <option value="100">100ms</option>
              <option value="300">300ms</option>
              <option value="500">500ms</option>
              <option value="1000">1000ms</option>
            </select>
            <RevSwitch
              checked={fanRampRate}
              onChange={onToggleFanRampRate}
              size="sm"
              activeColor="var(--color-brand)"
            />
          </div>
        </div>
      )}

      {/* Fan Isolated Output (Only for models with independent fan hardware channels) */}
      {fanIndependentChannels && (
        <div
          className="tuning-sub-card"
          style={{
            flexDirection: 'row',
            justifyContent: 'space-between',
            alignItems: 'center',
            padding: '7px 10px',
          }}
        >
          <div>
            <div style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '11px', whiteSpace: 'nowrap', flexShrink: 0 }}>风扇独立输出</div>
          </div>
          <RevSwitch
            checked={fanIsolated}
            onChange={onToggleFanIsolated}
            size="sm"
            activeColor="var(--color-brand)"
          />
        </div>
      )}

      {/* Custom Fan Curve Tab & Points */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '8px',
          flex: 1,
          marginTop: '2px',
          borderTop: '1px solid var(--border-subtle)',
          paddingTop: '8px',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          {fanIndependentChannels ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
              <button
                type="button"
                onClick={() => onTabChange('cpu')}
                style={{
                  padding: '2px 8px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '10px',
                  fontWeight: 600,
                  border: 'none',
                  cursor: 'pointer',
                  background: fanCurveTab === 'cpu' ? 'rgba(99, 102, 241, 0.35)' : 'transparent',
                  color: fanCurveTab === 'cpu' ? 'var(--text-primary)' : 'var(--text-dim)',
                  transition: 'all var(--transition-fast)',
                }}
              >
                CPU风扇
              </button>

              {/* Sync Link Chain Button */}
              <button
                type="button"
                onClick={() => onToggleFanCurveLinked && onToggleFanCurveLinked(!fanCurveLinked)}
                style={{
                  padding: '2px 5px',
                  borderRadius: 'var(--radius-xs)',
                  border: fanCurveLinked ? '1px solid var(--border-glow)' : '1px solid var(--border-subtle)',
                  background: fanCurveLinked ? 'rgba(99, 102, 241, 0.25)' : 'var(--surface-badge)',
                  color: fanCurveLinked ? 'var(--color-brand)' : 'var(--text-dim)',
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  transition: 'all var(--transition-fast)',
                }}
                title={fanCurveLinked ? '双风扇曲线已锁定联动（调节任意一侧将实时镜像同步）' : '双风扇独立调节（点击开启联动同步）'}
              >
                {fanCurveLinked ? <Link2 className="w-3 h-3" /> : <Link2Off className="w-3 h-3 opacity-60" />}
              </button>

              <button
                type="button"
                onClick={() => onTabChange('gpu')}
                style={{
                  padding: '2px 8px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '10px',
                  fontWeight: 600,
                  border: 'none',
                  cursor: 'pointer',
                  background: fanCurveTab === 'gpu' ? 'rgba(99, 102, 241, 0.35)' : 'transparent',
                  color: fanCurveTab === 'gpu' ? 'var(--text-primary)' : 'var(--text-dim)',
                  transition: 'all var(--transition-fast)',
                }}
              >
                GPU风扇
              </button>
            </div>
          ) : (
            <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
              <Link2 className="w-3.5 h-3.5" style={{ color: 'var(--status-info)' }} />
              <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap' }}>
                双风扇联动
              </span>
            </div>
          )}

          <span style={{ fontSize: '10px', color: isEditingAllowed ? 'var(--color-brand)' : 'var(--text-dim)', fontWeight: 600, whiteSpace: 'nowrap' }}>
            {isEditingAllowed ? '● 8点曲线调校' : '○ 出厂预设托管'}
          </span>
        </div>

        {/* Interactive SVG Fan Curve Chart (Taller by 50% for generous precision) */}
        <div
          style={{
            background: 'var(--surface-sunken)',
            borderRadius: 'var(--radius-md)',
            border: '1px solid var(--border-subtle)',
            padding: '6px 8px',
            userSelect: 'none',
            touchAction: 'none',
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'center',
          }}
        >
          <svg
            ref={svgRef}
            viewBox={`0 0 ${SVG_W} ${SVG_H}`}
            style={{ width: '100%', height: '280px', display: 'block', overflow: 'visible' }}
            onPointerMove={handleSvgPointerMove}
            onPointerUp={() => setDraggingIdx(null)}
            onPointerLeave={() => setDraggingIdx(null)}
          >
            <defs>
              <linearGradient id="fanCurveGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={isEditingAllowed ? '#6366f1' : '#64748b'} stopOpacity="0.45" />
                <stop offset="100%" stopColor={isEditingAllowed ? '#6366f1' : '#64748b'} stopOpacity="0.02" />
              </linearGradient>
              <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
                <feDropShadow dx="0" dy="0" stdDeviation="3.5" floodColor="#6366f1" floodOpacity="0.6" />
              </filter>
            </defs>

            {/* Horizontal Grid & Labels */}
            <line x1={PAD_L} y1={PAD_T} x2={endX} y2={PAD_T} stroke="rgba(255,255,255,0.08)" strokeDasharray="3 3" />
            <text x={PAD_L - 4} y={PAD_T + 3} textAnchor="end" fontSize="9" fill="var(--text-dim)">100%</text>

            <line x1={PAD_L} y1={PAD_T + PLOT_H * 0.25} x2={endX} y2={PAD_T + PLOT_H * 0.25} stroke="rgba(255,255,255,0.05)" strokeDasharray="2 2" />
            <text x={PAD_L - 4} y={PAD_T + PLOT_H * 0.25 + 3} textAnchor="end" fontSize="8.5" fill="var(--text-dim)">75%</text>

            <line x1={PAD_L} y1={PAD_T + PLOT_H * 0.5} x2={endX} y2={PAD_T + PLOT_H * 0.5} stroke="rgba(255,255,255,0.08)" strokeDasharray="3 3" />
            <text x={PAD_L - 4} y={PAD_T + PLOT_H * 0.5 + 3} textAnchor="end" fontSize="9" fill="var(--text-dim)">50%</text>

            <line x1={PAD_L} y1={PAD_T + PLOT_H * 0.75} x2={endX} y2={PAD_T + PLOT_H * 0.75} stroke="rgba(255,255,255,0.05)" strokeDasharray="2 2" />
            <text x={PAD_L - 4} y={PAD_T + PLOT_H * 0.75 + 3} textAnchor="end" fontSize="8.5" fill="var(--text-dim)">25%</text>

            <line x1={PAD_L} y1={floorY} x2={endX} y2={floorY} stroke="rgba(255,255,255,0.15)" />
            <text x={PAD_L - 4} y={floorY + 3} textAnchor="end" fontSize="9" fill="var(--text-dim)">0%</text>

            {/* Vertical Guide Lines & Temperature X Axis Labels for each node */}
            {mappedPts.map((pt, idx) => (
              <g key={`guide-${idx}`}>
                <line
                  x1={pt.x}
                  y1={PAD_T}
                  x2={pt.x}
                  y2={floorY}
                  stroke={draggingIdx === idx ? 'rgba(99, 102, 241, 0.4)' : 'rgba(255,255,255,0.04)'}
                  strokeDasharray="2 3"
                />
                <text
                  x={pt.x}
                  y={floorY + 14}
                  textAnchor="middle"
                  fontSize="8.5"
                  fontWeight={draggingIdx === idx ? '700' : '500'}
                  fill={draggingIdx === idx ? 'var(--color-brand)' : 'var(--text-dim)'}
                >
                  {pt.temp}°
                </text>
              </g>
            ))}

            {/* Area Fill */}
            <path d={areaPath} fill="url(#fanCurveGrad)" />

            {/* Curve Stroke */}
            <path
              d={linePath}
              fill="none"
              stroke={isEditingAllowed ? '#6366f1' : '#64748b'}
              strokeWidth="2.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              filter={isEditingAllowed ? 'url(#glow)' : undefined}
            />

            {/* 8 Anchor Points with direct drag, wheel scroll, & hover support */}
            {mappedPts.map((pt, idx) => {
              const isDragging = draggingIdx === idx;
              return (
                <g
                  key={idx}
                  style={{ cursor: isEditingAllowed ? 'ns-resize' : 'default' }}
                  onPointerDown={(e) => {
                    if (!isEditingAllowed) return;
                    (e.target as Element).setPointerCapture(e.pointerId);
                    setDraggingIdx(idx);
                  }}
                  onWheel={(e) => {
                    if (!isEditingAllowed) return;
                    e.preventDefault();
                    handleStepDuty(idx, e.deltaY < 0 ? 1 : -1);
                  }}
                >
                  {/* Invisible enlarged hit target */}
                  <circle cx={pt.x} cy={pt.y} r={18} fill="transparent" />
                  {/* Outer pulse circle */}
                  <circle
                    cx={pt.x}
                    cy={pt.y}
                    r={isDragging ? 12 : 7.5}
                    fill={isEditingAllowed ? (isDragging ? 'rgba(244, 114, 182, 0.4)' : 'rgba(99, 102, 241, 0.3)') : 'rgba(100, 116, 139, 0.2)'}
                    stroke={isEditingAllowed ? (isDragging ? '#f472b6' : '#6366f1') : '#64748b'}
                    strokeWidth={isDragging ? 1.5 : 1}
                  />
                  {/* Core circle */}
                  <circle
                    cx={pt.x}
                    cy={pt.y}
                    r={isDragging ? 6.5 : 4.5}
                    fill={isEditingAllowed ? (isDragging ? '#f472b6' : '#e0e7ff') : '#94a3b8'}
                    stroke={isEditingAllowed ? '#4338ca' : '#334155'}
                    strokeWidth="1.5"
                  />
                  {/* Duty Percentage Text Tag */}
                  <text
                    x={pt.x}
                    y={pt.y - (isDragging ? 14 : 10)}
                    textAnchor="middle"
                    fontSize="9.5"
                    fontWeight="800"
                    fill={fanIsolated ? (isDragging ? '#f472b6' : '#e0e7ff') : 'var(--text-dim)'}
                  >
                    {pt.duty}%
                  </text>
                </g>
              );
            })}
          </svg>
        </div>

        {/* 8-Node Fine-Tuning Grid (1% Step Controls with Fast Long-Press Repeat) */}
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(8, 1fr)',
            gap: '3px',
            background: 'var(--surface-badge)',
            borderRadius: 'var(--radius-sm)',
            padding: '5px 3px',
            border: '1px solid var(--border-subtle)',
            marginTop: '2px',
            userSelect: 'none',
          }}
        >
          {pts.map((pt, idx) => (
            <div
              key={idx}
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '2px',
              }}
            >
              {/* [+] Step Button */}
              <button
                type="button"
                disabled={!isEditingAllowed || pt.duty >= 100}
                onPointerDown={(e) => {
                  e.preventDefault();
                  startRepeat(idx, 1);
                }}
                onPointerUp={stopRepeat}
                onPointerLeave={stopRepeat}
                style={{
                  width: '100%',
                  maxWidth: '28px',
                  height: '18px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: 0,
                  background: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid var(--border-subtle)',
                  borderRadius: 'var(--radius-xs)',
                  color: isEditingAllowed ? 'var(--text-primary)' : 'var(--text-dim)',
                  cursor: isEditingAllowed ? 'pointer' : 'not-allowed',
                  transition: 'background var(--transition-fast)',
                }}
                title={`增加 1% (当前 ${pt.temp}°C: ${pt.duty}%)`}
              >
                <Plus className="w-2.5 h-2.5" />
              </button>

              {/* Duty Percentage */}
              <div
                style={{
                  fontSize: '9.5px',
                  fontWeight: 700,
                  color: isEditingAllowed ? 'var(--color-brand)' : 'var(--text-dim)',
                  lineHeight: '1.2',
                }}
              >
                {pt.duty}%
              </div>

              {/* [-] Step Button */}
              <button
                type="button"
                disabled={!isEditingAllowed || pt.duty <= minDuty}
                onPointerDown={(e) => {
                  e.preventDefault();
                  startRepeat(idx, -1);
                }}
                onPointerUp={stopRepeat}
                onPointerLeave={stopRepeat}
                style={{
                  width: '100%',
                  maxWidth: '28px',
                  height: '18px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: 0,
                  background: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid var(--border-subtle)',
                  borderRadius: 'var(--radius-xs)',
                  color: isEditingAllowed && pt.duty > minDuty ? 'var(--text-primary)' : 'var(--text-dim)',
                  cursor: isEditingAllowed && pt.duty > minDuty ? 'pointer' : 'not-allowed',
                  transition: 'background var(--transition-fast)',
                }}
                title={pt.duty <= minDuty ? `已达硬件起旋最低限制 (${minDuty}%)` : `减少 1% (当前 ${pt.temp}°C: ${pt.duty}%)`}
              >
                <Minus className="w-2.5 h-2.5" />
              </button>

              {/* Node Temperature Tag */}
              <div style={{ fontSize: '8.5px', color: 'var(--text-dim)', marginTop: '1px' }}>
                {pt.temp}°
              </div>
            </div>
          ))}
        </div>

        {/* Compact Bottom Instruction Tag */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            fontSize: '9.5px',
            color: 'var(--text-dim)',
            padding: '2px 4px',
          }}
        >
          <span>{minDuty > 0 ? `最低起旋 ${minDuty}% (2200 RPM) · 滚轮微调` : '鼠标悬停滚轮微调'}</span>
          <span style={{ color: fanIsolated ? 'var(--color-brand)' : 'var(--text-dim)' }}>
            8 节点单调递增
          </span>
        </div>
      </div>
    </div>
  );
};
