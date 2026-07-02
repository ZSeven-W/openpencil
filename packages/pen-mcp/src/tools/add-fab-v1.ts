import { assignIdsRecursively, buildFabV1, type FabV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFabV1Params extends FabV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Floating action button (v1) — theme-aware variant of add_fab_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * FAB bg → accent (brand-invariant: same visual intent in all modes).
 * Icon is always white (white on accent — brand decision).
 */
export async function handleAddFabV1(
  params: AddFabV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildFabV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'fab', tree, ...params });
}
