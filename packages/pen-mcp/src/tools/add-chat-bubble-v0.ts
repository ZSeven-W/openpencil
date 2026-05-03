import { assignIdsRecursively, buildChatBubble, type ChatBubbleParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChatBubbleV0Params extends ChatBubbleParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Chat message bubble. Tree build delegated to `buildChatBubble`. */
export async function handleAddChatBubbleV0(
  params: AddChatBubbleV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const b = buildChatBubble(params);
  assignIdsRecursively(b);
  return insertElementTree({ binding: 'bubble', tree: b, ...params });
}
