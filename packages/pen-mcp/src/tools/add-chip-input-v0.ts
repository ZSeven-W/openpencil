import { assignIdsRecursively, buildChipInput, type ChipInputParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddChipInputV0Params extends ChipInputParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/** Chip / tag input. Tree build delegated to `buildChipInput`. */
export async function handleAddChipInputV0(
  params: AddChipInputV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const c = buildChipInput(params);
  assignIdsRecursively(c);
  return insertElementTree({ binding: 'chipInput', tree: c, ...params });
}
