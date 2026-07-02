import { assignIdsRecursively, buildBadgeV1, type BadgeV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBadgeV1Params extends BadgeV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Short inline badge / pill / tag (v1) — theme-aware variant of add_badge_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Since buildBadge emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildBadgeV1` in `@zseven-w/pen-core`.
 */
export async function handleAddBadgeV1(
  params: AddBadgeV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildBadgeV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'bg', tree, ...params });
}
