import { assignIdsRecursively, buildToolbarV1, type ToolbarV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddToolbarV1Params extends ToolbarV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Desktop toolbar (v1) — theme-aware variant of add_toolbar_v0.
 * Surface, border, active-bg, icon fills are tokenized.
 */
export async function handleAddToolbarV1(
  params: AddToolbarV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildToolbarV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'toolbar', tree, ...params });
}
