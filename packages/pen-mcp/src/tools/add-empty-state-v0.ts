import { assignIdsRecursively, buildEmptyState, type EmptyStateParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddEmptyStateV0Params extends EmptyStateParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Empty-state block. Tree build delegated to `@zseven-w/pen-core`'s
 * `buildEmptyState`.
 */
export async function handleAddEmptyStateV0(
  params: AddEmptyStateV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const frame = buildEmptyState(params);
  assignIdsRecursively(frame);
  return insertElementTree({ binding: 'empty', tree: frame, ...params });
}
