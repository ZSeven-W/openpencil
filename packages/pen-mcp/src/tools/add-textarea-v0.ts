import { assignIdsRecursively, buildTextarea, type TextareaParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTextareaV0Params extends TextareaParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Multi-line text input. Same vertical label-above-input shape as
 * add_form_field_v0 but the input area grows by `rows`. Tree build
 * delegated to `buildTextarea`.
 */
export async function handleAddTextareaV0(
  params: AddTextareaV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const textarea = buildTextarea(params);
  assignIdsRecursively(textarea);
  return insertElementTree({ binding: 'ta', tree: textarea, ...params });
}
