import {
  assignIdsRecursively,
  buildCarouselDots,
  type CarouselDotsParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCarouselDotsV0Params extends CarouselDotsParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Carousel pagination dots. Tree build delegated to `buildCarouselDots`. */
export async function handleAddCarouselDotsV0(
  params: AddCarouselDotsV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildCarouselDots(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'carousel_dots', tree: row, ...params });
}
