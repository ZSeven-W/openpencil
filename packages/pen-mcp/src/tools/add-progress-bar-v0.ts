import { assignIdsRecursively, buildProgressBar, type ProgressBarParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddProgressBarV0Params extends ProgressBarParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Linear progress bar. Tree build delegated to `buildProgressBar`. */
export async function handleAddProgressBarV0(
  params: AddProgressBarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const track = buildProgressBar(params);
  assignIdsRecursively(track);
  return insertElementTree({ binding: 'progress', tree: track, ...params });
}
