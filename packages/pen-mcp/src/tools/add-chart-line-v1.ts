import { assignIdsRecursively, buildChartLineV1, type ChartLineV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChartLineV1Params extends ChartLineV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Line-chart skeleton (v1) — theme-aware variant of add_chart_line_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Line/dot color maps to chart-1 token in dark/system modes.
 * Tree build delegated to `buildChartLineV1` in `@zseven-w/pen-core`.
 */
export async function handleAddChartLineV1(
  params: AddChartLineV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildChartLineV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cl', tree, ...params });
}
