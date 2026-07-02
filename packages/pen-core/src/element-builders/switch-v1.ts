import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface SwitchV1Params {
  active?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_switch_v0.
   * - `'dark'`: identical (iOS HIG values #34C759/#E5E5EA are builder-private
   *   literals per spec §3.4 — not tokenized across theme modes).
   * - `'system'`: identical.
   *
   * NOTE: #34C759 (iOS green) and #E5E5EA (iOS off-gray) are iOS HIG
   * builder-private constants. #FFFFFF (thumb) is a gray-scale constant.
   * All stay hardcoded across all theme modes per spec §3.4.
   */
  theme?: V1Theme;
}

/**
 * Toggle switch (v1) — theme-aware variant of buildSwitch.
 * Light mode is byte-equal to add_switch_v0.
 *
 * iOS/Material toggle switch. Fixed 51×31 (iOS HIG), thumb 27×27 white.
 * active=true → iOS green track + thumb pushed right.
 *
 * #34C759 and #E5E5EA are iOS HIG builder-private literals — not surface
 * colors — so they remain hardcoded across all three theme modes.
 */
export function buildSwitchV1(params: SwitchV1Params): ElementTree {
  const active = params.active === true;
  return {
    type: 'frame',
    name: active ? 'Switch (on)' : 'Switch (off)',
    role: 'switch',
    width: 51,
    height: 31,
    cornerRadius: 16,
    fill: [{ type: 'solid', color: active ? '#34C759' : '#E5E5EA' }],
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: active ? 'flex-end' : 'flex-start',
    padding: [2],
    children: [
      {
        type: 'frame',
        name: 'Thumb',
        role: 'switch-thumb',
        width: 27,
        height: 27,
        cornerRadius: 14,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
      },
    ],
  };
}
