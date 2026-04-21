import { assignIdsRecursively, buildNavChipRow, type NavChipRowParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddNavChipRowV0Params extends NavChipRowParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export type { NavChipRowItem as AddNavChipRowV0Item } from '@zseven-w/pen-core';

/** Horizontal scroll row of nav chips. Tree build delegated to `buildNavChipRow`. */
export async function handleAddNavChipRowV0(
  params: AddNavChipRowV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const wrapper = buildNavChipRow(params);
  assignIdsRecursively(wrapper);
  return insertElementTree({ binding: 'row', tree: wrapper, ...params });
}
