import { assignIdsRecursively, buildTooltipV1, type TooltipV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTooltipV1Params extends TooltipV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Tooltip (v1) — theme-aware variant of add_tooltip_v0.
 * Intentionally dark surface (#111827) in all modes — inverted-contrast UI pattern
 * (spec §3.4). All theme modes produce identical trees.
 */
export async function handleAddTooltipV1(
  params: AddTooltipV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTooltipV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'tooltip', tree, ...params });
}
