import { assignIdsRecursively, buildChartPieV1, type ChartPieV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChartPieV1Params extends ChartPieV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Pie-chart skeleton (v1) — theme-aware variant of add_chart_pie_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Default slice palette maps to chart-1..6 tokens in dark/system modes.
 * Caller-supplied `colors` are always passed through unchanged.
 * Tree build delegated to `buildChartPieV1` in `@zseven-w/pen-core`.
 */
export async function handleAddChartPieV1(
  params: AddChartPieV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildChartPieV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cp', tree, ...params });
}
