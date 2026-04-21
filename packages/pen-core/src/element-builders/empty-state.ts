import type { ElementTree } from './helpers.js';

export interface EmptyStateParams {
  title: string;
  subtitle?: string;
  icon?: string;
  cta_label?: string;
}

/**
 * Empty-state block — vertical centered stack: [icon?] + title +
 * [subtitle?] + [CTA?]. width=fill_container, alignItems=center,
 * padding=[48,24], gap=16. Locks the 4-piece structure so weak
 * models emit composable results.
 */
export function buildEmptyState(params: EmptyStateParams): ElementTree {
  const children: ElementTree[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      role: 'empty-state-icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 48,
      height: 48,
    });
  }
  children.push({
    type: 'text',
    name: 'Title',
    role: 'empty-state-title',
    content: params.title,
    fontSize: 18,
    fontWeight: 600,
  });
  if (params.subtitle) {
    children.push({
      type: 'text',
      name: 'Subtitle',
      role: 'empty-state-subtitle',
      content: params.subtitle,
      fontSize: 14,
      fontWeight: 400,
    });
  }
  if (params.cta_label) {
    children.push({
      type: 'frame',
      name: 'CTA',
      role: 'button',
      cornerRadius: 24,
      padding: [12, 24],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'CTA Label',
          role: 'label',
          content: params.cta_label,
          fontSize: 14,
          fontWeight: 500,
        },
      ],
    });
  }
  return {
    type: 'frame',
    name: 'Empty State',
    role: 'empty-state',
    width: 'fill_container',
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 16,
    padding: [48, 24],
    children,
  };
}
