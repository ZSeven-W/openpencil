import {
  assignIdsRecursively,
  buildCalendarGrid,
  type CalendarGridParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCalendarGridV0Params extends CalendarGridParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Month calendar grid. Tree build delegated to `buildCalendarGrid`. */
export async function handleAddCalendarGridV0(
  params: AddCalendarGridV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCalendarGrid(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'calendar', tree, ...params });
}
