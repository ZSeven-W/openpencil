import { assignIdsRecursively, buildTopNavBarV1, type TopNavBarV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTopNavBarV1Params extends TopNavBarV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Mobile top navigation bar (v1) — theme-aware variant of add_top_nav_bar_v0.
 * No hardcoded surface colors in v0; all theme modes produce identical trees.
 */
export async function handleAddTopNavBarV1(
  params: AddTopNavBarV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTopNavBarV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'top-nav-bar', tree, ...params });
}
