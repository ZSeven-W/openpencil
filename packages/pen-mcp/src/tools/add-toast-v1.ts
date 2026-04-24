import { assignIdsRecursively, buildToastV1, type ToastV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddToastV1Params extends ToastV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Toast (v1) — theme-aware variant of add_toast_v0. Tree build
 * delegated to `buildToastV1`. See the builder's JSDoc for the
 * theme-variant contract (light / dark / system) and the
 * intentional-inverted-contrast design rationale.
 */
export async function handleAddToastV1(
  params: AddToastV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const t = buildToastV1(params);
  assignIdsRecursively(t);
  return insertElementTree({ binding: 'toast', tree: t, ...params });
}
