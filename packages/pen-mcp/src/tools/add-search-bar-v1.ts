import { assignIdsRecursively, buildSearchBarV1, type SearchBarV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSearchBarV1Params extends SearchBarV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Search bar (v1) — theme-aware variant of add_search_bar_v0.
 * Supports 'light' (v0 byte-parity, no fill), 'dark', and 'system' theme modes.
 * Dark/system adds surface2 fill so the bar is visible against the dark page bg.
 */
export async function handleAddSearchBarV1(
  params: AddSearchBarV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSearchBarV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'searchBar', tree, ...params });
}
