import { assignIdsRecursively, buildOtpInput, type OtpInputParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddOtpInputV0Params extends OtpInputParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** OTP / PIN code input. Tree build delegated to `buildOtpInput`. */
export async function handleAddOtpInputV0(
  params: AddOtpInputV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const o = buildOtpInput(params);
  assignIdsRecursively(o);
  return insertElementTree({ binding: 'otp', tree: o, ...params });
}
