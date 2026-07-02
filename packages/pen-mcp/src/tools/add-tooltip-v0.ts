import { assignIdsRecursively, buildTooltip, type TooltipParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTooltipV0Params extends TooltipParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Tooltip pill — small dark pill with white text. Tree build
 * delegated to `buildTooltip`.
 */
export async function handleAddTooltipV0(
  params: AddTooltipV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const t = buildTooltip(params);
  assignIdsRecursively(t);
  return insertElementTree({ binding: 'tt', tree: t, ...params });
}
