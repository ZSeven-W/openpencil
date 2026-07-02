import {
  assignIdsRecursively,
  buildVideoPlaceholderV1,
  type VideoPlaceholderV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddVideoPlaceholderV1Params extends VideoPlaceholderV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Video placeholder (v1) — theme-aware variant of add_video_placeholder_v0.
 * Dark bg (#334155), play icon (#FFFFFF), and caption (#FFFFFFB3) are builder-private
 * constants (spec §3.4) — intentionally dark surface in all modes.
 * All theme modes produce identical trees.
 */
export async function handleAddVideoPlaceholderV1(
  params: AddVideoPlaceholderV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildVideoPlaceholderV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'video-placeholder', tree, ...params });
}
