import { assignIdsRecursively, buildOtpInputV1, type OtpInputV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddOtpInputV1Params extends OtpInputV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * OTP / PIN code input (v1) — theme-aware variant of add_otp_input_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * slot bg → surface, borders → border/borderStrong, digit → textPrimary in dark/system.
 */
export async function handleAddOtpInputV1(
  params: AddOtpInputV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildOtpInputV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'otpInput', tree, ...params });
}
