import {
  assignIdsRecursively,
  buildInboxMessageV1,
  type InboxMessageV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInboxMessageV1Params extends InboxMessageV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Inbox / email list row (v1) — theme-aware variant of add_inbox_message_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * primary → textPrimary, timestamp → textSubtle, preview → textMuted,
 * unread dot → accent in dark/system modes.
 */
export async function handleAddInboxMessageV1(
  params: AddInboxMessageV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildInboxMessageV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'inboxMsg', tree, ...params });
}
