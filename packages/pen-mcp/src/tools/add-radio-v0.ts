import { assignIdsRecursively, buildRadio, type RadioParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddRadioV0Params extends RadioParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Radio button + label. Tree build delegated to `@zseven-w/pen-core`'s
 * `buildRadio`.
 */
export async function handleAddRadioV0(
  params: AddRadioV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const row = buildRadio(params);
  assignIdsRecursively(row);
  return insertElementTree({ binding: 'radio', tree: row, ...params });
}
