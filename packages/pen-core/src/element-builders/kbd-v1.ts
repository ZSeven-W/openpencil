import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface KbdV1Params {
  keys: string[];
  separator?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_kbd_v0.
   * - `'dark'`: key cell bg → surface2, stroke → border.
   * - `'system'`: emits `$color-*` refs for fill and stroke.
   */
  theme?: V1Theme;
}

/**
 * Keyboard shortcut — theme-aware variant of buildKbd.
 * Light mode is byte-equal to add_kbd_v0.
 *
 * Color mapping:
 *   key bg  (#F3F4F6 gray-100) → surface2
 *   stroke  (#D1D5DB gray-300) → border
 *   glyph text has no explicit fill in v0 (inherits canvas text color)
 */
export function buildKbdV1(params: KbdV1Params): ElementTree {
  const keys = params.keys.filter((k): k is string => typeof k === 'string' && k.length > 0);
  if (keys.length === 0) {
    throw new Error('buildKbdV1 requires at least one non-empty key in `keys`.');
  }
  const separator = params.separator ?? '+';
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const keyBg = isLight ? '#F3F4F6' : t.colors.surface2;
  const keyStroke = isLight ? '#D1D5DB' : t.colors.border;

  const children: ElementTree[] = [];
  keys.forEach((key, idx) => {
    children.push({
      type: 'frame',
      name: `Key ${idx + 1}`,
      role: 'kbd-key',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      padding: [2, 6],
      cornerRadius: 4,
      stroke: { thickness: 1, fill: [{ type: 'solid', color: keyStroke }] },
      fill: [{ type: 'solid', color: keyBg }],
      children: [
        {
          type: 'text',
          name: 'Glyph',
          role: 'kbd-glyph',
          content: key,
          fontSize: 12,
          fontWeight: 500,
        },
      ],
    });
    if (idx < keys.length - 1 && separator.length > 0) {
      children.push({
        type: 'text',
        name: 'Separator',
        role: 'kbd-separator',
        content: separator,
        fontSize: 12,
        fontWeight: 400,
      });
    }
  });
  return {
    type: 'frame',
    name: 'Keyboard Shortcut',
    role: 'kbd',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
