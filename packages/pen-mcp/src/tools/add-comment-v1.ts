import { assignIdsRecursively, buildCommentV1, type CommentV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCommentV1Params extends CommentV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Comment row (v1) — theme-aware variant of add_comment_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Avatar bg → surface2, initial text → textBody, timestamp → textMuted.
 * Tree build delegated to `buildCommentV1` in `@zseven-w/pen-core`.
 */
export async function handleAddCommentV1(
  params: AddCommentV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCommentV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cmt', tree, ...params });
}
