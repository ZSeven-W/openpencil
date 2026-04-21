import type { ElementTree } from './helpers.js';

export interface ChartPieParams {
  /** Slice values (any numeric scale; normalized internally). */
  values: number[];
  /** Pie diameter (px, width=height). Default 160, min 40. */
  diameter?: number;
  /**
   * Optional per-slice colors (hex). If fewer than values.length,
   * the default rotation palette fills the rest.
   */
  colors?: string[];
  /** Inner cut-out radius as fraction of outer (0..0.9). Default 0 (full pie). */
  inner_radius_ratio?: number;
}

const DEFAULT_PALETTE = ['#2563EB', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899'];

/**
 * Pie-chart skeleton — N colored slice ellipses filling a circle via
 * the ellipse node's `startAngle` / `sweepAngle` arc support.
 *
 * Key distinction from the "stacked ellipses for a ring" anti-pattern:
 * each slice here has a UNIQUE sweep range (not stacked full ellipses).
 * The angular positions don't overlap, so this is correct arc usage.
 *
 * Set `inner_radius_ratio > 0` to cut a donut hole — passes through to
 * `innerRadius` on each slice.
 *
 * Negative / non-finite values clamp to 0. All-zero input throws
 * (no slice can have 0° sweep — the chart would be empty).
 */
export function buildChartPie(params: ChartPieParams): ElementTree {
  const raw = Array.isArray(params.values) ? params.values : [];
  if (raw.length === 0) {
    throw new Error('buildChartPie: values must contain at least one number');
  }
  const values = raw.map((v) => (Number.isFinite(v) ? Math.max(0, v) : 0));
  const total = values.reduce((s, v) => s + v, 0);
  if (total <= 0) {
    throw new Error('buildChartPie: values must sum to > 0');
  }
  const diameter = Math.max(40, Math.floor(params.diameter ?? 160));
  const innerRatio = Math.max(0, Math.min(0.9, params.inner_radius_ratio ?? 0));
  // EllipseNode.innerRadius is a RATIO 0..1 (the renderer does `rx * inner`
  // — see packages/pen-core/src/arc-path.ts docstring + skia-interaction.ts
  // which clamps to [0, 0.99]). Pass the ratio directly; storing pixels
  // would be interpreted as a huge ratio and clip all slices.
  const innerRadius = innerRatio > 0 ? innerRatio : undefined;
  const colors = params.colors ?? [];

  const children: ElementTree[] = [];
  let currentAngle = -90; // start at top of circle (12 o'clock)
  for (let i = 0; i < values.length; i++) {
    const fraction = values[i] / total;
    const sweep = fraction * 360;
    const color = colors[i] ?? DEFAULT_PALETTE[i % DEFAULT_PALETTE.length];
    const slice: ElementTree = {
      type: 'ellipse',
      name: `Slice ${i + 1}`,
      role: 'chart-pie-slice',
      x: 0,
      y: 0,
      width: diameter,
      height: diameter,
      startAngle: currentAngle,
      sweepAngle: sweep,
      fill: [{ type: 'solid', color }],
    };
    if (innerRadius !== undefined) slice.innerRadius = innerRadius;
    children.push(slice);
    currentAngle += sweep;
  }

  return {
    type: 'frame',
    name: 'Chart Pie',
    role: 'chart-pie',
    width: diameter,
    height: diameter,
    layout: 'none',
    children,
  };
}
