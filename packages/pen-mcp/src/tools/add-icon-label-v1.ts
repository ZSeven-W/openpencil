import { assignIdsRecursively, buildIconLabelV1, type IconLabelV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddIconLabelV1Params extends IconLabelV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Atomic icon + label pair (v1) — theme-aware variant of add_icon_label_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Since buildIconLabel emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildIconLabelV1` in `@zseven-w/pen-core`.
 */
export async function handleAddIconLabelV1(
  params: AddIconLabelV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildIconLabelV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'il', tree, ...params });
}
