import { assignIdsRecursively, buildTextareaV1, type TextareaV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTextareaV1Params extends TextareaV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Textarea (v1) — theme-aware variant of add_textarea_v0.
 * No hardcoded surface colors in v0; all theme modes produce identical trees.
 */
export async function handleAddTextareaV1(
  params: AddTextareaV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTextareaV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'textarea', tree, ...params });
}
