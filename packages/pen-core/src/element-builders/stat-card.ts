import type { ElementTree } from './helpers.js';

export type StatCardTrend = 'up' | 'down' | 'flat';

export interface StatCardParams {
  /** Small label above the big number (e.g. "Monthly revenue"). Required. */
  label: string;
  /** The big number / primary metric (e.g. "$12.4k", "1,284", "98.2%"). Required. */
  value: string;
  /** Optional leading icon (lucide name, e.g. "trending-up" / "users" / "flame"). */
  icon?: string;
  /**
   * Optional delta text shown beneath the big number (e.g. "+8% vs last week").
   * Rendered in a tone-matched color.
   */
  delta?: string;
  /** Trend direction for the delta tone. Default 'flat'. */
  trend?: StatCardTrend;
  /** Card width in px. Default 240. Min 160. */
  width?: number;
  /** Corner radius. Default 16. */
  corner_radius?: number;
}

/**
 * Big-number stat card — a single standalone metric tile with label
 * above and optional delta below the big number. Different from its
 * siblings:
 *
 *   - `add_stat_grid_v0`: 2-5 mini-stats side-by-side in one strip
 *     (cell metric is small; designed for "at-a-glance summary bar")
 *   - `add_metric_comparison_v0`: labeled KPI with explicit trend
 *     arrow inline next to the value ("$12k ↑ 8%") — horizontal-
 *     compact row shape
 *   - `add_stat_card_v0` (this tool): the whole-card featured metric
 *     — big 32-size number, label above, delta below, icon in corner
 *
 * Dashboard tiles, KPI widgets, "your XYZ today" card components.
 *
 * Structure:
 *   frame(width, fit_content, cornerRadius, padding=20, bg=#FFFFFF,
 *         stroke=#E2E8F0, layout=vertical, gap=12, role='stat-card')
 *     ├ frame(horizontal, space-between, align=center, role='stat-card-header')
 *     │   ├ text(label, 12/500 muted uppercase, role='stat-card-label')
 *     │   └ icon_font(icon, 18×18, muted, role='stat-card-icon')   ← optional
 *     ├ text(value, 32/700 slate-900, role='stat-card-value')
 *     └ text(delta, 12/500 tone-color, role='stat-card-delta')     ← optional
 *
 * Trend tones (delta color only — value is always slate-900):
 *   up    → #10B981 (emerald-500)
 *   down  → #EF4444 (red-500)
 *   flat  → #64748B (slate-500)
 */
export function buildStatCard(params: StatCardParams): ElementTree {
  const width = Math.max(160, Math.floor(params.width ?? 240));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 16));
  const trend: StatCardTrend = params.trend ?? 'flat';
  const deltaColor = trend === 'up' ? '#10B981' : trend === 'down' ? '#EF4444' : '#64748B';

  const headerChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Label',
      role: 'stat-card-label',
      content: params.label.toUpperCase(),
      fontSize: 12,
      fontWeight: 500,
      letterSpacing: 1,
      fill: [{ type: 'solid', color: '#64748B' }],
    },
  ];
  if (params.icon) {
    headerChildren.push({
      type: 'icon_font',
      name: 'Icon',
      role: 'stat-card-icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 18,
      height: 18,
      fill: [{ type: 'solid', color: '#94A3B8' }],
    });
  }

  const cardChildren: ElementTree[] = [
    {
      type: 'frame',
      name: 'Header',
      role: 'stat-card-header',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'space-between',
      children: headerChildren,
    },
    {
      type: 'text',
      name: 'Value',
      role: 'stat-card-value',
      content: params.value,
      fontSize: 32,
      fontWeight: 700,
      lineHeight: 1.1,
      fill: [{ type: 'solid', color: '#0F172A' }],
    },
  ];

  if (params.delta) {
    cardChildren.push({
      type: 'text',
      name: 'Delta',
      role: 'stat-card-delta',
      content: params.delta,
      fontSize: 12,
      fontWeight: 500,
      fill: [{ type: 'solid', color: deltaColor }],
    });
  }

  return {
    type: 'frame',
    name: 'Stat Card',
    role: 'stat-card',
    width,
    height: 'fit_content',
    cornerRadius,
    layout: 'vertical',
    gap: 12,
    padding: 20,
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
    children: cardChildren,
  };
}
