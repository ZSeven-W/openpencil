import { assignIdsRecursively, buildStepCardV1, type StepCardV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddStepCardV1Params extends StepCardV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Onboarding step card (v1) — theme-aware variant of add_step_card_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Accent (#2563EB) and check-icon white (#FFFFFF) stay hardcoded.
 */
export async function handleAddStepCardV1(
  params: AddStepCardV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildStepCardV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'stepCard', tree, ...params });
}
