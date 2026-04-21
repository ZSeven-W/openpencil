import type { ElementTree } from './helpers.js';

export interface QuoteBlockParams {
  quote: string;
  author?: string;
}

/**
 * Quoted passage block — rounded container with quote text above
 * optional attribution. No left vertical bar (pen-core lacks
 * alignItems='stretch' + fit_content circular-dep). Role is
 * enough for a follow-up U-op to theme. Multi-line quotes wrap via
 * fill_container + textGrowth='fixed-width'.
 */
export function buildQuoteBlock(params: QuoteBlockParams): ElementTree {
  const children: ElementTree[] = [
    {
      type: 'text',
      name: 'Quote',
      role: 'quote-text',
      content: params.quote,
      fontSize: 16,
      fontWeight: 400,
      lineHeight: 1.5,
      width: 'fill_container',
      textGrowth: 'fixed-width',
    },
  ];
  if (params.author) {
    children.push({
      type: 'text',
      name: 'Author',
      role: 'quote-author',
      content: `— ${params.author}`,
      fontSize: 13,
      fontWeight: 500,
    });
  }
  return {
    type: 'frame',
    name: 'Quote Block',
    role: 'quote-block',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    padding: [16, 20],
    gap: 8,
    cornerRadius: 8,
    fill: [{ type: 'solid', color: '#F9FAFB' }],
    children,
  };
}
