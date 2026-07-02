import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface TopNavBarV1Params {
  title: string;
  leading_icon?: string;
  trailing_icon?: string;
  height?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_top_nav_bar_v0.
   * - `'dark'`: identical — no hardcoded surface colors in v0.
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Mobile top navigation bar (v1) — theme-aware variant of buildTopNavBar.
 * Light mode is byte-equal to add_top_nav_bar_v0.
 *
 * No hardcoded surface colors in v0 (icons and title text inherit canvas default,
 * no fill/border on the bar). All theme modes produce identical trees.
 */
export function buildTopNavBarV1(params: TopNavBarV1Params): ElementTree {
  const height = params.height ?? 56;
  return {
    type: 'frame',
    name: 'Top Nav Bar',
    role: 'top-nav-bar',
    width: 'fill_container',
    height,
    layout: 'horizontal',
    justifyContent: 'space_between',
    alignItems: 'center',
    padding: [0, 16],
    children: [
      buildIconSlot(params.leading_icon, 'leading'),
      {
        type: 'text',
        name: 'Title',
        role: 'heading',
        content: params.title,
        fontSize: 17,
        fontWeight: 600,
      },
      buildIconSlot(params.trailing_icon, 'trailing'),
    ],
  };
}

function buildIconSlot(icon: string | undefined, position: 'leading' | 'trailing'): ElementTree {
  if (!icon) {
    return {
      type: 'frame',
      name: `${position} Spacer`,
      role: 'nav-spacer',
      width: 44,
      height: 44,
      layout: 'none',
      children: [],
    };
  }
  return {
    type: 'frame',
    name: `${position} Icon Button`,
    role: 'icon-button',
    width: 44,
    height: 44,
    layout: 'horizontal',
    justifyContent: 'center',
    alignItems: 'center',
    cornerRadius: 8,
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 24,
        height: 24,
      },
    ],
  };
}
