import type { ElementTree } from './helpers.js';

export interface KbdParams {
  keys: string[];
  separator?: string;
}

/**
 * Keyboard shortcut ("⌘ K" / "Ctrl + Shift + P"). Each entry
 * becomes a bordered cell (padding=[2,6], cornerRadius=4, 1px
 * neutral stroke); entries joined with `separator` text (default
 * "+"; pass "" for no separator).
 */
export function buildKbd(params: KbdParams): ElementTree {
  const keys = params.keys.filter((k): k is string => typeof k === 'string' && k.length > 0);
  if (keys.length === 0) {
    throw new Error('buildKbd requires at least one non-empty key in `keys`.');
  }
  const separator = params.separator ?? '+';
  const children: ElementTree[] = [];
  keys.forEach((key, idx) => {
    children.push({
      type: 'frame',
      name: `Key ${idx + 1}`,
      role: 'kbd-key',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      padding: [2, 6],
      cornerRadius: 4,
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#D1D5DB' }] },
      fill: [{ type: 'solid', color: '#F3F4F6' }],
      children: [
        {
          type: 'text',
          name: 'Glyph',
          role: 'kbd-glyph',
          content: key,
          fontSize: 12,
          fontWeight: 500,
        },
      ],
    });
    if (idx < keys.length - 1 && separator.length > 0) {
      children.push({
        type: 'text',
        name: 'Separator',
        role: 'kbd-separator',
        content: separator,
        fontSize: 12,
        fontWeight: 400,
      });
    }
  });
  return {
    type: 'frame',
    name: 'Keyboard Shortcut',
    role: 'kbd',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
}
