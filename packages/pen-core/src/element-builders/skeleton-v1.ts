import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface SkeletonV1Params {
  /** Number of skeleton rows to emit (clamped 1..20). Default 3. */
  rows?: number;
  /** Height per row in px (clamped 4..48). Default 16. */
  row_height?: number;
  /** Gap between rows in px (clamped 0..32). Default 12. */
  row_gap?: number;
  /**
   * When true, the LAST row is 60% width (simulates an unfinished
   * paragraph line — the classic "shimmer" pattern). Default true.
   */
  last_row_short?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_skeleton_v0 (slate-200 rows).
   * - `'dark'`: row fill → surface2 (#334155) — visible against dark page bg.
   * - `'system'`: $color-surface-2 ref for row fills.
   */
  theme?: V1Theme;
}

/**
 * Loading skeleton (v1) — theme-aware variant of buildSkeleton.
 * Light mode is byte-equal to add_skeleton_v0.
 *
 * Color mapping:
 *   skeleton row fill (#E2E8F0 slate-200) → surface2 token
 */
export function buildSkeletonV1(params: SkeletonV1Params): ElementTree {
  const rows = Math.max(1, Math.min(20, params.rows ?? 3));
  const rowHeight = Math.max(4, Math.min(48, params.row_height ?? 16));
  const rowGap = Math.max(0, Math.min(32, params.row_gap ?? 12));
  const lastShort = params.last_row_short ?? true;
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Row fill: light → slate-200 (#E2E8F0), dark/system → surface2
  const rowFill = isLight ? '#E2E8F0' : t.colors.surface2;

  const children: ElementTree[] = [];
  for (let i = 0; i < rows; i++) {
    const isLast = i === rows - 1;
    if (isLast && lastShort && rows > 1) {
      children.push({
        type: 'rectangle',
        role: 'skeleton-row',
        width: 220,
        height: rowHeight,
        cornerRadius: 4,
        fill: [{ type: 'solid', color: rowFill }],
      });
    } else {
      children.push({
        type: 'rectangle',
        role: 'skeleton-row',
        width: 'fill_container',
        height: rowHeight,
        cornerRadius: 4,
        fill: [{ type: 'solid', color: rowFill }],
      });
    }
  }

  return {
    type: 'frame',
    name: 'Skeleton',
    role: 'skeleton',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: rowGap,
    alignItems: 'start',
    children,
  };
}
