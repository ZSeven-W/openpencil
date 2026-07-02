import { assignIdsRecursively, buildCheckboxV1, type CheckboxV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCheckboxV1Params extends CheckboxV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Checkbox + label (v1) — theme-aware variant of add_checkbox_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Checked: accent fill + white icon. Unchecked: transparent + border stroke.
 * Tree build delegated to `buildCheckboxV1` in `@zseven-w/pen-core`.
 */
export async function handleAddCheckboxV1(
  params: AddCheckboxV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCheckboxV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ch', tree, ...params });
}
