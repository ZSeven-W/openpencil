import {
  assignIdsRecursively,
  buildCarouselDotsV1,
  type CarouselDotsV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddCarouselDotsV1Params extends CarouselDotsV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Carousel pagination dots (v1) — theme-aware variant of add_carousel_dots_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Active dot uses textPrimary; inactive uses border token in dark/system modes.
 * Tree build delegated to `buildCarouselDotsV1` in `@zseven-w/pen-core`.
 */
export async function handleAddCarouselDotsV1(
  params: AddCarouselDotsV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildCarouselDotsV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'cd', tree, ...params });
}
