import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type StatCardV1Trend = 'up' | 'down' | 'flat';

export interface StatCardV1Params {
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
  trend?: StatCardV1Trend;
  /** Card width in px. Default 240. Min 160. */
  width?: number;
  /** Corner radius. Default 16. */
  corner_radius?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_stat_card_v0.
   * - `'dark'`: card bg → surface; border → border; label → textMuted;
   *   icon fill → textSubtle; value → textPrimary. Delta tone colors
   *   (success/destructive/slate) are kept hardcoded — status semantics.
   * - `'system'`: $color-* refs for surface/border/text slots.
   */
  theme?: V1Theme;
}

/**
 * Stat card (v1) — theme-aware variant of buildStatCard.
 * Light mode is byte-equal to add_stat_card_v0.
 *
 * Color mapping:
 *   card bg     (#FFFFFF white)        → surface token
 *   border      (#E2E8F0 slate-200)    → border token
 *   label text  (#64748B slate-500)    → textMuted token
 *   icon fill   (#94A3B8 slate-400)    → textSubtle token
 *   value text  (#0F172A slate-900)    → textPrimary token
 *
 * Delta colors stay hardcoded (status semantics, theme-independent):
 *   up    → #10B981 (emerald-500)
 *   down  → #EF4444 (red-500)
 *   flat  → #64748B (slate-500)
 */
export function buildStatCardV1(params: StatCardV1Params): ElementTree {
  const width = Math.max(160, Math.floor(params.width ?? 240));
  const cornerRadius = Math.max(0, Math.floor(params.corner_radius ?? 16));
  const trend: StatCardV1Trend = params.trend ?? 'flat';
  const deltaColor = trend === 'up' ? '#10B981' : trend === 'down' ? '#EF4444' : '#64748B';
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Card bg: light → #FFFFFF, dark/system → surface
  const cardBg = isLight ? '#FFFFFF' : t.colors.surface;
  // Border: light → slate-200 (#E2E8F0), dark/system → border
  const borderColor = isLight ? '#E2E8F0' : t.colors.border;
  // Label text: light → slate-500 (#64748B), dark/system → textMuted
  const labelColor = isLight ? '#64748B' : t.colors.textMuted;
  // Icon fill: light → slate-400 (#94A3B8), dark/system → textSubtle
  const iconColor = isLight ? '#94A3B8' : t.colors.textSubtle;
  // Value text: light → slate-900 (#0F172A), dark/system → textPrimary
  const valueColor = isLight ? '#0F172A' : t.colors.textPrimary;

  const headerChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Label',
      role: 'stat-card-label',
      content: params.label.toUpperCase(),
      fontSize: 12,
      fontWeight: 500,
      letterSpacing: 1,
      fill: [{ type: 'solid', color: labelColor }],
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
      fill: [{ type: 'solid', color: iconColor }],
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
      fill: [{ type: 'solid', color: valueColor }],
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
    fill: [{ type: 'solid', color: cardBg }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: borderColor }] },
    children: cardChildren,
  };
}
