import { coerceNumberArray } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ChartBarsV1Params {
  values: number[];
  bar_width?: number;
  gap?: number;
  chart_height?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_chart_bars_v0.
   * - `'dark'`: bar color uses chart-1 token from semantic palette (same hue, dark-mode variant).
   * - `'system'`: emits `$color-chart-1` ref for bar fill.
   */
  theme?: V1Theme;
}

/**
 * Bar-chart skeleton — theme-aware version of buildChartBars.
 * Light mode is byte-equal to add_chart_bars_v0.
 *
 * Bar color maps to chart-1 (blue series). In all three modes the
 * chart-1 token resolves the same way — chart colors are single-value
 * (no theme axis) — so light/dark produce the same hex (#3B82F6 in
 * the palette, but v0 used #2563EB for byte-parity; light mode keeps
 * #2563EB for the migration contract).
 */
export function buildChartBarsV1(params: ChartBarsV1Params): ElementTree {
  const inputValues = coerceNumberArray(params.values, [1], 'buildChartBarsV1', 'values');
  const values = inputValues.map((v) => Math.max(0, v));
  const max = Math.max(1, ...values);
  const barWidth = Math.max(4, Math.floor(params.bar_width ?? 24));
  const gap = Math.max(0, Math.floor(params.gap ?? 12));
  const chartHeight = Math.max(40, Math.floor(params.chart_height ?? 160));
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);

  // Light mode: byte-parity with v0 (#2563EB). Dark/system: use chart-1 token.
  const barColor = theme === 'light' ? '#2563EB' : t.chartColors.chart1;

  const children: ElementTree[] = values.map((v, i) => ({
    type: 'rectangle',
    name: `Bar ${i + 1}`,
    role: 'chart-bar',
    width: barWidth,
    height: Math.max(2, Math.round((v / max) * chartHeight)),
    cornerRadius: 4,
    fill: [{ type: 'solid', color: barColor }],
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
