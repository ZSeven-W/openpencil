import {
  assignIdsRecursively,
  buildRangeSliderV1,
  type RangeSliderV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddRangeSliderV1Params extends RangeSliderV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Single-thumb range slider (v1) — theme-aware variant of add_range_slider_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Accent stays brand-invariant; thumb bg → surface; remaining → border;
 * label → textPrimary; value → textMuted in dark/system.
 */
export async function handleAddRangeSliderV1(
  params: AddRangeSliderV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildRangeSliderV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'rangeSlider', tree, ...params });
}
