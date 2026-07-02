import {
  assignIdsRecursively,
  buildModalShellV1,
  type ModalShellV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddModalShellV1Params extends ModalShellV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Modal shell (v1) — theme-aware variant of add_modal_shell_v0.
 * Tree build delegated to `buildModalShellV1`. See the builder's
 * JSDoc for the theme-variant contract (light / dark / system).
 */
export async function handleAddModalShellV1(
  params: AddModalShellV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const m = buildModalShellV1(params);
  assignIdsRecursively(m);
  return insertElementTree({ binding: 'modal', tree: m, ...params });
}
