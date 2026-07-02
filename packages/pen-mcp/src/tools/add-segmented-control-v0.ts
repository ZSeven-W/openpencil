import {
  assignIdsRecursively,
  buildSegmentedControl,
  type SegmentedControlParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSegmentedControlV0Params extends SegmentedControlParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { SegmentedControlItem as AddSegmentedControlV0Item } from '@zseven-w/pen-core';

/**
 * iOS pill-style segmented control. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildSegmentedControl` — fill_container
 * segments, active segment white on gray-100 track.
 */
export async function handleAddSegmentedControlV0(
  params: AddSegmentedControlV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const container = buildSegmentedControl(params);
  assignIdsRecursively(container);
  return insertElementTree({ binding: 'segmented', tree: container, ...params });
}
