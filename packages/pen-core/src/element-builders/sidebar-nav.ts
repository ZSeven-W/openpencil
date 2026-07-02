import type { ElementTree } from './helpers.js';

export interface SidebarNavItem {
  label: string;
  icon: string;
  active?: boolean;
}

export interface SidebarNavParams {
  items: SidebarNavItem[];
  /** Optional brand / section title row above the items. */
  title?: string;
  /** Sidebar width in px. Default 240. Clamped 180..320. */
  width?: number;
}

/**
 * Persistent vertical sidebar navigation — desktop / dashboard left
 * rail. Stack of icon+label rows with an optional brand/title row at
 * top. Active item gets a slate-100 pill bg + bolder, darker label;
 * inactive items have no fill and a muted slate label.
 *
 * Distinct from `add_bottom_nav_v0` (mobile bottom tab bar — horizontal
 * flow, label below icon) and `add_top_nav_bar_v0` (mobile top header
 * with single centered title). Use this for dashboards, settings rails,
 * docs sidebars, admin panels.
 *
 * Structure:
 *   frame(role='sidebar-nav', width, fill_container, fill=[#FFFFFF],
 *         layout=vertical, gap=4, padding=[16,12])
 *     ├ optional frame(role='sidebar-nav-title') > text(title, 16/700)
 *     └ N × frame(role='sidebar-nav-item' | 'sidebar-nav-item-active',
 *                 fill_container, h=40, cornerRadius=8,
 *                 layout=horizontal, alignItems=center, gap=12,
 *                 padding=[0,12], optional fill=[#F1F5F9] when active)
 *           ├ icon_font(18, lucide, role='sidebar-nav-icon')
 *           └ text(label, 14/500 or 14/600 if active)
 *
 * NOTE: pen-core's layout engine only reads the unified `padding`
 * field (`number | [T,R] | [T,R,B,L] | string`); the CSS-style
 * `paddingTop/Right/Bottom/Left` siblings are silently dropped. Stay
 * on the array form here so insets actually render.
 */
export function buildSidebarNav(params: SidebarNavParams): ElementTree {
  const width = Math.min(320, Math.max(180, Math.floor(params.width ?? 240)));
  const children: ElementTree[] = [];
  if (params.title) {
    children.push({
      type: 'frame',
      name: 'Sidebar Title',
      role: 'sidebar-nav-title',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      padding: [8, 12, 24, 12],
      children: [
        {
          type: 'text',
          name: 'Title',
          role: 'sidebar-nav-title-text',
          content: params.title,
          fontSize: 16,
          fontWeight: 700,
          fill: [{ type: 'solid', color: '#0F172A' }],
        },
      ],
    });
  }
  for (const item of params.items) {
    children.push(buildItem(item));
  }
  return {
    type: 'frame',
    name: 'Sidebar Nav',
    role: 'sidebar-nav',
    width,
    height: 'fill_container',
    layout: 'vertical',
    gap: 4,
    padding: [16, 12],
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    children,
  };
}

function buildItem(item: SidebarNavItem): ElementTree {
  const active = item.active === true;
  const node: ElementTree = {
    type: 'frame',
    name: `Item (${item.label})`,
    role: active ? 'sidebar-nav-item-active' : 'sidebar-nav-item',
    width: 'fill_container',
    height: 40,
    cornerRadius: 8,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    padding: [0, 12],
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        role: 'sidebar-nav-icon',
        iconFontName: item.icon,
        iconFontFamily: 'lucide',
        width: 18,
        height: 18,
      },
      {
        type: 'text',
        name: 'Label',
        role: 'sidebar-nav-label',
        content: item.label,
        fontSize: 14,
        fontWeight: active ? 600 : 500,
        fill: [{ type: 'solid', color: active ? '#0F172A' : '#475569' }],
      },
    ],
  };
  if (active) {
    node.fill = [{ type: 'solid', color: '#F1F5F9' }];
  }
  return node;
}
