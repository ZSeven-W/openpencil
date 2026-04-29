import { assignIdsRecursively, buildComboboxV1, type ComboboxV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddComboboxV1Params extends ComboboxV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Combobox / autocomplete (v1) — theme-aware variant of add_combobox_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Input/dropdown bg → surface, option hover → surface2, stroke → border/accent.
 * Tree build delegated to `buildComboboxV1` in `@zseven-w/pen-core`.
 */
export async function handleAddComboboxV1(
  params: AddComboboxV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildComboboxV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cmb', tree, ...params });
}
