import {
  assignIdsRecursively,
  buildEmptyChartV1,
  type EmptyChartV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddEmptyChartV1Params extends EmptyChartV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Empty chart placeholder (v1) — theme-aware variant of
 * add_empty_chart_v0. Tree build delegated to `buildEmptyChartV1`.
 * See the builder's JSDoc for the theme-variant contract
 * (light / dark / system).
 */
export async function handleAddEmptyChartV1(
  params: AddEmptyChartV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const e = buildEmptyChartV1(params);
  assignIdsRecursively(e);
  return insertElementTree({ binding: 'emptyChart', tree: e, ...params });
}
