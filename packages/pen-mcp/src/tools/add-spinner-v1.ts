import { assignIdsRecursively, buildSpinnerV1, type SpinnerV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSpinnerV1Params extends SpinnerV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Loading spinner (v1) — theme-aware variant of add_spinner_v0.
 * Accepts theme param for API consistency. track_color/active_color are
 * caller-overridable; no surface colors are hardcoded in the builder.
 */
export async function handleAddSpinnerV1(
  params: AddSpinnerV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSpinnerV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'spinner', tree, ...params });
}
