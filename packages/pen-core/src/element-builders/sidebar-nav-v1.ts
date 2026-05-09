import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface SidebarNavV1Item {
  label: string;
  icon: string;
  active?: boolean;
}

export interface SidebarNavV1Params {
  items: SidebarNavV1Item[];
  /** Optional brand / section title row above the items. */
  title?: string;
  /** Sidebar width in px. Default 240. Clamped 180..320. */
  width?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_sidebar_nav_v0.
   * - `'dark'`: sidebar bg → surface; title + active label → textPrimary;
   *   inactive label → textMuted; active item bg → surface2.
   * - `'system'`: $color-* refs for all color slots.
   */
  theme?: V1Theme;
}

/**
 * Sidebar navigation (v1) — theme-aware variant of buildSidebarNav.
 * Light mode is byte-equal to add_sidebar_nav_v0.
 *
 * Color mapping:
 *   sidebar bg         (#FFFFFF)         → surface token
 *   title text         (#0F172A slate-900) → textPrimary token
 *   active label       (#0F172A slate-900) → textPrimary token
 *   active item fill   (#F1F5F9 slate-100) → surface2 token
 *   inactive label     (#475569 slate-600) → textMuted token
 */
export function buildSidebarNavV1(params: SidebarNavV1Params): ElementTree {
  const width = Math.min(320, Math.max(180, Math.floor(params.width ?? 240)));
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Sidebar bg: light → #FFFFFF, dark/system → surface
  const sidebarBg = isLight ? '#FFFFFF' : t.colors.surface;
  // Title text: light → slate-900 (#0F172A), dark/system → textPrimary
  const titleColor = isLight ? '#0F172A' : t.colors.textPrimary;
  // Active label: light → slate-900 (#0F172A), dark/system → textPrimary
  const activeLabelColor = isLight ? '#0F172A' : t.colors.textPrimary;
  // Active item bg: light → slate-100 (#F1F5F9), dark/system → surface2
  const activeItemBg = isLight ? '#F1F5F9' : t.colors.surface2;
  // Inactive label: light → slate-600 (#475569), dark/system → textMuted
  const inactiveLabelColor = isLight ? '#475569' : t.colors.textMuted;

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
          fill: [{ type: 'solid', color: titleColor }],
        },
      ],
    });
  }
  for (const item of params.items) {
    children.push(buildItemV1(item, activeLabelColor, activeItemBg, inactiveLabelColor));
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
    fill: [{ type: 'solid', color: sidebarBg }],
    children,
  };
}

function buildItemV1(
  item: SidebarNavV1Item,
  activeLabelColor: string,
  activeItemBg: string,
  inactiveLabelColor: string,
): ElementTree {
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
        fill: [{ type: 'solid', color: active ? activeLabelColor : inactiveLabelColor }],
      },
    ],
  };
  if (active) {
    node.fill = [{ type: 'solid', color: activeItemBg }];
  }
  return node;
}
