import { assignIdsRecursively, buildSelect, type SelectParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSelectV0Params extends SelectParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Dropdown select (display/closed state). Same label-above-input
 * shape as form-field, with trailing chevron-down. Tree build
 * delegated to `buildSelect`.
 */
export async function handleAddSelectV0(
  params: AddSelectV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const sel = buildSelect(params);
  assignIdsRecursively(sel);
  return insertElementTree({ binding: 'sel', tree: sel, ...params });
}
