import type { ElementTree } from './helpers.js';

export interface TextButtonParams {
  label: string;
  leading_icon?: string;
}

/**
 * Padding-based text button. `frame(padding=[12,20], justifyContent=center)
 * > [optional icon + text]` — height auto-derives from padding + text
 * metrics, no explicit fixed height.
 */
export function buildTextButton(params: TextButtonParams): ElementTree {
  const children: ElementTree[] = [];
  if (params.leading_icon) {
    children.push({
      type: 'icon_font',
      name: 'Icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  children.push({
    type: 'text',
    name: 'Label',
    role: 'label',
    content: params.label,
    fontSize: 14,
    fontWeight: 500,
  });
  return {
    type: 'frame',
    name: 'Text Button',
    role: 'button',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
    padding: [12, 20],
    cornerRadius: 8,
    children,
  };
}
