import { assignIdsRecursively, buildTimeline, type TimelineParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTimelineV0Params extends TimelineParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { TimelineItem as TimelineItemV0 } from '@zseven-w/pen-core';

/** Vertical timeline. Tree build delegated to `buildTimeline`. */
export async function handleAddTimelineV0(
  params: AddTimelineV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTimeline(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'timeline', tree, ...params });
}
