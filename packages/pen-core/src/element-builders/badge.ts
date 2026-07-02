import type { ElementTree } from './helpers.js';

export interface BadgeParams {
  label: string;
}

/**
 * Short inline badge / pill / tag (e.g. "NEW", "BETA", "42").
 * Forces the standard pill: horizontal frame with padding=[4,10],
 * cornerRadius=999 (full pill), alignItems=center, fontSize 11, weight 600.
 * Colors kept neutral — override via batch_design U-op.
 */
export function buildBadge(params: BadgeParams): ElementTree {
  return {
    type: 'frame',
    name: 'Badge',
    role: 'badge',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    padding: [4, 10],
    cornerRadius: 999,
    children: [
      {
        type: 'text',
        name: 'Label',
        role: 'label',
        content: params.label,
        fontSize: 11,
        fontWeight: 600,
      },
    ],
  };
}
