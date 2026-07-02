import type { ElementTree } from './helpers.js';

export interface ChartBarsParams {
  values: number[];
  bar_width?: number;
  gap?: number;
  chart_height?: number;
}

/**
 * Bar-chart skeleton — one rectangle per `values` entry, bottom-
 * aligned via alignItems=flex-end. Heights proportional to max;
 * zero-valued bars get 2px floor so pen-core doesn't collapse them.
 * Negative / non-finite values clamp to 0. No axes or labels —
 * caller stitches via batch_design U-op.
 */
export function buildChartBars(params: ChartBarsParams): ElementTree {
  const raw = Array.isArray(params.values) ? params.values : [];
  if (raw.length === 0) {
    throw new Error('buildChartBars: values must contain at least one number');
  }
  const values = raw.map((v) => (Number.isFinite(v) ? Math.max(0, v) : 0));
  const max = Math.max(1, ...values);
  const barWidth = Math.max(4, Math.floor(params.bar_width ?? 24));
  const gap = Math.max(0, Math.floor(params.gap ?? 12));
  const chartHeight = Math.max(40, Math.floor(params.chart_height ?? 160));
  const children: ElementTree[] = values.map((v, i) => ({
    type: 'rectangle',
    name: `Bar ${i + 1}`,
    role: 'chart-bar',
    width: barWidth,
    height: Math.max(2, Math.round((v / max) * chartHeight)),
    cornerRadius: 4,
    fill: [{ type: 'solid', color: '#2563EB' }],
  }));
  return {
    type: 'frame',
    name: 'Chart Bars',
    role: 'chart-bars',
    width: 'fit_content',
    height: chartHeight,
    layout: 'horizontal',
    alignItems: 'flex-end',
    gap,
    children,
  };
}
