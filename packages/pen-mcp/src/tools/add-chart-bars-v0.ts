import { assignIdsRecursively, buildChartBars, type ChartBarsParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChartBarsV0Params extends ChartBarsParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Bar-chart skeleton. Tree build delegated to `buildChartBars`. */
export async function handleAddChartBarsV0(
  params: AddChartBarsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildChartBars(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'chart', tree, ...params });
}
