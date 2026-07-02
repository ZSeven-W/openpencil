import { assignIdsRecursively, buildChartBarsV1, type ChartBarsV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChartBarsV1Params extends ChartBarsV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Bar-chart skeleton (v1) — theme-aware variant of add_chart_bars_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Bar color maps to chart-1 token in dark/system modes.
 * Tree build delegated to `buildChartBarsV1` in `@zseven-w/pen-core`.
 */
export async function handleAddChartBarsV1(
  params: AddChartBarsV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildChartBarsV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cb', tree, ...params });
}
