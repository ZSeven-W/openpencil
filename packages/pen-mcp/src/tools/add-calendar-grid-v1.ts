import {
  assignIdsRecursively,
  buildCalendarGridV1,
  type CalendarGridV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCalendarGridV1Params extends CalendarGridV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Month calendar grid (v1) — theme-aware variant of add_calendar_grid_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Header text, day numbers, selected/today fills respond to theme.
 * Tree build delegated to `buildCalendarGridV1` in `@zseven-w/pen-core`.
 */
export async function handleAddCalendarGridV1(
  params: AddCalendarGridV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCalendarGridV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cg', tree, ...params });
}
