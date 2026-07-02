import type { ElementTree } from './helpers.js';

export interface ChartLineParams {
  values: number[];
  /** Width per data point slot (px). Default 32, min 8. */
  point_spacing?: number;
  /** Chart height (px). Default 160, min 40. */
  chart_height?: number;
  /** Whether to emit a filled dot at each data point. Default true. */
  dots?: boolean;
  /** Stroke color for the line. Default #2563EB (Tailwind blue-600). */
  stroke_color?: string;
}

/**
 * Line-chart skeleton — polyline through N data points, optionally
 * with dots at each vertex. No axes / labels — caller stitches via
 * batch_design U-op.
 *
 * Geometry:
 *   - Wrapper width = N × point_spacing (fit_content)
 *   - Each point lands at x=i*spacing + spacing/2, y=H-(v/max)*H
 *     (higher value → closer to top edge; 0 value → bottom edge)
 *   - Zero-valued points get a tiny floor (2px above bottom) so the
 *     line doesn't visually clip
 *   - Path emitted as an open `path` node with stroke
 *   - Dots (if enabled) emitted as separate `ellipse` children after
 *     the path so they paint on top
 *
 * Negative / non-finite values clamp to 0. Empty input throws.
 */
export function buildChartLine(params: ChartLineParams): ElementTree {
  const raw = Array.isArray(params.values) ? params.values : [];
  if (raw.length === 0) {
    throw new Error('buildChartLine: values must contain at least one number');
  }
  const values = raw.map((v) => (Number.isFinite(v) ? Math.max(0, v) : 0));
  const max = Math.max(1, ...values);
  const spacing = Math.max(8, Math.floor(params.point_spacing ?? 32));
  const chartHeight = Math.max(40, Math.floor(params.chart_height ?? 160));
  const dots = params.dots ?? true;
  const strokeColor = params.stroke_color ?? '#2563EB';
  const totalWidth = spacing * values.length;

  // Compute (x, y) for each data point. y-axis inverted (Skia/canvas
  // convention: 0 is top). Non-zero values get a 2px floor.
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
        // Ellipses in pen-core use top-left positioning, so offset by
        // half the dot diameter to center on the data point.
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
