import { assignIdsRecursively, buildUserCardV1, type UserCardV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddUserCardV1Params extends UserCardV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * User card (v1) — theme-aware variant of add_user_card_v0.
 * name (textPrimary) and role (textMuted) are tokenized.
 * Avatar bg (#3B82F6) and initial (#FFFFFF) are hardcoded per spec §3.4.
 */
export async function handleAddUserCardV1(
  params: AddUserCardV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildUserCardV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'user-card', tree, ...params });
}
