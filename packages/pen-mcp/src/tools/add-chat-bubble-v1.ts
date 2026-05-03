import {
  assignIdsRecursively,
  buildChatBubbleV1,
  type ChatBubbleV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChatBubbleV1Params extends ChatBubbleV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Chat bubble (v1) — theme-aware variant of add_chat_bubble_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Left-side: surface2 bg, textPrimary text. Right-side: accent bg, white text.
 * Tree build delegated to `buildChatBubbleV1` in `@zseven-w/pen-core`.
 */
export async function handleAddChatBubbleV1(
  params: AddChatBubbleV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildChatBubbleV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cb', tree, ...params });
}
