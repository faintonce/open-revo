import React, { useRef } from 'react';
import { RpmCurvePoint } from '../../types/hardware';

const W = 400, H = 210, LEFT = 42, RIGHT = 14, TOP = 14, BOTTOM = 28;
const x = (temp: number) => LEFT + (temp - 30) / 70 * (W - LEFT - RIGHT);

export const RpmFanCurveEditor: React.FC<{
  curve: RpmCurvePoint[];
  onChange: (curve: RpmCurvePoint[]) => void;
  disabled?: boolean;
  minRpm: number;
  maxRpm: number;
  rpmStep: number;
}> = ({ curve, onChange, disabled = false, minRpm, maxRpm, rpmStep }) => {
  const y = (rpm: number) => TOP + (maxRpm - rpm) / (maxRpm - minRpm) * (H - TOP - BOTTOM);
  const svg = useRef<SVGSVGElement>(null);
  const dragging = useRef<number | null>(null);
  const setRpm = (index: number, raw: number) => {
    if (disabled || !Number.isFinite(raw)) return;
    const rpm = Math.max(minRpm, Math.min(maxRpm, Math.round(raw / rpmStep) * rpmStep));
    onChange(curve.map((p, i) => ({ ...p, rpm: i === index ? rpm : i < index ? Math.min(p.rpm, rpm) : Math.max(p.rpm, rpm) })));
    return rpm;
  };
  const setTemperature = (index: number, raw: number) => {
    if (disabled || !Number.isFinite(raw)) return;
    const min = index > 0 ? curve[index - 1].temp + 1 : 30;
    const max = index + 1 < curve.length ? curve[index + 1].temp - 1 : 100;
    const temp = Math.max(min, Math.min(max, Math.round(raw)));
    onChange(curve.map((p, i) => i === index ? { ...p, temp } : p));
    return temp;
  };
  return <div className="tuning-card rpm-curve-editor">
    <div className="tuning-header"><span className="tuning-header-title">温度与目标转速</span><span className="sys-row-desc">CPU 封装 °C → RPM</span></div>
    <svg ref={svg} className="rpm-curve-chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="温度与目标转速曲线"
      onPointerMove={event => {
        if (dragging.current === null || !svg.current || disabled) return;
        const rect = svg.current.getBoundingClientRect();
        const py = (event.clientY - rect.top) * H / rect.height;
        setRpm(dragging.current, maxRpm - (py - TOP) / (H - TOP - BOTTOM) * (maxRpm - minRpm));
      }} onPointerUp={() => { dragging.current = null; }} onPointerCancel={() => { dragging.current = null; }}>
      {[...new Set([minRpm, 2000, 3000, 4000, 5000, 6000, maxRpm])].filter(rpm => rpm >= minRpm && rpm <= maxRpm).sort((a,b) => a-b).map(rpm => <g key={rpm}>
        <line className="rpm-curve-grid" x1={LEFT} x2={W - RIGHT} y1={y(rpm)} y2={y(rpm)} />
        <text className="rpm-curve-label" x={LEFT - 5} y={y(rpm) + 3} textAnchor="end">{rpm}</text>
      </g>)}
      {[30, 40, 50, 60, 70, 80, 90, 100].map(temp => <text className="rpm-curve-label" key={temp} x={x(temp)} y={H - 9} textAnchor="middle">{temp}°</text>)}
      <polyline className="rpm-curve-line" points={curve.map(p => `${x(p.temp)},${y(p.rpm)}`).join(' ')} />
      {curve.map((point, index) => <circle className="rpm-curve-point" key={index} cx={x(point.temp)} cy={y(point.rpm)} r={5}
        onPointerDown={event => { if (!disabled) { event.preventDefault(); dragging.current = index; event.currentTarget.setPointerCapture(event.pointerId); } }}>
        <title>{point.temp}°C → {point.rpm} RPM</title></circle>)}
    </svg>
    <div className="rpm-curve-node-list">
      {curve.map((point, index) => <div className="rpm-curve-node" key={index}>
        <label>温度 °C<input aria-label={`节点 ${index + 1} 温度`} type="number" defaultValue={point.temp} key={`t-${point.temp}`} disabled={disabled}
          min={index > 0 ? curve[index - 1].temp + 1 : 30} max={index + 1 < curve.length ? curve[index + 1].temp - 1 : 100}
          onBlur={e => { e.currentTarget.value = String(setTemperature(index, e.currentTarget.valueAsNumber) ?? point.temp); }} onKeyDown={e => { if (e.key === 'Enter') e.currentTarget.blur(); }} /></label>
        <label>目标 RPM<input aria-label={`节点 ${index + 1} 目标 RPM`} type="number" defaultValue={point.rpm} key={`r-${point.rpm}`} disabled={disabled}
          min={minRpm} max={maxRpm} step={rpmStep} onBlur={e => { e.currentTarget.value = String(setRpm(index, e.currentTarget.valueAsNumber) ?? point.rpm); }}
          onKeyDown={e => { if (e.key === 'Enter') e.currentTarget.blur(); }} /></label>
      </div>)}
    </div>
    <div className="sys-row-desc">范围 {minRpm}–{maxRpm} RPM，步进 {rpmStep} RPM；升温时目标转速保持不降。实际转速可能有小幅波动。</div>
  </div>;
};
