import { assignIdsRecursively, buildSkeletonV1, type SkeletonV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSkeletonV1Params extends SkeletonV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Loading skeleton (v1) — theme-aware variant of add_skeleton_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 */
export async function handleAddSkeletonV1(
  params: AddSkeletonV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSkeletonV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'skeleton', tree, ...params });
}
