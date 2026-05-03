import {
  assignIdsRecursively,
  buildNotificationRow,
  type NotificationRowParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddNotificationRowV0Params extends NotificationRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddNotificationRowV0(
  params: AddNotificationRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const n = buildNotificationRow(params);
  assignIdsRecursively(n);
  return insertElementTree({ binding: 'notif', tree: n, ...params });
}
