import { assignIdsRecursively, buildPagination, type PaginationParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPaginationV0Params extends PaginationParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Pagination bar. Tree build delegated to `buildPagination`. */
export async function handleAddPaginationV0(
  params: AddPaginationV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const p = buildPagination(params);
  assignIdsRecursively(p);
  return insertElementTree({ binding: 'pagination', tree: p, ...params });
}
