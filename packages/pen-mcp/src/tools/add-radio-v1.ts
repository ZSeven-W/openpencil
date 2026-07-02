import { assignIdsRecursively, buildRadioV1, type RadioV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddRadioV1Params extends RadioV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Radio button + label (v1) — theme-aware variant of add_radio_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Accent stays brand-invariant; unselected ring → border token in dark/system.
 */
export async function handleAddRadioV1(
  params: AddRadioV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildRadioV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'radio', tree, ...params });
}
