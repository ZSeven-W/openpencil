import { assignIdsRecursively, buildStatusBadge, type StatusBadgeParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStatusBadgeV0Params extends StatusBadgeParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Status indicator: colored dot + label ("● Online"). Tree build
 * delegated to `buildStatusBadge`.
 */
export async function handleAddStatusBadgeV0(
  params: AddStatusBadgeV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const badge = buildStatusBadge(params);
  assignIdsRecursively(badge);
  return insertElementTree({ binding: 'sb', tree: badge, ...params });
}
