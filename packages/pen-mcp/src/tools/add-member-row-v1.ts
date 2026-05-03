import { assignIdsRecursively, buildMemberRowV1, type MemberRowV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddMemberRowV1Params extends MemberRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Team/member list row (v1) — theme-aware variant of add_member_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Tree build delegated to `buildMemberRowV1` in `@zseven-w/pen-core`.
 */
export async function handleAddMemberRowV1(
  params: AddMemberRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildMemberRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'mr', tree, ...params });
}
