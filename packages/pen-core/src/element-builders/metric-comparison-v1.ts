import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';
import type { MetricTrend } from './metric-comparison.js';

export interface MetricComparisonV1Params {
  label: string;
  /** Main metric value (string so callers can format — "$12,480" / "98.7%" / "2.3k"). */
  value: string;
  /** Numeric change (+ / - sign carries via trend, not here). E.g. "8%" / "1.2k". */
  change?: string;
  /** up / down / flat — picks arrow icon + color (green / red / neutral). */
  trend?: MetricTrend;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_metric_comparison_v0.
   * - `'dark'`: label → textMuted; trend colors → success/destructive/textMuted.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

/**
 * KPI with trend indicator (v1) — theme-aware variant of buildMetricComparison.
 * Light mode is byte-equal to add_metric_comparison_v0.
 *
 * Color mapping:
 *   label text     (#64748B slate-500) → textMuted
 *   trend up       (#10B981 emerald)   → success
 *   trend down     (#EF4444 red)       → destructive
 *   trend flat     (#64748B slate-500) → textMuted
 */
export function buildMetricComparisonV1(params: MetricComparisonV1Params): ElementTree {
  const trend = params.trend ?? (params.change ? 'flat' : 'flat');
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const labelColor = isLight ? '#64748B' : t.colors.textMuted;
  const trendColor = trendColorFor(trend, isLight, t);
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
        fill: [{ type: 'solid', color: labelColor }],
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

function trendColorFor(
  trend: MetricTrend,
  isLight: boolean,
  t: ReturnType<typeof resolveTheme>,
): string {
  switch (trend) {
    case 'up':
      return isLight ? '#10B981' : t.colors.success;
    case 'down':
      return isLight ? '#EF4444' : t.colors.destructive;
    case 'flat':
    default:
      return isLight ? '#64748B' : t.colors.textMuted;
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
