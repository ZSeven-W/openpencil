import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ChartLineV1Params {
  values: number[];
  /** Width per data point slot (px). Default 32, min 8. */
  point_spacing?: number;
  /** Chart height (px). Default 160, min 40. */
  chart_height?: number;
  /** Whether to emit a filled dot at each data point. Default true. */
  dots?: boolean;
  /**
   * Stroke color for the line. Default #2563EB (Tailwind blue-600).
   * Only used when `theme='light'` (v0 byte-parity). For dark/system
   * modes, chart-1 token is used instead (ignores this field).
   */
  stroke_color?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_chart_line_v0.
   * - `'dark'`: line/dot color uses chart-1 token from semantic palette.
   * - `'system'`: emits `$color-chart-1` ref for line and dot fills.
   */
  theme?: V1Theme;
}

/**
 * Line-chart skeleton — theme-aware version of buildChartLine.
 * Light mode is byte-equal to add_chart_line_v0.
 *
 * Stroke/dot color maps to chart-1 (blue series). Light mode preserves
 * v0's #2563EB (or caller-supplied stroke_color) for byte-parity.
 */
export function buildChartLineV1(params: ChartLineV1Params): ElementTree {
  const raw = Array.isArray(params.values) ? params.values : [];
  if (raw.length === 0) {
    throw new Error('buildChartLineV1: values must contain at least one number');
  }
  const values = raw.map((v) => (Number.isFinite(v) ? Math.max(0, v) : 0));
  const max = Math.max(1, ...values);
  const spacing = Math.max(8, Math.floor(params.point_spacing ?? 32));
  const chartHeight = Math.max(40, Math.floor(params.chart_height ?? 160));
  const dots = params.dots ?? true;
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);

  // Light mode: byte-parity with v0 (stroke_color param or #2563EB).
  // Dark/system: use chart-1 token.
  const strokeColor = theme === 'light' ? (params.stroke_color ?? '#2563EB') : t.chartColors.chart1;
  const totalWidth = spacing * values.length;

  const points = values.map((v, i) => {
    const x = i * spacing + spacing / 2;
    const yRaw = chartHeight - (v / max) * chartHeight;
    const y = v > 0 ? Math.min(yRaw, chartHeight - 2) : chartHeight - 2;
    return { x, y };
  });

  const dPath = points
    .map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x.toFixed(1)} ${p.y.toFixed(1)}`)
    .join(' ');

  const children: ElementTree[] = [
    {
      type: 'path',
      name: 'Line',
      role: 'chart-line-path',
      d: dPath,
      width: totalWidth,
      height: chartHeight,
      fill: [],
      stroke: { thickness: 2, fill: [{ type: 'solid', color: strokeColor }] },
    },
  ];

  if (dots) {
    for (let i = 0; i < points.length; i++) {
      const p = points[i];
      children.push({
        type: 'ellipse',
        name: `Dot ${i + 1}`,
        role: 'chart-line-dot',
        x: p.x - 4,
        y: p.y - 4,
        width: 8,
        height: 8,
        fill: [{ type: 'solid', color: strokeColor }],
      });
    }
  }

  return {
    type: 'frame',
    name: 'Chart Line',
    role: 'chart-line',
    width: totalWidth,
    height: chartHeight,
    layout: 'none',
    children,
  };
}
