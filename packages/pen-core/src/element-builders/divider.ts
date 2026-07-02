import type { ElementTree } from './helpers.js';

export type DividerOrientation = 'horizontal' | 'vertical';

export interface DividerParams {
  orientation?: DividerOrientation;
  thickness?: number;
}

/**
 * Hairline divider. Horizontal = fill_container × thickness px;
 * vertical = thickness px × fill_container. Fills inherited from
 * ambient theme / Style Guide via a follow-up batch_design U-op.
 */
export function buildDivider(params: DividerParams): ElementTree {
  const orientation = params.orientation ?? 'horizontal';
  const thickness = params.thickness ?? 1;
  return {
    type: 'rectangle',
    name: 'Divider',
    role: 'divider',
    width: orientation === 'horizontal' ? 'fill_container' : thickness,
    height: orientation === 'horizontal' ? thickness : 'fill_container',
  };
}
