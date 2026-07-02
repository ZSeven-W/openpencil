import type { ElementTree } from './helpers.js';

export interface SectionHeaderAction {
  label: string;
  icon?: string;
}

export interface SectionHeaderParams {
  title: string;
  action?: SectionHeaderAction;
}

/**
 * Dashboard / landing section header: big title on the left, optional
 * trailing action (e.g. "See all →"). Horizontal + space_between via
 * title (fill_container) + action (fit_content).
 *
 * Title is wrapped in a vertical container with textGrowth=fixed-width
 * so multi-line titles correctly push downstream content down — see
 * `packages/pen-ai-skills/skills/phases/generation/overflow.md`
 * §"Text in VERTICAL layout".
 */
export function buildSectionHeader(params: SectionHeaderParams): ElementTree {
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
    children.push(buildActionGroup(params.action));
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

function buildActionGroup(action: SectionHeaderAction): ElementTree {
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
