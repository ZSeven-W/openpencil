import { assignIdsRecursively, buildStatCard, type StatCardParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStatCardV0Params extends StatCardParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Big-number stat card. Tree build delegated to `buildStatCard`. */
export async function handleAddStatCardV0(
  params: AddStatCardV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const s = buildStatCard(params);
  assignIdsRecursively(s);
  return insertElementTree({ binding: 'statCard', tree: s, ...params });
}
