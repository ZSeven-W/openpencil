import {
  assignIdsRecursively,
  buildDrawerShellV1,
  type DrawerShellV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDrawerShellV1Params extends DrawerShellV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Drawer shell (v1) — theme-aware variant of add_drawer_shell_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Drawer bg → surface, header border → border, title → textPrimary,
 * close icon → textMuted.
 */
export async function handleAddDrawerShellV1(
  params: AddDrawerShellV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildDrawerShellV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ds', tree, ...params });
}
