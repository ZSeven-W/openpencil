import { assignIdsRecursively, buildDatePicker, type DatePickerParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDatePickerV0Params extends DatePickerParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Date picker closed state. Tree build delegated to `buildDatePicker`. */
export async function handleAddDatePickerV0(
  params: AddDatePickerV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const d = buildDatePicker(params);
  assignIdsRecursively(d);
  return insertElementTree({ binding: 'datePicker', tree: d, ...params });
}
