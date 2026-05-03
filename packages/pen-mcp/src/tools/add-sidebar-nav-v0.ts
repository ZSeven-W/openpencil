import { assignIdsRecursively, buildSidebarNav, type SidebarNavParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSidebarNavV0Params extends SidebarNavParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { SidebarNavItem as AddSidebarNavV0Item } from '@zseven-w/pen-core';

/**
 * Persistent vertical sidebar navigation. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildSidebarNav`.
 *
 * Distinct from `add_bottom_nav_v0` (mobile, horizontal, label-below-
 * icon) and `add_top_nav_bar_v0` (mobile, single-row header). Use this
 * for desktop dashboards / settings / docs / admin rails.
 */
export async function handleAddSidebarNavV0(
  params: AddSidebarNavV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const nav = buildSidebarNav(params);
  assignIdsRecursively(nav);
  return insertElementTree({ binding: 'sidebarNav', tree: nav, ...params });
}
