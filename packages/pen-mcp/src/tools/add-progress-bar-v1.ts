import {
  assignIdsRecursively,
  buildProgressBarV1,
  type ProgressBarV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddProgressBarV1Params extends ProgressBarV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Linear progress bar (v1) — theme-aware variant of add_progress_bar_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Track bg: surface2 in dark/system; accent fill stays brand-invariant.
 */
export async function handleAddProgressBarV1(
  params: AddProgressBarV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildProgressBarV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'progressBar', tree, ...params });
}
