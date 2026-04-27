import { assignIdsRecursively, buildTag, type TagParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTagV0Params extends TagParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Single closable tag (filter / selection chip). Tree build delegated
 * to `@zseven-w/pen-core`'s `buildTag`. Distinct from add_badge_v0
 * (read-only label, no × icon) and add_chip_input_v0 (multi-tag
 * input field with inline caret).
 */
export async function handleAddTagV0(
  params: AddTagV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tag = buildTag(params);
  assignIdsRecursively(tag);
  return insertElementTree({ binding: 'tag', tree: tag, ...params });
}
