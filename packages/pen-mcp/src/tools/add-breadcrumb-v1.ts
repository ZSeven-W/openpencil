import {
  assignIdsRecursively,
  buildBreadcrumbV1,
  type BreadcrumbV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBreadcrumbV1Params extends BreadcrumbV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Breadcrumb trail (v1) — theme-aware variant of add_breadcrumb_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Since buildBreadcrumb emits no color fills, all three modes produce identical output.
 * Tree build delegated to `buildBreadcrumbV1` in `@zseven-w/pen-core`.
 */
export async function handleAddBreadcrumbV1(
  params: AddBreadcrumbV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildBreadcrumbV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'bc', tree, ...params });
}
