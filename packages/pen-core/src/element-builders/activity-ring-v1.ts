import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface ActivityRingV1Params {
  center_text: string;
  size?: number;
  thickness?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_activity_ring_v0.
   * - `'dark'`: byte-parity with light (ring stroke color is intentionally
   *   theme-independent — #000000 is a colorless placeholder; callers
   *   override via batch_design U-op with the actual ring hue).
   * - `'system'`: byte-parity with light (no $color-* refs needed for
   *   the placeholder black stroke).
   *
   * `buildActivityRing` emits #000000 as a stroke placeholder per the
   * "Colorless by default" contract; all three theme modes produce identical
   * output. Theme param is accepted for API consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Apple-style progress ring with centered text — theme-aware version of
 * buildActivityRing. Light mode is byte-equal to add_activity_ring_v0.
 *
 * The #000000 ring stroke is a colorless placeholder (overridden by the
 * caller); it is intentionally theme-independent across all three modes.
 * The theme parameter exists for API consistency across all v1 tools.
 */
export function buildActivityRingV1(params: ActivityRingV1Params): ElementTree {
  const size = params.size ?? 80;
  const thickness = params.thickness ?? 8;
  return {
    type: 'frame',
    name: 'Activity Ring',
    role: 'activity-ring',
    width: size,
    height: size,
    cornerRadius: size / 2,
    fill: [],
    stroke: {
      thickness,
      fill: [{ type: 'solid', color: '#000000' }],
    },
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [
      {
        type: 'text',
        name: 'Center Text',
        role: 'heading',
        content: params.center_text,
        fontSize: 16,
        fontWeight: 700,
      },
    ],
  };
}
