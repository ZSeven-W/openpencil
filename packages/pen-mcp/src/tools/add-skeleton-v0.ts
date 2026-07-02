import { assignIdsRecursively, buildSkeleton, type SkeletonParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddSkeletonV0Params extends SkeletonParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Loading skeleton: stacked gray rectangles mimicking future text
 * lines while content fetches. Tree build delegated to `buildSkeleton`.
 */
export async function handleAddSkeletonV0(
  params: AddSkeletonV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const skeleton = buildSkeleton(params);
  assignIdsRecursively(skeleton);
  return insertElementTree({ binding: 's', tree: skeleton, ...params });
}
