import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface ListRowV1Params {
  title: string;
  subtitle?: string;
  leading_icon?: string;
  trailing_icon?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_list_row_v0.
   * - `'dark'`: identical (no hardcoded colors in v0 — title/subtitle
   *   inherit text color from canvas; icons have no explicit fill).
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * iOS / Material-style list row — theme-aware variant of buildListRow.
 * No hardcoded colors in v0, so light/dark/system modes are identical
 * (byte-parity with v0 in all modes). Accepts theme param for API
 * consistency across all v1 tools.
 *
 * Text-stack sibling uses width=fill_container inside a VERTICAL wrapper
 * so the title wraps correctly + wrap height propagates to the row's
 * fit_content height.
 */
export function buildListRowV1(params: ListRowV1Params): ElementTree {
  const rowChildren: ElementTree[] = [];
  if (params.leading_icon) {
    rowChildren.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
    });
  }
  const textStackChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'label',
      content: params.title,
      fontSize: 15,
      fontWeight: 500,
      width: 'fill_container',
      textGrowth: 'fixed-width',
    },
  ];
  if (params.subtitle) {
    textStackChildren.push({
      type: 'text',
      name: 'Subtitle',
      role: 'body',
      content: params.subtitle,
      fontSize: 13,
      fontWeight: 400,
      width: 'fill_container',
      textGrowth: 'fixed-width',
    });
  }
  rowChildren.push({
    type: 'frame',
    name: 'Text Stack',
    role: 'list-row-text',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 2,
    children: textStackChildren,
  });
  if (params.trailing_icon) {
    rowChildren.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  return {
    type: 'frame',
    name: 'List Row',
    role: 'list-row',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    padding: [12, 16],
    children: rowChildren,
  };
}
