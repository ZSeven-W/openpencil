import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface QuoteBlockV1Params {
  quote: string;
  author?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_quote_block_v0.
   * - `'dark'`: container bg → surface (#1E293B); text inherits (no explicit fill in v0).
   * - `'system'`: $color-surface ref for container bg.
   */
  theme?: V1Theme;
}

/**
 * Quoted passage block (v1) — theme-aware variant of buildQuoteBlock.
 * Light mode is byte-equal to add_quote_block_v0.
 *
 * Color mapping:
 *   container bg (#F9FAFB gray-50) → surface token
 */
export function buildQuoteBlockV1(params: QuoteBlockV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Container bg: light → gray-50 (#F9FAFB), dark/system → surface
  const bgColor = isLight ? '#F9FAFB' : t.colors.surface;

  const children: ElementTree[] = [
    {
      type: 'text',
      name: 'Quote',
      role: 'quote-text',
      content: params.quote,
      fontSize: 16,
      fontWeight: 400,
      lineHeight: 1.5,
      width: 'fill_container',
      textGrowth: 'fixed-width',
    },
  ];
  if (params.author) {
    children.push({
      type: 'text',
      name: 'Author',
      role: 'quote-author',
      content: `— ${params.author}`,
      fontSize: 13,
      fontWeight: 500,
    });
  }
  return {
    type: 'frame',
    name: 'Quote Block',
    role: 'quote-block',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    padding: [16, 20],
    gap: 8,
    cornerRadius: 8,
    fill: [{ type: 'solid', color: bgColor }],
    children,
  };
}
