import type { ElementTree } from './helpers.js';

export interface ProgressBarParams {
  value?: number;
  bar_width?: number;
}

/**
 * Linear progress bar. Fixed-pixel track (default 240) so fill can
 * be computed as value/100 × bar_width (pen-core has no percent /
 * flex-basis sizing). value clamped to [0, 100].
 */
export function buildProgressBar(params: ProgressBarParams): ElementTree {
  const raw = params.value ?? 50;
  const value = Math.max(0, Math.min(100, raw));
  const barWidth = params.bar_width ?? 240;
  const fillWidth = Math.max(0, Math.round((barWidth * value) / 100));
  const children: ElementTree[] = [];
  if (fillWidth > 0) {
    children.push({
      type: 'rectangle',
      name: 'Fill',
      role: 'progress-bar-fill',
      width: fillWidth,
      height: 8,
      cornerRadius: 4,
      fill: [{ type: 'solid', color: '#2563EB' }],
    });
  }
  return {
    type: 'frame',
    name: 'Progress Bar',
    role: 'progress-bar',
    width: barWidth,
    height: 8,
    cornerRadius: 4,
    fill: [{ type: 'solid', color: '#E5E7EB' }],
    layout: 'horizontal',
    alignItems: 'center',
    children,
  };
}
