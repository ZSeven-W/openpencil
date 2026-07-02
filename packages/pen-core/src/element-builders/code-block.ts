import type { ElementTree } from './helpers.js';

export interface CodeBlockParams {
  code: string;
  language?: string;
}

/**
 * Preformatted code block. fill_container frame (padding=[12,16],
 * cornerRadius=8, gray-50 fill) with one text child that preserves
 * `code` verbatim including newlines. `language` becomes part of
 * the frame name only (no syntax highlighting — pen-core has no
 * highlighter). fontFamily unset; renderer default carries.
 */
export function buildCodeBlock(params: CodeBlockParams): ElementTree {
  const nameSuffix = params.language ? ` (${params.language})` : '';
  return {
    type: 'frame',
    name: `Code Block${nameSuffix}`,
    role: 'code-block',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    padding: [12, 16],
    cornerRadius: 8,
    fill: [{ type: 'solid', color: '#F3F4F6' }],
    children: [
      {
        type: 'text',
        name: 'Code',
        role: 'code',
        content: params.code,
        fontSize: 13,
        fontWeight: 400,
        lineHeight: 1.5,
        width: 'fill_container',
        textGrowth: 'fixed-width',
      },
    ],
  };
}
