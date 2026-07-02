import {
  assignIdsRecursively,
  buildPaginationV1,
  type PaginationV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPaginationV1Params extends PaginationV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Pagination bar (v1) — theme-aware variant of add_pagination_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * arrow/inactive → textBody, ellipsis → textMuted in dark/system;
 * active pill bg stays brand-invariant (accent_color), active text stays white.
 */
export async function handleAddPaginationV1(
  params: AddPaginationV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildPaginationV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'pagination', tree, ...params });
}
