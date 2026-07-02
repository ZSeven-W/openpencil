import { assignIdsRecursively, buildLegendItem, type LegendItemParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddLegendItemV0Params extends LegendItemParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddLegendItemV0(
  params: AddLegendItemV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const item = buildLegendItem(params);
  assignIdsRecursively(item);
  return insertElementTree({ binding: 'legendItem', tree: item, ...params });
}
