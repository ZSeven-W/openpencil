import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface CodeBlockV1Params {
  code: string;
  language?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_code_block_v0.
   * - `'dark'`: dark background for the code block surface.
   * - `'system'`: emits `$color-surface-2` ref for background fill.
   */
  theme?: V1Theme;
}

/**
 * Preformatted code block — theme-aware version of buildCodeBlock.
 * Light mode is byte-equal to add_code_block_v0.
 *
 * Background (#F3F4F6 = gray-100) maps to surface2 token in dark/system modes.
 * Code text has no explicit fill in v0 (relies on canvas default) so we
 * keep it unfilled in all modes — theme-agnostic.
 */
export function buildCodeBlockV1(params: CodeBlockV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  // Light: byte-parity (#F3F4F6). Dark/system: surface2 token.
  const bgColor = isLight ? '#F3F4F6' : t.colors.surface2;

  const nameSuffix = params.language ? ` (${params.language})` : '';
  return {
    type: 'frame',
    name: `Code Block${nameSuffix}`,
    role: 'code-block',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    padding: [12, 16],
    cornerRadius: 8,
    fill: [{ type: 'solid', color: bgColor }],
    children: [
      {
        type: 'text',
        name: 'Code',
        role: 'code',
        content: params.code,
        fontSize: 13,
        fontWeight: 400,
        lineHeight: 1.5,
        width: 'fill_container',
        textGrowth: 'fixed-width',
      },
    ],
  };
}
