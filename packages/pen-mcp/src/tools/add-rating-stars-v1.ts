import {
  assignIdsRecursively,
  buildRatingStarsV1,
  type RatingStarsV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddRatingStarsV1Params extends RatingStarsV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Star rating row (v1) — theme-aware variant of add_rating_stars_v0.
 * No hardcoded colors in v0; all theme modes produce identical output.
 * Accepts theme param for API consistency across all v1 tools.
 */
export async function handleAddRatingStarsV1(
  params: AddRatingStarsV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildRatingStarsV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ratingStars', tree, ...params });
}
