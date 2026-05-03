import { assignIdsRecursively, buildUserCard, type UserCardParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddUserCardV0Params extends UserCardParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddUserCardV0(
  params: AddUserCardV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const card = buildUserCard(params);
  assignIdsRecursively(card);
  return insertElementTree({ binding: 'userCard', tree: card, ...params });
}
