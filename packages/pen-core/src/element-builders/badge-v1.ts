import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface BadgeV1Params {
  label: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_badge_v0.
   * - `'dark'`: byte-parity with light (badge has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildBadge` emits no fill literals — colors are applied by the caller
   * via a follow-up batch_design U-op. Theme param is accepted for API
   * consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Short inline badge / pill / tag — theme-aware version of buildBadge.
 * Light mode is byte-equal to add_badge_v0.
 *
 * Since buildBadge emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildBadgeV1(params: BadgeV1Params): ElementTree {
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
