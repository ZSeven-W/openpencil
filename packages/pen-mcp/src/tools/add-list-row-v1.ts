import { assignIdsRecursively, buildListRowV1, type ListRowV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddListRowV1Params extends ListRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * iOS / Material-style list row (v1) — theme-aware variant of
 * add_list_row_v0. No hardcoded colors in v0, so light/dark/system
 * modes are identical (byte-parity with v0 in all modes). Accepts
 * theme param for API consistency across all v1 tools.
 */
export async function handleAddListRowV1(
  params: AddListRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildListRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'row', tree, ...params });
}
