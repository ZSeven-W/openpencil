import { assignIdsRecursively, buildFaqItemV1, type FaqItemV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFaqItemV1Params extends FaqItemV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * FAQ accordion item (v1) — theme-aware variant of add_faq_item_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Chevron → textMuted, answer → textMuted, divider → border.
 */
export async function handleAddFaqItemV1(
  params: AddFaqItemV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildFaqItemV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'faq', tree, ...params });
}
