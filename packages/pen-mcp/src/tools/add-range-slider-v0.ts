import { assignIdsRecursively, buildRangeSlider, type RangeSliderParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddRangeSliderV0Params extends RangeSliderParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Single-thumb range slider. Tree build delegated to `buildRangeSlider`. */
export async function handleAddRangeSliderV0(
  params: AddRangeSliderV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const r = buildRangeSlider(params);
  assignIdsRecursively(r);
  return insertElementTree({ binding: 'rangeSlider', tree: r, ...params });
}
