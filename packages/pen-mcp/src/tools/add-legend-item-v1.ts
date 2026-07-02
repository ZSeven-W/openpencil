import {
  assignIdsRecursively,
  buildLegendItemV1,
  type LegendItemV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddLegendItemV1Params extends LegendItemV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Chart legend entry (v1) — theme-aware variant of add_legend_item_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * label → textBody, value → textPrimary in dark/system modes.
 * Marker color (caller-supplied chart series color) is kept as-is in
 * all modes.
 */
export async function handleAddLegendItemV1(
  params: AddLegendItemV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildLegendItemV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'legendItem', tree, ...params });
}
