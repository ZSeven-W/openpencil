import { buildScrollWrapper, type ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';
import type { NavChipRowItem } from './nav-chip-row.js';

export interface NavChipRowV1Params {
  items: NavChipRowItem[];
  chip_width?: number;
  gap?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_nav_chip_row_v0.
   * - `'dark'`: identical (v0 emits no hardcoded colors — chip has no
   *   fill; text inherits canvas default color).
   * - `'system'`: identical.
   * Accepts theme param for API consistency across all v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Horizontal scroll row of NAV CHIPS (v1) — theme-aware variant of buildNavChipRow.
 * No hardcoded colors in v0; light/dark/system modes are byte-identical.
 * Accepts theme param for API consistency.
 */
export function buildNavChipRowV1(params: NavChipRowV1Params): ElementTree {
  const chipWidth = params.chip_width ?? 72;
  const gap = params.gap ?? 12;
  const chips = params.items.map((item) => buildChip(item, chipWidth));
  return buildScrollWrapper({ rowName: 'Nav Chip Row', innerChildren: chips, gap });
}

function buildChip(item: NavChipRowItem, chipWidth: number): ElementTree {
  const children: ElementTree[] = [];
  if (item.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: item.icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
    });
  }
  children.push({
    type: 'text',
    name: 'Label',
    role: 'label',
    content: item.label,
    fontSize: 11,
    fontWeight: item.active ? 600 : 500,
  });
  return {
    type: 'frame',
    name: `Chip (${item.label})`,
    role: item.active ? 'nav-chip-active' : 'nav-chip',
    width: chipWidth,
    height: 'fit_content',
    cornerRadius: 12,
    padding: [8, 12],
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
