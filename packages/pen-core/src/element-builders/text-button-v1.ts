import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface TextButtonV1Params {
  label: string;
  leading_icon?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_text_button_v0.
   * - `'dark'`: identical — no hardcoded surface colors in v0.
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Padding-based text button (v1) — theme-aware variant of buildTextButton.
 * Light mode is byte-equal to add_text_button_v0.
 *
 * No hardcoded surface colors in v0 (icon and text inherit canvas default).
 * All theme modes produce identical trees.
 */
export function buildTextButtonV1(params: TextButtonV1Params): ElementTree {
  const children: ElementTree[] = [];
  if (params.leading_icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  children.push({
    type: 'text',
    name: 'Label',
    role: 'label',
    content: params.label,
    fontSize: 14,
    fontWeight: 500,
  });
  return {
    type: 'frame',
    name: 'Text Button',
    role: 'button',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
    padding: [12, 20],
    cornerRadius: 8,
    children,
  };
}
