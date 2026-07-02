import {
  assignIdsRecursively,
  buildMetricComparison,
  type MetricComparisonParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddMetricComparisonV0Params extends MetricComparisonParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddMetricComparisonV0(
  params: AddMetricComparisonV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const m = buildMetricComparison(params);
  assignIdsRecursively(m);
  return insertElementTree({ binding: 'mc', tree: m, ...params });
}
