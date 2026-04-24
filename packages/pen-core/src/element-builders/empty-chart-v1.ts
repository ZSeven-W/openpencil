import type { ElementTree } from './helpers.js';

export type EmptyChartV1Theme = 'light' | 'dark' | 'system';

export interface EmptyChartV1Params {
  /** Width in px. Default 320. Min 120. */
  width?: number;
  /** Height in px. Default 200. Min 100. */
  height?: number;
  /** Title shown beneath the icon. Default "No data yet". */
  title?: string;
  /** Subtitle / hint. Default "Data will appear here once tracking begins." */
  subtitle?: string;
  /** Icon above the title. Default "bar-chart-2". Use "line-chart" / "pie-chart" to hint the widget type. */
  icon?: string;
  /** Corner radius. Default 12. */
  corner_radius?: number;
  /**
   * Theme variant. Default `'light'` — byte-parity with
   * `add_empty_chart_v0` so callers upgrade by changing the tool
   * name only.
   *
   * - `'light'`: v0 parity (slate-50 bg, slate-300 dashed border,
   *   slate-400 icon, slate-700 title, slate-500 subtitle)
   * - `'dark'`: hardcoded dark-mode hex (slate-800 bg, slate-600
   *   dashed border, slate-400 icon, slate-200 title, slate-400
   *   subtitle)
   * - `'system'`: emits `$color-surface-2` bg / `$color-border`
   *   dashed stroke / `$color-text-muted` icon+subtitle / `$color-text-primary`
   *   title refs; render tracks `themes.Mode` at paint time. Caller
   *   MUST have seeded the document with `applySemanticPalette()`.
   */
  theme?: EmptyChartV1Theme;
}

interface ResolvedColors {
  bg: string;
  border: string;
  icon: string;
  title: string;
  subtitle: string;
}

function resolveTheme(theme: EmptyChartV1Theme): ResolvedColors {
  if (theme === 'system') {
    return {
      bg: '$color-surface-2',
      border: '$color-border',
      icon: '$color-text-muted',
      title: '$color-text-primary',
      subtitle: '$color-text-muted',
    };
  }
  if (theme === 'dark') {
    return {
      bg: '#1E293B', // slate-800 — surface-2 Dark
      border: '#475569', // slate-600 — border Dark
      icon: '#94A3B8', // slate-400 — text-muted Dark
      title: '#E2E8F0', // slate-200 — text-primary Dark
      subtitle: '#94A3B8', // slate-400 — text-muted Dark
    };
  }
  // Default: light — byte-parity with v0
  return {
    bg: '#F8FAFC',
    border: '#CBD5E1',
    icon: '#94A3B8',
    title: '#334155',
    subtitle: '#64748B',
  };
}

/**
 * Theme-aware empty-chart placeholder (v1). Same dashed-border tile
 * shape as `add_empty_chart_v0` with an added `theme` param that
 * swaps the 5 colors (bg, border, icon, title, subtitle) while
 * preserving layout. Use inside dark-theme dashboards so the empty
 * slot doesn't punch a light rectangle out of the surrounding
 * cards.
 *
 * v0 parity: `buildEmptyChartV1({ ..., theme: 'light' })` (or theme
 * omitted) produces a tree byte-equivalent to v0 output, modulo ids.
 */
export function buildEmptyChartV1(params: EmptyChartV1Params): ElementTree {
  const width = Math.max(120, Math.floor(params.width ?? 320));
  const height = Math.max(100, Math.floor(params.height ?? 200));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 12));
  const title = params.title ?? 'No data yet';
  const subtitle = params.subtitle ?? 'Data will appear here once tracking begins.';
  const icon = params.icon ?? 'bar-chart-2';
  const theme = params.theme ?? 'light';
  const c = resolveTheme(theme);

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
    fill: [{ type: 'solid', color: c.bg }],
    stroke: {
      thickness: 1,
      fill: [{ type: 'solid', color: c.border }],
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
        fill: [{ type: 'solid', color: c.icon }],
      },
      {
        type: 'text',
        name: 'Title',
        role: 'empty-chart-title',
        content: title,
        fontSize: 14,
        fontWeight: 600,
        fill: [{ type: 'solid', color: c.title }],
      },
      {
        type: 'text',
        name: 'Subtitle',
        role: 'empty-chart-subtitle',
        content: subtitle,
        fontSize: 12,
        fontWeight: 400,
        fill: [{ type: 'solid', color: c.subtitle }],
      },
    ],
  };
}
