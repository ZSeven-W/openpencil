import { assignIdsRecursively, buildCheckbox, type CheckboxParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCheckboxV0Params extends CheckboxParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Checkbox + label. Tree build delegated to `@zseven-w/pen-core`'s
 * `buildCheckbox`.
 */
export async function handleAddCheckboxV0(
  params: AddCheckboxV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildCheckbox(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'checkbox', tree: row, ...params });
}
