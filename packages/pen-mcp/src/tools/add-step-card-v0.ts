import { assignIdsRecursively, buildStepCard, type StepCardParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStepCardV0Params extends StepCardParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddStepCardV0(
  params: AddStepCardV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildStepCard(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'stepCard', tree, ...params });
}
