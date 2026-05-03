import { assignIdsRecursively, buildBreadcrumb, type BreadcrumbParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddBreadcrumbV0Params extends BreadcrumbParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { BreadcrumbItem as AddBreadcrumbV0Item } from '@zseven-w/pen-core';

/** Breadcrumb trail. Tree build delegated to `buildBreadcrumb`. */
export async function handleAddBreadcrumbV0(
  params: AddBreadcrumbV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const bc = buildBreadcrumb(params);
  assignIdsRecursively(bc);
  return insertElementTree({ binding: 'breadcrumb', tree: bc, ...params });
}
