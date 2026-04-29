import { assignIdsRecursively, buildCardRowV1, type CardRowV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCardRowV1Params extends CardRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal scroll row of cards (v1) — theme-aware variant of add_card_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Tree build delegated to `buildCardRowV1` in `@zseven-w/pen-core`.
 */
export async function handleAddCardRowV1(
  params: AddCardRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCardRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cr', tree, ...params });
}
