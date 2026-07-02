import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface SearchBarV1Params {
  placeholder?: string;
  leading_icon?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_search_bar_v0.
   * - `'dark'`: container bg → surface2 (#334155); icon and placeholder keep no
   *   explicit fill (inherits canvas text color — dark theme gives white).
   * - `'system'`: $color-surface-2 ref for bg.
   */
  theme?: V1Theme;
}

/**
 * Search bar (v1) — theme-aware variant of buildSearchBar.
 * Light mode is byte-equal to add_search_bar_v0.
 *
 * v0 has no explicit fill (transparent by default). In dark/system mode
 * we add a surface2 background so the bar is visible against the page bg.
 * In light mode we omit the fill (byte-parity: v0 omits it too).
 *
 * Color mapping:
 *   bg fill  (absent in v0 light) → added as surface2 in dark/system
 */
export function buildSearchBarV1(params: SearchBarV1Params): ElementTree {
  const icon = params.leading_icon ?? 'search';
  const placeholder = params.placeholder ?? 'Search...';

  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  const node: ElementTree = {
    type: 'frame',
    name: 'Search Bar',
    role: 'search-bar',
    width: 'fill_container',
    height: 44,
    cornerRadius: 22,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    padding: [0, 16],
    children: [
      {
        type: 'icon_font',
        name: 'Leading Icon',
        iconFontName: icon,
        iconFontFamily: 'lucide',
        width: 20,
        height: 20,
      },
      {
        type: 'text',
        name: 'Placeholder',
        content: placeholder,
        fontSize: 14,
        fontWeight: 400,
      },
    ],
  };

  // In dark/system modes add a surface2 fill; light stays without fill (v0 parity)
  if (!isLight) {
    node.fill = [{ type: 'solid', color: t.colors.surface2 }];
  }

  return node;
}
