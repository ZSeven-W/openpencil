import { assignIdsRecursively, buildChartLine, type ChartLineParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChartLineV0Params extends ChartLineParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Line-chart skeleton. Polyline + optional dots. Tree build delegated
 * to `buildChartLine`.
 */
export async function handleAddChartLineV0(
  params: AddChartLineV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const chart = buildChartLine(params);
  assignIdsRecursively(chart);
  return insertElementTree({ binding: 'cl', tree: chart, ...params });
}
