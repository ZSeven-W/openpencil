import type { ElementTree } from './helpers.js';

export type MetricTrend = 'up' | 'down' | 'flat';

export interface MetricComparisonParams {
  label: string;
  /** Main metric value (string so callers can format — "$12,480" / "98.7%" / "2.3k"). */
  value: string;
  /** Numeric change (+ / - sign carries via trend, not here). E.g. "8%" / "1.2k". */
  change?: string;
  /** up / down / flat — picks arrow icon + color (green / red / neutral). */
  trend?: MetricTrend;
}

/**
 * KPI with trend indicator: big value + small label above + optional
 * arrow + percent change on the right. The "$12k ↑ 8%" card-cell
 * pattern. Distinct from `add_metric_row_v0` (that's a scroll row of
 * label+value cells without trend affordance).
 *
 * Structure:
 *   frame(fit_content × fit_content, vertical, gap=4, role='metric-comparison')
 *     ├ text(label, 12/500 slate-500)
 *     └ frame(horizontal, gap=8, alignItems=baseline)
 *          ├ text(value, 28/700)
 *          └ optional frame(horizontal, gap=2) [when `change` set]
 *                ├ icon_font(trending-up/down/minus 14×14, role='metric-arrow')
 *                └ text(change, 12/500, fill=green/red/slate)
 */
export function buildMetricComparison(params: MetricComparisonParams): ElementTree {
  const trend = params.trend ?? (params.change ? 'flat' : 'flat');
  const trendColor = trendColorFor(trend);
  const trendIcon = trendIconFor(trend);

  const rowChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Value',
      role: 'metric-comparison-value',
      content: params.value,
      fontSize: 28,
      fontWeight: 700,
    },
  ];
  if (params.change) {
    rowChildren.push({
      type: 'frame',
      name: 'Change',
      role: 'metric-comparison-change',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      gap: 2,
      children: [
        {
          type: 'icon_font',
          name: 'Trend Arrow',
          role: 'metric-comparison-arrow',
          iconFontName: trendIcon,
          iconFontFamily: 'lucide',
          width: 14,
          height: 14,
          fill: [{ type: 'solid', color: trendColor }],
        },
        {
          type: 'text',
          name: 'Change Amount',
          role: 'metric-comparison-change-text',
          content: params.change,
          fontSize: 12,
          fontWeight: 500,
          fill: [{ type: 'solid', color: trendColor }],
        },
      ],
    });
  }

  return {
    type: 'frame',
    name: 'Metric Comparison',
    role: 'metric-comparison',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'vertical',
    gap: 4,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'metric-comparison-label',
        content: params.label,
        fontSize: 12,
        fontWeight: 500,
        fill: [{ type: 'solid', color: '#64748B' }],
      },
      {
        type: 'frame',
        name: 'Value Row',
        role: 'metric-comparison-row',
        width: 'fit_content',
        height: 'fit_content',
        layout: 'horizontal',
        alignItems: 'baseline',
        gap: 8,
        children: rowChildren,
      },
    ],
  };
}

function trendColorFor(trend: MetricTrend): string {
  switch (trend) {
    case 'up':
      return '#10B981';
    case 'down':
      return '#EF4444';
    case 'flat':
    default:
      return '#64748B';
  }
}

function trendIconFor(trend: MetricTrend): string {
  switch (trend) {
    case 'up':
      return 'trending-up';
    case 'down':
      return 'trending-down';
    case 'flat':
    default:
      return 'minus';
  }
}
