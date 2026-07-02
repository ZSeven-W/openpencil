import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export type DividerV1Orientation = 'horizontal' | 'vertical';

export interface DividerV1Params {
  orientation?: DividerV1Orientation;
  thickness?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_divider_v0.
   * - `'dark'`: byte-parity with light (divider has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildDivider` emits no fill literals — colors are inherited from ambient
   * theme / Style Guide via a follow-up batch_design U-op. Theme param is
   * accepted for API consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Hairline divider — theme-aware version of buildDivider.
 * Light mode is byte-equal to add_divider_v0.
 *
 * Since buildDivider emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildDividerV1(params: DividerV1Params): ElementTree {
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
