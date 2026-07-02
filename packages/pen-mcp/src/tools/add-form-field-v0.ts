import { assignIdsRecursively, buildFormField, type FormFieldParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFormFieldV0Params extends FormFieldParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Form field (label + input). Tree build delegated to `buildFormField`. */
export async function handleAddFormFieldV0(
  params: AddFormFieldV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const field = buildFormField(params);
  assignIdsRecursively(field);
  return insertElementTree({ binding: 'field', tree: field, ...params });
}
