import { assignIdsRecursively, buildActionMenu, type ActionMenuParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddActionMenuV0Params extends ActionMenuParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { ActionMenuItem as AddActionMenuV0Item } from '@zseven-w/pen-core';

/** Action / context menu panel. Tree build delegated to `buildActionMenu`. */
export async function handleAddActionMenuV0(
  params: AddActionMenuV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const m = buildActionMenu(params);
  assignIdsRecursively(m);
  return insertElementTree({ binding: 'actionMenu', tree: m, ...params });
}
