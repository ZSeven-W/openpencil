import {
  assignIdsRecursively,
  buildSegmentedControlV1,
  type SegmentedControlV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSegmentedControlV1Params extends SegmentedControlV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Segmented control (v1) — theme-aware variant of add_segmented_control_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Track → surface2; active seg → surface; active label → textPrimary;
 * inactive label → textMuted in dark/system.
 */
export async function handleAddSegmentedControlV1(
  params: AddSegmentedControlV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSegmentedControlV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'segmentedControl', tree, ...params });
}
