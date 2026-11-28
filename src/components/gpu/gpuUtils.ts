import { trText } from '../../i18n/I18nContext';
// Pure formatting utilities for GPU mode display — no React dependency.

export function formatGpuShort(name?: string, hasDgpu: boolean = true): string {
  if (!hasDgpu) {
    if (!name || name === trText('独立显卡') || name === trText('高性能独立显卡') || name === trText('未知显卡')) return trText('CPU 内建核显');
    return name
      .replace(/\(R\)/gi, '')
      .replace(/\(TM\)/gi, '')
      .replace(/Graphics/gi, '')
      .trim() || trText('CPU 内建核显');
  }
  if (!name) return trText('独立显卡');
  const s = name
    .replace(/NVIDIA\s+GeForce\s+/gi, '')
    .replace(/\s+Laptop\s+GPU/gi, '')
    .trim();
  return s || trText('独立显卡');
}

export function formatCpuShort(name?: string): string {
  if (!name) return trText('通用硬件平台');
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
    case 'dgpu':   return '独显直连';
    case 'hybrid': return '混合输出';
    case 'igpu':   return '核显模式';
    default:       return trText('未知模式');
  }
}
