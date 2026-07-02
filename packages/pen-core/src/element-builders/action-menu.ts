import type { ElementTree } from './helpers.js';

export interface ActionMenuItem {
  /** Label shown inline. Required. */
  label: string;
  /** Optional leading lucide icon. */
  icon?: string;
  /** When true, renders the item in red (destructive variant). */
  destructive?: boolean;
  /** When true, a 1px divider is drawn ABOVE this item. Use to group sections. */
  divider_before?: boolean;
}

export interface ActionMenuParams {
  /** Menu items (at least one). */
  items: ActionMenuItem[];
  /** Panel width in px. Default 200. Min 140. */
  width?: number;
}

/**
 * Action / context menu panel — the floating card that drops down
 * from a "⋯ more" button or right-click. Rendered as an open / fully-
 * expanded state; positioning and show/hide animation are caller /
 * runtime concerns. Each item is a padded row (icon + label), with
 * destructive items rendered red and optional dividers between
 * groups.
 *
 * Structure:
 *   frame(w, fit_content, cornerRadius=10, bg=#FFFFFF, stroke=#E2E8F0,
 *         shadow, vertical, padding=[8,0], role='action-menu')
 *     ├ rectangle(h=1, fill_container, #E2E8F0, role='action-menu-divider')  ← optional
 *     ├ frame(fill_container, horizontal, padding=[8,12], gap=10, align=center, role='action-menu-item')
 *     │   ├ icon_font(icon, 18×18)  ← optional
 *     │   └ text(label, 14/500, destructive? #EF4444 : #0F172A)
 *     └ ... more items
 *
 * Destructive items use `role: 'action-menu-item-destructive'` so
 * callers can restyle (e.g. hover:bg-red-50 in the renderer) without
 * a structural change.
 */
export function buildActionMenu(params: ActionMenuParams): ElementTree {
  const width = Math.max(140, Math.floor(params.width ?? 200));
  const items = params.items ?? [];

  const children: ElementTree[] = [];
  items.forEach((item, i) => {
    if (item.divider_before && i > 0) {
      children.push({
        type: 'rectangle',
        name: 'Divider',
        role: 'action-menu-divider',
        width: 'fill_container',
        height: 1,
        fill: [{ type: 'solid', color: '#E2E8F0' }],
      });
    }
    const itemChildren: ElementTree[] = [];
    if (item.icon) {
      itemChildren.push({
        type: 'icon_font',
        name: 'Icon',
        iconFontName: item.icon,
        iconFontFamily: 'lucide',
        width: 18,
        height: 18,
        fill: [{ type: 'solid', color: item.destructive ? '#EF4444' : '#334155' }],
      });
    }
    itemChildren.push({
      type: 'text',
      name: 'Label',
      content: item.label,
      fontSize: 14,
      fontWeight: 500,
      fill: [{ type: 'solid', color: item.destructive ? '#EF4444' : '#0F172A' }],
    });
    children.push({
      type: 'frame',
      name: `Item (${item.label})`,
      role: item.destructive ? 'action-menu-item-destructive' : 'action-menu-item',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      gap: 10,
      padding: [8, 12],
      children: itemChildren,
    });
  });

  return {
    type: 'frame',
    name: 'Action Menu',
    role: 'action-menu',
    width,
    height: 'fit_content',
    layout: 'vertical',
    padding: [8, 0],
    cornerRadius: 10,
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
    effects: [
      {
        type: 'shadow',
        color: 'rgba(15, 23, 42, 0.08)',
        offsetX: 0,
        offsetY: 8,
        blur: 24,
      },
    ],
    children,
  };
}
