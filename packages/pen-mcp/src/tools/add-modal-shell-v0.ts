import { assignIdsRecursively, buildModalShell, type ModalShellParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddModalShellV0Params extends ModalShellParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Modal dialog shell: dimmed backdrop + centered card with title +
 * optional subtitle. Body content goes into the `modal-shell-card`
 * role via a follow-up call. Tree build delegated to `buildModalShell`.
 */
export async function handleAddModalShellV0(
  params: AddModalShellV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const m = buildModalShell(params);
  assignIdsRecursively(m);
  return insertElementTree({ binding: 'modal', tree: m, ...params });
}
