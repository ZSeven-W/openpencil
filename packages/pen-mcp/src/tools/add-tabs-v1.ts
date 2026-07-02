import { assignIdsRecursively, buildTabsV1, type TabsV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTabsV1Params extends TabsV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Tabs (v1) — theme-aware variant of add_tabs_v0.
 * #2563EB accent underline is a brand token (spec §3.4), kept hardcoded across all modes.
 * All theme modes produce identical trees.
 */
export async function handleAddTabsV1(
  params: AddTabsV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTabsV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'tabs', tree, ...params });
}
