import type { ElementTree } from './helpers.js';

export interface ToolbarItem {
  /** Lucide icon slug. */
  icon: string;
  /** Optional active state (slate-100 bg + slate-900 icon). */
  active?: boolean;
  /** Render a vertical divider AFTER this item. */
  divider_after?: boolean;
}

export interface ToolbarParams {
  items: ToolbarItem[];
}

/**
 * Desktop toolbar — horizontal row of 36×36 icon buttons with optional
 * vertical dividers between groups. Distinct from `add_top_nav_bar_v0`
 * (mobile single-row header with title) and `add_icon_button_v0`
 * (single 44×44 button, no row). Use for editor toolbars, formatting
 * controls, kanban board action strips.
 */
export function buildToolbar(params: ToolbarParams): ElementTree {
  const children: ElementTree[] = [];
  params.items.forEach((item, i) => {
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
      ...(item.active ? { fill: [{ type: 'solid', color: '#F1F5F9' }] } : {}),
      children: [
        {
          type: 'icon_font',
          name: 'Icon',
          iconFontName: item.icon,
          iconFontFamily: 'lucide',
          width: 18,
          height: 18,
          fill: [{ type: 'solid', color: item.active ? '#0F172A' : '#475569' }],
        },
      ],
    });
    if (item.divider_after && i < params.items.length - 1) {
      children.push({
        type: 'frame',
        name: 'Toolbar Divider',
        role: 'toolbar-divider',
        width: 1,
        height: 20,
        fill: [{ type: 'solid', color: '#E2E8F0' }],
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
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
    children,
  };
}
