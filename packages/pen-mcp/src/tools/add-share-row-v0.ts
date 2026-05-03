import { assignIdsRecursively, buildShareRow, type ShareRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddShareRowV0Params extends ShareRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddShareRowV0(
  params: AddShareRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildShareRow(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'shareRow', tree: row, ...params });
}
