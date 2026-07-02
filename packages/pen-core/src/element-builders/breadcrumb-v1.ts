import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface BreadcrumbV1Item {
  label: string;
  active?: boolean;
}

export interface BreadcrumbV1Params {
  items: BreadcrumbV1Item[];
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_breadcrumb_v0.
   * - `'dark'`: byte-parity with light (breadcrumb has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildBreadcrumb` emits no fill literals — text colors are inherited
   * from the canvas default / parent context. Theme param is accepted for
   * API consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Breadcrumb trail — theme-aware version of buildBreadcrumb.
 * Light mode is byte-equal to add_breadcrumb_v0.
 *
 * Since buildBreadcrumb emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildBreadcrumbV1(params: BreadcrumbV1Params): ElementTree {
  const children: ElementTree[] = [];
  const lastIdx = params.items.length - 1;
  params.items.forEach((item, i) => {
    const isActive = item.active === true || i === lastIdx;
    children.push({
      type: 'text',
      name: `Item (${item.label})`,
      role: isActive ? 'breadcrumb-item-active' : 'breadcrumb-item',
      content: item.label,
      fontSize: 13,
      fontWeight: isActive ? 600 : 400,
    });
    if (i < lastIdx) {
      children.push({
        type: 'icon_font',
        name: 'Separator',
        role: 'breadcrumb-separator',
        iconFontName: 'chevron-right',
        iconFontFamily: 'lucide',
        width: 14,
        height: 14,
      });
    }
  });
  return {
    type: 'frame',
    name: 'Breadcrumb',
    role: 'breadcrumb',
    width: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 6,
    children,
  };
}
