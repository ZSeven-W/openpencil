import {
  assignIdsRecursively,
  buildTextButtonV1,
  type TextButtonV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTextButtonV1Params extends TextButtonV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Text button (v1) — theme-aware variant of add_text_button_v0.
 * No hardcoded surface colors in v0; all theme modes produce identical trees.
 */
export async function handleAddTextButtonV1(
  params: AddTextButtonV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTextButtonV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'text-button', tree, ...params });
}
