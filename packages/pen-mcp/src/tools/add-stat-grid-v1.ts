import { assignIdsRecursively, buildStatGridV1, type StatGridV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStatGridV1Params extends StatGridV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { StatGridV1Item as AddStatGridV1Item } from '@zseven-w/pen-core';

/**
 * Stat grid (v1) — theme-aware variant of add_stat_grid_v0.
 * No hardcoded colors in v0 (text inherits). All modes are byte-parity with v0.
 * Accepts theme param for API consistency.
 */
export async function handleAddStatGridV1(
  params: AddStatGridV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildStatGridV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'statGrid', tree, ...params });
}
