import { assignIdsRecursively, buildInviteRow, type InviteRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInviteRowV0Params extends InviteRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddInviteRowV0(
  params: AddInviteRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildInviteRow(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'inviteRow', tree, ...params });
}
