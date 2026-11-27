// Pure formatting utilities for GPU mode display — no React dependency.

export function formatGpuShort(name?: string, hasDgpu: boolean = true): string {
  if (!hasDgpu) {
    if (!name || name === 'Дискретная графика' || name === 'Производительная дискретная графика' || name === 'Неизвестная графика') return 'Встроенное в CPU iGPU';
    return name
      .replace(/\(R\)/gi, '')
      .replace(/\(TM\)/gi, '')
      .replace(/Graphics/gi, '')
      .trim() || 'Встроенное в CPU iGPU';
  }
  if (!name) return 'Дискретная графика';
  const s = name
    .replace(/NVIDIA\s+GeForce\s+/gi, '')
    .replace(/\s+Laptop\s+GPU/gi, '')
    .trim();
  return s || 'Дискретная графика';
}

export function formatCpuShort(name?: string): string {
  if (!name) return 'Универсальная аппаратная платформа';
  return name
    .replace(/\(R\)/gi, '')
    .replace(/\(TM\)/gi, '')
    .replace(/14th Gen /gi, '')
    .replace(/13th Gen /gi, '')
    .replace(/12th Gen /gi, '')
    .replace(/Processor/gi, '')
    .trim();
}

export function getModeHexDisplay(mode: string, platform?: string): string {
  const isAmd = platform === 'amd';
  switch (mode) {
    case 'dgpu':   return isAmd ? '0x01' : '0x02';
    case 'hybrid': return isAmd ? '0x00' : '0x04';
    case 'igpu':   return isAmd ? '0x02' : '0x01';
    default:       return '--';
  }
}

export function getModeTitle(mode: string): string {
  switch (mode) {
    case 'dgpu':   return 'Прямой вывод dGPU';
    case 'hybrid': return 'Гибридный вывод';
    case 'igpu':   return 'Режим iGPU';
    default:       return 'Неизвестный режим';
  }
}
