import { assignIdsRecursively, buildDividerV1, type DividerV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDividerV1Params extends DividerV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Hairline divider (v1) — theme-aware variant of add_divider_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Since buildDivider emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildDividerV1` in `@zseven-w/pen-core`.
 */
export async function handleAddDividerV1(
  params: AddDividerV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildDividerV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'dv', tree, ...params });
}
