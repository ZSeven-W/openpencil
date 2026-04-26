import type { ElementTree } from './helpers.js';

export interface BottomNavItem {
  title: string;
  icon: string;
  active?: boolean;
}

export interface BottomNavParams {
  items: BottomNavItem[];
  height?: number;
}

/**
 * Inline bottom navigation bar — 3-5 tab items, icon + label stack,
 * active tab gets `nav-item-active` role + fontWeight 600.
 *
 * Emits one frame with role='bottom-tab-bar'; caller inserts it as
 * the LAST child of the page (no spacer needed — spec §NO FIXED-POSITION).
 */
export function buildBottomNav(params: BottomNavParams): ElementTree {
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

function buildTab(item: BottomNavItem): ElementTree {
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
        iconFontName: item.icon,
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
