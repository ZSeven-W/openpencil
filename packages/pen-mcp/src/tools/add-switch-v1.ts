import { assignIdsRecursively, buildSwitchV1, type SwitchV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSwitchV1Params extends SwitchV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Toggle switch (v1) — theme-aware variant of add_switch_v0.
 * iOS HIG values (#34C759 active, #E5E5EA inactive, #FFFFFF thumb) are builder-private
 * literals (spec §3.4) — kept hardcoded across all theme modes.
 * Accepts theme param for API consistency.
 */
export async function handleAddSwitchV1(
  params: AddSwitchV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSwitchV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'switch', tree, ...params });
}
