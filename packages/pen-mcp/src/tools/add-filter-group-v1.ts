import {
  assignIdsRecursively,
  buildFilterGroupV1,
  type FilterGroupV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddFilterGroupV1Params extends FilterGroupV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Filter group / facet (v1) — theme-aware variant of add_filter_group_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Title → textPrimary, label → textBody, count → textSubtle,
 * unselected box → surface + border, selected box → accent,
 * check icon always white (white on accent — brand decision).
 */
export async function handleAddFilterGroupV1(
  params: AddFilterGroupV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildFilterGroupV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'fg', tree, ...params });
}
