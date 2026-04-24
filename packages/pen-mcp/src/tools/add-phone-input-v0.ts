import { assignIdsRecursively, buildPhoneInput, type PhoneInputParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddPhoneInputV0Params extends PhoneInputParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** International phone input with country-code prefix selector. Tree build delegated to `buildPhoneInput`. */
export async function handleAddPhoneInputV0(
  params: AddPhoneInputV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const r = buildPhoneInput(params);
  assignIdsRecursively(r);
  return insertElementTree({ binding: 'phoneInput', tree: r, ...params });
}
