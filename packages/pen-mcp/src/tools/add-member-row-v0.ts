import { assignIdsRecursively, buildMemberRow, type MemberRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddMemberRowV0Params extends MemberRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddMemberRowV0(
  params: AddMemberRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildMemberRow(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'memberRow', tree: row, ...params });
}
