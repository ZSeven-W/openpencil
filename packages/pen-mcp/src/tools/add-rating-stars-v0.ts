import { assignIdsRecursively, buildRatingStars, type RatingStarsParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddRatingStarsV0Params extends RatingStarsParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Star rating row. Tree build delegated to `buildRatingStars`. */
export async function handleAddRatingStarsV0(
  params: AddRatingStarsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildRatingStars(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'rating_stars', tree: row, ...params });
}
