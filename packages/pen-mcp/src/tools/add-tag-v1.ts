import { assignIdsRecursively, buildTagV1, type TagV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddTagV1Params extends TagV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Tag (v1) — theme-aware variant of add_tag_v0.
 * Tone bg/fg pairs are status semantic colors (spec §3.4), kept hardcoded across all modes.
 * All theme modes produce identical trees.
 */
export async function handleAddTagV1(
  params: AddTagV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildTagV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'tag', tree, ...params });
}
