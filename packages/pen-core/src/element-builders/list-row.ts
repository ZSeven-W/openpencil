import type { ElementTree } from './helpers.js';

export interface ListRowParams {
  title: string;
  subtitle?: string;
  leading_icon?: string;
  trailing_icon?: string;
}

/**
 * iOS / Material-style list row: [optional leading icon] + [vertical
 * text stack (title + optional subtitle)] + [optional trailing icon,
 * typically chevron-right].
 *
 * No-overlap invariant: the text-stack sibling uses width=fill_container
 * inside a VERTICAL wrapper so the title wraps correctly + wrap
 * height propagates to the row's fit_content height. Text directly in
 * horizontal parents with fill_container+fixed-width does NOT
 * propagate wrap height per the layout engine — the vertical wrapper
 * is why this tool needs it.
 */
export function buildListRow(params: ListRowParams): ElementTree {
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
