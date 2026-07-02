import {
  assignIdsRecursively,
  buildNotificationRowV1,
  type NotificationRowV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddNotificationRowV1Params extends NotificationRowV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Notification list row (v1) — theme-aware variant of add_notification_row_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * timestamp → textSubtle, body → textBody, unread dot → destructive in dark/system.
 */
export async function handleAddNotificationRowV1(
  params: AddNotificationRowV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildNotificationRowV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'notificationRow', tree, ...params });
}
