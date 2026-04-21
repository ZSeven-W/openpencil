import { assignIdsRecursively, buildChartPie, type ChartPieParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChartPieV0Params extends ChartPieParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Pie-chart skeleton. N colored slices via ellipse startAngle/sweepAngle.
 * Tree build delegated to `buildChartPie`.
 */
export async function handleAddChartPieV0(
  params: AddChartPieV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const chart = buildChartPie(params);
  assignIdsRecursively(chart);
  return insertElementTree({ binding: 'cp', tree: chart, ...params });
}
