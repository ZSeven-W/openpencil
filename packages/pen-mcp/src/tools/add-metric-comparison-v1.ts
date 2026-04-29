import {
  assignIdsRecursively,
  buildMetricComparisonV1,
  type MetricComparisonV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddMetricComparisonV1Params extends MetricComparisonV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * KPI with trend indicator (v1) — theme-aware variant of add_metric_comparison_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * label → textMuted; trend colors → success/destructive/textMuted in dark/system.
 */
export async function handleAddMetricComparisonV1(
  params: AddMetricComparisonV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildMetricComparisonV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'metricComparison', tree, ...params });
}
