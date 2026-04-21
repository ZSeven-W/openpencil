import type { ElementTree } from './helpers.js';

export interface SearchBarParams {
  placeholder?: string;
  leading_icon?: string;
}

/**
 * Search bar — height=44, cornerRadius=22 (iOS HIG); leading icon
 * (defaults to 'search') + placeholder text in a horizontal row.
 * fill_container width so the bar stretches in a form / header.
 */
export function buildSearchBar(params: SearchBarParams): ElementTree {
  const icon = params.leading_icon ?? 'search';
  const placeholder = params.placeholder ?? 'Search...';
  return {
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
}
