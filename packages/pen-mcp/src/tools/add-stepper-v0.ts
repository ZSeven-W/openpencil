import { assignIdsRecursively, buildStepper, type StepperParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStepperV0Params extends StepperParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Horizontal numbered stepper. Tree build delegated to `buildStepper`. */
export async function handleAddStepperV0(
  params: AddStepperV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const stepper = buildStepper(params);
  assignIdsRecursively(stepper);
  return insertElementTree({ binding: 'stepper', tree: stepper, ...params });
}
