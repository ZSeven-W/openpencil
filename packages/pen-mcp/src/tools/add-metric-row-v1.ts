import { assignIdsRecursively, buildMetricRowV1, type MetricRowV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddMetricRowV1Params extends MetricRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal scroll row of metric tiles (v1) — theme-aware variant of add_metric_row_v0.
 * No hardcoded colors in v0; light/dark/system modes are identical (byte-parity with v0
 * in all modes). Accepts theme param for API consistency across all v1 tools.
 */
export async function handleAddMetricRowV1(
  params: AddMetricRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildMetricRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'metricRow', tree, ...params });
}
