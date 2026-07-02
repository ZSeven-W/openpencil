import {
  assignIdsRecursively,
  buildActivityLogV1,
  type ActivityLogV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddActivityLogV1Params extends ActivityLogV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Audit / activity log entry (v1) — theme-aware variant of add_activity_log_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Tree build delegated to `buildActivityLogV1` in `@zseven-w/pen-core`.
 */
export async function handleAddActivityLogV1(
  params: AddActivityLogV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildActivityLogV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'al', tree, ...params });
}
