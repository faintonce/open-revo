import React from 'react';

export interface RevSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  size?: 'sm' | 'md' | 'lg';
  activeColor?: string;
  activeGlow?: string;
  className?: string;
  style?: React.CSSProperties;
  ariaLabel?: string;
}

export const RevSwitch: React.FC<RevSwitchProps> = ({
  checked,
  onChange,
  disabled = false,
  size = 'md',
  activeColor = 'var(--status-ok)',
  activeGlow,
  className = '',
  style,
  ariaLabel,
}) => {
  const getDims = () => {
    switch (size) {
      case 'sm':
        return {
          trackW: 28,
          trackH: 16,
          thumbSize: 12,
          offsetOff: 2,
          offsetOn: 14,
        };
      case 'lg':
        return {
          trackW: 42,
          trackH: 22,
          thumbSize: 18,
          offsetOff: 2,
          offsetOn: 22,
        };
      case 'md':
      default:
        return {
          trackW: 34,
          trackH: 18,
          thumbSize: 14,
          offsetOff: 2,
          offsetOn: 18,
        };
    }
  };

  const { trackW, trackH, thumbSize, offsetOff, offsetOn } = getDims();
  const glow = activeGlow || `0 0 10px ${activeColor}55`;

  return (
    <div
      role="switch"
      aria-checked={checked}
      aria-disabled={disabled}
      aria-label={ariaLabel}
      tabIndex={disabled ? -1 : 0}
      onKeyDown={(e) => {
        if (e.key === ' ' || e.key === 'Enter') {
          e.preventDefault();
          e.stopPropagation();
          if (!disabled) onChange(!checked);
        }
      }}
      onClick={(e) => {
        e.stopPropagation();
        if (!disabled) onChange(!checked);
      }}
      className={`rev-switch-pill ${checked ? 'active' : ''} ${disabled ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer'} ${className}`}
      style={{
        width: `${trackW}px`,
        height: `${trackH}px`,
        borderRadius: '9999px',
        backgroundColor: checked ? activeColor : 'rgba(255, 255, 255, 0.12)',
        border: `1px solid ${checked ? activeColor : 'rgba(255, 255, 255, 0.15)'}`,
        position: 'relative',
        display: 'inline-flex',
        alignItems: 'center',
        boxShadow: checked ? glow : 'inset 0 1px 2px rgba(0, 0, 0, 0.4)',
        transition: 'background-color 0.22s ease, border-color 0.22s ease, box-shadow 0.22s ease',
        userSelect: 'none',
        cursor: disabled ? 'not-allowed' : 'pointer',
        flexShrink: 0,
        ...style,
      }}
    >
      <div
        style={{
          width: `${thumbSize}px`,
          height: `${thumbSize}px`,
          borderRadius: '50%',
          backgroundColor: '#ffffff',
          boxShadow: '0 1.5px 4px rgba(0, 0, 0, 0.45)',
          transform: `translateX(${checked ? offsetOn : offsetOff}px)`,
          transition: 'transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1)',
          position: 'absolute',
          left: 0,
        }}
      />
    </div>
  );
};
