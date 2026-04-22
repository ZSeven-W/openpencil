import { assignIdsRecursively, buildEmptyChart, type EmptyChartParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddEmptyChartV0Params extends EmptyChartParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Empty state for chart widgets. Tree build delegated to `buildEmptyChart`. */
export async function handleAddEmptyChartV0(
  params: AddEmptyChartV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const c = buildEmptyChart(params);
  assignIdsRecursively(c);
  return insertElementTree({ binding: 'emptyChart', tree: c, ...params });
}
