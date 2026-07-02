import { assignIdsRecursively, buildBottomNavV1, type BottomNavV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBottomNavV1Params extends BottomNavV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Bottom tab bar (v1) — theme-aware variant of add_bottom_nav_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Since buildBottomNav emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildBottomNavV1` in `@zseven-w/pen-core`.
 */
export async function handleAddBottomNavV1(
  params: AddBottomNavV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildBottomNavV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'bn', tree, ...params });
}
