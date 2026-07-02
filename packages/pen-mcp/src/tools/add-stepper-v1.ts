import { assignIdsRecursively, buildStepperV1, type StepperV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStepperV1Params extends StepperV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal numbered stepper (v1) — theme-aware variant of add_stepper_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Accent (#2563EB) and done-state white (#FFFFFF) stay hardcoded.
 */
export async function handleAddStepperV1(
  params: AddStepperV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildStepperV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'stepper', tree, ...params });
}
