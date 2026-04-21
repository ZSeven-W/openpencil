import type { ElementTree } from './helpers.js';

export interface TopNavBarParams {
  title: string;
  leading_icon?: string;
  trailing_icon?: string;
  height?: number;
}

/**
 * Mobile top navigation bar: optional leading icon (back/menu) +
 * centered title + optional trailing icon (search/more). Dual of
 * bottom-nav. 44×44 hit targets for icons (Apple HIG + Material).
 * Empty slots become same-footprint spacers so the title stays centered.
 */
export function buildTopNavBar(params: TopNavBarParams): ElementTree {
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
