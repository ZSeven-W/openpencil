import type { ElementTree } from './helpers.js';

export interface ColorSwatchParams {
  color: string;
  label?: string;
  size?: number;
}

/**
 * Design-system color swatch — one colored square (cornerRadius=12,
 * default 64px) with optional label underneath. `color` accepts
 * hex ("#2563EB") or $variable ref ("$color-primary"); pen-core's
 * resolver converts $refs to the active theme's color at render.
 */
export function buildColorSwatch(params: ColorSwatchParams): ElementTree {
  const size = Math.max(16, Math.floor(params.size ?? 64));
  const children: ElementTree[] = [
    {
      type: 'frame',
      name: 'Swatch Square',
      role: 'color-swatch-square',
      width: size,
      height: size,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: params.color }],
    },
  ];
  if (params.label) {
    children.push({
      type: 'text',
      name: 'Label',
      role: 'color-swatch-label',
      content: params.label,
      fontSize: 12,
      fontWeight: 500,
    });
  }
  return {
    type: 'frame',
    name: 'Color Swatch',
    role: 'color-swatch',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 8,
    children,
  };
}
