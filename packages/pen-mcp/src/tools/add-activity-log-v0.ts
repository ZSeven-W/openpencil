import { assignIdsRecursively, buildActivityLog, type ActivityLogParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddActivityLogV0Params extends ActivityLogParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddActivityLogV0(
  params: AddActivityLogV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildActivityLog(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'activityLog', tree, ...params });
}
