import { coerceNavTabIcon } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface BottomNavV1Item {
  title: string;
  icon: string;
  active?: boolean;
}

export interface BottomNavV1Params {
  items: BottomNavV1Item[];
  height?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_bottom_nav_v0.
   * - `'dark'`: byte-parity with light (bottom nav has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildBottomNav` emits no fill literals — colors are inherited from
   * ambient theme / Style Guide via a follow-up batch_design U-op. Theme
   * param is accepted for API consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Inline bottom navigation bar — theme-aware version of buildBottomNav.
 * Light mode is byte-equal to add_bottom_nav_v0.
 *
 * Since buildBottomNav emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildBottomNavV1(params: BottomNavV1Params): ElementTree {
  const height = params.height ?? 62;
  const tabs = params.items.map((item) => buildTab(item));
  return {
    type: 'frame',
    name: 'Bottom Tab Bar',
    role: 'bottom-tab-bar',
    width: 'fill_container',
    height,
    layout: 'horizontal',
    justifyContent: 'space_around',
    alignItems: 'center',
    children: tabs,
  };
}

function buildTab(item: BottomNavV1Item): ElementTree {
  // Common slip: model emits icon='shopping-bag' for a Cart tab. Bag and
  // cart are visually different glyphs in lucide; the cart tab gets a bag
  // shape and the user-reported "icon shape wrong" bug. coerceNavTabIcon
  // normalises a small set of known wrong-glyph choices per title without
  // touching legitimate custom icons.
  const icon = coerceNavTabIcon(item.title, item.icon, 'buildBottomNavV1');
  return {
    type: 'frame',
    name: `Tab (${item.title})`,
    role: item.active ? 'nav-item-active' : 'nav-item',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 4,
    padding: [4, 12],
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 24,
        height: 24,
      },
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: item.title,
        fontSize: 11,
        fontWeight: item.active ? 600 : 500,
      },
    ],
  };
}
