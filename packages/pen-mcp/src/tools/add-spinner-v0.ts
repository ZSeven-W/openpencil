import { assignIdsRecursively, buildSpinner, type SpinnerParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSpinnerV0Params extends SpinnerParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Loading spinner — ring + 3/4 sweep active arc (static, no animation).
 * Tree build delegated to `buildSpinner`.
 */
export async function handleAddSpinnerV0(
  params: AddSpinnerV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const s = buildSpinner(params);
  assignIdsRecursively(s);
  return insertElementTree({ binding: 'sp', tree: s, ...params });
}
