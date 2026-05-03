import { assignIdsRecursively, buildFilterGroup, type FilterGroupParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFilterGroupV0Params extends FilterGroupParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddFilterGroupV0(
  params: AddFilterGroupV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildFilterGroup(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'filterGroup', tree, ...params });
}
