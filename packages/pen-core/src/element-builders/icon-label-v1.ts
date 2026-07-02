import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface IconLabelV1Params {
  icon: string;
  label: string;
  gap?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_icon_label_v0.
   * - `'dark'`: byte-parity with light (icon_label has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildIconLabel` emits no fill literals — icon and text colors are
   * inherited from the canvas default / parent context. Theme param is
   * accepted for API consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Atomic icon + label pair — theme-aware version of buildIconLabel.
 * Light mode is byte-equal to add_icon_label_v0.
 *
 * Since buildIconLabel emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildIconLabelV1(params: IconLabelV1Params): ElementTree {
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
