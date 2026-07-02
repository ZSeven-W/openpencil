import type { ElementTree } from './helpers.js';

export interface LinkParams {
  label: string;
  trailing_icon?: string;
}

/**
 * Inline text link with optional trailing icon — "Learn more →".
 * Roles stay Style-Guide orthogonal so colors + underline get
 * applied via batch_design U-op later.
 */
export function buildLink(params: LinkParams): ElementTree {
  const children: ElementTree[] = [
    {
      type: 'text',
      name: 'Label',
      role: 'link-label',
      content: params.label,
      fontSize: 14,
      fontWeight: 500,
    },
  ];
  if (params.trailing_icon) {
    children.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      role: 'link-icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 14,
      height: 14,
    });
  }
  return {
    type: 'frame',
    name: 'Link',
    role: 'link',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
