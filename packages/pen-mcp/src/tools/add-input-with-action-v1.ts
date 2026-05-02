import {
  assignIdsRecursively,
  buildInputWithActionV1,
  type InputWithActionV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInputWithActionV1Params extends InputWithActionV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Input field with inline action button (v1) — theme-aware variant of
 * add_input_with_action_v0. Supports 'light' (v0 byte-parity), 'dark',
 * and 'system' theme modes.
 * input bg → surface, stroke → border, text → textPrimary,
 * placeholder/icon → textMuted; button bg → accent (brand-invariant),
 * button text/icon → white in all modes.
 */
export async function handleAddInputWithActionV1(
  params: AddInputWithActionV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildInputWithActionV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'inputAction', tree, ...params });
}
