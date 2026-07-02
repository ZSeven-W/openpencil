import type { ElementTree } from './helpers.js';

export interface FabParams {
  icon: string;
  size?: number;
}

/**
 * Floating action button — circular 56×56 (Material FAB default)
 * with centered icon at ~43% of button size.
 */
export function buildFab(params: FabParams): ElementTree {
  const size = params.size ?? 56;
  const iconSize = Math.round(size * 0.43);
  return {
    type: 'frame',
    name: 'FAB',
    role: 'fab',
    width: size,
    height: size,
    cornerRadius: size / 2,
    fill: [{ type: 'solid', color: '#2563EB' }],
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: params.icon,
        iconFontFamily: 'lucide',
        width: iconSize,
        height: iconSize,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
      },
    ],
  };
}
