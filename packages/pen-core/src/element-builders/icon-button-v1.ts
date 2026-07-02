import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface IconButtonV1Params {
  icon: string;
  size?: number;
  icon_size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_icon_button_v0.
   * - `'dark'`: identical (no hardcoded colors in v0 — icon inherits
   *   text color from the canvas theme, frame has no fill).
   * - `'system'`: identical (no color refs needed).
   */
  theme?: V1Theme;
}

/**
 * Icon-only button — theme-aware variant of buildIconButton.
 * No hardcoded colors in v0, so light/dark/system modes are identical
 * (byte-parity with v0 in all modes). Accepts theme param for API
 * consistency across all v1 tools.
 *
 * 44×44 default (Apple HIG / Material min-hit-target) with flex-centered
 * icon. Never layout=none + manual x/y (renderer unreliability).
 */
export function buildIconButtonV1(params: IconButtonV1Params): ElementTree {
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
