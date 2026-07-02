import {
  assignIdsRecursively,
  buildImagePlaceholder,
  type ImagePlaceholderParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddImagePlaceholderV0Params extends ImagePlaceholderParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Image placeholder — gray box with centered icon + optional label.
 * The "this will be an image later" affordance. Tree build delegated
 * to `buildImagePlaceholder`.
 */
export async function handleAddImagePlaceholderV0(
  params: AddImagePlaceholderV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const ph = buildImagePlaceholder(params);
  assignIdsRecursively(ph);
  return insertElementTree({ binding: 'img', tree: ph, ...params });
}
