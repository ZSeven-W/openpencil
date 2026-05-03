import {
  assignIdsRecursively,
  buildActivityRingV1,
  type ActivityRingV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddActivityRingV1Params extends ActivityRingV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Apple-style progress ring with centered text (v1) — theme-aware variant of
 * add_activity_ring_v0. Supports 'light' (v0 byte-parity), 'dark', and 'system'
 * theme modes. The #000000 ring stroke is a colorless placeholder; all three
 * modes produce identical output.
 * Tree build delegated to `buildActivityRingV1` in `@zseven-w/pen-core`.
 */
export async function handleAddActivityRingV1(
  params: AddActivityRingV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildActivityRingV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ar', tree, ...params });
}
