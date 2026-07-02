import {
  assignIdsRecursively,
  buildActionMenuV1,
  type ActionMenuV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddActionMenuV1Params extends ActionMenuV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Action / context menu panel (v1) — theme-aware variant of add_action_menu_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Surface, border, icon, and label colors respond to theme.
 * Tree build delegated to `buildActionMenuV1` in `@zseven-w/pen-core`.
 */
export async function handleAddActionMenuV1(
  params: AddActionMenuV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildActionMenuV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'am', tree, ...params });
}
