import {
  assignIdsRecursively,
  buildInlineActionV1,
  type InlineActionV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddInlineActionV1Params extends InlineActionV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Inline status + action row (v1) — theme-aware variant of add_inline_action_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * icon/message → textBody, CTA → accent in dark/system modes.
 */
export async function handleAddInlineActionV1(
  params: AddInlineActionV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildInlineActionV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'inlineAction', tree, ...params });
}
