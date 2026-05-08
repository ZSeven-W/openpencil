import { coerceNumberArray } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ChartPieV1Params {
  /** Slice values (any numeric scale; normalized internally). */
  values: number[];
  /** Pie diameter (px, width=height). Default 160, min 40. */
  diameter?: number;
  /**
   * Optional per-slice colors (hex). If fewer than values.length,
   * the default chart palette fills the rest.
   *
   * Note: In dark/system modes, caller-supplied colors are passed
   * through unchanged (they are treated as intentional brand colors).
   * Only the default palette rotates through chart tokens.
   */
  colors?: string[];
  /** Inner cut-out radius as fraction of outer (0..0.9). Default 0 (full pie). */
  inner_radius_ratio?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_chart_pie_v0 (uses v0 DEFAULT_PALETTE).
   * - `'dark'`: default palette uses chart-1..6 tokens from semantic palette.
   * - `'system'`: default palette emits `$color-chart-1..6` refs.
   *
   * When `colors` param is supplied, those values are always used as-is
   * regardless of theme (caller takes ownership of color choice).
   */
  theme?: V1Theme;
}

// v0 byte-parity palette — MUST match buildChartPie's DEFAULT_PALETTE exactly
const V0_DEFAULT_PALETTE = ['#2563EB', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899'];

/**
 * Pie-chart skeleton — theme-aware version of buildChartPie.
 * Light mode is byte-equal to add_chart_pie_v0.
 *
 * Default slice colors map to chart-1..6 tokens in dark/system modes.
 * Caller-supplied `colors` are passed through unchanged in all modes.
 */
export function buildChartPieV1(params: ChartPieV1Params): ElementTree {
  const inputValues = coerceNumberArray(params.values, [1], 'buildChartPieV1', 'values');
  let values = inputValues.map((v) => Math.max(0, v));
  let total = values.reduce((s, v) => s + v, 0);
  if (total <= 0) {
    values = [1];
    total = 1;
  }
  const diameter = Math.max(40, Math.floor(params.diameter ?? 160));
  const innerRatio = Math.max(0, Math.min(0.9, params.inner_radius_ratio ?? 0));
  const innerRadius = innerRatio > 0 ? innerRatio : undefined;
  const callerColors = params.colors ?? [];
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);

  // Build default palette for this theme
  const defaultPalette =
    theme === 'light'
      ? V0_DEFAULT_PALETTE
      : [
          t.chartColors.chart1,
          t.chartColors.chart2,
          t.chartColors.chart3,
          t.chartColors.chart4,
          t.chartColors.chart5,
          t.chartColors.chart6,
        ];

  const children: ElementTree[] = [];
  let currentAngle = -90;
  for (let i = 0; i < values.length; i++) {
    const fraction = values[i] / total;
    const sweep = fraction * 360;
    // Caller-supplied colors take precedence; fall back to default palette
    const color = callerColors[i] ?? defaultPalette[i % defaultPalette.length];
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
