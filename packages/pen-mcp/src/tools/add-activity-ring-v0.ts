import {
  assignIdsRecursively,
  buildActivityRing,
  type ActivityRingParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddActivityRingV0Params extends ActivityRingParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Apple-style activity ring with centered text. Tree build delegated
 * to `@zseven-w/pen-core`'s `buildActivityRing` — frame+cornerRadius
 * pattern, NEVER ellipse+sibling text (layout.md §RING).
 */
export async function handleAddActivityRingV0(
  params: AddActivityRingV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const ring = buildActivityRing(params);
  assignIdsRecursively(ring);
  return insertElementTree({ binding: 'ring', tree: ring, ...params });
}
