import { assignIdsRecursively, buildKbdV1, type KbdV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddKbdV1Params extends KbdV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Keyboard shortcut (v1) — theme-aware variant of add_kbd_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * key bg → surface2, stroke → border in dark/system modes.
 */
export async function handleAddKbdV1(
  params: AddKbdV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildKbdV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'kbd', tree, ...params });
}
