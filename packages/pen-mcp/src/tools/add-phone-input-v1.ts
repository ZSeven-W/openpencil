import {
  assignIdsRecursively,
  buildPhoneInputV1,
  type PhoneInputV1Params,
} from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPhoneInputV1Params extends PhoneInputV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Phone number input with country-code selector (v1) — theme-aware variant of add_phone_input_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * field bg → surface, stroke/divider → border, label → textBody,
 * code → textPrimary, chevron → textMuted in dark/system.
 */
export async function handleAddPhoneInputV1(
  params: AddPhoneInputV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildPhoneInputV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'phoneInput', tree, ...params });
}
