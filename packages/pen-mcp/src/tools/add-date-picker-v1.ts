import {
  assignIdsRecursively,
  buildDatePickerV1,
  type DatePickerV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDatePickerV1Params extends DatePickerV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Date picker (v1) — theme-aware variant of add_date_picker_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Input bg → surface, stroke → border, value → textPrimary,
 * placeholder/clear → textSubtle, calendar icon → textMuted.
 */
export async function handleAddDatePickerV1(
  params: AddDatePickerV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildDatePickerV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'dp', tree, ...params });
}
