export interface CpuPowerLimits {
  pl1: number;
  pl2: number;
  pl4: number;
}

// Keep the confirmation preview and the applied power limits on the same rules.
export function resolveCpuPowerLimits(
  current: CpuPowerLimits,
  limit: keyof CpuPowerLimits,
  target: number,
  margin: number,
): CpuPowerLimits {
  if (limit === 'pl1') {
    const pl2 = Math.max(target, current.pl2);
    return { pl1: target, pl2, pl4: Math.max(pl2 + margin, current.pl4) };
  }
  if (limit === 'pl2') {
    return { pl1: Math.min(target, current.pl1), pl2: target, pl4: Math.max(target + margin, current.pl4) };
  }
  const pl2 = Math.min(target - margin, current.pl2);
  return { pl1: Math.min(pl2, current.pl1), pl2, pl4: target };
}
