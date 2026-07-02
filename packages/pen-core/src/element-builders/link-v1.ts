import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface LinkV1Params {
  label: string;
  trailing_icon?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_link_v0.
   * - `'dark'`: identical (no hardcoded colors in v0 — link inherits
   *   text color from canvas; caller typically adds fill via batch_design).
   * - `'system'`: identical.
   */
  theme?: V1Theme;
}

/**
 * Inline text link — theme-aware variant of buildLink.
 * No hardcoded colors in v0, so all theme modes are byte-identical.
 * Accepts theme param for API consistency across all v1 tools.
 *
 * Roles stay Style-Guide orthogonal so colors + underline get
 * applied via batch_design U-op later.
 */
export function buildLinkV1(params: LinkV1Params): ElementTree {
  const children: ElementTree[] = [
    {
      type: 'text',
      name: 'Label',
      role: 'link-label',
      content: params.label,
      fontSize: 14,
      fontWeight: 500,
    },
  ];
  if (params.trailing_icon) {
    children.push({
      type: 'icon_font',
      name: 'Trailing Icon',
      role: 'link-icon',
      iconFontName: params.trailing_icon,
      iconFontFamily: 'lucide',
      width: 14,
      height: 14,
    });
  }
  return {
    type: 'frame',
    name: 'Link',
    role: 'link',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
