import {
  assignIdsRecursively,
  buildVideoPlaceholder,
  type VideoPlaceholderParams,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddVideoPlaceholderV0Params extends VideoPlaceholderParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddVideoPlaceholderV0(
  params: AddVideoPlaceholderV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const v = buildVideoPlaceholder(params);
  assignIdsRecursively(v);
  return insertElementTree({ binding: 'vid', tree: v, ...params });
}
