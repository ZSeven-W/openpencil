import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface SectionHeaderV1Action {
  label: string;
  icon?: string;
}

export interface SectionHeaderV1Params {
  title: string;
  action?: SectionHeaderV1Action;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_section_header_v0.
   * - `'dark'`: identical (no hardcoded colors in v0 — text inherits).
   * - `'system'`: identical (no color refs needed).
   */
  theme?: V1Theme;
}

/**
 * Section header (v1) — theme-aware variant of buildSectionHeader.
 * No hardcoded colors in v0 (text inherits canvas theme color),
 * so all theme modes are identical and byte-parity with v0 is guaranteed.
 * Accepts theme param for API consistency.
 *
 * Big title on the left, optional trailing action (e.g. "See all →").
 * Title uses fill_container + fixed-width textGrowth for multi-line safety.
 */
export function buildSectionHeaderV1(params: SectionHeaderV1Params): ElementTree {
  const children: ElementTree[] = [
    {
      type: 'frame',
      name: 'Title Container',
      role: 'section-header-title',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      children: [
        {
          type: 'text',
          name: 'Title',
          role: 'heading',
          content: params.title,
          fontSize: 20,
          fontWeight: 700,
          width: 'fill_container',
          textGrowth: 'fixed-width',
        },
      ],
    },
  ];
  if (params.action) {
    children.push(buildActionGroupV1(params.action));
  }
  return {
    type: 'frame',
    name: 'Section Header',
    role: 'section-header',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 16,
    children,
  };
}

function buildActionGroupV1(action: SectionHeaderV1Action): ElementTree {
  const children: ElementTree[] = [
    {
      type: 'text',
      name: 'Action Label',
      role: 'label',
      content: action.label,
      fontSize: 14,
      fontWeight: 500,
    },
  ];
  if (action.icon) {
    children.push({
      type: 'icon_font',
      name: 'Action Icon',
      iconFontName: action.icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  return {
    type: 'frame',
    name: 'Action',
    role: 'section-header-action',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
