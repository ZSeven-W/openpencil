import type { ElementTree } from './helpers.js';

export interface IconButtonParams {
  icon: string;
  size?: number;
  icon_size?: number;
}

/**
 * Icon-only button — 44×44 default (Apple HIG / Material min-hit-target)
 * with flex-centered icon. Never use layout=none + manual x/y (renderer
 * unreliability documented in pen-ai-skills memory).
 */
export function buildIconButton(params: IconButtonParams): ElementTree {
  const size = params.size ?? 44;
  const iconSize = params.icon_size ?? 24;
  return {
    type: 'frame',
    name: 'Icon Button',
    role: 'icon-button',
    width: size,
    height: size,
    layout: 'horizontal',
    justifyContent: 'center',
    alignItems: 'center',
    cornerRadius: 8,
    children: [
      {
        type: 'icon_font',
        name: 'Icon',
        iconFontName: params.icon,
        iconFontFamily: 'lucide',
        width: iconSize,
        height: iconSize,
      },
    ],
  };
}
