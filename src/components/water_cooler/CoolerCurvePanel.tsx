import React, { useState, useRef, useEffect } from 'react';
import {
  TrendingUp,
  Star,
  VolumeX,
  Flame,
  Zap,
} from 'lucide-react';
import { CoolerCurvePoint, DEFAULT_COOLER_CURVE } from './coolerTypes';
import { trText, useI18n } from '../../i18n/I18nContext';

interface CoolerCurvePanelProps {
  strategy?: string;
  activeCurve: CoolerCurvePoint[];
  liveMaxTemp: number;
  liveInterpolated: { duty: number; volt: number };
  curveTemplate: 'standard' | 'quiet' | 'aggressive' | 'linear';
  pumpStartTemp?: number;
  pumpTurboTemp?: number;
  onApplyTemplate: (type: 'standard' | 'quiet' | 'aggressive' | 'linear') => void;
  onCurveChange: (newCurve: CoolerCurvePoint[]) => void;
  onPumpThresholdsChange?: (startTemp: number, turboTemp: number, committed: boolean) => void;
}

export const CoolerCurvePanel: React.FC<CoolerCurvePanelProps> = ({
  strategy,
  activeCurve,
  liveMaxTemp,
  liveInterpolated,
  curveTemplate,
  pumpStartTemp,
  pumpTurboTemp,
  onApplyTemplate,
  onCurveChange,
  onPumpThresholdsChange,
}) => {
  useI18n();
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState<number>(910);
  const [localPumpStart, setLocalPumpStart] = useState<number>(pumpStartTemp ?? 55);
  const [localPumpTurbo, setLocalPumpTurbo] = useState<number>(pumpTurboTemp ?? 90);
  const [activeDragPump, setActiveDragPump] = useState<'start' | 'turbo' | null>(null);

  // Floating coordinates for subpixel fluid dragging
  const [draggingFloatStart, setDraggingFloatStart] = useState<number | null>(null);
  const [draggingFloatTurbo, setDraggingFloatTurbo] = useState<number | null>(null);
  const isDraggingRef = useRef(false);

  useEffect(() => {
    if (!isDraggingRef.current && pumpStartTemp !== undefined) {
      setLocalPumpStart(pumpStartTemp);
    }
  }, [pumpStartTemp]);

  useEffect(() => {
    if (!isDraggingRef.current && pumpTurboTemp !== undefined) {
      setLocalPumpTurbo(pumpTurboTemp);
    }
  }, [pumpTurboTemp]);

  useEffect(() => {
    if (!containerRef.current) return;
    const updateWidth = () => {
      if (containerRef.current) {
        const w = containerRef.current.clientWidth;
        if (w > 100) {
          setContainerWidth(w);
        }
      }
    };
    updateWidth();
    const ro = new ResizeObserver(() => updateWidth());
    ro.observe(containerRef.current);
    return () => ro.disconnect();
  }, []);

  const startX = 45;
  const endX = Math.max(startX + 100, containerWidth - 35);
  const plotW = endX - startX;

  const tempToX = (t: number) => startX + (t - 30) * (plotW / 70);
  const dutyToY = (d: number) => 120 - d * 1.05;
  const xToTemp = (x: number) => Math.round(30 + (x - startX) * (70 / plotW));
  const yToDuty = (y: number) => Math.round((120 - y) / 1.05);

  const effectiveStart = draggingFloatStart ?? localPumpStart;
  const effectiveTurbo = draggingFloatTurbo ?? localPumpTurbo;

  const xStart = tempToX(effectiveStart);
  const xTurbo = tempToX(effectiveTurbo);

  const displayStart = Math.round(effectiveStart);
  const displayTurbo = Math.round(effectiveTurbo);

  const sortedPts = [...activeCurve].sort((a, b) => a.temp - b.temp);
  let curvePathD = `M ${tempToX(30)} ${dutyToY(0)} `;
  sortedPts.forEach((p) => {
    curvePathD += `L ${tempToX(p.temp)} ${dutyToY(p.fan_duty)} `;
  });
  curvePathD += `L ${tempToX(100)} ${dutyToY(sortedPts[sortedPts.length - 1].fan_duty)} `;
  const curveAreaD = curvePathD + `L ${tempToX(100)} 120 L ${tempToX(30)} 120 Z`;

  const probeX = tempToX(liveMaxTemp);
  const probeY = dutyToY(liveInterpolated.duty);

  const startDragHandle = (e: React.MouseEvent, idx: number) => {
    e.preventDefault();
    e.stopPropagation();
    const svg = (e.currentTarget as any).ownerSVGElement as SVGSVGElement | null;
    if (!svg) return;
    const rect = svg.getBoundingClientRect();

    let currentCurve = [...activeCurve];

    const handleMouseMove = (moveEvt: MouseEvent) => {
      const scaleX = containerWidth / rect.width;
      const scaleY = 140 / rect.height;
      const svgX = (moveEvt.clientX - rect.left) * scaleX;
      const svgY = (moveEvt.clientY - rect.top) * scaleY;

      let newTemp = Math.max(30, Math.min(100, xToTemp(svgX)));
      let newDuty = Math.max(0, Math.min(100, yToDuty(svgY)));

      const pts = currentCurve.length ? [...currentCurve] : [...DEFAULT_COOLER_CURVE];
      if (idx > 0) newTemp = Math.max(pts[idx - 1].temp + 2, newTemp);
      if (idx < pts.length - 1) newTemp = Math.min(pts[idx + 1].temp - 2, newTemp);
      pts[idx] = { ...pts[idx], temp: newTemp, fan_duty: newDuty };
      currentCurve = pts;
      onCurveChange(pts);
    };

    const handleMouseUp = () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      onCurveChange(currentCurve);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  const startDragPump = (e: React.MouseEvent, type: 'start' | 'turbo') => {
    e.preventDefault();
    e.stopPropagation();
    const svg = (e.currentTarget as any).ownerSVGElement as SVGSVGElement | null;
    if (!svg) return;
    const rect = svg.getBoundingClientRect();

    isDraggingRef.current = true;
    setActiveDragPump(type);
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';

    const scaleX = containerWidth / rect.width;
    const initialMouseSvgX = (e.clientX - rect.left) * scaleX;
    const initialHandleSvgX = tempToX(type === 'start' ? effectiveStart : effectiveTurbo);
    // Relative grab offset prevents any jumping on click
    const grabOffsetSvgX = initialMouseSvgX - initialHandleSvgX;

    let curIntStart = localPumpStart;
    let curIntTurbo = localPumpTurbo;

    let latestSvgX = initialMouseSvgX;
    let rafId: number | null = null;

    const handleMouseMove = (moveEvt: MouseEvent) => {
      latestSvgX = (moveEvt.clientX - rect.left) * scaleX;
      if (rafId === null) {
        rafId = requestAnimationFrame(() => {
          rafId = null;
          const targetHandleSvgX = latestSvgX - grabOffsetSvgX;
          const floatTemp = 30 + (targetHandleSvgX - startX) * (70 / plotW);

          if (type === 'start') {
            const clamped = Math.max(35, Math.min(localPumpTurbo - 5, floatTemp));
            curIntStart = Math.round(clamped);
            setDraggingFloatStart(clamped);
            setLocalPumpStart(curIntStart);
          } else {
            const clamped = Math.max(localPumpStart + 5, Math.min(98, floatTemp));
            curIntTurbo = Math.round(clamped);
            setDraggingFloatTurbo(clamped);
            setLocalPumpTurbo(curIntTurbo);
          }
        });
      }
    };

    const handleMouseUp = () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      if (rafId !== null) {
        cancelAnimationFrame(rafId);
        rafId = null;
      }

      setActiveDragPump(null);
      setDraggingFloatStart(null);
      setDraggingFloatTurbo(null);
      isDraggingRef.current = false;

      const finalStart = type === 'start' ? curIntStart : localPumpStart;
      const finalTurbo = type === 'turbo' ? curIntTurbo : localPumpTurbo;

      setLocalPumpStart(finalStart);
      setLocalPumpTurbo(finalTurbo);
      onPumpThresholdsChange?.(finalStart, finalTurbo, true);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  return (
    <div
      style={{
        background: 'var(--surface-sunken)',
        border: '1px solid var(--border-subtle)',
        borderRadius: 'var(--radius-md)',
        padding: '7px 10px',
        display: 'flex',
        flexDirection: 'column',
        gap: '5px',
        flexShrink: 0,
        opacity: strategy === 'smart' ? 1 : 0.55,
        transition: 'opacity var(--transition-normal)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexShrink: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '11px', fontWeight: 800, color: 'var(--text-primary)', display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
            <TrendingUp size={13} style={{ color: 'var(--status-info)' }} /> {trText('自适应温控曲线 (Fan Duty & Pump Voltage vs Temp)')} </span>
          <span
            style={{
              fontSize: '8.5px',
              color: 'var(--status-info)',
              background: 'var(--status-info-dim)',
              border: '1px solid rgba(56, 189, 248, 0.3)',
              padding: '1px 5px',
              borderRadius: 'var(--radius-xs)',
              fontWeight: 700,
            }}
          > {trText('支持节点与水泵阈值拖拽')} </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', fontSize: '9px', color: 'var(--text-muted)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: 'var(--radius-full)', background: 'var(--status-info)' }} />
            <span>{trText('风扇 (0%~100%)')}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: 'var(--radius-full)', background: 'var(--color-brand)' }} />
            <span>{trText('水泵工况 (双杆可拖拽: 启泵/狂暴)')}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: 'var(--radius-full)', background: 'var(--status-error)' }} />
            <span>{trText('实时探针')}</span>
          </div>
        </div>
      </div>

      {/* SVG 容器：保持 130px 高度不变，严防 UI 溢出下方 */}
      <div
        ref={containerRef}
        style={{
          position: 'relative',
          width: '100%',
          height: '130px',
          background: 'rgba(0, 0, 0, 0.35)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-sm)',
          overflow: 'hidden',
        }}
      >
        <svg
          viewBox={`0 0 ${containerWidth} 140`}
          style={{ width: '100%', height: '100%', display: 'block' }}
        >
          <defs>
            <linearGradient id="coolerFanFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#06b6d4" stopOpacity="0.32" />
              <stop offset="100%" stopColor="#06b6d4" stopOpacity="0.0" />
            </linearGradient>
          </defs>

          {/* 网格水平横线 */}
          <g stroke="rgba(255,255,255,0.05)" strokeWidth="1">
            <line x1={startX} y1="15" x2={endX} y2="15" />
            <line x1={startX} y1="43" x2={endX} y2="43" />
            <line x1={startX} y1="71" x2={endX} y2="71" />
            <line x1={startX} y1="99" x2={endX} y2="99" />
            <line x1={startX} y1="120" x2={endX} y2="120" stroke="rgba(255,255,255,0.15)" />

            {/* 网格垂直纵线 */}
            {[30, 40, 50, 60, 70, 80, 90, 100].map((t) => (
              <line key={t} x1={tempToX(t)} y1="15" x2={tempToX(t)} y2="120" />
            ))}
          </g>

          {/* 水泵工况分界动态背景色块 */}
          <rect
            x={startX}
            y="15"
            width={Math.max(0, xStart - startX)}
            height="105"
            fill="rgba(16, 185, 129, 0.04)"
          />
          {xStart - startX > 65 && (
            <text x={startX + 8} y="42" fill="#34d399" fontSize="9" fontWeight="700" opacity="0.7"> {trText('低温停转区 (&lt;')}{displayStart}°C)
            </text>
          )}

          <rect
            x={xStart}
            y="15"
            width={Math.max(0, xTurbo - xStart)}
            height="105"
            fill="rgba(6, 182, 212, 0.05)"
          />
          {xTurbo - xStart > 90 && (
            <text x={xStart + 8} y="42" fill="#06b6d4" fontSize="9" fontWeight="700" opacity="0.85">
              水泵标准巡航区 ({displayStart}°C~{displayTurbo}°C)
            </text>
          )}

          <rect
            x={xTurbo}
            y="15"
            width={Math.max(0, endX - xTurbo)}
            height="105"
            fill="rgba(244, 63, 94, 0.08)"
          />
          {endX - xTurbo > 65 && (
            <text x={xTurbo + 8} y="42" fill="#f43f5e" fontSize="9" fontWeight="700" opacity="0.85"> {trText('水泵极限狂暴区 (≥')}{displayTurbo}°C)
            </text>
          )}

          {/* Y 轴百分比刻度 */}
          <text x={startX - 7} y="18" fill="var(--text-muted)" fontSize="9" textAnchor="end">100%</text>
          <text x={startX - 7} y="46" fill="var(--text-muted)" fontSize="9" textAnchor="end">75%</text>
          <text x={startX - 7} y="74" fill="var(--text-muted)" fontSize="9" textAnchor="end">50%</text>
          <text x={startX - 7} y="102" fill="var(--text-muted)" fontSize="9" textAnchor="end">25%</text>
          <text x={startX - 7} y="123" fill="#34d399" fontSize="9" fontWeight="700" textAnchor="end">0 RPM</text>

          {/* X 轴温度刻度：动态过滤与手柄重合的刻度 */}
          {[30, 40, 50, 60, 70, 80, 90, 100].map((t) => {
            if (Math.abs(t - displayStart) < 4 || Math.abs(t - displayTurbo) < 4) {
              return null;
            }
            return (
              <text key={t} x={tempToX(t)} y="134" fill="var(--text-dim)" fontSize="9" textAnchor="middle">
                {t}°
              </text>
            );
          })}
          <text x={xStart} y="134" fill="#34d399" fontSize="9" fontWeight="800" textAnchor="middle">
            {displayStart}{trText('°启泵')} </text>
          <text x={xTurbo} y="134" fill="#f43f5e" fontSize="9" fontWeight="800" textAnchor="middle">
            {displayTurbo}{trText('°狂暴')} </text>

          {/* 填充面积与平滑折线 */}
          <path d={curveAreaD} fill="url(#coolerFanFill)" />
          <path
            d={curvePathD}
            fill="none"
            stroke="#06b6d4"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />

          {/* 风扇温控曲线节点控制手柄 */}
          {sortedPts.map((p, idx) => (
            <circle
              key={idx}
              cx={tempToX(p.temp)}
              cy={dutyToY(p.fan_duty)}
              r="5"
              fill="#06b6d4"
              stroke="#ffffff"
              strokeWidth="1.6"
              style={{ cursor: 'grab' }}
              onMouseDown={(e) => startDragHandle(e, idx)}
            />
          ))}

          {/* 启泵阈值手柄杆 (可左右拖拽) */}
          <g
            style={{ cursor: 'ew-resize', userSelect: 'none' }}
            onMouseDown={(e) => startDragPump(e, 'start')}
          >
            <line
              x1={xStart}
              y1="15"
              x2={xStart}
              y2="120"
              stroke="#10b981"
              strokeWidth={activeDragPump === 'start' ? 2.5 : 1.8}
              strokeDasharray={activeDragPump === 'start' ? undefined : '3,2'}
              opacity={activeDragPump === 'start' ? 1 : 0.85}
            />
            {/* 顶部手柄徽标 Badge */}
            <g transform={`translate(${xStart}, 15)`}>
              <rect
                x="-25"
                y="0"
                width="50"
                height="14"
                rx="3"
                fill={activeDragPump === 'start' ? 'rgba(5, 150, 105, 0.98)' : 'rgba(16, 185, 129, 0.92)'}
                stroke="#34d399"
                strokeWidth={activeDragPump === 'start' ? 1.4 : 0.8}
              />
              <text x="0" y="10" fill="#ffffff" fontSize="8.5" fontWeight="700" textAnchor="middle">
                {displayStart}{trText('° 启泵')} </text>
            </g>
            {/* 中间握持滑块 Thumb */}
            <g transform={`translate(${xStart}, 64)`}>
              <rect
                x="-5"
                y="-9"
                width="10"
                height="18"
                rx="3"
                fill={activeDragPump === 'start' ? '#059669' : '#10b981'}
                stroke="#ffffff"
                strokeWidth="1.2"
              />
              <line x1="-1.5" y1="-4" x2="-1.5" y2="4" stroke="#ffffff" strokeWidth="1" strokeLinecap="round" />
              <line x1="1.5" y1="-4" x2="1.5" y2="4" stroke="#ffffff" strokeWidth="1" strokeLinecap="round" />
            </g>
            {/* 扩大拖拽感应热区 Hitbox (±14px) */}
            <rect
              x={xStart - 14}
              y="15"
              width="28"
              height="105"
              fill="transparent"
            />
          </g>

          {/* 狂暴阈值手柄杆 (可左右拖拽) */}
          <g
            style={{ cursor: 'ew-resize', userSelect: 'none' }}
            onMouseDown={(e) => startDragPump(e, 'turbo')}
          >
            <line
              x1={xTurbo}
              y1="15"
              x2={xTurbo}
              y2="120"
              stroke="#f43f5e"
              strokeWidth={activeDragPump === 'turbo' ? 2.5 : 1.8}
              strokeDasharray={activeDragPump === 'turbo' ? undefined : '3,2'}
              opacity={activeDragPump === 'turbo' ? 1 : 0.85}
            />
            {/* 顶部手柄徽标 Badge */}
            <g transform={`translate(${xTurbo}, 15)`}>
              <rect
                x="-25"
                y="0"
                width="50"
                height="14"
                rx="3"
                fill={activeDragPump === 'turbo' ? 'rgba(225, 29, 72, 0.98)' : 'rgba(244, 63, 94, 0.92)'}
                stroke="#fda4af"
                strokeWidth={activeDragPump === 'turbo' ? 1.4 : 0.8}
              />
              <text x="0" y="10" fill="#ffffff" fontSize="8.5" fontWeight="700" textAnchor="middle">
                {displayTurbo}{trText('° 狂暴')} </text>
            </g>
            {/* 中间握持滑块 Thumb */}
            <g transform={`translate(${xTurbo}, 64)`}>
              <rect
                x="-5"
                y="-9"
                width="10"
                height="18"
                rx="3"
                fill={activeDragPump === 'turbo' ? '#e11d48' : '#f43f5e'}
                stroke="#ffffff"
                strokeWidth="1.2"
              />
              <line x1="-1.5" y1="-4" x2="-1.5" y2="4" stroke="#ffffff" strokeWidth="1" strokeLinecap="round" />
              <line x1="1.5" y1="-4" x2="1.5" y2="4" stroke="#ffffff" strokeWidth="1" strokeLinecap="round" />
            </g>
            {/* 扩大拖拽感应热区 Hitbox (±14px) */}
            <rect
              x={xTurbo - 14}
              y="15"
              width="28"
              height="105"
              fill="transparent"
            />
          </g>

          {/* 实时温度探针 */}
          <g>
            <line
              x1={probeX}
              y1="15"
              x2={probeX}
              y2="120"
              stroke="#f43f5e"
              strokeWidth="1.8"
              strokeDasharray="3,2"
            />
            <circle cx={probeX} cy={probeY} r="7" fill="#f43f5e" opacity="0.35" />
            <circle cx={probeX} cy={probeY} r="3.5" fill="#ffffff" stroke="#f43f5e" strokeWidth="2" />
            <g transform={`translate(${probeX}, ${Math.max(25, probeY - 8)})`}>
              <rect
                x="-36"
                y="-17"
                width="72"
                height="14"
                rx="3"
                fill="rgba(15, 23, 42, 0.95)"
                stroke="#f43f5e"
                strokeWidth="0.8"
              />
              <text x="0" y="-7" fill="#fff" fontSize="8.5" fontWeight="700" textAnchor="middle">
                {liveMaxTemp}°C / {liveInterpolated.duty}%
              </text>
            </g>
          </g>
        </svg>
      </div>

      {/* 曲线底部预设模板 */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', paddingTop: '3px', borderTop: '1px solid var(--border-subtle)', fontSize: '9.5px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
          <span style={{ color: 'var(--text-dim)', fontWeight: 600 }}>{trText('曲线预设：')}</span>
          {[
            { id: 'standard', label: trText('出厂智慧 (55°起转)'), Icon: Star },
            { id: 'quiet', label: trText('极致静音 (65°起转)'), Icon: VolumeX },
            { id: 'aggressive', label: trText('激进压温 (45°起转)'), Icon: Flame },
            { id: 'linear', label: trText('线性爬升'), Icon: Zap },
          ].map((t) => {
            const IconComponent = t.Icon;
            const isActive = curveTemplate === t.id;
            return (
              <button
                type="button"
                key={t.id}
                onClick={() => onApplyTemplate(t.id as any)}
                className={`cooler-curve-preset-btn ${isActive ? 'active' : ''}`}
                style={{
                  background: isActive ? 'var(--status-info-dim)' : 'var(--surface-badge)',
                  border: `1px solid ${isActive ? 'rgba(56, 189, 248, 0.5)' : 'var(--border-subtle)'}`,
                  color: isActive ? 'var(--status-info)' : 'var(--text-muted)',
                  padding: '2px 7px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: '9.5px',
                  fontWeight: 600,
                  cursor: 'pointer',
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '4px',
                  transition: 'all var(--transition-fast)',
                }}
              >
                <IconComponent size={11} />
                <span>{t.label}</span>
              </button>
            );
          })}
        </div>

        <div style={{ color: 'var(--text-dim)', display: 'flex', alignItems: 'center', gap: '4px' }}>
          <span style={{ display: 'inline-block', width: '6px', height: '6px', borderRadius: 'var(--radius-full)', backgroundColor: 'var(--status-ok)' }} />
          <span>{trText('内置 3°C 迟滞滤波防抖')}</span>
        </div>
      </div>
    </div>
  );
};
