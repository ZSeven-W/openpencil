import {
  assignIdsRecursively,
  buildStatusBadgeV1,
  type StatusBadgeV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStatusBadgeV1Params extends StatusBadgeV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Status badge (v1) — theme-aware variant of add_status_badge_v0.
 * Dot tone colors are status semantics (kept hardcoded). All modes byte-parity with v0.
 * Accepts theme param for API consistency.
 */
export async function handleAddStatusBadgeV1(
  params: AddStatusBadgeV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildStatusBadgeV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'statusBadge', tree, ...params });
}
