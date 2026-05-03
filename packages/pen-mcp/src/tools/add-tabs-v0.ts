import { assignIdsRecursively, buildTabs, type TabsParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTabsV0Params extends TabsParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { TabsItem as AddTabsV0Item } from '@zseven-w/pen-core';

/**
 * Horizontal top tabs with underline on active. Tree build delegated
 * to `@zseven-w/pen-core`'s `buildTabs` — fill_container tabs +
 * sibling rectangle underline (not directional stroke).
 */
export async function handleAddTabsV0(
  params: AddTabsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const bar = buildTabs(params);
  assignIdsRecursively(bar);
  return insertElementTree({ binding: 'tabs', tree: bar, ...params });
}
