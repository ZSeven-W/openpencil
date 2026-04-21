import { assignIdsRecursively, buildKbd, type KbdParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddKbdV0Params extends KbdParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Keyboard shortcut badge. Tree build delegated to `buildKbd`. */
export async function handleAddKbdV0(
  params: AddKbdV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const kbd = buildKbd(params);
  assignIdsRecursively(kbd);
  return insertElementTree({ binding: 'kbd', tree: kbd, ...params });
}
