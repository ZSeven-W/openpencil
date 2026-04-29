import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface ColorSwatchV1Params {
  color: string;
  label?: string;
  size?: number;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_color_swatch_v0.
   * - `'dark'`: identical tree — color swatch passes `color` through unchanged.
   * - `'system'`: identical tree — color swatch passes `color` through unchanged.
   *
   * Note: `color` is intentionally theme-agnostic — the whole point of a
   * design-system swatch is to display a specific color. No fill tokens are
   * applied; the caller supplies the swatch color directly. Theme parameter
   * exists only for API consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Design-system color swatch — theme-aware version of buildColorSwatch.
 * Light mode is byte-equal to add_color_swatch_v0.
 *
 * This v1 is structurally identical across all theme modes — the swatch
 * color is caller-supplied and not tokenized. Theme parameter exists for
 * consistency; dark/system produce the same tree as light.
 */
export function buildColorSwatchV1(params: ColorSwatchV1Params): ElementTree {
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
