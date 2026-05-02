import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface StatGridV1Item {
  value: string;
  label: string;
  icon?: string;
}

export interface StatGridV1Params {
  items: StatGridV1Item[];
  gap?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_stat_grid_v0.
   * - `'dark'`: identical (no explicit fill colors in v0 — text inherits).
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Stat grid (v1) — theme-aware variant of buildStatGrid.
 * No hardcoded colors in v0 (text inherits canvas theme color),
 * so all theme modes are identical and byte-parity with v0 is guaranteed.
 * Accepts theme param for API consistency.
 *
 * Non-scrolling stat grid — 2-5 items share the row via fill_container.
 */
export function buildStatGridV1(params: StatGridV1Params): ElementTree {
  const gap = params.gap ?? 16;
  const cells = params.items.map((item) => buildCellV1(item));
  return {
    type: 'frame',
    name: 'Stat Grid',
    role: 'stat-grid',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    gap,
    alignItems: 'center',
    justifyContent: 'space_between',
    children: cells,
  };
}

function buildCellV1(item: StatGridV1Item): ElementTree {
  const children: ElementTree[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  children.push({
    type: 'text',
    name: 'Value',
    role: 'heading',
    content: item.value,
    fontSize: 24,
    fontWeight: 700,
    width: 'fill_container',
  });
  children.push({
    type: 'text',
    name: 'Label',
    role: 'body',
    content: item.label,
    fontSize: 12,
    fontWeight: 500,
    width: 'fill_container',
  });
  return {
    type: 'frame',
    name: 'Stat Cell',
    role: 'stat-cell',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
