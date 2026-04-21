import { assignIdsRecursively, buildQuoteBlock, type QuoteBlockParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddQuoteBlockV0Params extends QuoteBlockParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Quoted passage block. Tree build delegated to `buildQuoteBlock`. */
export async function handleAddQuoteBlockV0(
  params: AddQuoteBlockV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const block = buildQuoteBlock(params);
  assignIdsRecursively(block);
  return insertElementTree({ binding: 'quote_block', tree: block, ...params });
}
