import type { ElementTree } from './helpers.js';

export interface EmptyChartParams {
  /** Width in px. Default 320. Min 120. */
  width?: number;
  /** Height in px. Default 200. Min 100. */
  height?: number;
  /** Title shown beneath the icon. Default "No data yet". */
  title?: string;
  /** Subtitle / hint. Default "Data will appear here once tracking begins." */
  subtitle?: string;
  /** Icon to show above the title. Default "bar-chart-2". Use "line-chart" / "pie-chart" to match the widget it replaces. */
  icon?: string;
  /** Corner radius. Default 12. */
  corner_radius?: number;
}

/**
 * Empty state for chart widgets — a "no data yet" tile that sits in
 * the exact footprint where the real chart would go. Semantically
 * the same as `add_empty_state_v0` but structurally sized like a
 * chart (default 320×200 to match add_chart_line_v0 / add_chart_bars_v0
 * footprints) and wrapped in a dashed-border container so it reads
 * as "the space for a chart, currently empty" rather than a
 * generic empty state.
 *
 * Structure:
 *   frame(w×h, cornerRadius=12, dashed stroke, bg=slate-50, layout=vertical,
 *         center/center, gap=8, role='empty-chart')
 *     ├ icon_font(icon, 40×40, slate-400, role='empty-chart-icon')
 *     ├ text(title, 14/600, slate-700, role='empty-chart-title')
 *     └ text(subtitle, 12/400, slate-500, role='empty-chart-subtitle')
 *
 * For non-chart empty states (onboarding, inbox, search zero-results)
 * prefer `add_empty_state_v0` — it has optional CTA and doesn't have
 * the dashed border that specifically signals "chart slot".
 */
export function buildEmptyChart(params: EmptyChartParams): ElementTree {
  const width = Math.max(120, Math.floor(params.width ?? 320));
  const height = Math.max(100, Math.floor(params.height ?? 200));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 12));
  const title = params.title ?? 'No data yet';
  const subtitle = params.subtitle ?? 'Data will appear here once tracking begins.';
  const icon = params.icon ?? 'bar-chart-2';

  return {
    type: 'frame',
    name: 'Empty Chart',
    role: 'empty-chart',
    width,
    height,
    cornerRadius,
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
    paddingTop: 24,
    paddingBottom: 24,
    paddingLeft: 24,
    paddingRight: 24,
    fill: [{ type: 'solid', color: '#F8FAFC' }],
    stroke: {
      thickness: 1,
      fill: [{ type: 'solid', color: '#CBD5E1' }],
      strokeDashArray: [4, 4],
    },
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        role: 'empty-chart-icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 40,
        height: 40,
        fill: [{ type: 'solid', color: '#94A3B8' }],
      },
      {
        type: 'text',
        name: 'Title',
        role: 'empty-chart-title',
        content: title,
        fontSize: 14,
        fontWeight: 600,
        fill: [{ type: 'solid', color: '#334155' }],
      },
      {
        type: 'text',
        name: 'Subtitle',
        role: 'empty-chart-subtitle',
        content: subtitle,
        fontSize: 12,
        fontWeight: 400,
        fill: [{ type: 'solid', color: '#64748B' }],
      },
    ],
  };
}
