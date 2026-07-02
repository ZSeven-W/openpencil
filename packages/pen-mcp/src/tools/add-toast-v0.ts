import { assignIdsRecursively, buildToast, type ToastParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddToastV0Params extends ToastParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Floating pill notification. Tree build delegated to `buildToast`. */
export async function handleAddToastV0(
  params: AddToastV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const toast = buildToast(params);
  assignIdsRecursively(toast);
  return insertElementTree({ binding: 'toast', tree: toast, ...params });
}
