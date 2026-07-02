import {
  assignIdsRecursively,
  buildDataTableRowV1,
  type DataTableRowV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDataTableRowV1Params extends DataTableRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Data-table row (v1) — theme-aware variant of add_data_table_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Header text → textMuted, body text → textPrimary, selected row → bgDeep.
 */
export async function handleAddDataTableRowV1(
  params: AddDataTableRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildDataTableRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'dtr', tree, ...params });
}
