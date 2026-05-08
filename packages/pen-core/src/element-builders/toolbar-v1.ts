import { coerceNonEmptyArray } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ToolbarV1Item {
  /** Lucide icon slug. */
  icon: string;
  /** Optional active state. */
  active?: boolean;
  /** Render a vertical divider AFTER this item. */
  divider_after?: boolean;
}

export interface ToolbarV1Params {
  items: ToolbarV1Item[];
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_toolbar_v0.
   * - `'dark'`: dark-mode fills for surface, active bg, icon, divider.
   * - `'system'`: emits `$color-*` ref strings.
   */
  theme?: V1Theme;
}

/**
 * Desktop toolbar (v1) — theme-aware variant of buildToolbar.
 * Light mode is byte-equal to add_toolbar_v0.
 *
 * Color slots:
 * - Outer bg: #FFFFFF → surface
 * - Outer border: #E2E8F0 → border
 * - Active item bg: #F1F5F9 → surface-2
 * - Active icon fill: #0F172A → textPrimary
 * - Inactive icon fill: #475569 → textMuted
 * - Divider fill: #E2E8F0 → border
 */
export function buildToolbarV1(params: ToolbarV1Params): ElementTree {
  const items = coerceNonEmptyArray<ToolbarV1Item>(
    params.items,
    [{ icon: 'circle' }],
    'buildToolbarV1',
    'items',
  );
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);

  // Light mode: v0 literals for byte-parity
  const surface = theme === 'light' ? '#FFFFFF' : t.colors.surface;
  const border = theme === 'light' ? '#E2E8F0' : t.colors.border;
  const activeBg = theme === 'light' ? '#F1F5F9' : t.colors.surface2;
  const activeIcon = theme === 'light' ? '#0F172A' : t.colors.textPrimary;
  const inactiveIcon = theme === 'light' ? '#475569' : t.colors.textMuted;
  const divider = theme === 'light' ? '#E2E8F0' : t.colors.border;

  const children: ElementTree[] = [];
  items.forEach((item, i) => {
    children.push({
      type: 'frame',
      name: `Tool (${item.icon})`,
      role: item.active ? 'toolbar-item-active' : 'toolbar-item',
      width: 36,
      height: 36,
      cornerRadius: 6,
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      ...(item.active ? { fill: [{ type: 'solid', color: activeBg }] } : {}),
      children: [
        {
          type: 'icon_font',
          name: 'Icon',
          iconFontName: item.icon,
          iconFontFamily: 'lucide',
          width: 18,
          height: 18,
          fill: [{ type: 'solid', color: item.active ? activeIcon : inactiveIcon }],
        },
      ],
    });
    if (item.divider_after && i < items.length - 1) {
      children.push({
        type: 'frame',
        name: 'Toolbar Divider',
        role: 'toolbar-divider',
        width: 1,
        height: 20,
        fill: [{ type: 'solid', color: divider }],
        children: [],
      });
    }
  });
  return {
    type: 'frame',
    name: 'Toolbar',
    role: 'toolbar',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    padding: [4, 6],
    cornerRadius: 8,
    fill: [{ type: 'solid', color: surface }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: border }] },
    children,
  };
}
