import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface PaginationV1Params {
  /** Total number of pages. Clamped to >= 1. */
  total: number;
  /** 1-based index of the current page. Clamped to [1, total]. */
  current?: number;
  /**
   * How many pages to show on each side of `current` before collapsing
   * the rest into an ellipsis. Default 1 → e.g. "1 … 4 [5] 6 … 10".
   */
  siblings?: number;
  /** Include arrow buttons on either end. Default true. */
  show_arrows?: boolean;
  /** Accent color for the active page pill. Default slate-900. */
  accent_color?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_pagination_v0.
   * - `'dark'`: arrow/inactive text → textBody, ellipsis → textMuted,
   *   active pill bg → accent_color (brand-invariant), active text → #FFFFFF.
   * - `'system'`: emits `$color-*` refs for fill fields.
   */
  theme?: V1Theme;
}

/**
 * Pagination bar (v1) — theme-aware variant of buildPagination.
 * Light mode is byte-equal to add_pagination_v0.
 *
 * Color mapping:
 *   arrow / inactive page text (#334155 slate-700) → textBody
 *   ellipsis text              (#64748B slate-500)  → textMuted
 *   active pill bg             (accent_color / #0F172A) — brand-invariant, kept
 *   active pill text           (#FFFFFF white)      — white-on-accent, invariant
 */
export function buildPaginationV1(params: PaginationV1Params): ElementTree {
  const total = Math.max(1, Math.floor(params.total));
  const current = Math.max(1, Math.min(total, Math.floor(params.current ?? 1)));
  const siblings = Math.max(0, Math.floor(params.siblings ?? 1));
  const showArrows = params.show_arrows !== false;
  const accent = params.accent_color ?? '#0F172A';

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const arrowColor = isLight ? '#334155' : t.colors.textBody;
  const inactiveColor = isLight ? '#334155' : t.colors.textBody;
  const ellipsisColor = isLight ? '#64748B' : t.colors.textMuted;
  // Active pill bg stays as-is (brand-invariant accent); active text is always white
  const activePillText = '#FFFFFF';

  // Build the visible-page window: always include 1 and total, plus a
  // [current - siblings, current + siblings] band. Insert ellipses
  // where there's a gap.
  const pages = new Set<number>([1, total, current]);
  for (let i = 1; i <= siblings; i += 1) {
    pages.add(current - i);
    pages.add(current + i);
  }
  const sorted = Array.from(pages)
    .filter((p) => p >= 1 && p <= total)
    .sort((a, b) => a - b);

  type Entry = { kind: 'page'; n: number } | { kind: 'ellipsis' };
  const entries: Entry[] = [];
  for (let i = 0; i < sorted.length; i += 1) {
    entries.push({ kind: 'page', n: sorted[i] });
    const next = sorted[i + 1];
    if (next !== undefined && next > sorted[i] + 1) {
      entries.push({ kind: 'ellipsis' });
    }
  }

  const children: ElementTree[] = [];

  if (showArrows) {
    children.push({
      type: 'frame',
      name: 'Prev',
      role: 'pagination-prev',
      width: 32,
      height: 32,
      cornerRadius: 6,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: [],
      children: [
        {
          type: 'icon_font',
          name: 'Prev Icon',
          iconFontName: 'chevron-left',
          iconFontFamily: 'lucide',
          width: 16,
          height: 16,
          fill: [{ type: 'solid', color: arrowColor }],
        },
      ],
    });
  }

  for (const e of entries) {
    if (e.kind === 'ellipsis') {
      children.push({
        type: 'text',
        name: 'Ellipsis',
        role: 'pagination-ellipsis',
        content: '…',
        fontSize: 13,
        fontWeight: 400,
        fill: [{ type: 'solid', color: ellipsisColor }],
      });
      continue;
    }
    const isActive = e.n === current;
    children.push({
      type: 'frame',
      name: `Page ${e.n}`,
      role: isActive ? 'pagination-page-active' : 'pagination-page',
      width: 36,
      height: 32,
      cornerRadius: 6,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: isActive ? [{ type: 'solid', color: accent }] : [],
      children: [
        {
          type: 'text',
          name: 'Label',
          content: String(e.n),
          fontSize: 13,
          fontWeight: isActive ? 600 : 400,
          fill: [{ type: 'solid', color: isActive ? activePillText : inactiveColor }],
        },
      ],
    });
  }

  if (showArrows) {
    children.push({
      type: 'frame',
      name: 'Next',
      role: 'pagination-next',
      width: 32,
      height: 32,
      cornerRadius: 6,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      fill: [],
      children: [
        {
          type: 'icon_font',
          name: 'Next Icon',
          iconFontName: 'chevron-right',
          iconFontFamily: 'lucide',
          width: 16,
          height: 16,
          fill: [{ type: 'solid', color: arrowColor }],
        },
      ],
    });
  }

  return {
    type: 'frame',
    name: 'Pagination',
    role: 'pagination',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
