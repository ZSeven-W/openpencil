import {
  assignIdsRecursively,
  buildDataTableRow,
  type DataTableRowParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDataTableRowV0Params extends DataTableRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { DataTableRowColumn as AddDataTableRowV0Column } from '@zseven-w/pen-core';

/**
 * Desktop / dashboard data-table row. Tree build delegated to
 * `@zseven-w/pen-core`'s `buildDataTableRow`. Distinct from
 * `add_list_row_v0` (mobile / iOS list cell) — use this for tabular
 * customer / order / report data with column-aligned cells.
 */
export async function handleAddDataTableRowV0(
  params: AddDataTableRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildDataTableRow(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'dataTableRow', tree: row, ...params });
}
