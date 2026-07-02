import { assignIdsRecursively, buildFaqItem, type FaqItemParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFaqItemV0Params extends FaqItemParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** FAQ / accordion item. Tree build delegated to `buildFaqItem`. */
export async function handleAddFaqItemV0(
  params: AddFaqItemV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const f = buildFaqItem(params);
  assignIdsRecursively(f);
  return insertElementTree({ binding: 'faq', tree: f, ...params });
}
