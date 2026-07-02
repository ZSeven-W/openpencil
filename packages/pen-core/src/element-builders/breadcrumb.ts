import type { ElementTree } from './helpers.js';

export interface BreadcrumbItem {
  label: string;
  active?: boolean;
}

export interface BreadcrumbParams {
  items: BreadcrumbItem[];
}

/**
 * Breadcrumb trail: Home › Settings › Billing. Interleaves item
 * text with `chevron-right` separators. Last item (or one marked
 * active=true) gets fontWeight=600.
 */
export function buildBreadcrumb(params: BreadcrumbParams): ElementTree {
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
