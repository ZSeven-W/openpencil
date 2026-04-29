import { assignIdsRecursively, buildTimelineV1, type TimelineV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTimelineV1Params extends TimelineV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Timeline (v1) — theme-aware variant of add_timeline_v0.
 * Active dot (#2563EB accent) is hardcoded per spec §3.4.
 * Inactive dot, connector (border token), and subtitle (textMuted) are tokenized.
 */
export async function handleAddTimelineV1(
  params: AddTimelineV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTimelineV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'timeline', tree, ...params });
}
