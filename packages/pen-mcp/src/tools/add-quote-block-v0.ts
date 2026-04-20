import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddQuoteBlockV0Params {
  quote: string;
  author?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Quoted passage block — rounded, lightly filled container with the
 * quote text stacked above an optional attribution line ("— Einstein").
 *
 * Why no left vertical bar: pen-core layout does not implement
 * `alignItems: 'stretch'`, so a `rectangle(width=3, height='fill_container')`
 * sibling would collide with the `height: 'fit_content'` parent's
 * circular-dependency rule. The bar would either not stretch or trip
 * the layout-engine invariant. A filled container with role='quote-block'
 * is equally legible as a quotation cue and survives every layout path.
 * Callers who want a left-bar look can apply a stroke or replace the
 * background with a gradient via batch_design U-op.
 *
 * Quote text uses textGrowth='fixed-width' + width='fill_container' so
 * multi-line quotes wrap at the container edge (the repo's "never fixed
 * pixel width on text inside layout frames" rule — see text-rules.md).
 */
export async function handleAddQuoteBlockV0(
  params: AddQuoteBlockV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [
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
  const block = {
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
  assignIdsRecursively(block);
  return insertElementTree({ binding: 'quote_block', tree: block, ...params });
}
