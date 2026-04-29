import { assignIdsRecursively, buildSelectV1, type SelectV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSelectV1Params extends SelectV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Dropdown select — closed state (v1). Theme-aware variant of add_select_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Placeholder color: textSubtle in dark/system.
 */
export async function handleAddSelectV1(
  params: AddSelectV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildSelectV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'select', tree, ...params });
}
