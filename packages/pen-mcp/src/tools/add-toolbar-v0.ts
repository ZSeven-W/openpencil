import { assignIdsRecursively, buildToolbar, type ToolbarParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddToolbarV0Params extends ToolbarParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddToolbarV0(
  params: AddToolbarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const toolbar = buildToolbar(params);
  assignIdsRecursively(toolbar);
  return insertElementTree({ binding: 'toolbar', tree: toolbar, ...params });
}
