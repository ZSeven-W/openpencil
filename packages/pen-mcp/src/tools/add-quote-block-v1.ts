import {
  assignIdsRecursively,
  buildQuoteBlockV1,
  type QuoteBlockV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddQuoteBlockV1Params extends QuoteBlockV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Quoted passage block (v1) — theme-aware variant of add_quote_block_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Container bg: surface in dark/system; text inherits canvas color.
 */
export async function handleAddQuoteBlockV1(
  params: AddQuoteBlockV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildQuoteBlockV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'quoteBlock', tree, ...params });
}
