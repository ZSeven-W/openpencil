import { assignIdsRecursively, buildCombobox, type ComboboxParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddComboboxV0Params extends ComboboxParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddComboboxV0(
  params: AddComboboxV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const combobox = buildCombobox(params);
  assignIdsRecursively(combobox);
  return insertElementTree({ binding: 'combobox', tree: combobox, ...params });
}
