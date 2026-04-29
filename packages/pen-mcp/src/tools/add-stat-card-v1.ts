import { assignIdsRecursively, buildStatCardV1, type StatCardV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStatCardV1Params extends StatCardV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Big-number stat card (v1) — theme-aware variant of add_stat_card_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Delta tone colors (success/error/flat) are kept hardcoded — status semantics.
 */
export async function handleAddStatCardV1(
  params: AddStatCardV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildStatCardV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'statCard', tree, ...params });
}
