import type { ElementTree } from './helpers.js';

export interface IconLabelParams {
  icon: string;
  label: string;
  gap?: number;
}

/**
 * Atomic icon + label pair (horizontal). Common building block for
 * menu items, breadcrumbs, status indicators, inline "with icon"
 * text. alignItems=center so icon and text visually baseline-align.
 * Narrow: icons always lead; defaults 16/14/500. For variants use
 * batch_design.
 */
export function buildIconLabel(params: IconLabelParams): ElementTree {
  const gap = params.gap ?? 8;
  return {
    type: 'frame',
    name: 'Icon Label',
    role: 'icon-label',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap,
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: params.icon,
        iconFontFamily: 'lucide',
        width: 16,
        height: 16,
      },
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: params.label,
        fontSize: 14,
        fontWeight: 500,
      },
    ],
  };
}
