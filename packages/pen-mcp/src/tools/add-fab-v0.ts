import { assignIdsRecursively, buildFab, type FabParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFabV0Params extends FabParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Floating action button. Tree build delegated to `buildFab`. */
export async function handleAddFabV0(
  params: AddFabV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const fab = buildFab(params);
  assignIdsRecursively(fab);
  return insertElementTree({ binding: 'fab', tree: fab, ...params });
}
