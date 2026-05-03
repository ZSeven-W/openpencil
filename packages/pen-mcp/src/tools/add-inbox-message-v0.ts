import {
  assignIdsRecursively,
  buildInboxMessage,
  type InboxMessageParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInboxMessageV0Params extends InboxMessageParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddInboxMessageV0(
  params: AddInboxMessageV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const msg = buildInboxMessage(params);
  assignIdsRecursively(msg);
  return insertElementTree({ binding: 'inboxMessage', tree: msg, ...params });
}
