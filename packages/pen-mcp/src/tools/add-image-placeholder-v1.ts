import {
  assignIdsRecursively,
  buildImagePlaceholderV1,
  type ImagePlaceholderV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddImagePlaceholderV1Params extends ImagePlaceholderV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Image placeholder (v1) — theme-aware variant of add_image_placeholder_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * bg → bgDeep, icon → textMuted, label → textMuted in dark/system.
 */
export async function handleAddImagePlaceholderV1(
  params: AddImagePlaceholderV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildImagePlaceholderV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'imgPlaceholder', tree, ...params });
}
