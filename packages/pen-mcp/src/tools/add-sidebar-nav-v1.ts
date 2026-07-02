import {
  assignIdsRecursively,
  buildSidebarNavV1,
  type SidebarNavV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSidebarNavV1Params extends SidebarNavV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { SidebarNavV1Item as AddSidebarNavV1Item } from '@zseven-w/pen-core';

/**
 * Persistent vertical sidebar navigation (v1) — theme-aware variant of add_sidebar_nav_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 */
export async function handleAddSidebarNavV1(
  params: AddSidebarNavV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSidebarNavV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'sidebarNav', tree, ...params });
}
