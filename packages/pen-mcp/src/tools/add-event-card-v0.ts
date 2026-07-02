import { assignIdsRecursively, buildEventCard, type EventCardParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddEventCardV0Params extends EventCardParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddEventCardV0(
  params: AddEventCardV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildEventCard(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'eventCard', tree, ...params });
}
