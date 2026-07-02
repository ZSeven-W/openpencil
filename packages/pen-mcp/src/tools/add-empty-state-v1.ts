import {
  assignIdsRecursively,
  buildEmptyStateV1,
  type EmptyStateV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddEmptyStateV1Params extends EmptyStateV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Empty state (v1) — theme-aware variant of add_empty_state_v0.
 * No hardcoded colors in v0, so light/dark/system modes are identical
 * (byte-parity with v0 in all modes). Accepts theme param for API
 * consistency across all v1 tools.
 */
export async function handleAddEmptyStateV1(
  params: AddEmptyStateV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildEmptyStateV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'es', tree, ...params });
}
