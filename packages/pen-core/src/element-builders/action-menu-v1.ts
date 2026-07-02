import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ActionMenuV1Item {
  /** Label shown inline. Required. */
  label: string;
  /** Optional leading lucide icon. */
  icon?: string;
  /** When true, renders the item in red (destructive variant). */
  destructive?: boolean;
  /** When true, a 1px divider is drawn ABOVE this item. Use to group sections. */
  divider_before?: boolean;
}

export interface ActionMenuV1Params {
  /** Menu items (at least one). */
  items: ActionMenuV1Item[];
  /** Panel width in px. Default 200. Min 140. */
  width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_action_menu_v0.
   * - `'dark'`: dark surface + dark-mode text and border fills.
   * - `'system'`: emits `$color-*` ref strings for all fill fields.
   *
   * Destructive items always use `$color-destructive` / its dark-mode hex (#F87171)
   * as that is a semantic brand token, not a surface color.
   */
  theme?: V1Theme;
}

/**
 * Action / context menu panel — theme-aware version of buildActionMenu.
 * Light mode is byte-equal to add_action_menu_v0.
 */
export function buildActionMenuV1(params: ActionMenuV1Params): ElementTree {
  const width = Math.max(140, Math.floor(params.width ?? 200));
  const items = params.items ?? [];
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  // Light-mode constants (v0 byte-parity)
  const surfaceFill = isLight ? '#FFFFFF' : t.colors.surface;
  const borderFill = isLight ? '#E2E8F0' : t.colors.border;
  const iconFill = isLight ? '#334155' : t.colors.textBody;
  const labelFill = isLight ? '#0F172A' : t.colors.textPrimary;
  const destructiveFill = isLight ? '#EF4444' : t.colors.destructive;

  const children: ElementTree[] = [];
  items.forEach((item, i) => {
    if (item.divider_before && i > 0) {
      children.push({
        type: 'rectangle',
        name: 'Divider',
        role: 'action-menu-divider',
        width: 'fill_container',
        height: 1,
        fill: [{ type: 'solid', color: borderFill }],
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
        fill: [{ type: 'solid', color: item.destructive ? destructiveFill : iconFill }],
      });
    }
    itemChildren.push({
      type: 'text',
      name: 'Label',
      content: item.label,
      fontSize: 14,
      fontWeight: 500,
      fill: [{ type: 'solid', color: item.destructive ? destructiveFill : labelFill }],
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
    fill: [{ type: 'solid', color: surfaceFill }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: borderFill }] },
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
