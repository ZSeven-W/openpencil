import { assignIdsRecursively, buildComment, type CommentParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCommentV0Params extends CommentParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Single-comment unit: avatar + author/timestamp header + body.
 * Tree build delegated to `buildComment`. Compose multiple comments
 * by calling the tool N times inside a vertical parent.
 */
export async function handleAddCommentV0(
  params: AddCommentV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const c = buildComment(params);
  assignIdsRecursively(c);
  return insertElementTree({ binding: 'cm', tree: c, ...params });
}
